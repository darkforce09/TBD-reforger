//! The names a caller imports with `use ci_task_catalog::prelude::*;`.

pub use crate::error::{Error, Result, cause_chain};
pub use crate::task_runner::{Lane, Step, TASKS, Task};
