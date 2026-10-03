//! The names a caller imports with `use ticket_registry::prelude::*;`.

pub use crate::registry::{Registry, load_registry};
pub use crate::sync::cmd_sync;
pub use crate::validation::cmd_check;
pub use crate::{Error, OpOutcome, Result};
