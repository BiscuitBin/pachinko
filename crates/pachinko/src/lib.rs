//! The pachinko service: `libpachinko`'s queue over HTTP (decision 0019).
//!
//! With no argument the binary writes `pachinko <version>`. With `serve [flags]` it
//! runs a tokio HTTP/1.1 JSON server over one in-memory [`libpachinko::Queue`]
//! held by one owner task, which applies every command in arrival order
//! (`docs/reference/http-api.md` has the routes), and stops cleanly on SIGTERM or
//! SIGINT.
//!
//! `GET /readyz` reports whether the owner is serving. `--log-level` sets the log
//! level, `--request-timeout-ms` bounds each request and each idle connection, and
//! `--shutdown-grace-ms` bounds the shutdown: open connections are cut after one
//! grace period and the server's own tasks still running are aborted after a second
//! (see [`serve`]).
//!
//! The service supplies the real implementations of the library's effect traits,
//! [`SystemClock`] and [`EpochSerialLeaseIds`], and owns the runtime, the wire format
//! and the logging. The binary's `main` calls [`run`] and [`exit_code`] and nothing
//! else. See `docs/explanation/architecture.md`.

mod actor;
mod clock;
mod config;
mod error;
mod http;
mod lease_ids;
mod serve;

use std::io::Write;
use std::process::ExitCode;

pub use clock::SystemClock;
pub use config::ServeConfig;
pub use error::ServeError;
pub use lease_ids::EpochSerialLeaseIds;
pub use serve::serve;

/// Runs the command line `args`, without the program name: none writes
/// `pachinko <version>` to `out`; `serve [flags]` serves until a signal stops it
/// (see [`ServeConfig::parse`] for the flags), writing `listening on <address>` to
/// `out` once it is ready.
///
/// # Errors
///
/// [`ServeError::Usage`] for a command line it does not read, and whatever `serve`
/// or writing to `out` fails with.
///
/// ```
/// let mut out = Vec::new();
/// pachinko::run([], &mut out)?;
/// assert_eq!(out, format!("pachinko {}\n", libpachinko::version()).as_bytes());
/// assert!(pachinko::run([String::from("bogus")], &mut out).is_err());
/// # Ok::<(), pachinko::ServeError>(())
/// ```
pub fn run(args: impl IntoIterator<Item = String>, out: &mut impl Write) -> Result<(), ServeError> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        None => version(out),
        Some("serve") => serve::run(&ServeConfig::parse(args)?, out),
        Some(other) => Err(ServeError::Usage(format!(
            "unknown command {other:?}; run with no arguments, or `serve [flags]`"
        ))),
    }
}

/// The process's exit code for `result`, with the error, if any, written to `err` as
/// `pachinko: <error>`.
///
/// ```
/// use std::process::ExitCode;
///
/// let mut err = Vec::new();
/// assert_eq!(pachinko::exit_code(Ok(()), &mut err), ExitCode::SUCCESS);
/// let failed = pachinko::exit_code(Err(pachinko::ServeError::Usage(String::from("x"))), &mut err);
/// assert_eq!(failed, ExitCode::FAILURE);
/// assert_eq!(err, b"pachinko: usage: x\n");
/// ```
pub fn exit_code(result: Result<(), ServeError>, err: &mut impl Write) -> ExitCode {
    result.map_or_else(
        |error| {
            writeln!(err, "pachinko: {error}").ok();
            ExitCode::FAILURE
        },
        |()| ExitCode::SUCCESS,
    )
}

/// Writes `pachinko <version>`.
fn version(out: &mut impl Write) -> Result<(), ServeError> {
    writeln!(out, "pachinko {}", libpachinko::version())
        .map_err(ServeError::io("writing the version"))
}

#[cfg(test)]
mod tests {
    use std::process::ExitCode;

    use super::{ServeError, exit_code, run};

    fn args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| String::from(*arg)).collect()
    }

    #[test]
    fn run_writes_the_name_and_the_library_version() {
        let mut out = Vec::new();
        run([], &mut out).unwrap();
        assert_eq!(
            out,
            format!("pachinko {}\n", libpachinko::version()).as_bytes()
        );
    }

    #[test]
    fn an_unknown_command_or_flag_is_a_usage_error_and_writes_nothing() {
        let mut out = Vec::new();
        assert!(matches!(
            run(args(&["bogus"]), &mut out),
            Err(ServeError::Usage(_))
        ));
        assert_eq!(
            run(args(&["serve", "--bogus", "1"]), &mut out),
            Err(ServeError::Usage(String::from("unknown flag --bogus")))
        );
        assert_eq!(out, b"");
    }

    #[test]
    fn a_listener_that_cannot_bind_is_an_io_error() {
        let mut out = Vec::new();
        let ran = run(args(&["serve", "--listen", "not an address"]), &mut out);
        assert!(
            matches!(
                &ran,
                Err(ServeError::Io {
                    what: "binding the listener",
                    ..
                })
            ),
            "{ran:?}"
        );
        assert_eq!(out, b"");
    }

    #[test]
    fn a_failed_write_is_an_io_error() {
        struct Broken;
        impl std::io::Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("broken pipe"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert_eq!(
            run([], &mut Broken),
            Err(ServeError::Io {
                what: "writing the version",
                message: String::from("broken pipe")
            })
        );
        assert_eq!(exit_code(Ok(()), &mut Broken), ExitCode::SUCCESS);
    }
}
