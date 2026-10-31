//! The pachinko binary: one call to [`pachinko::run`] with the command line and
//! standard output, its result turned into the exit code by [`pachinko::exit_code`].

use std::process::ExitCode;

fn main() -> ExitCode {
    pachinko::exit_code(
        pachinko::run(std::env::args().skip(1), &mut std::io::stdout().lock()),
        &mut std::io::stderr().lock(),
    )
}
