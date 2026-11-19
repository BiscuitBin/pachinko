//! The server: one structured task scope over the queue's owner (decision 0019).
//!
//! The scope is one [`JoinSet`] holding four tasks: the queue's owner, its ticker,
//! the HTTP listener and the shutdown signal. The first to end, for whatever
//! reason, starts the shutdown; all of them are joined before `serve` returns, and
//! the first error is the one returned. No other task is spawned here; axum spawns one
//! task per connection outside the scope, which the scope neither ends nor joins.
//!
//! Shutdown runs in this order. Readiness goes `Stopping`, so `GET /readyz` answers
//! 503 at once; the listener stops accepting and in-flight requests finish. If every
//! task ends within `--shutdown-grace-ms` (the owner ends when the listener has
//! dropped the last handle), that is the end. Otherwise the cut comes: the listener
//! stops waiting for the connections still open and the owner is told to stop, and
//! the tasks are joined again for at most one more grace period; only then is any of
//! the scope's tasks still running aborted, at its next yield point. The owner yields
//! between commands, so `serve` returns within two grace periods plus the one command
//! or tick the owner was applying, and releasing the queue's state when the owner
//! ends. A zero grace period cuts at once and joins the tasks for at most one second
//! before aborting what is left.
//!
//! The cut does not close a connection. The task axum serves one on lasts until the
//! connection ends or the runtime is dropped, which for the binary is its exit, right
//! after `serve` returns. Until then a request that completes on it is still
//! answered, and a command it sends finds the owner stopped: 503 `not_ready`.
//! Nothing bounds a connection that sends nothing or trickles its head either; T38
//! owns both gaps.
//!
//! Every task in the scope must end once the shutdown begins, on the lifecycle's
//! `shutdown` watch or on the owner ending, without waiting for the cut: the cut is
//! sent only after the grace period, so a task that waits for it turns a clean
//! shutdown into an overrun. The cut is the backstop for a task that overran.

use std::io::{self, Write};
use std::time::Duration;

use libpachinko::Queue;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};
use tokio::task::{JoinError, JoinSet};
use tokio::time::timeout;

use crate::actor::{self, QueueHandle, Readiness, ServiceQueue, Tick};
use crate::http::{Limits, router};
use crate::{EpochSerialLeaseIds, ServeConfig, ServeError, SystemClock};

/// What each task of the scope ends with.
type TaskResult = Result<(), ServeError>;

/// The queue's handle and its owner, still to be spawned.
type Owner<F> = (QueueHandle, F);

/// What the scope shares with the owner, its bounds and the router's request-shape
/// limits (T27): the readiness the owner sets to `Serving` and the scope to
/// `Stopping`; the shutdown, which turns true when the shutdown begins and which
/// every task beside the owner ends on; the cut, which ends the listener's wait for
/// the open connections and tells the owner to stop; and the tick lane's sender,
/// which the ticker holds. The lane's receiver is not kept here: [`Lifecycle::new`]
/// hands it back, and [`Lifecycle::start`] takes it by value for the owner, so one
/// receiver serves one start.
pub(crate) struct Lifecycle {
    readiness: watch::Sender<Readiness>,
    shutdown: watch::Sender<bool>,
    cut: watch::Sender<bool>,
    bounds: Bounds,
    limits: Limits,
    lane: mpsc::Sender<Tick>,
}

impl Lifecycle {
    /// A lifecycle `Starting`, not shutting down, not cut, with the bounds and the
    /// limits `config` sets, and the receiving end of its tick lane, for
    /// [`Lifecycle::start`].
    pub(crate) fn new(config: &ServeConfig) -> (Self, mpsc::Receiver<Tick>) {
        let (lane, ticks) = actor::tick_lane();
        let lifecycle = Self {
            readiness: watch::channel(Readiness::Starting).0,
            shutdown: watch::channel(false).0,
            cut: watch::channel(false).0,
            bounds: Bounds::of(config),
            limits: Limits::from(config),
            lane,
        };
        (lifecycle, ticks)
    }

    /// The owner of `queue`, reporting to this lifecycle, stopping on its cut and
    /// taking its ticks from `ticks`, the receiver [`Lifecycle::new`] handed back.
    pub(crate) fn start(
        &self,
        queue: ServiceQueue,
        ticks: mpsc::Receiver<Tick>,
    ) -> Owner<impl Future<Output = TaskResult> + Send + 'static> {
        actor::start(queue, self.readiness.clone(), self.cut.subscribe(), ticks)
    }
}

/// The scope's bounds, from the flags.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    /// `--shutdown-grace-ms`: each of shutdown's two waits.
    grace: Duration,
    /// `--request-timeout-ms`: each handler.
    request: Duration,
    /// `--sweep-ms`: the ticker's sleep between one tick's answer and the next.
    sweep: Duration,
}

impl Bounds {
    /// The bounds `config` sets.
    const fn of(config: &ServeConfig) -> Self {
        Self {
            grace: config.shutdown_grace(),
            request: config.request_timeout(),
            sweep: config.sweep(),
        }
    }
}

/// The `serve` command: installs the log writer on standard error at
/// `--log-level`, builds the runtime and the queue, binds `--listen`, installs the
/// signal handlers, writes `listening on <address>` to `out`, then serves until a
/// signal stops it. The writer's level comes from the flag alone: no environment
/// variable, `RUST_LOG` included, is read.
pub(crate) fn run(config: &ServeConfig, out: &mut impl Write) -> Result<(), ServeError> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_max_level(config.log_level())
        .try_init()
        .ok();
    let queue = service_queue(config)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(ServeError::io("building the runtime"))?;
    runtime.block_on(async {
        let listener = TcpListener::bind(config.listen())
            .await
            .map_err(ServeError::io("binding the listener"))?;
        let signals = install_signals()?;
        announce(&listener, out)?;
        let (_never_sent, stop) = watch::channel(());
        let (lifecycle, ticks) = Lifecycle::new(config);
        let owner = lifecycle.start(queue, ticks);
        scope(listener, owner, lifecycle, stop, signals).await
    })
}

/// Serves the HTTP API on `listener` over a fresh, empty queue until SIGTERM or
/// SIGINT arrives, `stop`'s sender sends or is dropped, or a task fails.
///
/// The queue reads [`SystemClock`] and draws [`EpochSerialLeaseIds`]; either failing
/// to start ends `serve` before it answers anything.
///
/// # Errors
///
/// [`ServeError::ClockBeforeEpoch`], [`ServeError::RandomSource`] or
/// [`ServeError::Config`] before anything is served; then the first error of the
/// owner, the listener or the signal handling, or [`ServeError::Task`] when a task
/// panicked or did not finish within `--shutdown-grace-ms`, in which case the
/// listener stopped waiting for the connections still open, the owner was told to
/// stop, and the scope's tasks still running a grace period later were aborted,
/// before `serve` returned. The connections' own tasks are axum's and are not ended:
/// each lasts until its connection ends or the runtime is dropped. The binary drops
/// its runtime as soon as `serve` returns, which closes them. Under a caller's
/// runtime one can outlive `serve`; a request that completes on it then is still
/// answered, and a command it sends is refused 503 `not_ready`, because the owner
/// has stopped.
///
/// ```no_run
/// # async fn example() -> Result<(), pachinko::ServeError> {
/// let config = pachinko::ServeConfig::parse(["--listen", "127.0.0.1:0"])?;
/// let listener = tokio::net::TcpListener::bind(config.listen()).await.unwrap();
/// let (stop, stopped) = tokio::sync::watch::channel(());
/// let server = pachinko::serve(listener, &config, stopped);
/// drop(stop); // the server stops once it starts
/// server.await
/// # }
/// ```
pub async fn serve(
    listener: TcpListener,
    config: &ServeConfig,
    stop: watch::Receiver<()>,
) -> Result<(), ServeError> {
    let queue = service_queue(config)?;
    let signals = install_signals()?;
    let (lifecycle, ticks) = Lifecycle::new(config);
    let owner = lifecycle.start(queue, ticks);
    scope(listener, owner, lifecycle, stop, signals).await
}

/// The service's queue over the real clock and lease ids.
fn service_queue(config: &ServeConfig) -> Result<ServiceQueue, ServeError> {
    let clock = SystemClock::new()?;
    let lease_ids = EpochSerialLeaseIds::new()?;
    let queue = Queue::new(config.queue().clone(), clock, lease_ids)?;
    Ok(queue)
}

/// Writes `listening on <address>` and flushes it, so a reader sees it at once.
fn announce(listener: &TcpListener, out: &mut impl Write) -> Result<(), ServeError> {
    let address = listener
        .local_addr()
        .map_err(ServeError::io("reading the bound address"))?;
    writeln!(out, "listening on {address}")
        .and_then(|()| out.flush())
        .map_err(ServeError::io("writing the bound address"))
}

/// The task scope over `owner`, its ticker, the listener under its limits and the
/// signals, in `lifecycle`.
pub(crate) async fn scope<F>(
    listener: TcpListener,
    owner: Owner<F>,
    lifecycle: Lifecycle,
    stop: watch::Receiver<()>,
    signals: impl Future<Output = ()> + Send + 'static,
) -> TaskResult
where
    F: Future<Output = TaskResult> + Send + 'static,
{
    let (handle, owner) = owner;
    let Lifecycle {
        readiness,
        shutdown,
        cut,
        bounds,
        limits,
        lane,
    } = lifecycle;
    let shutting_down = shutdown.subscribe();
    let routes = router(handle, readiness.subscribe(), bounds.request, limits);
    let mut tasks = JoinSet::new();
    tasks.spawn(owner);
    tasks.spawn(actor::ticker(lane, bounds.sweep, shutting_down.clone()));
    tasks.spawn(listen(
        listener,
        routes,
        shutting_down.clone(),
        cut.subscribe(),
    ));
    tasks.spawn(signal(signals, stop, shutting_down));
    let first = tasks.join_next().await.map_or(Ok(()), task_result);
    readiness.send_replace(Readiness::Stopping);
    shutdown.send_replace(true);
    let rest = drain(&mut tasks, &cut, bounds.grace).await;
    first.and(rest)
}

/// A joined task's own result, or why it did not finish.
fn task_result(joined: Result<TaskResult, JoinError>) -> TaskResult {
    joined.unwrap_or_else(|error| Err(ServeError::Task(error.to_string())))
}

/// Joins every remaining task, keeping the first error; each is expected to end
/// because the shutdown has begun, which the scope sent before this, so the cut is
/// only the backstop. Past the grace period the cut is sent, which ends the
/// listener's wait for the open connections and tells the owner to stop between
/// commands; the tasks are then joined again for at most one more grace period, and
/// whatever is still running after it is aborted. Nothing is aborted before then.
/// The abort waits for the one command or tick the owner was applying, and for the
/// queue's state to be released when the owner ends; neither is bounded by the
/// grace. The connection tasks are axum's and sit outside `tasks`: neither the cut
/// nor the abort ends or joins them (the module's doc says what becomes of them). An
/// error a task ended with inside the grace period is returned; failing that, the
/// overrun's own error. A task that fails after the cut is logged. A zero grace
/// period cuts at once and is no overrun: the tasks, which end on the cut, are joined
/// for at most [`ZERO_GRACE_BACKSTOP`], and whatever is still running after it is
/// aborted.
async fn drain(
    tasks: &mut JoinSet<TaskResult>,
    cut: &watch::Sender<bool>,
    grace: Duration,
) -> TaskResult {
    let mut first = Ok(());
    if grace.is_zero() {
        cut.send_replace(true);
        if timeout(ZERO_GRACE_BACKSTOP, join_into(tasks, &mut first))
            .await
            .is_ok()
        {
            return first;
        }
        return abort(tasks, first, ZERO_GRACE_BACKSTOP).await;
    }
    if timeout(grace, join_into(tasks, &mut first)).await.is_ok() {
        return first;
    }
    let millis = grace.as_millis();
    let overran = ServeError::Task(format!(
        "shutdown took longer than {millis}ms; open connections were cut"
    ));
    tracing::warn!(error = %overran, "cutting open connections");
    cut.send_replace(true);
    if timeout(grace, join_logging(tasks)).await.is_ok() {
        return first.and(Err(overran));
    }
    abort(tasks, first, grace).await
}

/// How long a zero grace period waits, after its immediate cut, for the tasks to end
/// before aborting them: the backstop the second grace period gives any other grace.
const ZERO_GRACE_BACKSTOP: Duration = Duration::from_secs(1);

/// Aborts the tasks still running `waited` after the cut, keeping `first`, an error a
/// task ended with before then.
async fn abort(tasks: &mut JoinSet<TaskResult>, first: TaskResult, waited: Duration) -> TaskResult {
    let millis = waited.as_millis();
    let aborted = ServeError::Task(format!(
        "shutdown took longer than {millis}ms; open connections were cut, and {} tasks \
        still running {millis}ms later were aborted",
        tasks.len()
    ));
    tracing::warn!(error = %aborted, "aborting the tasks still running");
    tasks.shutdown().await;
    first.and(Err(aborted))
}

/// Joins every task in `tasks`, logging each failure.
async fn join_logging(tasks: &mut JoinSet<TaskResult>) {
    while let Some(joined) = tasks.join_next().await {
        if let Err(error) = task_result(joined) {
            tracing::warn!(%error, "a task failed after the cut");
        }
    }
}

/// Joins every task in `tasks`, keeping the first error in `first`, which is the
/// caller's so that it survives this future being dropped part way: `join_next` is
/// cancel-safe.
async fn join_into(tasks: &mut JoinSet<TaskResult>, first: &mut TaskResult) {
    while let Some(joined) = tasks.join_next().await {
        if first.is_ok() {
            *first = task_result(joined);
        }
    }
}

/// The HTTP listener: it stops accepting when the shutdown starts, and returns once
/// the in-flight requests have finished or at the cut, whichever comes first. axum
/// waits for every open connection, so without the cut a stalled one would hold this
/// task until the abort. Returning drops the router, which is the owner's last handle
/// unless the cut left a connection's task holding a clone of it.
async fn listen(
    listener: TcpListener,
    routes: axum::Router,
    shutting_down: watch::Receiver<bool>,
    mut cut: watch::Receiver<bool>,
) -> TaskResult {
    let served =
        axum::serve(listener, routes).with_graceful_shutdown(shutdown_started(shutting_down));
    tokio::select! {
        served = served.into_future() => served.map_err(ServeError::io("serving HTTP")),
        _ = cut.wait_for(|cut| *cut) => Ok(()),
    }
}

/// Ends on SIGTERM or SIGINT, when `stop`'s sender sends or is dropped, or when
/// another task has started the shutdown.
async fn signal(
    signals: impl Future<Output = ()>,
    mut stop: watch::Receiver<()>,
    shutting_down: watch::Receiver<bool>,
) -> TaskResult {
    tokio::select! {
        () = signals => tracing::info!("signal received; shutting down"),
        _ = stop.changed() => tracing::info!("stop requested; shutting down"),
        () = shutdown_started(shutting_down) => {}
    }
    Ok(())
}

/// Ends once the shutdown has started.
async fn shutdown_started(mut shutting_down: watch::Receiver<bool>) {
    drop(shutting_down.wait_for(|stopping| *stopping).await);
}

/// Installs the handlers for SIGTERM and SIGINT (ctrl-c) and returns the future that
/// waits for either. They are installed before the address is announced, so a
/// signal sent the moment the server is ready is not lost.
#[cfg(unix)]
fn install_signals() -> Result<impl Future<Output = ()> + Send + 'static, ServeError> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut terminate =
        signal(SignalKind::terminate()).map_err(ServeError::io("handling SIGTERM"))?;
    let mut interrupt =
        signal(SignalKind::interrupt()).map_err(ServeError::io("handling SIGINT"))?;
    Ok(async move {
        tokio::select! {
            _ = terminate.recv() => {}
            _ = interrupt.recv() => {}
        }
    })
}

/// Off unix, ctrl-c is the signal; its handler is installed on first poll.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the unix twin installs handlers that can fail"
)]
fn install_signals() -> Result<impl Future<Output = ()> + Send + 'static, ServeError> {
    Ok(async {
        drop(tokio::signal::ctrl_c().await);
    })
}

#[cfg(test)]
mod tests {
    use std::io::{ErrorKind, Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::process::ExitCode;
    use std::time::{Duration, Instant};

    use tokio::net::TcpListener;
    use tokio::sync::watch;
    use tokio::task::JoinSet;
    use tokio::time::timeout;

    use super::{Lifecycle, drain, scope, service_queue};
    use crate::actor::{self, Readiness};
    use crate::{ServeConfig, ServeError, exit_code};

    /// How long a test waits for `serve` before it fails.
    const BOUND: Duration = Duration::from_secs(10);

    const CONTINUE: &[u8] = b"HTTP/1.1 100 Continue\r\n\r\n";

    /// A whole publish body.
    const PUBLISH: &[u8] = br#"{"publisher_id": "p", "rank": 1, "payload": ""}"#;

    /// A head promising [`PUBLISH`], asking to be told when the handler starts reading
    /// it. The body is the caller's to send, or to withhold. It does not ask for the
    /// connection to be closed, so a test that reads its reply to the end shows the
    /// server closing it.
    fn publish_head() -> String {
        format!(
            "PUT /pockets/a/records/k HTTP/1.1\r\nhost: test\r\n\
content-type: application/json\r\ncontent-length: {}\r\n\
expect: 100-continue\r\n\r\n",
            PUBLISH.len()
        )
    }

    /// Asserts `reply` is the whole 503 `not_ready` of a command that found the owner
    /// stopped.
    fn assert_owner_stopped(reply: &str) {
        assert!(
            reply.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "{reply:?}"
        );
        assert!(
            reply.ends_with(r#"{"error":"not_ready","message":"the queue's owner has stopped"}"#),
            "{reply:?}"
        );
    }

    /// A config on port 0 with `flags` added.
    fn config(flags: &[&str]) -> ServeConfig {
        let mut args = vec!["--listen", "127.0.0.1:0"];
        args.extend_from_slice(flags);
        ServeConfig::parse(args).unwrap()
    }

    /// A multi-threaded runtime, so a blocking client can run beside the server.
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// An owner that panics at its first turn.
    async fn panicking_owner() -> Result<(), ServeError> {
        tokio::task::yield_now().await;
        panic!("the owner failed")
    }

    #[tokio::test]
    async fn a_panicked_owner_makes_serve_fail_within_three_grace_periods() {
        let config = config(&["--shutdown-grace-ms", "200"]);
        let listener = TcpListener::bind(config.listen()).await.unwrap();
        let queue = service_queue(&config).unwrap();
        let (lifecycle, ticks) = Lifecycle::new(&config);
        let (handle, _real_owner) = lifecycle.start(queue, ticks);
        let (_stop, stopped) = watch::channel(());
        let owner = (handle, panicking_owner());
        let served = timeout(
            config.shutdown_grace() * 3,
            scope(listener, owner, lifecycle, stopped, std::future::pending()),
        )
        .await
        .expect("serve returned within three grace periods");
        assert!(
            matches!(&served, Err(ServeError::Task(message)) if message.contains("panic")),
            "{served:?}"
        );
        let mut err = Vec::new();
        assert_eq!(exit_code(served, &mut err), ExitCode::FAILURE);
        assert!(
            String::from_utf8(err)
                .unwrap()
                .starts_with("pachinko: server task: ")
        );
    }

    #[tokio::test]
    async fn a_task_that_ignores_the_cut_is_aborted_after_the_second_grace_period() {
        let config = config(&["--shutdown-grace-ms", "200"]);
        let listener = TcpListener::bind(config.listen()).await.unwrap();
        let queue = service_queue(&config).unwrap();
        let (lifecycle, ticks) = Lifecycle::new(&config);
        let (handle, _real_owner) = lifecycle.start(queue, ticks);
        // The owner's place is taken by a future parked at a yield point that never
        // looks at the cut, so only the abort can end it.
        let owner = (handle, std::future::pending());
        let (stop, stopped) = watch::channel(());
        drop(stop);
        let started = Instant::now();
        let served = timeout(
            config.shutdown_grace() * 3,
            scope(listener, owner, lifecycle, stopped, std::future::pending()),
        )
        .await
        .expect("serve returned within three grace periods");
        assert!(
            matches!(&served, Err(ServeError::Task(message)) if message.contains("1 tasks still running 200ms later were aborted")),
            "{served:?}"
        );
        assert!(
            started.elapsed() >= config.shutdown_grace() * 2,
            "aborted before the second grace period ended: {:?}",
            started.elapsed()
        );
    }

    // The ticker ends when the shutdown begins, not on the cut: a sweep far longer
    // than the grace period still stops cleanly, inside the first grace period.
    #[tokio::test]
    async fn a_sweep_longer_than_the_grace_period_still_stops_cleanly() {
        let config = config(&["--sweep-ms", "3600000", "--shutdown-grace-ms", "2000"]);
        let listener = TcpListener::bind(config.listen()).await.unwrap();
        let (stop, stopped) = watch::channel(());
        drop(stop);
        let served = timeout(
            config.shutdown_grace(),
            super::serve(listener, &config, stopped),
        );
        assert_eq!(
            served.await.expect("serve stopped inside the grace period"),
            Ok(())
        );
    }

    // A zero grace period cuts at once; it is not an overrun, so a clean stop still
    // ends serve with `Ok`.
    #[tokio::test]
    async fn a_stop_with_a_zero_grace_period_ends_serve_with_ok() {
        let config = config(&["--shutdown-grace-ms", "0"]);
        let listener = TcpListener::bind(config.listen()).await.unwrap();
        let (stop, stopped) = watch::channel(());
        drop(stop);
        let served = timeout(BOUND, super::serve(listener, &config, stopped)).await;
        assert_eq!(served.unwrap(), Ok(()));
    }

    #[tokio::test]
    async fn a_stop_ends_serve_with_ok() {
        let config = config(&[]);
        let listener = TcpListener::bind(config.listen()).await.unwrap();
        let (stop, stopped) = watch::channel(());
        drop(stop);
        let served = timeout(BOUND, super::serve(listener, &config, stopped)).await;
        assert_eq!(served.unwrap(), Ok(()));
    }

    /// Sends [`publish_head`] and waits until the handler is reading the body.
    fn start_a_request(address: SocketAddr) -> TcpStream {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(publish_head().as_bytes()).unwrap();
        let mut continued = vec![0; CONTINUE.len()];
        stream.read_exact(&mut continued).unwrap();
        assert_eq!(continued, CONTINUE, "the handler is reading the body");
        stream
    }

    /// Serves under `flags`, over the real owner, while `client` runs against the
    /// listener's address, holding the sender whose drop stops the server. Returns
    /// what `serve` returned and what the client returned; `runtime` lives on.
    fn serve_beside<T: Send>(
        runtime: &tokio::runtime::Runtime,
        flags: &[&str],
        client: impl FnOnce(SocketAddr, watch::Sender<()>) -> T + Send,
    ) -> (Result<(), ServeError>, T) {
        runtime.block_on(async {
            let config = config(flags);
            let listener = TcpListener::bind(config.listen()).await.unwrap();
            let address = listener.local_addr().unwrap();
            let (stop, stopped) = watch::channel(());
            let client = async move { tokio::task::block_in_place(|| client(address, stop)) };
            let queue = service_queue(&config).unwrap();
            let (lifecycle, ticks) = Lifecycle::new(&config);
            let owner = lifecycle.start(queue, ticks);
            let server = scope(listener, owner, lifecycle, stopped, std::future::pending());
            timeout(BOUND, async { tokio::join!(server, client) })
                .await
                .expect("serve returned within the bound")
        })
    }

    #[test]
    fn a_connection_open_past_the_grace_period_gets_no_command_applied_after_serve_returns() {
        let runtime = runtime();
        let grace = ["--shutdown-grace-ms", "100"];
        let (served, (mut finishing, mut stalled)) =
            serve_beside(&runtime, &grace, |address, stop| {
                let finishing = start_a_request(address);
                let stalled = start_a_request(address);
                drop(stop);
                (finishing, stalled)
            });
        // The listener ended on the cut, not by the abort a grace period later, whose
        // error goes on to count the tasks it aborted.
        assert!(
            matches!(&served, Err(ServeError::Task(message)) if message.ends_with("open connections were cut")),
            "{served:?}"
        );
        // `serve` has returned while the runtime lives on, and with it the tasks axum
        // serves the two connections on. Finishing the body reaches the handler, whose
        // command finds the owner stopped: 503, never applied.
        finishing.write_all(PUBLISH).unwrap();
        let mut reply = String::new();
        finishing.read_to_string(&mut reply).unwrap();
        assert_owner_stopped(&reply);
        // The stalled connection is still open while the runtime lives: a short read
        // finds neither a reply nor the end.
        stalled
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let open = stalled.read(&mut [0; 1]);
        assert!(
            matches!(&open, Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)),
            "the stalled connection closed or was answered before the runtime was dropped: {open:?}"
        );
        stalled
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        // Dropping the runtime, as the binary does once `serve` returns, closes the
        // connection still stalled. It must be closed, not merely silent: a read that
        // times out fails the test.
        drop(runtime);
        let mut rest = [0; 256];
        let read = match stalled.read(&mut rest) {
            Ok(read) => read,
            Err(error) if matches!(error.kind(), ErrorKind::ConnectionReset) => 0,
            Err(error) => panic!("the connection neither closed nor answered: {error}"),
        };
        assert_eq!(
            read,
            0,
            "a handler answered the stalled request: {}",
            String::from_utf8_lossy(&rest[..read])
        );
    }

    /// Reads `stream` until a refusal's JSON body has ended.
    fn read_reply(stream: &mut TcpStream) -> String {
        let mut reply = Vec::new();
        let mut chunk = [0; 512];
        while !reply.ends_with(b"}") {
            let read = stream.read(&mut chunk).unwrap();
            assert_ne!(read, 0, "the connection closed before the reply ended");
            reply.extend_from_slice(&chunk[..read]);
        }
        String::from_utf8_lossy(&reply).into_owned()
    }

    /// Asserts `reply` is the 408 `request_timeout` of a handler that ran past a bound
    /// of 100 ms.
    fn assert_timed_out(reply: &str) {
        assert!(
            reply.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
            "{reply:?}"
        );
        assert!(reply.contains(r#""error":"request_timeout""#), "{reply:?}");
        assert!(
            reply.contains("the request took longer than 100ms"),
            "{reply:?}"
        );
    }

    /// Waits until `readiness` reads `wanted`, failing after [`BOUND`].
    fn wait_for_readiness(readiness: &watch::Receiver<Readiness>, wanted: Readiness) {
        let deadline = Instant::now() + BOUND;
        while *readiness.borrow() != wanted {
            assert!(Instant::now() < deadline, "readiness never became {wanted}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn a_request_whose_owner_has_stopped_answers_503_not_ready() {
        let runtime = runtime();
        let (served, reply) = runtime.block_on(async {
            let config = config(&[]);
            let listener = TcpListener::bind(config.listen()).await.unwrap();
            let address = listener.local_addr().unwrap();
            let (_stop, stopped) = watch::channel(());
            let (lifecycle, ticks) = Lifecycle::new(&config);
            let readiness = lifecycle.readiness.subscribe();
            // The owner's stop is the test's to send: the same watch and the same
            // branch of the owner loop the scope's cut reaches.
            let (stop_owner, owner_stop) = watch::channel(false);
            let queue = service_queue(&config).unwrap();
            let readiness_sender = lifecycle.readiness.clone();
            let owner = actor::start(queue, readiness_sender, owner_stop, ticks);
            // On a blocking thread of its own: it waits for `Stopping`, which the
            // scope sends, so it must not hold the task the scope runs on.
            let client = tokio::task::spawn_blocking(move || {
                wait_for_readiness(&readiness, Readiness::Serving);
                let mut stream = start_a_request(address);
                stop_owner.send_replace(true);
                // `Stopping` is sent once the owner's task has ended.
                wait_for_readiness(&readiness, Readiness::Stopping);
                stream.write_all(PUBLISH).unwrap();
                let mut reply = String::new();
                stream.read_to_string(&mut reply).unwrap();
                reply
            });
            let client = async { client.await.unwrap() };
            let server = scope(listener, owner, lifecycle, stopped, std::future::pending());
            timeout(BOUND, async { tokio::join!(server, client) })
                .await
                .expect("serve returned within the bound")
        });
        assert_eq!(served, Ok(()));
        assert_owner_stopped(&reply);
        drop(runtime);
    }

    #[test]
    fn a_handler_slower_than_the_bound_is_answered_408_on_its_connection() {
        let runtime = runtime();
        let (served, reply) = runtime.block_on(async {
            let config = config(&["--request-timeout-ms", "100", "--shutdown-grace-ms", "100"]);
            let listener = TcpListener::bind(config.listen()).await.unwrap();
            let address = listener.local_addr().unwrap();
            let (stop, stopped) = watch::channel(());
            let queue = service_queue(&config).unwrap();
            let (lifecycle, ticks) = Lifecycle::new(&config);
            // The real owner is kept but never run, so the command is never answered
            // and the handler outlives the bound.
            let (handle, _real_owner) = lifecycle.start(queue, ticks);
            let owner = (handle, std::future::pending());
            let client = tokio::task::spawn_blocking(move || {
                let mut stream = TcpStream::connect(address).unwrap();
                stream.set_read_timeout(Some(BOUND)).unwrap();
                stream
                    .write_all(b"POST /publishers/p HTTP/1.1\r\nhost: test\r\n\r\n")
                    .unwrap();
                let reply = read_reply(&mut stream);
                drop(stop);
                reply
            });
            let server = scope(listener, owner, lifecycle, stopped, std::future::pending());
            let client = async { client.await.unwrap() };
            timeout(BOUND, async { tokio::join!(server, client) })
                .await
                .expect("serve returned within the bound")
        });
        assert_timed_out(&reply);
        // The parked owner ignores the cut, so serve ends by the abort.
        assert!(matches!(served, Err(ServeError::Task(_))), "{served:?}");
        drop(runtime);
    }

    #[test]
    fn a_body_that_never_arrives_is_answered_408_at_the_handlers_bound() {
        let runtime = runtime();
        let bound = ["--request-timeout-ms", "100"];
        let (served, (reply, waited)) = serve_beside(&runtime, &bound, |address, stop| {
            let started = Instant::now();
            // The handler is reading a body that is never sent.
            let mut stream = start_a_request(address);
            let reply = read_reply(&mut stream);
            let waited = started.elapsed();
            drop(stop);
            (reply, waited)
        });
        assert_timed_out(&reply);
        // No sooner than the bound, which starts after the client does, and with a wide
        // margin after it: a handler bounded at ten times the flag fails here.
        assert!(
            waited >= Duration::from_millis(100) && waited < Duration::from_millis(1000),
            "answered after {waited:?}, not at the 100 ms bound"
        );
        assert_eq!(served, Ok(()));
        drop(runtime);
    }

    /// Drains a task that fails at once, if `early` is set, beside one that waits
    /// for the cut and then fails with `late`, over a 50 ms grace period. Returns
    /// what `drain` returned and how many tasks it left unjoined.
    async fn drain_a_late_task(early: bool, late: &'static str) -> (Result<(), ServeError>, usize) {
        let (cut, mut cutting) = watch::channel(false);
        let mut tasks = JoinSet::new();
        if early {
            tasks.spawn(async { Err(ServeError::Task(String::from("early"))) });
        }
        tasks.spawn(async move {
            drop(cutting.wait_for(|cut| *cut).await);
            Err(ServeError::Task(String::from(late)))
        });
        let drained = drain(&mut tasks, &cut, Duration::from_millis(50)).await;
        (drained, tasks.len())
    }

    #[tokio::test]
    async fn a_failure_inside_the_grace_period_outlives_its_timeout() {
        let (drained, left) = drain_a_late_task(true, "late").await;
        assert_eq!(drained, Err(ServeError::Task(String::from("early"))));
        assert_eq!(left, 0, "every task is joined");
    }

    #[tokio::test]
    async fn a_failure_after_the_cut_comes_after_the_grace_periods_error() {
        let (drained, left) = drain_a_late_task(false, "late").await;
        assert_eq!(
            drained,
            Err(ServeError::Task(String::from(
                "shutdown took longer than 50ms; open connections were cut"
            )))
        );
        assert_eq!(left, 0, "every task is joined");
    }

    #[tokio::test]
    async fn a_zero_grace_period_cuts_at_once_and_joins_the_tasks_that_end_on_the_cut() {
        let (cut, mut cutting) = watch::channel(false);
        let mut tasks = JoinSet::new();
        tasks.spawn(async move {
            drop(cutting.wait_for(|cut| *cut).await);
            Ok(())
        });
        let drained = timeout(BOUND, drain(&mut tasks, &cut, Duration::ZERO))
            .await
            .expect("drain returned within the bound");
        assert_eq!(drained, Ok(()));
        assert!(*cut.borrow(), "the cut was sent");
        assert_eq!(tasks.len(), 0, "every task is joined");
    }

    #[tokio::test]
    async fn a_task_that_ignores_the_cut_under_a_zero_grace_period_is_aborted_after_the_backstop() {
        let (cut, _cutting) = watch::channel(false);
        let mut tasks = JoinSet::new();
        tasks.spawn(std::future::pending::<Result<(), ServeError>>());
        let drained = timeout(BOUND, drain(&mut tasks, &cut, Duration::ZERO))
            .await
            .expect("drain returned within the bound");
        assert!(matches!(drained, Err(ServeError::Task(_))), "{drained:?}");
        assert!(*cut.borrow(), "the cut was sent");
        assert_eq!(tasks.len(), 0, "the parked task was aborted");
    }

    #[tokio::test]
    async fn an_early_failure_outlives_the_abort_too() {
        let (cut, _cutting) = watch::channel(false);
        let mut tasks = JoinSet::new();
        tasks.spawn(async { Err(ServeError::Task(String::from("early"))) });
        tasks.spawn(std::future::pending());
        let drained = timeout(BOUND, drain(&mut tasks, &cut, Duration::from_millis(50)))
            .await
            .expect("drain returned within the bound");
        assert_eq!(drained, Err(ServeError::Task(String::from("early"))));
        assert!(*cut.borrow(), "the cut was sent");
        assert_eq!(tasks.len(), 0, "the parked task was aborted");
    }
}
