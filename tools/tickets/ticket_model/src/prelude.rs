//! The names a caller imports with `use ticket_model::prelude::*;`.

pub use crate::model::{Domain, ProgramTicket, ScopeV2, Status, StatusName, Ticket, WorkTicket};
pub use crate::store::Corpus;
pub use crate::ticket_id::TicketId;
pub use crate::vocab::ScopeVocab;
pub use crate::{TicketFile, parse_ticket_toml, render_ticket_toml};
