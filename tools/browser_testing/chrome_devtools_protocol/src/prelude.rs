//! The names a caller imports with `use chrome_devtools_protocol::prelude::*;`.

pub use crate::browser_session::{Browser, GpuBackend, Page, launch, new_page, sleep_ms};
pub use crate::error::{Error, Result};
pub use crate::intercepted_request::InterceptedRequestId;
