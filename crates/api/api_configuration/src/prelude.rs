//! The names a caller imports with `use api_configuration::prelude::*;`.

pub use crate::configuration::{Config, ConfigError};
pub use crate::process_lifecycle::{ShutdownSignal, process_shutdown};
