//! Why the service stopped or could not start.

use libpachinko::error::ConfigError;

/// A failure of the service: a command line it does not read, a configuration the
/// library refuses, an I/O failure, a task that failed, or an effect that cannot be
/// supplied.
///
/// ```
/// use pachinko::ServeError;
///
/// let error = ServeError::Usage(String::from("unknown flag --bogus"));
/// assert_eq!(error.to_string(), "usage: unknown flag --bogus");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ServeError {
    /// The command line is not one the binary reads; the text says why.
    ///
    /// ```
    /// let error = pachinko::ServeError::Usage(String::from("x"));
    /// assert_eq!(error.to_string(), "usage: x");
    /// ```
    #[error("usage: {0}")]
    Usage(String),
    /// The flags set a configuration the library's config check refuses.
    ///
    /// ```
    /// use libpachinko::error::ConfigError;
    /// let error = pachinko::ServeError::from(ConfigError::MaxPullBatchZero);
    /// assert!(error.to_string().starts_with("invalid configuration: max_pull_batch"));
    /// ```
    #[error("invalid configuration: {0}")]
    Config(#[from] ConfigError),
    /// An I/O operation failed: what the service was doing, and the error's text.
    ///
    /// ```
    /// let error = pachinko::ServeError::Io { what: "binding", message: String::from("in use") };
    /// assert_eq!(error.to_string(), "binding: in use");
    /// ```
    #[error("{what}: {message}")]
    Io {
        /// What the service was doing.
        what: &'static str,
        /// The I/O error as text: `std::io::Error` is not `Clone`, so it travels as
        /// text to keep this type `Clone`.
        message: String,
    },
    /// One of the server's tasks panicked or was cancelled.
    ///
    /// ```
    /// let error = pachinko::ServeError::Task(String::from("the owner panicked"));
    /// assert_eq!(error.to_string(), "server task: the owner panicked");
    /// ```
    #[error("server task: {0}")]
    Task(String),
    /// The system clock reads a time before the Unix epoch.
    ///
    /// ```
    /// let error = pachinko::ServeError::ClockBeforeEpoch;
    /// assert_eq!(error.to_string(), "the system clock is before the Unix epoch");
    /// ```
    #[error("the system clock is before the Unix epoch")]
    ClockBeforeEpoch,
    /// The operating system's random source failed; the text is its error.
    ///
    /// ```
    /// let error = pachinko::ServeError::RandomSource(String::from("no entropy"));
    /// assert_eq!(error.to_string(), "the random source failed: no entropy");
    /// ```
    #[error("the random source failed: {0}")]
    RandomSource(String),
    /// The lease-id serial cannot advance, so the next id would repeat one already
    /// issued.
    ///
    /// ```
    /// let error = pachinko::ServeError::LeaseIdsExhausted;
    /// assert!(error.to_string().contains("exhausted"));
    /// ```
    #[error("the lease-id serial is exhausted; no new lease can be granted")]
    LeaseIdsExhausted,
}

impl ServeError {
    /// A builder of [`ServeError::Io`] with `what`, for `map_err`.
    pub(crate) fn io(what: &'static str) -> impl Fn(std::io::Error) -> Self {
        move |error| Self::Io {
            what,
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ServeError;
    use libpachinko::error::ConfigError;

    #[test]
    fn io_carries_what_was_being_done_and_the_errors_text() {
        let error = ServeError::io("binding")(std::io::Error::other("address in use"));
        assert_eq!(
            error,
            ServeError::Io {
                what: "binding",
                message: String::from("address in use")
            }
        );
        assert_eq!(error.to_string(), "binding: address in use");
    }

    #[test]
    fn a_config_refusal_names_the_invariant() {
        let error = ServeError::from(ConfigError::MaxConsecutiveExpiriesZero);
        assert_eq!(
            error.to_string(),
            "invalid configuration: max_consecutive_expiries must be at least 1 (ExpiryLimitPositive)"
        );
    }
}
