//! Why a browser launch, a page setup or a protocol call failed.
//!
//! **Role:** the crate's [`Error`], its [`Result`] alias, the crate-private `ResultExt` that adds
//! the step that failed to an error or turns a missing value into a refusal, and the
//! crate-private `refusal!` macro that builds a refusal like `format!`.
//! **Position:** returned by every fallible call of the client; `browser_gate_suites` wraps it in
//! its own error.
//! **Signals & state:** none; plain data.
//! **Invariants:** an [`Error::Context`] displays its step alone and exposes the failure
//! underneath as its source, so `{error}` prints the outermost step and a chain walk
//! (`anyhow`'s `{error:#}`) prints `step: cause: …`, each cause exactly once; a wrapped library
//! error displays and chains exactly as that library's error does.

use std::fmt::Display;

/// Why a browser launch, a page setup or a protocol call failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A refusal with its whole explanation.
    #[error("{0}")]
    Message(String),
    /// A failure with the step it interrupted; the failure underneath is its source.
    #[error("{context}")]
    Context {
        /// The step that failed, as the gate output names it.
        context: String,
        /// The failure underneath.
        #[source]
        cause: Box<Error>,
    },
    /// A file, a folder or a child process could not be read, written or started.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A protocol message could not be parsed or written as JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// An HTTP request to the browser's debugging endpoint failed.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// The page's WebSocket could not connect, send or receive.
    #[error(transparent)]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),
    /// A page's socket reader stopped before it answered a call.
    #[error(transparent)]
    ChannelClosed(#[from] tokio::sync::oneshot::error::RecvError),
    /// A screenshot's base64 payload does not decode.
    #[error(transparent)]
    Base64(#[from] base64::DecodeError),
}

impl Error {
    /// A refusal carrying `text` as its whole message.
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Message(text.into())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(error: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(Box::new(error))
    }
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// A refusal ([`Error::Message`]) whose text is built like `format!`.
macro_rules! refusal {
    ($($argument:tt)*) => {
        $crate::error::Error::msg(format!($($argument)*))
    };
}
pub(crate) use refusal;

/// Adds the step that failed to an error, or turns a missing value into a refusal.
pub(crate) trait ResultExt<T> {
    /// Wraps the failure under `context`.
    fn context(self, context: impl Display) -> Result<T>;
    /// Wraps the failure under the context `make` builds, only when there is a failure.
    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T>;
}

impl<T, E: Into<Error>> ResultExt<T> for std::result::Result<T, E> {
    fn context(self, context: impl Display) -> Result<T> {
        self.map_err(|cause| Error::Context {
            context: context.to_string(),
            cause: Box::new(cause.into()),
        })
    }

    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T> {
        self.map_err(|cause| Error::Context {
            context: make().to_string(),
            cause: Box::new(cause.into()),
        })
    }
}

impl<T> ResultExt<T> for Option<T> {
    fn context(self, context: impl Display) -> Result<T> {
        self.ok_or_else(|| Error::Message(context.to_string()))
    }

    fn with_context<C: Display>(self, make: impl FnOnce() -> C) -> Result<T> {
        self.ok_or_else(|| Error::Message(make().to_string()))
    }
}
