//! The server: one structured task scope over the queue's owner (decision 0019).
//!
//! The scope is one [`JoinSet`] holding three tasks: the queue's owner, the HTTP
//! listener and the shutdown signal. The first to end, for whatever reason, starts
//! the shutdown; all of them are joined before `serve` returns, and the first error
//! is the one returned. The listener stops accepting, lets in-flight requests finish
//! and, once the grace period is over, cuts the connections still open. The owner
//! ends when the listener has dropped the last handle. No task is spawned anywhere
//! else, and none outlives `serve`.

use std::io::{self, Write};
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use libpachinko::queue::Queue;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::{JoinError, JoinSet};
use tokio::time::timeout;

use crate::actor::{self, QueueHandle, ServiceQueue};
use crate::http::{Limits, router};
use crate::{EpochSerialLeaseIds, ServeConfig, ServeError, SystemClock};

/// How long the tasks have to finish once shutdown starts before open connections
/// are cut. T29 makes it `--shutdown-grace-ms` and bounds the final join.
const GRACE: Duration = Duration::from_secs(5);

/// What each task of the scope ends with.
type TaskResult = Result<(), ServeError>;

/// The queue's handle and its owner, still to be spawned.
type Owner<F> = (QueueHandle, F);

/// The listener and the request-shape limits its router enforces (T27).
type Http = (TcpListener, Limits);

/// The `serve` command: installs the log writer on standard error at `info`, builds
/// the runtime and the queue, binds `--listen`, installs the signal handlers, writes
/// `listening on <address>` to `out`, then serves until a signal stops it.
pub(crate) fn run(config: &ServeConfig, out: &mut impl Write) -> Result<(), ServeError> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .ok();
    let (queue, lease_ids) = service_queue(config)?;
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
        let owner = actor::start(queue, lease_ids);
        let http = (listener, Limits::from(config));
        scope(http, owner, stop, signals, GRACE).await
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
/// panicked or did not finish within the grace period, in which case the
/// connections still open were cut before `serve` returned.
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
    let (queue, lease_ids) = service_queue(config)?;
    let signals = install_signals()?;
    scope(
        (listener, Limits::from(config)),
        actor::start(queue, lease_ids),
        stop,
        signals,
        GRACE,
    )
    .await
}

/// The service's queue over the real clock and lease ids, and the owner's clone of
/// the lease-id source.
fn service_queue(config: &ServeConfig) -> Result<(ServiceQueue, EpochSerialLeaseIds), ServeError> {
    let clock = SystemClock::new()?;
    let lease_ids = EpochSerialLeaseIds::new()?;
    let queue = Queue::new(config.queue().clone(), clock, lease_ids.clone())?;
    Ok((queue, lease_ids))
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

/// The task scope over `owner`, the listener under its limits and the signals.
pub(crate) async fn scope<F>(
    (listener, limits): Http,
    owner: Owner<F>,
    stop: watch::Receiver<()>,
    signals: impl Future<Output = ()> + Send + 'static,
    grace: Duration,
) -> TaskResult
where
    F: Future<Output = TaskResult> + Send + 'static,
{
    let (handle, owner) = owner;
    let (shutdown, shutting_down) = watch::channel(false);
    let (cut, cutting) = watch::channel(false);
    let connections = Connections {
        listener,
        cut: cutting,
    };
    let mut tasks = JoinSet::new();
    tasks.spawn(owner);
    tasks.spawn(listen(connections, handle, limits, shutting_down.clone()));
    tasks.spawn(signal(signals, stop, shutting_down));
    let first = tasks.join_next().await.map_or(Ok(()), task_result);
    shutdown.send_replace(true);
    let rest = drain(&mut tasks, &cut, grace).await;
    first.and(rest)
}

/// A joined task's own result, or why it did not finish.
fn task_result(joined: Result<TaskResult, JoinError>) -> TaskResult {
    joined.unwrap_or_else(|error| Err(ServeError::Task(error.to_string())))
}

/// Joins every remaining task, keeping the first error. Past the grace period every
/// open connection is cut, which ends the listener's wait for them at their next read
/// or write, and the tasks are joined again without a bound (T29 bounds it). An error
/// a task ended with inside the grace period is returned; failing that, the grace
/// period's own error. A task that fails after the cut is logged.
async fn drain(
    tasks: &mut JoinSet<TaskResult>,
    cut: &watch::Sender<bool>,
    grace: Duration,
) -> TaskResult {
    let mut first = Ok(());
    if timeout(grace, join_into(tasks, &mut first)).await.is_ok() {
        return first;
    }
    let overran = ServeError::Task(format!(
        "shutdown took longer than {}ms; open connections were cut",
        grace.as_millis()
    ));
    tracing::warn!(error = %overran, "cutting open connections");
    cut.send_replace(true);
    join_logging(tasks).await;
    first.and(Err(overran))
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
    queue: QueueHandle,
    limits: Limits,
    shutting_down: watch::Receiver<bool>,
) -> TaskResult {
    axum::serve(connections, router(queue, limits))
        .with_graceful_shutdown(shutdown_started(shutting_down))
        .await
        .map_err(ServeError::io("serving HTTP"))
}

/// The TCP listener, handing axum connections that `cut` can end.
struct Connections {
    listener: TcpListener,
    cut: watch::Receiver<bool>,
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
            },
            address,
        )
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

/// One accepted connection: its reads and writes fail once it is cut, so the task
/// axum serves it on ends at its next turn.
struct Connection {
    stream: TcpStream,
    /// Ends when the connection is cut; `None` once it has.
    cut: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
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
        Pin::new(&mut this.stream).poll_read(context, buf)
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
        Pin::new(&mut this.stream).poll_write(context, buf)
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
    use std::time::Duration;

    use tokio::net::TcpListener;
    use tokio::sync::watch;
    use tokio::task::JoinSet;
    use tokio::time::timeout;

    use super::{GRACE, Limits, drain, scope, service_queue};
    use crate::{ServeConfig, ServeError, actor, exit_code};

    /// How long a test waits for `serve` before it fails.
    const BOUND: Duration = Duration::from_secs(10);

    /// A publish whose head promises a 2-byte body, asking to be told when the
    /// handler starts reading it.
    const HEAD: &[u8] = b"PUT /pockets/a/records/k HTTP/1.1\r\nhost: test\r\n\
content-type: application/json\r\ncontent-length: 2\r\n\
expect: 100-continue\r\n\r\n";
    const CONTINUE: &[u8] = b"HTTP/1.1 100 Continue\r\n\r\n";

    /// An owner that panics at its first turn.
    async fn panicking_owner() -> Result<(), ServeError> {
        tokio::task::yield_now().await;
        panic!("the owner failed")
    }

    #[tokio::test]
    async fn a_panicked_owner_makes_serve_fail_and_the_exit_code_failure() {
        let config = ServeConfig::parse(["--listen", "127.0.0.1:0"]).unwrap();
        let listener = TcpListener::bind(config.listen()).await.unwrap();
        let (queue, lease_ids) = service_queue(&config).unwrap();
        let (handle, _real_owner) = actor::start(queue, lease_ids);
        let (_stop, stopped) = watch::channel(());
        let owner = (handle, panicking_owner());
        let served = timeout(
            BOUND,
            scope(
                (listener, Limits::from(&config)),
                owner,
                stopped,
                std::future::pending(),
                GRACE,
            ),
        )
        .await
        .unwrap();
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
    async fn a_stop_ends_serve_with_ok() {
        let config = ServeConfig::parse(["--listen", "127.0.0.1:0"]).unwrap();
        let listener = TcpListener::bind(config.listen()).await.unwrap();
        let (stop, stopped) = watch::channel(());
        drop(stop);
        let served = timeout(BOUND, super::serve(listener, &config, stopped)).await;
        assert_eq!(served.unwrap(), Ok(()));
    }

    /// Sends [`HEAD`] and waits until the handler is reading the body, then drops
    /// `stop` with the body still unsent.
    fn stall_a_request(address: SocketAddr, stop: watch::Sender<()>) -> TcpStream {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(HEAD).unwrap();
        let mut continued = vec![0; CONTINUE.len()];
        stream.read_exact(&mut continued).unwrap();
        assert_eq!(continued, CONTINUE, "the handler is reading the body");
        drop(stop);
        stream
    }

    #[test]
    fn a_connection_open_past_the_grace_period_is_cut_before_serve_returns() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (served, mut stream) = runtime.block_on(async {
            let config = ServeConfig::parse(["--listen", "127.0.0.1:0"]).unwrap();
            let listener = TcpListener::bind(config.listen()).await.unwrap();
            let address = listener.local_addr().unwrap();
            let (stop, stopped) = watch::channel(());
            let client =
                async move { tokio::task::block_in_place(|| stall_a_request(address, stop)) };
            let (queue, lease_ids) = service_queue(&config).unwrap();
            let server = scope(
                (listener, Limits::from(&config)),
                actor::start(queue, lease_ids),
                stopped,
                std::future::pending(),
                Duration::from_millis(100),
            );
            timeout(BOUND, async { tokio::join!(server, client) })
                .await
                .expect("serve returned within the bound")
        });
        assert!(matches!(served, Err(ServeError::Task(_))), "{served:?}");
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
        assert!(
            matches!(&drained, Err(ServeError::Task(message)) if message.starts_with("shutdown took longer")),
            "{drained:?}"
        );
        assert_eq!(left, 0, "every task is joined");
    }
}
