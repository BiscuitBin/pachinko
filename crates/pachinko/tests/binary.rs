//! The built binary, run as a user would run it.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use std::process::Command;

#[test]
fn the_binary_prints_its_line_and_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_pachinko"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        format!("pachinko {}\n", libpachinko::version()).as_bytes()
    );
    assert_eq!(output.stderr, b"");
}
