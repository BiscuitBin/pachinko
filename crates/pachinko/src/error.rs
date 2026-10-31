//! Why the service stopped or could not start.

/// A failure of the service: a bad command line, a refused configuration, an I/O
/// failure, or a task of the server that failed.
///
/// ```
/// use pachinko::Error;
///
/// let error = Error::Usage(String::from("unknown flag --bogus"));
/// assert_eq!(error.to_string(), "usage: unknown flag --bogus");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The command line is not one the service reads; the text says why.
    #[error("usage: {0}")]
    Usage(String),
    /// The queue's configuration breaks a config invariant.
    #[error(transparent)]
    Config(#[from] libpachinko::Error),
    /// An I/O operation failed: what was being done, and the error's text.
    #[error("{context}: {message}")]
    Io {
        /// What the service was doing.
        context: &'static str,
        /// The I/O error, as text, so that the error stays `Clone`.
        message: String,
    },
    /// One of the server's tasks panicked, was cancelled, or outlived the grace
    /// period.
    #[error("server task: {0}")]
    Task(String),
}

impl Error {
    /// An [`Error::Io`] with `context`, for `map_err`.
    pub(crate) fn io(context: &'static str) -> impl Fn(std::io::Error) -> Self {
        move |error| Self::Io {
            context,
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn every_variant_says_what_failed() {
        let io = Error::io("binding")(std::io::Error::other("address in use"));
        assert_eq!(io.to_string(), "binding: address in use");
        assert_eq!(Error::Task(String::from("x")).to_string(), "server task: x");
        let config = Error::from(libpachinko::Error::InvalidConfig("why"));
        assert_eq!(config.to_string(), "invalid configuration: why");
    }
}
