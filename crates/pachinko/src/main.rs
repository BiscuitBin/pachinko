//! The pachinko binary: one call to [`pachinko::run`] with the arguments, on
//! standard output; an error is one line on standard error and exit code 1.

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    match pachinko::run(std::env::args().skip(1), &mut std::io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Nothing is left to report a failed write of the report to.
            let _unreported = writeln!(std::io::stderr().lock(), "pachinko: {error}");
            ExitCode::FAILURE
        }
    }
}
