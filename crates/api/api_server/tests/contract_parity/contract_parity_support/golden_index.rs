//! The frontend golden corpus as `_index.tsv` declares it.
//!
//! **Role:** parses `contracts/fixtures/api_goldens/_index.tsv` into [`GoldenRow`]s and
//! reads each row's golden and stored request body; lists the corpus directory so the index can be
//! checked against it, with the folder's two index documents (`_index.tsv` and the `README.md`
//! that describes the folder) set apart from the goldens.
//!
//! **Position:** the index is read at test runtime, so a golden added with its index row is covered
//! by every case of `tests/contract_parity/goldens.rs` without a code change.
//!
//! **Signals & state:** none; every call reads the committed files afresh.
//!
//! **Invariants:** rows keep index order (the order the corpus was captured in, reads before the
//! writes that change them); the method is the file name's prefix before `__`; a write's body is
//! the sibling `<stem>.request.json`, and its absence means the request carries no body.

use std::path::PathBuf;

use serde_json::Value;

/// The index file inside the corpus directory.
pub(crate) const INDEX_FILE: &str = "_index.tsv";
/// Suffix of a stored request body; such files are inputs, not goldens.
pub(crate) const REQUEST_BODY_SUFFIX: &str = ".request.json";
/// Suffix of an event-stream golden (the stream's leading frames as received).
pub(crate) const EVENT_STREAM_SUFFIX: &str = ".sse.txt";
/// Suffix of a JSON golden.
pub(crate) const JSON_SUFFIX: &str = ".json";

/// The committed corpus directory.
pub(crate) fn corpus_dir() -> PathBuf {
    repository_root::find_repository_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the repository root above the API package")
        .join("contracts/fixtures/api_goldens")
}

/// One `_index.tsv` row: `status<TAB>path<TAB>file<TAB>size`.
#[derive(Clone, Debug)]
pub(crate) struct GoldenRow {
    /// The status the route answered with at capture.
    pub status: u16,
    /// Request path, possibly with a query string.
    pub path: String,
    /// Golden file name inside the corpus directory.
    pub file: String,
}

impl GoldenRow {
    /// The request method: the file name's prefix before `__`.
    pub(crate) fn method(&self) -> &str {
        self.file.split_once("__").map_or("", |(method, _)| method)
    }

    /// Whether the golden holds an event stream's leading frames.
    pub(crate) fn is_event_stream(&self) -> bool {
        self.file.ends_with(EVENT_STREAM_SUFFIX)
    }

    /// The file name without its golden suffix.
    pub(crate) fn stem(&self) -> &str {
        self.file
            .strip_suffix(EVENT_STREAM_SUFFIX)
            .or_else(|| self.file.strip_suffix(JSON_SUFFIX))
            .unwrap_or(&self.file)
    }

    /// The name of the request body a write sends, when one is stored.
    pub(crate) fn request_body_file(&self) -> String {
        format!("{}{REQUEST_BODY_SUFFIX}", self.stem())
    }

    /// The golden's committed bytes.
    ///
    /// # Errors
    /// Names the file when it cannot be read.
    pub(crate) fn read_bytes(&self) -> Result<Vec<u8>, String> {
        let path = corpus_dir().join(&self.file);
        std::fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))
    }

    /// The golden parsed as JSON.
    ///
    /// # Errors
    /// Names the file when it cannot be read or is not JSON.
    pub(crate) fn read_json(&self) -> Result<Value, String> {
        serde_json::from_slice(&self.read_bytes()?)
            .map_err(|error| format!("{} is not JSON: {error}", self.file))
    }

    /// The stored request body, or `None` when the request carries none.
    ///
    /// # Errors
    /// Names the file when it exists but cannot be read.
    pub(crate) fn read_request_body(&self) -> Result<Option<Vec<u8>>, String> {
        let path = corpus_dir().join(self.request_body_file());
        if !path.exists() {
            return Ok(None);
        }
        std::fs::read(&path)
            .map(Some)
            .map_err(|error| format!("read {}: {error}", path.display()))
    }
}

/// Every `_index.tsv` row in index order; a malformed row panics naming its line.
pub(crate) fn read_index() -> Vec<GoldenRow> {
    let path = corpus_dir().join(INDEX_FILE);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            let columns: Vec<&str> = line.split('\t').collect();
            let [status, request_path, file, _size] = columns.as_slice() else {
                panic!(
                    "{INDEX_FILE}:{}: expected status, path, file and size separated by tabs, \
                     got {line:?}",
                    index + 1
                );
            };
            GoldenRow {
                status: status.parse().unwrap_or_else(|_| {
                    panic!(
                        "{INDEX_FILE}:{}: status {status:?} is not a number",
                        index + 1
                    )
                }),
                path: (*request_path).to_string(),
                file: (*file).to_string(),
            }
        })
        .collect()
}
