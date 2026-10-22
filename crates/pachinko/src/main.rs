//! The pachinko binary: one call to [`pachinko::run`] on standard output.

use std::process::ExitCode;

fn main() -> ExitCode {
    pachinko::run(&mut std::io::stdout().lock()).map_or(ExitCode::FAILURE, |()| ExitCode::SUCCESS)
}
