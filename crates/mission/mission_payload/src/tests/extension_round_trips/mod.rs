//! Round trips of every authored extension block through the payload compiler.
//!
//! **Role:** groups the payload tests that compile a mission carrying one authored block and read
//! it back through the extension block registry, one file per block.
//! **Position:** a child of the payload tests; the blocks' parse and validate tests stay with the
//! mission model, which never depends on the compiler.
//! **Signals & state:** none.
//! **Invariants:** every file names the compiler, the registry and `serde_json` through this
//! module's imports.

use crate::compile_payload;
use mission_model::authored_blocks::{AUTHORED_BLOCKS, ExtensionBlocks, copy_authored_blocks};
use serde_json::{Value, json};

mod audio;
mod radio_plan;
mod spawn_modules;
mod tactical_graphics;
mod tasks;
mod weather_timeline;
