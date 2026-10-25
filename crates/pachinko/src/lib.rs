//! The pachinko service, built around `libpachinko`.
//!
//! It supplies the real implementations of the library's effect traits once the
//! specifications name them. Today it reports which version of the library it was
//! built with. The binary's `main` calls [`run`] and nothing else.

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
