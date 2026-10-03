//! The names a caller imports with `use staging_procedures::prelude::*;`.

pub use crate::error::{Error, Result};
pub use crate::remote_actions::host_fixture_commands::CredentialExecutor;
pub use crate::staging_command::{PlanOnly, ProcedureName, RecordSwitch, StagingCmd};
pub use crate::staging_dispatch::run;
