//! The pachinko service, built around `libpachinko`.
//!
//! It supplies the real implementations of the library's effect traits: the system
//! clock in [`clock`], and an Arrow record store in [`arrow`], where each pocket's
//! records are one `RecordBatch` (decision 0020). [`serve`] runs the queue behind an
//! Arrow Flight API on tokio (decision 0019): one task owns the queue, a ticker
//! processes lapses and decay, and both live in a `JoinSet` that `serve` joins
//! before it returns. The binary's `main` calls [`run`] and nothing else.

pub mod arrow;
pub mod clock;

mod actor;
mod cli;
mod flight;

use std::io::Write;
use std::time::Duration;

use libpachinko::config::Config;
use libpachinko::error::ConfigError;
use libpachinko::queue::Queue;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

pub use cli::CliError;

use clock::SystemClock;

/// Runs the command line `args`, the arguments after the program's name, writing
/// to `out`.
///
/// With no arguments it writes the service's one line, `pachinko <version>`. With
/// `serve [OPTIONS]` it binds the listener, writes `listening on <address>`, and
/// runs [`serve`] on a multi-thread runtime until an interrupt (Ctrl-C) arrives.
/// The options are `--listen ADDR` (default `127.0.0.1:0`), `--lease-ttl`,
/// `--superseded-ttl` and `--dead-letter-ttl` (each a duration such as `250ms`,
/// `30s`, `5m` or `2h`), and `--max-consecutive-expiries` and `--max-pull-batch`
/// (whole numbers); any option left out keeps the module's default.
///
/// # Errors
///
/// A [`CliError`] for an argument it cannot read, a configuration
/// [`Config::validate`] rejects, a listener it cannot bind, a line it cannot
/// write, or a failure of [`serve`].
///
/// ```
/// let mut out = Vec::new();
/// pachinko::run(Vec::new(), &mut out).unwrap();
/// assert_eq!(out, format!("pachinko {}\n", libpachinko::version()).as_bytes());
/// assert!(pachinko::run(["frobnicate".to_owned()], &mut out).is_err());
/// ```
pub fn run(args: impl IntoIterator<Item = String>, out: &mut impl Write) -> Result<(), CliError> {
    match cli::read_args(args)? {
        cli::Invocation::Version => write_version(out),
        cli::Invocation::Serve(serve_args) => serve_until_interrupted(serve_args, out),
    }
}

/// How [`serve`] runs, beyond the queue's [`Config`]: today, how often the ticker
/// processes lapses and decay.
///
/// A lapse is processed up to one tick after its lease's expiry. The default is
/// one second; a zero interval ticks every millisecond.
///
/// ```
/// use pachinko::ServeOptions;
/// use std::time::Duration;
///
/// assert_eq!(ServeOptions::default().tick_interval, Duration::from_secs(1));
/// let options = ServeOptions::default().with_tick_interval(Duration::from_millis(10));
/// assert_eq!(options.tick_interval, Duration::from_millis(10));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ServeOptions {
    /// How often the ticker sends `Tick` to the queue's owner.
    pub tick_interval: Duration,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(1),
        }
    }
}

impl ServeOptions {
    /// These options with the ticker's interval set to `tick_interval`.
    ///
    /// ```
    /// use pachinko::ServeOptions;
    /// use std::time::Duration;
    ///
    /// let options = ServeOptions::default().with_tick_interval(Duration::ZERO);
    /// assert_eq!(options.tick_interval, Duration::ZERO);
    /// ```
    #[must_use]
    pub const fn with_tick_interval(self, tick_interval: Duration) -> Self {
        Self { tick_interval }
    }
}

/// Why [`serve`] failed.
///
/// ```
/// use libpachinko::error::ConfigError;
/// use pachinko::ServeError;
///
/// let error = ServeError::Config(ConfigError::PullBatchNotPositive);
/// assert_eq!(error.to_string(), "invalid configuration: max_pull_batch must be at least 1");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ServeError {
    /// The configuration breaks one of the module's config invariants.
    #[error("invalid configuration: {0}")]
    Config(#[from] ConfigError),
    /// The gRPC server failed while it served.
    #[error("the gRPC server failed: {0}")]
    Transport(String),
    /// A task of the service panicked or was cancelled.
    #[error("a service task failed: {0}")]
    Task(String),
}

/// Serves the queue `config` describes over Arrow Flight on `listener` until
/// `shutdown` resolves.
///
/// One task owns the queue and a ticker sends it `Tick` every
/// [`ServeOptions::tick_interval`]; both are spawned into a `JoinSet` this function
/// owns. On shutdown tonic finishes its graceful shutdown first, then the ticker
/// stops, the owner drains the commands already sent and ends, and the set is
/// joined before this function returns.
///
/// # Errors
///
/// [`ServeError::Config`] before anything starts when `config` is invalid,
/// [`ServeError::Transport`] when the server fails, and [`ServeError::Task`] when a
/// task panicked; the tasks are joined in every case but the first.
///
/// ```
/// use libpachinko::config::Config;
/// use pachinko::{ServeOptions, serve};
///
/// let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
/// let served = runtime.block_on(async {
///     let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
///     // A shutdown that has already resolved: serve starts, stops and joins.
///     serve(Config::default(), listener, ServeOptions::default(), async {}).await
/// });
/// assert_eq!(served, Ok(()));
/// ```
pub async fn serve(
    config: Config,
    listener: TcpListener,
    options: ServeOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), ServeError> {
    let queue = Queue::new(config, SystemClock)?;
    let mut tasks = JoinSet::new();
    let started = actor::start(&mut tasks, queue, options.tick_interval);
    let served = flight::serve_flight(started.queue, listener, shutdown).await;
    drop(started.stop_ticker);
    let joined = actor::join_tasks(tasks).await;
    served.and(joined)
}

/// Writes `pachinko <version>`.
fn write_version(out: &mut impl Write) -> Result<(), CliError> {
    writeln!(out, "pachinko {}", libpachinko::version())
        .map_err(|error| CliError::Output(error.to_string()))
}

/// Builds a multi-thread runtime, binds the listener, writes and flushes the
/// `listening on` line, and serves until an interrupt.
fn serve_until_interrupted(args: cli::ServeArgs, out: &mut impl Write) -> Result<(), CliError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| CliError::Runtime(error.to_string()))?;
    runtime.block_on(async move {
        let listen = |error: std::io::Error| CliError::Listen {
            address: args.listen.to_string(),
            reason: error.to_string(),
        };
        let listener = TcpListener::bind(args.listen).await.map_err(listen)?;
        let address = listener.local_addr().map_err(listen)?;
        writeln!(out, "listening on {address}")
            .and_then(|()| out.flush())
            .map_err(|error| CliError::Output(error.to_string()))?;
        let options = ServeOptions::default();
        serve(args.config, listener, options, interrupted())
            .await
            .map_err(CliError::Serve)
    })
}

/// Resolves on the first interrupt. When no interrupt handler can be installed it
/// never resolves, so the service runs until it is killed rather than stopping at
/// once.
async fn interrupted() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}

#[cfg(test)]
mod tests {
    use super::{CliError, ServeError, ServeOptions, run};
    use std::io::{self, Write};
    use std::time::Duration;

    /// A writer that refuses every write.
    struct Refusing;

    impl Write for Refusing {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("refused"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn run_writes_the_name_and_the_library_version() {
        let mut out = Vec::new();
        run(Vec::new(), &mut out).unwrap();
        assert_eq!(
            out,
            format!("pachinko {}\n", libpachinko::version()).as_bytes()
        );
    }

    #[test]
    fn a_version_line_that_cannot_be_written_is_an_output_error() {
        let error = run(Vec::new(), &mut Refusing).unwrap_err();
        assert!(matches!(error, CliError::Output(_)));
    }

    #[test]
    fn serve_reports_an_address_it_cannot_bind() {
        let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = taken.local_addr().unwrap().to_string();
        let args = ["serve", "--listen", &address].map(str::to_owned);
        let error = run(args, &mut Vec::new()).unwrap_err();
        assert!(matches!(error, CliError::Listen { address: bound, .. } if bound == address));
    }

    #[test]
    fn serve_reports_a_listening_line_it_cannot_write() {
        let args = ["serve"].map(str::to_owned);
        let error = run(args, &mut Refusing).unwrap_err();
        assert!(matches!(error, CliError::Output(_)));
    }

    #[test]
    fn the_default_tick_is_one_second() {
        assert_eq!(
            ServeOptions::default().tick_interval,
            Duration::from_secs(1)
        );
    }

    #[test]
    fn serve_errors_describe_themselves() {
        let error = ServeError::Transport("closed".to_owned());
        assert_eq!(error.to_string(), "the gRPC server failed: closed");
        let error = ServeError::Task("panicked".to_owned());
        assert_eq!(error.to_string(), "a service task failed: panicked");
    }
}
