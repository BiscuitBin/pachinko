//! The demo binary: prints its usage until the measurement lands.

use std::io::Write;

/// Writes the usage line.
fn run(out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "worker-pool: the T20 execution-model spike; see spikes/worker-pool/README.md"
    )
}

fn main() -> std::io::Result<()> {
    run(&mut std::io::stdout())
}
