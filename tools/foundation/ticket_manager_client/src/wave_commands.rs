//! The wave-level calls of [`TicketManager`].
//!
//! **Role:** one method per `ttm wave` command the tools use: `show`, `repack`, `check`, `close`,
//! `history` and `collisions`.
//! **Position:** the platform and mod wave drivers read the plan and close waves here; the
//! preflight checks the plan.
//! **Signals & state:** none held; each method spawns `ttm` once.
//! **Invariants:** the ticket manager owns the wave plan and the close ledger, so no call here
//! writes a file in the checkout or makes a commit; a project without a wave plan is the
//! `not_found` refusal ([`crate::Error::is_not_found`]), never an empty plan.

use crate::error::Result;
use crate::ticket_commands::strings;
use crate::ticket_manager::{FailingVerdict, TicketManager};
use crate::wave_documents::{
    CollisionsDocument, RepackDocument, WaveCheckDocument, WaveCloseDocument, WaveHistoryDocument,
    WavePlan,
};

impl TicketManager {
    /// `ttm wave show`: the open and pending-close waves.
    pub fn wave_show(&self) -> Result<WavePlan> {
        self.document(&strings(&["wave", "show"]), FailingVerdict::Refused)
    }

    /// `ttm wave repack [--reserve <reference>…]`: recompiles the wave plan from the tickets.
    pub fn wave_repack(&self, reserve: &[String]) -> Result<RepackDocument> {
        let mut args = strings(&["wave", "repack"]);
        if !reserve.is_empty() {
            args.push("--reserve".to_string());
            args.extend(reserve.iter().cloned());
        }
        self.document(&args, FailingVerdict::Refused)
    }

    /// `ttm wave check`: whether the stored plan agrees with the tickets and the close ledger. A
    /// plan with findings is a document with `ok` false, not an error.
    pub fn wave_check(&self) -> Result<WaveCheckDocument> {
        self.document(
            &strings(&["wave", "check"]),
            FailingVerdict::CarriesDocument,
        )
    }

    /// `ttm wave close <n> --sha <sha> [--members <reference>…]`: records wave `n` closed at the
    /// marker commit `sha`. Idempotent for the same wave and sha; refuses a different sha.
    pub fn wave_close(&self, n: u32, sha: &str, members: &[String]) -> Result<WaveCloseDocument> {
        let label = n.to_string();
        let mut args = strings(&["wave", "close", &label, "--sha", sha]);
        if !members.is_empty() {
            args.push("--members".to_string());
            args.extend(members.iter().cloned());
        }
        self.document(&args, FailingVerdict::Refused)
    }

    /// `ttm wave history <n> [--as-of <instant>]`: wave `n`'s membership and each member's status,
    /// now and at `as_of` (RFC 3339).
    pub fn wave_history(&self, n: u32, as_of: Option<&str>) -> Result<WaveHistoryDocument> {
        let label = n.to_string();
        let mut args = strings(&["wave", "history", &label]);
        if let Some(as_of) = as_of {
            args.extend(strings(&["--as-of", as_of]));
        }
        self.document(&args, FailingVerdict::Refused)
    }

    /// `ttm wave collisions`: the next disjoint dispatch set.
    pub fn wave_collisions(&self) -> Result<CollisionsDocument> {
        self.document(&strings(&["wave", "collisions"]), FailingVerdict::Refused)
    }
}
