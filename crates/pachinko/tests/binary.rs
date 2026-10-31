//! The built binary, run as a user would run it.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::process::{Command, Stdio};

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

#[test]
fn serve_writes_the_address_it_listens_on() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pachinko"))
        .args(["serve", "--listen", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let read = BufReader::new(child.stdout.take().unwrap()).read_line(&mut line);
    // Stop the child before any assertion, so a failure never leaves it running.
    child.kill().unwrap();
    child.wait().unwrap();
    read.unwrap();
    let address = line
        .strip_prefix("listening on ")
        .and_then(|rest| rest.strip_suffix('\n'))
        .unwrap_or_else(|| panic!("not a listening line: {line:?}"));
    let address: SocketAddr = address.parse().unwrap();
    assert!(address.ip().is_loopback());
    assert_ne!(address.port(), 0);
}

#[test]
fn a_bad_argument_exits_non_zero_with_the_error_on_stderr() {
    let output = Command::new(env!("CARGO_BIN_EXE_pachinko"))
        .args(["serve", "--lease-ttl", "soon"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "pachinko: --lease-ttl does not take soon: it needs a duration such as 250ms, 30s, 5m or 2h\n"
    );
}
