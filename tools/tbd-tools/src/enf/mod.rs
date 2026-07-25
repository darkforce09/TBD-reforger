//! T-181 — the Enfusion oracle toolchain (`enf` bin).
//!
//! Turns two unreadable code piles into queryable indexes:
//!   * `apps/mod/crf_framework` — 266 `.c` / ~71k LOC of a working Reforger event framework
//!     (Arma Public License, reference only, gitignored).
//!   * the vanilla game's shipped scripts, carved out of `addons/data/*.pak` (T-181.3).
//!
//! Indexes are TSV so an agent greps them with `rg` at zero parse cost, and they carry only
//! symbol names and coordinates — never code bodies — so they can be committed without
//! vendoring anything.

pub mod apidoc;
pub mod capability;
pub mod carve;
pub mod citations;
pub mod index;
pub mod source;
pub mod symbols;
