//! Ticket files on disk: one `T-<id>.toml` per ticket, encoded, written and read back.
//!
//! **Role:** the key-order-preserving TOML encoding of an untyped ticket file, the whole-folder
//! load and save over it, the ticket path helpers, and the governed ticket key sets.
//! **Position:** under [`crate::registry`]; `load_registry` uses it for untyped folders, the typed
//! projection, the title lookup and the status history use its path helpers and decoder, and
//! `save_registry` writes untyped trees with [`save_toml_tree`].
//! **Signals & state:** none; reads and writes the files it is pointed at.
//! **Invariants:** a ticket written and read back reproduces its JSON value with its key order;
//! every key on disk is in [`ENCODING_C_KEYS`] or [`ALLOWED_NEW`].

use serde_json::{Map, Value};

use std::fs;
use std::path::{Path, PathBuf};

mod encoding;

pub use encoding::{
    derive_next_id, parent_toml_path, root_marker_path, ticket_from_toml_str,
    ticket_to_toml_string, tickets_dir,
};

mod storage;

pub use storage::{corpus_ids, load_toml_tree, on_disk_ids, save_toml_tree};

mod key_contract;

pub use key_contract::{ALLOWED_NEW, ENCODING_C_KEYS, FROZEN_27, union_ticket_keys};
