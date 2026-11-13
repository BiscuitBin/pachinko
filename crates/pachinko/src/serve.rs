//! The server: one structured task scope over the queue's owner (decision 0019).
//!
//! The scope is one [`JoinSet`] holding four tasks: the queue's owner, its ticker,
//! the HTTP listener and the shutdown signal. The first to end, for whatever
//! reason, starts the shutdown; all of them are joined before `serve` returns, and
//! the first error is the one returned. No other task is spawned here; axum spawns one task per
//! connection outside the scope, which the cut ends at its next read or write.
//!
//! Shutdown runs in this order. Readiness goes `Stopping`, so `GET /readyz` answers
//! 503 at once; the listener stops accepting and in-flight requests finish. If every
//! task ends within `--shutdown-grace-ms` (the owner ends when the listener has
//! dropped the last handle), that is the end. Otherwise the open connections are
//! cut and the owner is told to stop, and the tasks are joined again for at most
//! one more grace period; only then is any of the scope's tasks still running
//! aborted, at its next yield point. The owner yields between commands, so `serve`
//! returns within two grace periods plus the one command or tick the owner was
//! applying, and releasing the queue's state when the owner ends. A zero grace
//! period cuts at once and joins the tasks for at most one second before aborting
//! what is left.
//!
//! Every task in the scope must end once the shutdown begins, on the lifecycle's
//! `shutdown` watch or on the owner ending, without waiting for the cut: the cut is
//! sent only after the grace period, so a task that waits for it turns a clean
//! shutdown into an overrun. The cut is the backstop for a task that overran.

use std::io::{self, Write};
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use libpachinko::Queue;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::task::{JoinError, JoinSet};
use tokio::time::{Instant, Sleep, sleep_until, timeout};

use crate::actor::{self, QueueHandle, Readiness, ServiceQueue, Tick};
use crate::http::{Limits, router};
use crate::{EpochSerialLeaseIds, ServeConfig, ServeError, SystemClock};

/// What each task of the scope ends with.
type TaskResult = Result<(), ServeError>;

/// The queue's handle and its owner, still to be spawned.
type Owner<F> = (QueueHandle, F);

/// What the scope shares with the owner, its bounds and the router's request-shape
/// limits (T27): the readiness the owner
/// sets to `Serving` and the scope to `Stopping`; the shutdown, which turns true
/// when the shutdown begins and which every task beside the owner ends on; the
/// cut, which ends the open connections and tells the owner to stop; and the tick
/// lane's sender, which the ticker holds. The lane's receiver is not kept here:
/// [`Lifecycle::new`] hands it back, and [`Lifecycle::start`] takes it by value for
/// the owner, so one receiver serves one start.
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

/// The scope's two bounds, from the flags.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    /// `--shutdown-grace-ms`: each of shutdown's two waits.
    grace: Duration,
    /// `--request-timeout-ms`: each handler, and each connection's idle read.
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
/// connections still open were cut, and the scope's tasks still running a grace
/// period later were aborted, before `serve` returned; a cut connection's own task
/// can outlive `serve` until its next read or write.
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
    let connections = Connections {
        listener,
        cut: cut.subscribe(),
        idle: bounds.request,
    };
    let routes = router(handle, readiness.subscribe(), bounds.request, limits);
    let mut tasks = JoinSet::new();
    tasks.spawn(owner);
    tasks.spawn(actor::ticker(lane, bounds.sweep, shutting_down.clone()));
    tasks.spawn(listen(connections, routes, shutting_down.clone()));
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
/// only the backstop. Past the grace period every open connection is cut, which
/// ends the listener's wait for them at their next read or write, and tells the
/// owner to stop between commands; the tasks are then joined again for at most one
/// more grace period, and whatever is still running after it is aborted. Nothing is
/// aborted before then. The abort waits for the one command or tick the owner was
/// applying, and for the queue's state to be released when the owner ends; neither
/// is bounded by the grace. The connection tasks are axum's and sit outside `tasks`:
/// the abort neither ends nor joins them, and one can outlive `serve` until its next
/// read or write, which the cut fails. An error a task ended with inside the grace
/// period is returned; failing that, the overrun's own error. A task that fails
/// after the cut is logged. A zero grace period cuts at once and is no overrun: the
/// tasks, which end on the cut, are joined for at most [`ZERO_GRACE_BACKSTOP`], and
/// whatever is still running after it is aborted.
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
            tracing::warn!(%error, "a task failed after its connections were cut");
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

/// The HTTP listener, until shutdown starts; in-flight requests finish first, unless
/// their connections are cut. The router, and with it the last handle to the owner,
/// is dropped when this returns.
async fn listen(
    connections: Connections,
    routes: axum::Router,
    shutting_down: watch::Receiver<bool>,
) -> TaskResult {
    axum::serve(connections, routes)
        .with_graceful_shutdown(shutdown_started(shutting_down))
        .await
        .map_err(ServeError::io("serving HTTP"))
}

/// The TCP listener, handing axum connections that `cut` can end and that close
/// once idle for `idle`.
struct Connections {
    listener: TcpListener,
    cut: watch::Receiver<bool>,
    idle: Duration,
}

impl axum::serve::Listener for Connections {
    type Io = Connection;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        let (stream, address) = axum::serve::Listener::accept(&mut self.listener).await;
        let mut cut = self.cut.clone();
        let cut = Box::pin(async move { drop(cut.wait_for(|cut| *cut).await) });
        (
            Connection {
                stream,
                cut: Some(cut),
                idle: Idle {
                    bound: self.idle,
                    waiting: None,
                    owed_since: None,
                },
            },
            address,
        )
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

/// One accepted connection: its reads and writes fail once it is cut, so the task
/// axum serves it on ends at its next turn, and a read fails once [`Idle`] says the
/// client has been idle too long.
struct Connection {
    stream: TcpStream,
    /// Ends when the connection is cut; `None` once it has.
    cut: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
    idle: Idle,
}

/// A connection's idle bound: a read that stays pending for `bound` while the
/// server owes the client nothing fails, and so does one still pending two bounds
/// after the first byte that arrived while nothing was owed, until the server
/// writes.
struct Idle {
    /// How long a read may stay pending while the server owes no reply.
    bound: Duration,
    /// The deadline of the read now pending, if one is.
    waiting: Option<Pin<Box<Sleep>>>,
    /// When the first byte arrived since the server last wrote: a request is being
    /// read or handled. Later bytes leave it as it is, and only a write clears it.
    /// The deadline is two bounds from this instant. The handler starts no later
    /// than the head's last byte and the request timeout writes its 408 one bound
    /// after that, so a head that arrives within one bound always gets its reply
    /// or its 408 before the deadline. A slower head still gets its reply if the
    /// handler writes it within two bounds of the first byte; otherwise the read
    /// fails at the deadline, before the 408 is due. What it gets instead is the
    /// handler's: one still reading the body answers 400 for the failed read. A
    /// partial head is closed unanswered. Any write clears it, a `100 Continue`
    /// included, so a head that asked for one is timed afresh from that reply and
    /// its handler can still answer 408. The instant is the connection's, not the
    /// request's: a pipelined request whose first bytes arrived before the
    /// previous reply was written is timed from that write, and is closed within
    /// three bounds of it, one with nothing owed and two from its next byte; one
    /// whose whole head was buffered by then can get the 400 instead of its 408. A
    /// client trickling a head holds the connection at most two bounds per
    /// request, three after a pipelined reply; no deadline at all would let it
    /// hold the connection with a partial head.
    owed_since: Option<Instant>,
}

impl Idle {
    /// Fails once a read has stayed pending for `bound` while nothing is owed, or
    /// two bounds after the first byte that arrived while nothing was owed; until
    /// then, wakes `context` when it would.
    fn pending(&mut self, context: &mut Context<'_>) -> io::Result<()> {
        let bound = self.bound;
        let deadline = match self.owed_since {
            Some(since) => since + 2 * bound,
            None => Instant::now() + bound,
        };
        let waiting = self
            .waiting
            .get_or_insert_with(|| Box::pin(sleep_until(deadline)));
        if waiting.as_mut().poll(context).is_pending() {
            return Ok(());
        }
        self.waiting = None;
        let message = match self.owed_since {
            Some(_) => format!(
                "no reply was written within {}ms of the first byte",
                (2 * bound).as_millis()
            ),
            None => format!("no bytes arrived within {}ms", bound.as_millis()),
        };
        Err(io::Error::new(io::ErrorKind::TimedOut, message))
    }

    /// A read finished; `arrived` says whether it brought bytes, which the server
    /// now owes a reply to. The first such byte since the last write starts the
    /// deadline; later ones leave it where it is.
    fn read(&mut self, arrived: bool) {
        self.waiting = None;
        if arrived {
            self.owed_since.get_or_insert_with(Instant::now);
        }
    }

    /// The server wrote: whatever it owed is being answered.
    fn wrote(&mut self) {
        self.waiting = None;
        self.owed_since = None;
    }
}

impl Connection {
    /// Fails once the connection is cut; until then, wakes `context` when it is.
    fn open(&mut self, context: &mut Context<'_>) -> io::Result<()> {
        if let Some(cut) = &mut self.cut {
            if cut.as_mut().poll(context).is_pending() {
                return Ok(());
            }
            self.cut = None;
        }
        Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "the shutdown grace period is over",
        ))
    }
}

impl AsyncRead for Connection {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.open(context)?;
        let before = buf.filled().len();
        let read = Pin::new(&mut this.stream).poll_read(context, buf);
        if read.is_pending() {
            this.idle.pending(context)?;
        } else {
            this.idle.read(buf.filled().len() > before);
        }
        read
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        this.open(context)?;
        let written = Pin::new(&mut this.stream).poll_write(context, buf);
        if matches!(written, Poll::Ready(Ok(count)) if count > 0) {
            this.idle.wrote();
        }
        written
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.open(context)?;
        Pin::new(&mut this.stream).poll_flush(context)
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(context)
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

    /// A publish whose head promises a 2-byte body, asking to be told when the
    /// handler starts reading it.
    const HEAD: &[u8] = b"PUT /pockets/a/records/k HTTP/1.1\r\nhost: test\r\n\
content-type: application/json\r\ncontent-length: 2\r\n\
expect: 100-continue\r\n\r\n";
    const CONTINUE: &[u8] = b"HTTP/1.1 100 Continue\r\n\r\n";

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

    /// Sends `head` and waits until the handler is reading the body.
    fn start_a_request(address: SocketAddr, head: &[u8]) -> TcpStream {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(head).unwrap();
        let mut continued = vec![0; CONTINUE.len()];
        stream.read_exact(&mut continued).unwrap();
        assert_eq!(continued, CONTINUE, "the handler is reading the body");
        stream
    }

    /// Sends [`HEAD`] and waits until the handler is reading the body, then drops
    /// `stop` with the body still unsent.
    fn stall_a_request(address: SocketAddr, stop: watch::Sender<()>) -> TcpStream {
        let stream = start_a_request(address, HEAD);
        drop(stop);
        stream
    }

    #[test]
    fn a_connection_open_past_the_grace_period_is_cut_before_serve_returns() {
        let runtime = runtime();
        let (served, mut stream) = runtime.block_on(async {
            let config = config(&["--shutdown-grace-ms", "100"]);
            let listener = TcpListener::bind(config.listen()).await.unwrap();
            let address = listener.local_addr().unwrap();
            let (stop, stopped) = watch::channel(());
            let client =
                async move { tokio::task::block_in_place(|| stall_a_request(address, stop)) };
            let queue = service_queue(&config).unwrap();
            let (lifecycle, ticks) = Lifecycle::new(&config);
            let owner = lifecycle.start(queue, ticks);
            let server = scope(listener, owner, lifecycle, stopped, std::future::pending());
            timeout(BOUND, async { tokio::join!(server, client) })
                .await
                .expect("serve returned within the bound")
        });
        assert!(
            matches!(&served, Err(ServeError::Task(message)) if message.ends_with("open connections were cut")),
            "{served:?}"
        );
        // `serve` has returned while the runtime lives on: finishing the body must not
        // reach a handler. The connection must be closed, not merely silent: a read
        // that times out fails the test.
        stream.write_all(b"{}").ok();
        let mut reply = [0; 256];
        let read = match stream.read(&mut reply) {
            Ok(read) => read,
            Err(error) if matches!(error.kind(), ErrorKind::ConnectionReset) => 0,
            Err(error) => panic!("the connection neither closed nor answered: {error}"),
        };
        assert_eq!(
            read,
            0,
            "a handler answered after serve returned: {}",
            String::from_utf8_lossy(&reply[..read])
        );
        drop(runtime);
    }

    /// Connects to `address`, sends `sent`, and returns whether the server closed
    /// the connection and how long that took, dropping `stop` then. The read's own
    /// deadline of five seconds ends the wait otherwise, and its error fails the
    /// test.
    fn wait_for_the_close(
        address: SocketAddr,
        sent: &'static [u8],
        stop: watch::Sender<()>,
    ) -> (bool, Duration) {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let started = Instant::now();
        stream.write_all(sent).unwrap();
        let closed = match stream.read(&mut [0; 64]) {
            Ok(read) => read == 0,
            Err(error) => {
                assert_eq!(error.kind(), ErrorKind::ConnectionReset, "{error}");
                true
            }
        };
        drop(stop);
        (closed, started.elapsed())
    }

    /// Waits until `readiness` reads `wanted`, failing after [`BOUND`].
    fn wait_for_readiness(readiness: &watch::Receiver<Readiness>, wanted: Readiness) {
        let deadline = Instant::now() + BOUND;
        while *readiness.borrow() != wanted {
            assert!(Instant::now() < deadline, "readiness never became {wanted}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Serves with a request timeout of 100 ms while a client sends `sent` and
    /// waits for the close, returning whether it came and how long it took.
    fn serve_until_closed(sent: &'static [u8]) -> (bool, Duration) {
        serve_beside(move |address, stop| wait_for_the_close(address, sent, stop))
    }

    /// Serves with a request timeout of 100 ms while `client` runs against the
    /// listener's address, returning what the client returned once `serve` has
    /// ended with `Ok`. The client drops `stop` when it is done.
    fn serve_beside<T: Send>(client: impl FnOnce(SocketAddr, watch::Sender<()>) -> T + Send) -> T {
        let runtime = runtime();
        let (served, closed) = runtime.block_on(async {
            let config = config(&["--request-timeout-ms", "100"]);
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
        });
        assert_eq!(served, Ok(()));
        drop(runtime);
        closed
    }

    #[test]
    fn a_connection_that_sends_nothing_is_closed_within_the_request_timeout() {
        let (closed, waited) = serve_until_closed(b"");
        assert!(closed, "the server closed the idle connection");
        assert!(
            waited >= Duration::from_millis(90) && waited < Duration::from_secs(3),
            "closed after {waited:?}, not at the 100 ms bound"
        );
    }

    #[test]
    fn a_connection_that_stalls_inside_a_head_is_closed_within_two_request_timeouts() {
        let (closed, waited) = serve_until_closed(b"G");
        assert!(closed, "the server closed the stalled connection");
        assert!(
            waited >= Duration::from_millis(190) && waited < Duration::from_millis(1500),
            "closed after {waited:?}, not at twice the 100 ms bound"
        );
    }

    /// What a trickling client saw: whether the server closed the connection, how
    /// long after the first byte, and the longest the client went without sending,
    /// between two bytes or from its latest byte to the close.
    struct Trickle {
        closed: bool,
        waited: Duration,
        longest_silence: Duration,
    }

    /// Connects to `address`, sends `G`, then one more byte of a request line every
    /// 50 ms for three seconds, never ending the line, and returns what it saw,
    /// dropping `stop` then. A reply to the partial head fails the test.
    fn trickle_until_closed(address: SocketAddr, stop: watch::Sender<()>) -> Trickle {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let line = [b"GET /".as_slice(), &[b'a'; 64]].concat();
        let started = Instant::now();
        let mut sent = started;
        let mut longest_silence = Duration::ZERO;
        let mut closed = false;
        for byte in line {
            if started.elapsed() >= Duration::from_secs(3) {
                break;
            }
            longest_silence = longest_silence.max(sent.elapsed());
            if stream.write_all(&[byte]).is_err() {
                closed = true;
                break;
            }
            sent = Instant::now();
            match stream.read(&mut [0; 64]) {
                Ok(0) => closed = true,
                Ok(read) => panic!("the server replied to a partial head with {read} bytes"),
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(error) => {
                    assert_eq!(error.kind(), ErrorKind::ConnectionReset, "{error}");
                    closed = true;
                }
            }
            if closed {
                longest_silence = longest_silence.max(sent.elapsed());
                break;
            }
        }
        let waited = started.elapsed();
        drop(stop);
        Trickle {
            closed,
            waited,
            longest_silence,
        }
    }

    #[test]
    fn a_trickling_head_is_cut_at_the_header_read_deadline() {
        let Trickle {
            closed,
            waited,
            longest_silence,
        } = serve_beside(trickle_until_closed);
        assert!(
            closed,
            "the trickling connection was still open after {waited:?}"
        );
        // A connection is also closed two bounds after the latest byte it read, so
        // the close shows the deadline from the first byte only if the client was
        // never silent that long.
        assert!(
            longest_silence < Duration::from_millis(190),
            "the client was silent for {longest_silence:?}, long enough to be closed \
            two bounds after a later byte"
        );
        assert!(
            waited >= Duration::from_millis(190) && waited < Duration::from_millis(1500),
            "closed after {waited:?}, not at twice the 100 ms bound from the first byte"
        );
    }

    #[test]
    fn a_request_whose_owner_has_stopped_answers_503_not_ready() {
        let body = br#"{"publisher_id": "p", "rank": 1, "payload": ""}"#;
        let head = format!(
            "PUT /pockets/a/records/k HTTP/1.1\r\nhost: test\r\n\
content-type: application/json\r\ncontent-length: {}\r\n\
connection: close\r\nexpect: 100-continue\r\n\r\n",
            body.len()
        );
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
                let mut stream = start_a_request(address, head.as_bytes());
                stop_owner.send_replace(true);
                // `Stopping` is sent once the owner's task has ended.
                wait_for_readiness(&readiness, Readiness::Stopping);
                stream.write_all(body).unwrap();
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
        assert!(
            reply.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "{reply:?}"
        );
        assert!(
            reply.ends_with(r#"{"error":"not_ready","message":"the queue's owner has stopped"}"#),
            "{reply:?}"
        );
        drop(runtime);
    }

    #[test]
    fn a_handler_slower_than_the_bound_gets_its_408_before_the_idle_bound_closes_it() {
        let runtime = runtime();
        let (served, reply) = runtime.block_on(async {
            let config = config(&["--request-timeout-ms", "100", "--shutdown-grace-ms", "100"]);
            let listener = TcpListener::bind(config.listen()).await.unwrap();
            let address = listener.local_addr().unwrap();
            let (stop, stopped) = watch::channel(());
            let queue = service_queue(&config).unwrap();
            let (lifecycle, ticks) = Lifecycle::new(&config);
            // The real owner is kept but never run, so the command is never answered
            // and the handler outlives the bound; the connection's read stays pending
            // for as long, which must not close it first.
            let (handle, _real_owner) = lifecycle.start(queue, ticks);
            let owner = (handle, std::future::pending());
            let client = tokio::task::spawn_blocking(move || {
                let mut stream = TcpStream::connect(address).unwrap();
                stream.set_read_timeout(Some(BOUND)).unwrap();
                stream
                    .write_all(b"POST /publishers/p HTTP/1.1\r\nhost: test\r\n\r\n")
                    .unwrap();
                let mut reply = [0; 512];
                let read = stream.read(&mut reply).unwrap();
                drop(stop);
                String::from_utf8_lossy(&reply[..read]).into_owned()
            });
            let server = scope(listener, owner, lifecycle, stopped, std::future::pending());
            let client = async { client.await.unwrap() };
            timeout(BOUND, async { tokio::join!(server, client) })
                .await
                .expect("serve returned within the bound")
        });
        assert!(
            reply.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
            "{reply:?}"
        );
        assert!(reply.contains(r#""error":"request_timeout""#), "{reply:?}");
        // The parked owner ignores the cut, so serve ends by the abort.
        assert!(matches!(served, Err(ServeError::Task(_))), "{served:?}");
        drop(runtime);
    }

    /// Serves with a request timeout of 500 ms while a client sends the first byte
    /// of a publish's head, the rest of the head `pause` later, and never the body,
    /// returning what `serve` returned and the reply the client read.
    fn publish_with_a_slow_head(pause: Duration) -> (Result<(), ServeError>, String) {
        let body = br#"{"publisher_id": "p", "rank": 1, "payload": ""}"#;
        let rest = format!(
            "UT /pockets/a/records/k HTTP/1.1\r\nhost: test\r\n\
content-type: application/json\r\ncontent-length: {}\r\n\r\n",
            body.len()
        );
        let runtime = runtime();
        let (served, reply) = runtime.block_on(async {
            let config = config(&["--request-timeout-ms", "500"]);
            let listener = TcpListener::bind(config.listen()).await.unwrap();
            let address = listener.local_addr().unwrap();
            let (stop, stopped) = watch::channel(());
            let client = tokio::task::spawn_blocking(move || {
                let mut stream = TcpStream::connect(address).unwrap();
                stream.set_read_timeout(Some(BOUND)).unwrap();
                stream.write_all(b"P").unwrap();
                std::thread::sleep(pause);
                stream.write_all(rest.as_bytes()).unwrap();
                let mut reply = [0; 512];
                let read = stream.read(&mut reply).unwrap();
                drop(stop);
                String::from_utf8_lossy(&reply[..read]).into_owned()
            });
            let queue = service_queue(&config).unwrap();
            let (lifecycle, ticks) = Lifecycle::new(&config);
            let owner = lifecycle.start(queue, ticks);
            let server = scope(listener, owner, lifecycle, stopped, std::future::pending());
            let client = async { client.await.unwrap() };
            timeout(BOUND, async { tokio::join!(server, client) })
                .await
                .expect("serve returned within the bound")
        });
        drop(runtime);
        (served, reply)
    }

    #[test]
    fn a_head_within_the_bound_still_gets_its_408_before_the_idle_bound_closes_it() {
        // The head arrives slowly but within the bound, and the body never comes, so
        // the handler overruns waiting for it: its 408 one bound after the head
        // (about 600 ms) comes before the idle bound's cut, two bounds after the
        // first byte (1000 ms).
        let (served, reply) = publish_with_a_slow_head(Duration::from_millis(100));
        assert!(
            reply.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
            "{reply:?}"
        );
        assert!(reply.contains(r#""error":"request_timeout""#), "{reply:?}");
        assert_eq!(served, Ok(()));
    }

    #[test]
    fn a_head_slower_than_the_bound_is_answered_400_when_its_body_read_fails_at_the_deadline() {
        // The head ends after the bound (about 750 ms), and the body never comes: the
        // idle bound fails the handler's body read two bounds after the first byte
        // (1000 ms), before its 408 would come one bound after the head (1250 ms).
        let (served, reply) = publish_with_a_slow_head(Duration::from_millis(750));
        assert!(
            reply.starts_with("HTTP/1.1 400 Bad Request\r\n"),
            "{reply:?}"
        );
        assert!(reply.contains(r#""error":"bad_request""#), "{reply:?}");
        assert_eq!(served, Ok(()));
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
        assert!(*cut.borrow(), "the connections were cut");
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
        assert!(*cut.borrow(), "the connections were cut");
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
        assert!(*cut.borrow(), "the connections were cut");
        assert_eq!(tasks.len(), 0, "the parked task was aborted");
    }
}
