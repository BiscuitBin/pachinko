//! The pachinko service, built around `libpachinko`.
//!
//! It supplies the real implementations of the library's effect traits: the system
//! clock in [`clock`], and an Arrow record store in [`arrow`], where each pocket's
//! records are one `RecordBatch` (decision 0020). The binary's `main` calls [`run`]
//! and nothing else.

pub mod arrow;
pub mod clock;

use std::io::{self, Write};

/// Writes the service's one line, `pachinko <version>`, to `out`.
///
/// # Errors
///
/// Returns the error `out` gives if the line cannot be written.
pub fn run(out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "pachinko {}", libpachinko::version())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn run_writes_the_name_and_the_library_version() {
        let mut out = Vec::new();
        run(&mut out).unwrap();
        assert_eq!(
            out,
            format!("pachinko {}\n", libpachinko::version()).as_bytes()
        );
    }
}
