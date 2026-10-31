//! The server: one structured task scope over the queue (decision 0019).
//!
//! [`serve`] starts three tasks in one [`JoinSet`]: the HTTP listener, the sweep that
//! fires `LeaseLapses` and the decay rules on the clock, and the shutdown signal. The
//! first to end, for whatever reason, tells the others to stop; all of them are
//! joined before `serve` returns, and the first error is the one returned. axum serves
//! each connection on a task of its own and the listener waits for those; a connection
//! still open when the grace period ends is cut, so that wait ends too, and nothing is
//! aborted. No task is spawned anywhere else, and none outlives `serve`.

use std::io::{self, Write};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use libpachinko::Queue;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::{JoinError, JoinSet};
use tokio::time::{MissedTickBehavior, interval, timeout};

use crate::http::{SharedQueue, router, with_queue};
use crate::{Error, RandomLeaseIds, ServeConfig, SystemClock};

/// How long the tasks have to finish once shutdown starts before open connections
/// are cut.
const GRACE: Duration = Duration::from_secs(10);

/// What each task of the scope ends with.
type TaskResult = Result<(), Error>;

/// Runs `serve` on a multi-thread runtime until a signal stops it: the `serve`
/// command. Writes `listening on <address>` to `out` once the listener is bound and
/// the signal handlers are installed, and logs to standard error from then on.
pub(crate) fn run(config: &ServeConfig, out: &mut impl Write) -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .try_init()
        .ok();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(Error::io("building the runtime"))?;
    runtime.block_on(async {
        let listener = TcpListener::bind(config.listen())
            .await
            .map_err(Error::io("binding the listener"))?;
        let signals = install_signals()?;
        announce(&listener, out)?;
        let (_never_closed, stop) = watch::channel(());
        scope(listener, config, stop, signals, GRACE).await
    })
}

/// Serves the REST API on `listener` over a fresh, empty queue until a shutdown
/// signal arrives, `stop`'s sender is dropped, or a task fails.
///
/// The queue reads [`SystemClock`] and draws [`RandomLeaseIds`]. Every `sweep` it
/// runs `lapse` and then `decay`, logging what each did.
///
/// # Errors
///
/// The first error of the listener, the sweep or the signal handling, or
/// [`Error::Task`] when a task panicked or did not finish within the grace period, in
/// which case the connections still open were cut before `serve` returned.
///
/// ```no_run
/// # async fn example() -> Result<(), pachinko::Error> {
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
) -> Result<(), Error> {
    let signals = install_signals()?;
    scope(listener, config, stop, signals, GRACE).await
}

fn announce(listener: &TcpListener, out: &mut impl Write) -> Result<(), Error> {
    let address = listener
        .local_addr()
        .map_err(Error::io("reading the bound address"))?;
    writeln!(out, "listening on {address}")
        .and_then(|()| out.flush())
        .map_err(Error::io("writing the bound address"))
}

/// The task scope.
async fn scope(
    listener: TcpListener,
    config: &ServeConfig,
    stop: watch::Receiver<()>,
    signals: impl Future<Output = ()> + Send + 'static,
    grace: Duration,
) -> Result<(), Error> {
    let queue: SharedQueue = Arc::new(Mutex::new(Queue::new(
        config.queue().clone(),
        SystemClock,
        RandomLeaseIds::default(),
    )));
    let (shutdown, shutting_down) = watch::channel(false);
    let (cut, cutting) = watch::channel(false);
    let connections = Connections {
        listener,
        cut: cutting,
    };
    let mut tasks = JoinSet::new();
    tasks.spawn(listen(
        connections,
        Arc::clone(&queue),
        shutting_down.clone(),
    ));
    tasks.spawn(sweep(queue, config.sweep(), shutting_down.clone()));
    tasks.spawn(signal(signals, stop, shutting_down));
    let first = tasks.join_next().await.map_or(Ok(()), task_result);
    shutdown.send_replace(true);
    let rest = drain(&mut tasks, &cut, grace).await;
    first.and(rest)
}

/// A joined task's own result, or why it did not finish.
fn task_result(joined: Result<TaskResult, JoinError>) -> TaskResult {
    joined.unwrap_or_else(|error| Err(Error::Task(error.to_string())))
}

/// Joins every remaining task. Past the grace period every open connection is cut,
/// which ends the listener's wait for them at their next read or write, and the tasks
/// are joined again, without a bound: nothing is aborted, so a connection busy in
/// code that never reads or writes holds `serve` until it finishes, rather than
/// outliving it.
async fn drain(
    tasks: &mut JoinSet<TaskResult>,
    cut: &watch::Sender<bool>,
    grace: Duration,
) -> TaskResult {
    if let Ok(joined) = timeout(grace, join_all(tasks)).await {
        return joined;
    }
    cut.send_replace(true);
    join_all(tasks).await.ok();
    Err(Error::Task(format!(
        "shutdown took longer than {}ms; open connections were cut",
        grace.as_millis()
    )))
}

/// Joins every task in `tasks`, keeping the first error.
async fn join_all(tasks: &mut JoinSet<TaskResult>) -> TaskResult {
    let mut first = Ok(());
    while let Some(joined) = tasks.join_next().await {
        first = first.and(task_result(joined));
    }
    first
}

/// The HTTP listener, until shutdown starts; in-flight requests finish first, unless
/// their connections are cut.
async fn listen(
    connections: Connections,
    queue: SharedQueue,
    shutting_down: watch::Receiver<bool>,
) -> TaskResult {
    axum::serve(connections, router(queue))
        .with_graceful_shutdown(shutdown_started(shutting_down))
        .await
        .map_err(Error::io("serving HTTP"))
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

/// The sweep: `lapse` then `decay` every `every`, until shutdown starts.
async fn sweep(
    queue: SharedQueue,
    every: Duration,
    mut shutting_down: watch::Receiver<bool>,
) -> TaskResult {
    let mut ticks = interval(every);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = ticks.tick() => sweep_once(&queue),
            _ = shutting_down.wait_for(|stopping| *stopping) => return Ok(()),
        }
    }
}

/// One pass of the sweep, logging every lapse and any decay.
fn sweep_once(queue: &SharedQueue) {
    let (lapsed, decayed) = with_queue(queue, |q| (q.lapse(), q.decay()));
    for lapse in &lapsed {
        tracing::info!(
            lease = %lapse.lease_id,
            consumer = %lapse.consumer,
            pocket = %lapse.pocket_key,
            fate = ?lapse.fate,
            "lease lapsed"
        );
    }
    if decayed.superseded > 0 || decayed.dead_letters > 0 {
        tracing::info!(
            superseded = decayed.superseded,
            dead_letters = decayed.dead_letters,
            "decayed"
        );
    }
}

/// Ends on SIGTERM or SIGINT (ctrl-c), when `stop`'s sender is dropped or sent to,
/// or when another task has started the shutdown.
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

async fn shutdown_started(mut shutting_down: watch::Receiver<bool>) {
    drop(shutting_down.wait_for(|stopping| *stopping).await);
}

/// Installs the handlers for the process signals that stop the server, SIGTERM and
/// SIGINT (ctrl-c), and returns the future that waits for either. They are
/// installed before the address is announced, so that a signal sent the moment the
/// server is ready is not lost.
#[cfg(unix)]
fn install_signals() -> Result<impl Future<Output = ()> + Send + 'static, Error> {
    use tokio::signal::unix::{SignalKind, signal};
    let mut terminate = signal(SignalKind::terminate()).map_err(Error::io("handling SIGTERM"))?;
    let mut interrupt = signal(SignalKind::interrupt()).map_err(Error::io("handling SIGINT"))?;
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
fn install_signals() -> Result<impl Future<Output = ()> + Send + 'static, Error> {
    Ok(async {
        drop(tokio::signal::ctrl_c().await);
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::{self, ErrorKind, Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::pin::pin;
    use std::sync::mpsc;
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    use tracing::Metadata;
    use tracing::subscriber::DefaultGuard;
    use tracing_subscriber::fmt::MakeWriter;

    use tokio::net::TcpListener;
    use tokio::sync::watch;

    use tokio::time::timeout;

    use super::scope;
    use crate::{Error, ServeConfig};

    /// A publish whose head promises a 2-byte body, asking to be told when the handler
    /// starts reading it.
    const HEAD: &[u8] = b"PUT /pockets/a/records/k HTTP/1.1\r\nhost: test\r\n\
        content-type: application/json\r\ncontent-length: 2\r\n\
        expect: 100-continue\r\n\r\n";
    const CONTINUE: &[u8] = b"HTTP/1.1 100 Continue\r\n\r\n";

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
            let server = scope(
                listener,
                &config,
                stopped,
                std::future::pending(),
                Duration::from_millis(100),
            );
            tokio::join!(server, client)
        });
        assert!(matches!(served, Err(Error::Task(_))), "{served:?}");
        // `serve` has returned while the runtime lives on: finishing the body must not
        // reach a handler. The connection must be closed, not merely silent: a read that
        // times out fails the test.
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

    /// A gate a log line from `pachinko::http` waits at until it opens, reporting
    /// once that a line is waiting.
    #[derive(Clone)]
    struct HeldLog {
        open: Arc<(Mutex<bool>, Condvar)>,
        waiting: mpsc::Sender<()>,
    }

    impl HeldLog {
        /// Reports that a line is waiting, then waits until the gate opens.
        fn hold(&self) {
            self.waiting.send(()).ok();
            let open = self.open.0.lock().unwrap();
            drop(self.open.1.wait_while(open, |open| !*open).unwrap());
        }

        fn release(&self) {
            *self.open.0.lock().unwrap() = true;
            self.open.1.notify_all();
        }
    }

    /// One log line's writer: held at the gate when `held` is set.
    struct Line(Option<HeldLog>);

    impl Write for Line {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if let Some(log) = &self.0 {
                log.hold();
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for HeldLog {
        type Writer = Line;

        fn make_writer(&'a self) -> Line {
            Line(None)
        }

        fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Line {
            Line((meta.module_path() == Some("pachinko::http")).then(|| self.clone()))
        }
    }

    thread_local! {
        /// Each runtime thread's own subscriber, so no other test's logs are held.
        static LOG: RefCell<Option<DefaultGuard>> = const { RefCell::new(None) };
    }

    /// Sends a complete request, waits until its connection is stuck writing the
    /// request's log line, where cutting the connection cannot reach it, then drops
    /// `stop`.
    fn stick_a_connection(
        address: SocketAddr,
        stop: watch::Sender<()>,
        waiting: &mpsc::Receiver<()>,
    ) -> TcpStream {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nhost: test\r\n\r\n")
            .unwrap();
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(stop);
        stream
    }

    #[test]
    fn serve_does_not_return_while_a_connection_ignoring_the_cut_still_runs() {
        let (waiting, waited) = mpsc::channel();
        let log = HeldLog {
            open: Arc::new((Mutex::new(false), Condvar::new())),
            waiting,
        };
        let subscriber_log = log.clone();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .on_thread_start(move || {
                let subscriber = tracing_subscriber::fmt()
                    .with_writer(subscriber_log.clone())
                    .finish();
                let guard = tracing::subscriber::set_default(subscriber);
                LOG.with(|log| *log.borrow_mut() = Some(guard));
            })
            .build()
            .unwrap();
        let (early, served, stream) = runtime.block_on(async {
            let config = ServeConfig::parse(["--listen", "127.0.0.1:0"]).unwrap();
            let listener = TcpListener::bind(config.listen()).await.unwrap();
            let address = listener.local_addr().unwrap();
            let (stop, stopped) = watch::channel(());
            let grace = Duration::from_millis(50);
            let mut server = pin!(scope(
                listener,
                &config,
                stopped,
                std::future::pending(),
                grace
            ));
            let client = async {
                tokio::task::block_in_place(|| stick_a_connection(address, stop, &waited))
            };
            // Ten grace periods: both phases of the drain, many times over.
            let early = async { timeout(grace * 10, server.as_mut()).await.ok() };
            let (early, stream) = tokio::join!(early, client);
            log.release();
            let outcome = match early {
                Some(_) => None,
                None => Some(server.await),
            };
            (early, outcome, stream)
        });
        assert_eq!(
            early, None,
            "serve returned while a connection was still running"
        );
        assert!(matches!(served, Some(Err(Error::Task(_)))), "{served:?}");
        drop((stream, runtime));
    }
}
