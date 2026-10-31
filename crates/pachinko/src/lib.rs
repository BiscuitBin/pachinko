//! The pachinko service: `libpachinko`'s queue over HTTP.
//!
//! With no argument the binary writes `pachinko <version>`, the library's version.
//! With `serve [flags]` it runs a tokio HTTP server over one in-memory
//! [`libpachinko::Queue`], whose routes are the `Publishing` and `Consuming`
//! surfaces of `docs/specs/pachinko.allium` (`docs/reference/http-api.md`), with a
//! sweep that fires the clock's rules and a clean shutdown on SIGTERM or ctrl-c.
//!
//! The service supplies the real implementations of the library's effect traits,
//! [`SystemClock`] and [`RandomLeaseIds`], and owns the runtime, the wire format and
//! the logging. The binary's `main` calls [`run`] and [`exit_code`] and nothing else.

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
pub use error::Error;
pub use lease_ids::RandomLeaseIds;
pub use serve::serve;

/// What the command line asks for.
enum Command {
    Version,
    Serve(ServeConfig),
}

/// Runs the command line `args`, without the program name: nothing writes
/// `pachinko <version>` to `out`; `serve [flags]` serves until a signal stops it
/// (see [`ServeConfig::parse`] for the flags), writing `listening on <address>` to
/// `out` once it is ready.
///
/// # Errors
///
/// [`Error::Usage`] for a command line it does not read, and whatever `serve` or
/// writing to `out` fails with.
///
/// ```
/// let mut out = Vec::new();
/// pachinko::run([], &mut out).unwrap();
/// assert_eq!(out, format!("pachinko {}\n", libpachinko::version()).as_bytes());
/// assert!(pachinko::run([String::from("bogus")], &mut out).is_err());
/// ```
pub fn run(args: impl IntoIterator<Item = String>, out: &mut impl Write) -> Result<(), Error> {
    match command(args)? {
        Command::Version => version(out),
        Command::Serve(config) => serve::run(&config, out),
    }
}

/// The process's exit code for `result`, with the error, if any, written to `err`.
///
/// ```
/// use std::process::ExitCode;
/// let mut err = Vec::new();
/// assert_eq!(pachinko::exit_code(Ok(()), &mut err), ExitCode::SUCCESS);
/// let failed = pachinko::exit_code(Err(pachinko::Error::Usage(String::from("x"))), &mut err);
/// assert_eq!(failed, ExitCode::FAILURE);
/// assert_eq!(err, b"pachinko: usage: x\n");
/// ```
pub fn exit_code(result: Result<(), Error>, err: &mut impl Write) -> ExitCode {
    result.map_or_else(
        |error| {
            writeln!(err, "pachinko: {error}").ok();
            ExitCode::FAILURE
        },
        |()| ExitCode::SUCCESS,
    )
}

fn command(args: impl IntoIterator<Item = String>) -> Result<Command, Error> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        None => Ok(Command::Version),
        Some("serve") => ServeConfig::parse(args).map(Command::Serve),
        Some(other) => Err(Error::Usage(format!(
            "unknown command {other:?}; run with no arguments, or `serve [flags]`"
        ))),
    }
}

fn version(out: &mut impl Write) -> Result<(), Error> {
    writeln!(out, "pachinko {}", libpachinko::version()).map_err(Error::io("writing the version"))
}

#[cfg(test)]
mod tests {
    use std::process::ExitCode;

    use super::{Error, exit_code, run};

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
    fn an_unknown_command_or_flag_is_a_usage_error() {
        let mut out = Vec::new();
        let usage =
            |args: &[&str], out: &mut Vec<u8>| run(args.iter().map(|a| String::from(*a)), out);
        assert!(matches!(usage(&["bogus"], &mut out), Err(Error::Usage(_))));
        assert!(matches!(
            usage(&["serve", "--bogus", "1"], &mut out),
            Err(Error::Usage(_))
        ));
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
            Err(Error::Io {
                context: "writing the version",
                message: String::from("broken pipe")
            })
        );
        assert_eq!(exit_code(Ok(()), &mut Broken), ExitCode::SUCCESS);
    }
}
