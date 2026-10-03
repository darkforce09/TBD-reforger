//! Why the relay refused its settings, could not start, stopped, could not carry an exchange, or
//! got no status from a running relay.
//!
//! - **Role:** the crate's [`Error`], its [`Result`] alias and [`error_chain`], the one-line
//!   rendering of an error and every cause beneath it.
//! - **Position:** returned by the settings parsers, [`crate::start`], [`crate::serve`],
//!   [`crate::send_control_command`] and the upstream forwarding; the command line prints it to
//!   standard error and exits 1, and the relay's log writes a forwarding failure with
//!   [`error_chain`].
//! - **Signals & state:** none; plain data.
//! - **Invariants:** a flag is echoed only once it is known to carry no user part, so no message
//!   holds a password; the `Authorization` header never reaches a message; each message embeds
//!   the text of the failure beneath it, so the printed line is the whole explanation.

use std::error::Error as StandardError;
use std::io;

/// Why the relay or a control command failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A flag, a request target or a control command is refused, or the control socket's path is
    /// taken.
    #[error("{reason}")]
    Refused {
        /// What is refused and why.
        reason: String,
    },
    /// A file, socket or runtime operation failed.
    #[error("{action}: {error}")]
    Io {
        /// What the relay was doing, such as `cannot listen on 127.0.0.1:18085`.
        action: String,
        /// The failure; its text is part of this error's own message.
        error: io::Error,
    },
    /// A socket operation of a control exchange failed.
    #[error(transparent)]
    Transport(io::Error),
    /// The listener or the control socket stopped serving on its own.
    #[error("{what} stopped")]
    Stopped {
        /// `the relay listener` or `the control socket`.
        what: &'static str,
    },
    /// The task serving the relay panicked or was cancelled.
    #[error("the relay task failed: {error}")]
    TaskFailed {
        /// The join failure; its text is part of this error's own message.
        error: tokio::task::JoinError,
    },
    /// The upstream client could not be built.
    #[error("cannot build the upstream client: {error}")]
    UpstreamClientNotBuilt {
        /// The build failure; its text is part of this error's own message.
        error: reqwest::Error,
    },
    /// The upstream did not answer an exchange, or its answer could not be read whole.
    #[error(transparent)]
    Upstream(#[from] reqwest::Error),
    /// A running relay's answer is not a control answer.
    #[error("the relay on {socket} answered no status: {error}")]
    StatusUndecodable {
        /// The control socket, as given.
        socket: String,
        /// The decoding failure; its text is part of this error's own message.
        error: serde_json::Error,
    },
    /// The relay's status could not be encoded as JSON.
    #[error(transparent)]
    StatusUnencodable(serde_json::Error),
}

impl Error {
    /// A refusal with the text `reason`.
    pub(crate) fn refused(reason: impl Into<String>) -> Self {
        Self::Refused {
            reason: reason.into(),
        }
    }

    /// The failure `error` of what `action` describes.
    pub(crate) fn io(action: impl Into<String>, error: io::Error) -> Self {
        Self::Io {
            action: action.into(),
            error,
        }
    }
}

/// The result of a relay operation.
pub type Result<T> = std::result::Result<T, Error>;

/// `error` and each cause beneath it, joined by `: `, on one line.
pub fn error_chain(error: &dyn StandardError) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}
