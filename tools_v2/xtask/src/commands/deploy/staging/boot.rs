//! THE BOOT VERDICT.
//!
//! ── WHAT WAS BROKEN ──────────────────────────────────────────────────────────────────────────
//!
//! This script built two ExecStarts and neither gave a server that was both correct and joinable:
//!
//! | mode | flags | outcome |
//! |------|-------|---------|
//! | config | `-config`, no `-addonsDir` | registers a room, resolves the mod from the WORKSHOP — not from the checkout it just rsynced |
//! | addons | `-addonsDir` + `-addons` + `-server` | loads the checkout, registers NO backend room |
//!
//! The first is the expensive one and it is this program's signature defect wearing the engine's
//! clothes: **staging was validating a build it did not deploy.** `tbd-framework` is published
//! unlisted under the SAME id as the local gproj GUID (`B2C3D4E5F6A78901`), so `game.mods[]` is
//! satisfiable from the Workshop and the engine quietly does that. The deploy rsyncs a checkout to
//! the host, symlinks it into `$TBD_ADDONS_STAGING`, and then launches a server that never looks
//! at it. Every "staging is green" verdict since the June publish was a true statement about the
//! WRONG code.
//!
//! THE FIX: `-addonsDir <dir>` **plus** `-config <json>` does both at once. The
//! 2026-06-14 "mutually exclusive" finding was measured on `-addons`, which really is fatal with
//! `-config`; `-addonsDir` is a different flag and combines with it fine.
//!
//! ⚠ THE FORMAT CHECK NO LONGER DISCRIMINATES HERE. `cargo xtask mod remote-logs` separates builds
//! by counting `[TBD][` lines — stale Workshop 1.0.1 emits 0, any current build emits many. That
//! was sound while the Workshop copy was June's. The operator re-published on 2026-07-31, so the
//! Workshop now serves **1.0.2**, which is current-format: measured on a real `-config`-only boot
//! (2026-08-01 00:12, profile pak 570,489 B) that log carries **154** `[TBD][` lines and would
//! sail through the format threshold while running a pak the deploy never produced. The format
//! check answers "is this build ancient", which is a different question from "is this build the
//! one I just deployed". Only the gproj PATH answers the second. Do not "simplify" it into one —
//! [`selftest()`] asserts the non-redundancy as an executable statement.
//!
//! Pure functions over a log FILE on purpose: the deploy half needs ssh and a live host, and a
//! check that can only run during a real deploy is a check nobody runs. `--verify-boot` and
//! `--verify-boot-selftest` exercise every line of this logic with no ssh, no `deploy.env` and no
//! staging host.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use regex::Regex;

use super::Paths;

/// Output sink. `None` writes through to the real streams; `Some(buf)` accumulates BOTH streams
/// into one string, which is what the bash's `out="$(verify_boot_log … 2>&1)"` did.
///
/// This exists so [`selftest()`] can assert on the reporter's own text without re-executing the
/// process. The bash could only do that by shelling out to itself; capturing here keeps the
/// verdict a pure function of (log, guid, addons_dir, admin_count, profile_dir, rival_bytes).
pub struct Out {
    buf: Option<String>,
}

impl Out {
    pub fn streams() -> Out {
        Out { buf: None }
    }
    pub fn captured() -> Out {
        Out {
            buf: Some(String::new()),
        }
    }
    pub fn text(&self) -> &str {
        self.buf.as_deref().unwrap_or("")
    }
    /// stdout
    fn o(&mut self, line: impl AsRef<str>) {
        match self.buf {
            Some(ref mut b) => {
                let _ = writeln!(b, "{}", line.as_ref());
            }
            None => println!("{}", line.as_ref()),
        }
    }
    /// stderr
    fn e(&mut self, line: impl AsRef<str>) {
        match self.buf {
            Some(ref mut b) => {
                let _ = writeln!(b, "{}", line.as_ref());
            }
            None => eprintln!("{}", line.as_ref()),
        }
    }
}

// ── the selftest ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/boot/tests.rs"]
mod tests;

mod read_addon_guid;
pub use read_addon_guid::assert_admins_configured;
pub use read_addon_guid::assert_local_addon_won;
pub use read_addon_guid::assert_room_registered;
pub use read_addon_guid::read_addon_guid;
pub use read_addon_guid::verify_boot_cli;
pub use read_addon_guid::verify_boot_log;

mod selftest;
pub use selftest::selftest;

#[cfg(test)]
use read_addon_guid::grep_after;
