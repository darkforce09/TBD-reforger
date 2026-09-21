//! Ticket files on disk: one `T-<id>.toml` per ticket, encoded, written and read back.

use anyhow::{Result, bail};

use serde_json::{Map, Value};

use std::fs;
use std::path::{Path, PathBuf};

mod encoding;

pub use encoding::{
    derive_next_id, is_parent_id, parent_numeric_id, parent_toml_path, root_marker_path,
    ticket_from_toml_str, ticket_to_toml_string, tickets_dir,
};

mod storage;

pub use storage::{corpus_ids, load_toml_tree, on_disk_ids, save_toml_tree};

mod key_contract;

pub use key_contract::{ALLOWED_NEW, ENCODING_C_KEYS, FROZEN_27, union_ticket_keys};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
