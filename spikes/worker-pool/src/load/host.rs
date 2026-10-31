//! The host facts the table prints above the numbers, read once through
//! `std::process::Command` with "unknown" wherever a command fails.

use std::process::Command;

/// What the host is: enough to say which machine the numbers came from.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Host {
    /// `sw_vers -productVersion` on macOS, `uname -sr` elsewhere.
    pub os: String,
    /// `sysctl -n machdep.cpu.brand_string`.
    pub chip: String,
    /// `sysctl -n hw.ncpu`.
    pub ncpu: Option<u32>,
    /// `sysctl -n hw.perflevel0.physicalcpu`: the performance cores.
    pub perf_cores: Option<u32>,
    /// `sysctl -n hw.perflevel1.physicalcpu`: the efficiency cores.
    pub efficiency_cores: Option<u32>,
    /// `std::thread::available_parallelism`, 1 when it fails.
    pub parallelism: usize,
    /// `git rev-parse --short HEAD`, or "unknown".
    pub commit: String,
}

impl Host {
    /// A host that knows only its parallelism; the rest "unknown".
    ///
    /// ```
    /// use worker_pool::load::Host;
    /// let host = Host::new(4).with_perf_cores(Some(2));
    /// assert_eq!(host.parallelism, 4);
    /// assert_eq!(host.perf_cores, Some(2));
    /// assert_eq!(host.os, "unknown");
    /// ```
    #[must_use]
    pub fn new(parallelism: usize) -> Self {
        Self {
            os: "unknown".to_owned(),
            chip: "unknown".to_owned(),
            ncpu: None,
            perf_cores: None,
            efficiency_cores: None,
            parallelism: parallelism.max(1),
            commit: "unknown".to_owned(),
        }
    }

    /// The same host with this performance-core count.
    #[must_use]
    pub const fn with_perf_cores(mut self, perf_cores: Option<u32>) -> Self {
        self.perf_cores = perf_cores;
        self
    }

    /// Reads the host: the commands above, each falling back to "unknown" or `None`.
    #[must_use]
    pub fn detect() -> Self {
        let parallelism = std::thread::available_parallelism().map_or(1, usize::from);
        let os = if cfg!(target_os = "macos") {
            command("sw_vers", &["-productVersion"]).map(|v| format!("macOS {v}"))
        } else {
            command("uname", &["-sr"])
        };
        Self {
            os: os.unwrap_or_else(|| "unknown".to_owned()),
            chip: sysctl("machdep.cpu.brand_string").unwrap_or_else(|| "unknown".to_owned()),
            ncpu: sysctl("hw.ncpu").and_then(|v| v.parse().ok()),
            perf_cores: sysctl("hw.perflevel0.physicalcpu").and_then(|v| v.parse().ok()),
            efficiency_cores: sysctl("hw.perflevel1.physicalcpu").and_then(|v| v.parse().ok()),
            parallelism,
            commit: command("git", &["rev-parse", "--short", "HEAD"])
                .unwrap_or_else(|| "unknown".to_owned()),
        }
    }
}

/// `sysctl -n name`, trimmed; `None` when the command fails or the key is absent.
fn sysctl(name: &str) -> Option<String> {
    command("sysctl", &["-n", name])
}

/// A command's trimmed stdout when it ran and exited 0 and printed something.
fn command(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{Host, command};

    #[test]
    fn a_failing_command_is_none() {
        assert_eq!(command("this-command-does-not-exist-t20", &[]), None);
        assert_eq!(command("false", &[]), None);
    }

    #[test]
    fn detect_always_has_a_parallelism() {
        let host = Host::detect();
        assert!(host.parallelism >= 1);
        assert_ne!(host.os, "");
    }
}
