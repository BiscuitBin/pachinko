//! The built binary, run as a user would run it.

// Test-only from the root, so clippy treats the helpers here as test code, as it
// treats the tests: the unwrap, expect and panic allowances of clippy.toml cover them.
#![cfg(test)]

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
fn a_command_line_it_does_not_read_exits_one_with_the_reason_on_stderr() {
    let output = Command::new(env!("CARGO_BIN_EXE_pachinko"))
        .args(["serve", "--bogus", "1"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"pachinko: usage: unknown flag --bogus\n");
}

#[cfg(unix)]
mod serve {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpStream;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    /// How long the server has to answer and to exit before the test fails.
    const BOUND: Duration = Duration::from_secs(15);

    /// Starts `serve` on port 0 and returns the child and the announced address.
    fn start() -> (Child, String) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_pachinko"))
            .args(["serve", "--listen", "127.0.0.1:0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sent, announced) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let read = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            sent.send(read).ok();
        });
        let Ok(Ok(ready)) = announced.recv_timeout(BOUND) else {
            child.kill().ok();
            child.wait().ok();
            panic!("serve did not announce within {BOUND:?}");
        };
        let address = ready
            .strip_prefix("listening on ")
            .unwrap_or_else(|| panic!("no announcement: {ready:?}"))
            .trim_end()
            .to_owned();
        assert!(address.starts_with("127.0.0.1:"), "{address}");
        (child, address)
    }

    /// `GET /healthz` from `address` over a raw connection, bounded by [`BOUND`].
    fn healthz(address: &str) -> String {
        let mut stream = TcpStream::connect_timeout(&address.parse().unwrap(), BOUND).unwrap();
        stream.set_read_timeout(Some(BOUND)).unwrap();
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nhost: test\r\nconnection: close\r\n\r\n")
            .unwrap();
        let mut reply = String::new();
        stream.read_to_string(&mut reply).unwrap();
        reply
    }

    /// Sends `signal` to `child` and waits, bounded, for its exit code.
    fn stop_with(mut child: Child, signal: &str) -> Option<i32> {
        let sent = Command::new("kill")
            .args([signal, &child.id().to_string()])
            .status()
            .unwrap();
        assert!(sent.success());
        let deadline = Instant::now() + BOUND;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status.code();
            }
            if Instant::now() >= deadline {
                child.kill().ok();
                panic!("serve did not exit within {BOUND:?} of {signal}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn serves_then_exits_zero_on(signal: &str) {
        let (child, address) = start();
        let reply = healthz(&address);
        assert!(reply.starts_with("HTTP/1.1 200 OK\r\n"), "{reply:?}");
        assert!(reply.ends_with("\r\n\r\nok"), "{reply:?}");
        assert_eq!(stop_with(child, signal), Some(0));
    }

    #[test]
    fn serve_answers_healthz_and_exits_zero_on_sigterm() {
        serves_then_exits_zero_on("-TERM");
    }

    #[test]
    fn serve_answers_healthz_and_exits_zero_on_sigint() {
        serves_then_exits_zero_on("-INT");
    }
}
