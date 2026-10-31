//! The built binary, run as a user would run it.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// How long `serve` may take to write its `listening on` line before the test fails
/// instead of hanging; the line normally arrives in milliseconds.
const STARTUP_DEADLINE: Duration = Duration::from_secs(30);

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
    let stdout = child.stdout.take().unwrap();
    let (sent, received) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut line = String::new();
        let read = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        // After the deadline the test no longer reads the result, so its send is moot.
        let _unsent = sent.send(read);
    });
    let read = received.recv_timeout(STARTUP_DEADLINE);
    // Stop the child before any assertion, so neither a failure nor the deadline
    // leaves it running.
    child.kill().unwrap();
    child.wait().unwrap();
    // The kill closed the pipe, so the reader has reached the end and returns.
    reader.join().unwrap();
    let line = read
        .unwrap_or_else(|_| panic!("no listening line within {STARTUP_DEADLINE:?}"))
        .unwrap();
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
