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

#[test]
fn a_command_line_it_does_not_read_exits_non_zero_with_the_reason_on_stderr() {
    let output = Command::new(env!("CARGO_BIN_EXE_pachinko"))
        .args(["serve", "--bogus", "1"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"pachinko: usage: unknown flag --bogus\n");
}

#[cfg(unix)]
#[test]
fn serve_exits_zero_on_sigterm() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let mut child = Command::new(env!("CARGO_BIN_EXE_pachinko"))
        .args(["serve", "--listen", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    assert!(ready.starts_with("listening on 127.0.0.1:"), "{ready:?}");
    let killed = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(killed.success());
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "serve did not exit after SIGTERM"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(status.code(), Some(0));
}
