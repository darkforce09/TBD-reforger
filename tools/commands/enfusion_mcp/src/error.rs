//! Why a raw Workbench NET API call gave no answer.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the NET API call behind `cargo xtask mcp wbcall` returns it; the command prints
//! it after `wbcall <APIFunc>: ` and maps it to exit 2 (cannot connect) or 3 (any other failure).
//! **Signals & state:** none; plain data.
//! **Invariants:** each message names the endpoint or the response offset it is about; a variant
//! with a cause prints it after `: `, so the printed line is the whole chain.

use std::io;

/// Why a raw Workbench NET API call gave no answer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The response ended before the four-byte length of a string at `at`.
    #[error("response too short for a string length at {at}")]
    ResponseTooShort {
        /// The byte offset of the missing length.
        at: usize,
    },
    /// A string length in the response is negative or runs past the end of the response.
    #[error("response string length {length} at {at} exceeds {total} bytes")]
    ResponseStringOverrun {
        /// The length the response declared.
        length: i32,
        /// The byte offset of the length.
        at: usize,
        /// The whole response's size in bytes.
        total: usize,
    },
    /// The Workbench answered with a status other than `Ok`.
    #[error("Workbench error: {status}")]
    Workbench {
        /// The status string the Workbench sent.
        status: String,
    },
    /// The response payload is not JSON.
    #[error("response JSON: {excerpt}: {source}")]
    ResponseJson {
        /// The payload's first 200 bytes.
        excerpt: String,
        /// The parser's reason.
        source: serde_json::Error,
    },
    /// The Workbench host name could not be resolved.
    #[error("resolve {endpoint}: {source}")]
    Resolve {
        /// The `host:port` asked for.
        endpoint: String,
        /// The resolver's reason.
        source: io::Error,
    },
    /// The Workbench host name resolved to no address.
    #[error("no address for {endpoint}")]
    NoAddress {
        /// The `host:port` asked for.
        endpoint: String,
    },
    /// No connection to the Workbench NET API could be made within the timeout.
    #[error("connect to Workbench NET API at {endpoint}: {source}")]
    Connect {
        /// The `host:port` dialled.
        endpoint: String,
        /// The operating system's reason.
        source: io::Error,
    },
    /// The response could not be read to its end.
    #[error("read response for {api_func}: {source}")]
    ReadResponse {
        /// The NET API function called.
        api_func: String,
        /// The operating system's reason.
        source: io::Error,
    },
    /// Setting a socket timeout, sending the request or closing the write half failed.
    #[error(transparent)]
    Socket(#[from] io::Error),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
