//! The names a caller imports with `use api_configuration::prelude::*;`.

pub use crate::configuration::settings_file::{
    SETTINGS_FILE, SETTINGS_FILE_VARIABLE, load_settings_file,
};
pub use crate::configuration::{Config, ConfigError};
pub use crate::process_lifecycle::{ShutdownSignal, process_shutdown};
