//! The names a caller imports with `use ticket_manager_client::prelude::*;`.

pub use crate::error::{Error as TicketManagerError, Result as TicketManagerResult};
pub use crate::ticket_commands::{RunRecord, TokenCounts};
pub use crate::ticket_manager::TicketManager;
pub use crate::ticket_references::{TicketSlug, is_ticket_reference, parent_slice};
pub use crate::wave_documents::{WavePlan, WaveRow};
