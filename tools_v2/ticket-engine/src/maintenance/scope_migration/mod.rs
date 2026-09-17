//! Scope migration for the ticket domain.

use anyhow::{Result, bail};

use crate::{Corpus, ScopeVocab, Ticket, TicketFile, parse_ticket_toml, render_ticket_toml};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod mapping;

use mapping::{
    BACKEND_COMPONENTS, CHROME_MAP, MOD_LAYERS, MappedScope, ROUTE_SURFACES, infer_editor,
    infer_mod, infer_xtask, str_array,
};

mod classification;

use classification::map_scope;

mod migration;

pub use migration::{cmd_migrate_v2, cmd_scope_histogram};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
