use std::{
    collections::VecDeque,
    ffi::{OsStr, OsString},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};
mod streaming;
pub use streaming::*;
mod cargo_discovery;
pub use cargo_discovery::*;
mod bounded_log;
pub use bounded_log::*;
pub(crate) mod external_open;
#[cfg(test)]
#[path = "tests/process.rs"]
mod tests;
