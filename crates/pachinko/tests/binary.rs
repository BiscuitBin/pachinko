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
    use std::process::{Child, ChildStderr, Command, Stdio};
    use std::sync::mpsc::Receiver;
    use std::time::{Duration, Instant};

    /// How long the server has to answer and to exit before the test fails.
    const BOUND: Duration = Duration::from_secs(15);

    /// Starts `serve` on port 0 with `flags` and `RUST_LOG=trace` in its
    /// environment, and returns the child, the announced address and its standard
    /// error, read to the end on a thread of its own.
    fn start(flags: &[&str]) -> (Child, String, Receiver<String>) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_pachinko"))
            .args(["serve", "--listen", "127.0.0.1:0"])
            .args(flags)
            .env("RUST_LOG", "trace")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = read_to_end(child.stderr.take().unwrap());
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
        (child, address, stderr)
    }

    /// Reads `stderr` to its end on a thread and sends it as text.
    fn read_to_end(mut stderr: ChildStderr) -> Receiver<String> {
        let (sent, read) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut text = String::new();
            stderr
                .read_to_string(&mut text)
                .expect("read the child's standard error");
            sent.send(text).ok();
        });
        read
    }

    /// The child's whole standard error, once it has exited, bounded by [`BOUND`].
    fn stderr_of(stderr: &Receiver<String>) -> String {
        stderr
            .recv_timeout(BOUND)
            .expect("standard error closed within the bound")
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

    /// Sends `signal` to `child` and waits, bounded by `bound`, for its exit code
    /// and how long the exit took.
    fn stop_with(mut child: Child, signal: &str, bound: Duration) -> (Option<i32>, Duration) {
        let sent = Command::new("kill")
            .args([signal, &child.id().to_string()])
            .status()
            .unwrap();
        assert!(sent.success());
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return (status.code(), started.elapsed());
            }
            if started.elapsed() >= bound {
                child.kill().ok();
                panic!("serve did not exit within {bound:?} of {signal}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn serves_then_exits_zero_on(signal: &str) {
        let (child, address, _stderr) = start(&[]);
        let reply = healthz(&address);
        assert!(reply.starts_with("HTTP/1.1 200 OK\r\n"), "{reply:?}");
        assert!(reply.ends_with("\r\n\r\nok"), "{reply:?}");
        // With no connection open, the exit comes well inside the default grace of
        // 5000 ms; the margin covers a loaded machine.
        let (code, took) = stop_with(child, signal, BOUND);
        assert_eq!(code, Some(0));
        assert!(took < Duration::from_secs(7), "exited after {took:?}");
    }

    #[test]
    fn serve_answers_healthz_and_exits_zero_on_sigterm() {
        serves_then_exits_zero_on("-TERM");
    }

    #[test]
    fn serve_answers_healthz_and_exits_zero_on_sigint() {
        serves_then_exits_zero_on("-INT");
    }

    /// How many request lines `serve` with `flags` writes for one `GET /healthz`,
    /// with `RUST_LOG=trace` in its environment, and its whole standard error.
    fn request_lines(flags: &[&str]) -> (usize, String) {
        let (child, address, stderr) = start(flags);
        assert!(healthz(&address).starts_with("HTTP/1.1 200 OK\r\n"));
        assert_eq!(stop_with(child, "-TERM", BOUND).0, Some(0));
        let text = stderr_of(&stderr);
        let lines = text
            .lines()
            .filter(|line| line.contains(" request ") && line.contains("path=\"/healthz\""))
            .count();
        (lines, text)
    }

    #[test]
    fn the_default_level_writes_no_request_line_whatever_rust_log_says() {
        let (lines, text) = request_lines(&[]);
        assert_eq!(lines, 0, "{text}");
        assert!(text.contains("signal received; shutting down"), "{text}");
    }

    #[test]
    fn debug_writes_one_request_line() {
        let (lines, text) = request_lines(&["--log-level", "debug"]);
        assert_eq!(lines, 1, "{text}");
        let line = text
            .lines()
            .find(|line| line.contains(" request "))
            .unwrap();
        for field in ["DEBUG", "method=GET", "status=200", "elapsed_ms="] {
            assert!(line.contains(field), "{field} missing from {line:?}");
        }
    }

    #[test]
    fn sigterm_under_a_stalled_request_exits_non_zero_naming_the_cut() {
        let (child, address, stderr) = start(&["--shutdown-grace-ms", "100"]);
        // A head promising a body that is never sent, after the handler has begun
        // reading it.
        let mut stream = TcpStream::connect_timeout(&address.parse().unwrap(), BOUND).unwrap();
        stream.set_read_timeout(Some(BOUND)).unwrap();
        stream
            .write_all(
                b"PUT /pockets/a/records/k HTTP/1.1\r\nhost: test\r\n\
content-type: application/json\r\ncontent-length: 2\r\nexpect: 100-continue\r\n\r\n",
            )
            .unwrap();
        let mut continued = [0; 25];
        stream.read_exact(&mut continued).unwrap();
        assert_eq!(&continued, b"HTTP/1.1 100 Continue\r\n\r\n");
        let (code, took) = stop_with(child, "-TERM", Duration::from_secs(5));
        assert_eq!(code, Some(1));
        assert!(took < Duration::from_secs(5), "exited after {took:?}");
        let text = stderr_of(&stderr);
        assert!(
            text.contains(
                "pachinko: server task: shutdown took longer than 100ms; open connections were cut"
            ),
            "{text}"
        );
        drop(stream);
    }
}
