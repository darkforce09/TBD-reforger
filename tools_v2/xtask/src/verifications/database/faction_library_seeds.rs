//! The faction-library seed pin.
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Class-R. The cold and schema gates validate `faction-library.sample.json`, but nothing else
//! pins that the seeder actually applies `apps/website/api_v2/seeds/faction_library.sql` — so
//! deleting the seed from the seeder's list greens every one of them.
//!
//! Three facts must hold together: (1) the seed file carries a **live**
//! `INSERT INTO user_factions` naming the starter BLUFOR faction `'US Army 1980s'`, (2) the seeder
//! **applies** it, and (3) both wave gate drivers invoke this gate.
//!
//! ── THE SUBJECT IS THE LIST THE SEEDER WALKS ─────────────────────────────────────────────────
//!
//! Pin 2 reads [`crate::commands::db::operations::SEEDS`], the const `cargo xtask db seed`
//! iterates. A text subject would have to defend against a path named in a comment or echoed to
//! `/dev/null`; a `&[&str]` has no commented-out members and no echo form, so an entry is either
//! applied or absent, and that whole smuggle class is retired rather than guarded.
//!
//! ── THE GATE PROVES IT BITES, ON EVERY RUN ───────────────────────────────────────────────────
//!
//! Three false-green shapes this pin set is shaped to refuse:
//!
//! 1. a raw match on `US Army 1980s` passing on `-- US Army 1980s` plus `SELECT 1;`;
//! 2. a path substring on any list-like line passing on an echo or a commented-out entry;
//! 3. wiring pinned into one gate driver only, so the other path drifts green.
//!
//! So: SQL `--` and `/* */` are stripped before the name pin; a live `INSERT INTO user_factions`
//! including `'US Army 1980s'` as a string literal is required; membership is by equality on the
//! seed list; and both `gate_slice` and `cmd_gate` must carry the row.
//!
//! That is why this file is twice the size of an ordinary pin. [`verify_faction_library_seeds`]
//! runs the whole pin set six times — once against the live tree, once against each of four
//! deliberately-broken variants that must FAIL, and once more against the live tree to show
//! nothing was clobbered.
//!
//! ── RED ARMS CANNOT FAIL OPEN ────────────────────────────────────────────────────────────────
//!
//! The pins are a function returning [`Verdict`]s, so "the check reported a violation" and "the
//! check could not run" are different values and cannot be confused — a RED arm that could not
//! run does not read as "the pin correctly rejected the perturbation". Perturbations are string
//! transforms of text already in hand, so nothing is ever written to the filesystem and no temp
//! path can scribble on the live tree. The GREEN arm still re-reads from disk, because catching a
//! *concurrent* edit is the other half of its job. An unreadable input is a typed `NotRun` cause,
//! never a bare status.
//!
//! Two dead branches and a defeated blank-line filter are documented at their sites: `seed_pin`,
//! `seed_list_pin` and `wave_pin`.
//!
//! ── OUTPUT AND STATUS ARE A CONTRACT ─────────────────────────────────────────────────────────
//!
//! The wave gate captures a step's stdout+stderr and prints its last 15 lines on failure, so every
//! line emitted below is operator-facing evidence, not decoration. That includes the `repr()`-style
//! rendering of the list lines in the RED-2b arm, which is why `py_repr` exists rather than
//! `{:?}` — Rust's `Debug` for `str` escapes `'` and would differ.

use std::path::Path;

use anyhow::Result;
use verification_core::{Finding, Kind, NotRun, Pattern, Verdict};

use crate::commands::db::operations::SEEDS;

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// The command whose seed list is pinned, for operator-facing prose.
const RECIPE_SOURCE: &str = "cargo xtask db seed";
/// Where that list lives, quoted in failure hints. NAMED, never read: the list arrives as a
/// `&[&str]`, so no arrangement of text in that file can satisfy the gate.
const RECIPE_CONST: &str = "tools_v2/xtask/src/commands/db/operations.rs SEEDS";
/// The [`SEEDS`] entry that must be present. Bare file name (the seeder redirects `seeds/<entry>`),
/// matched by EQUALITY so a parked `faction_library.sql.bak` cannot satisfy the pin.
const SEED_ENTRY: &str = "faction_library.sql";
/// The seed the seeder must apply, repo-relative.
const SEED_REL: &str = "apps/website/api_v2/seeds/faction_library.sql";
/// The wave driver whose two gate paths must both invoke this gate: `VERIFY_STEPS`, iterated by
/// `gate_slice` and `cmd_gate`.
const WAVE_REL: &str = "tools_v2/xtask/src/commands/platform/wave_execution/gate.rs";
/// How the rust driver names this gate in `VERIFY_STEPS`. Const + call sites are one atomic
/// change — this gate READS those call sites.
const VERIFY_REL: &str = r#"("faction library seeds", "faction-library-seeds")"#;
/// The starter BLUFOR faction. Pinned as a SQL *string literal*, not a bare substring.
const STARTER_NAME: &str = "US Army 1980s";

/// RED 2b — a look-alike entry parked beside the real one's absence: the only way left to have
/// the seed's NAME in the list without the seed being applied, and equality matching is what
/// refuses it.
const SEED_LOOKALIKE: &str = "faction_library.sql.bak";
/// RED 3 — the `gate_slice` invocation, deleted to prove the dual-path pin is really dual. The
/// trailing newline is part of the needle: the deletion must not leave a blank line behind.
const WAVE_RUN_LINE: &str = "    (\"faction library seeds\", \"faction-library-seeds\"),\n";
const VERIFY_LOOP: &str = "for (label, name) in VERIFY_STEPS";

// ── THE PIN SET ──────────────────────────────────────────────────────────────────────────────

// ── COMMENT STRIPPERS ────────────────────────────────────────────────────────────────────────
//
// Both walk `Vec<char>` rather than `&[u8]`: a UTF-8 byte walk would land mid-character on the
// box-drawing runs that head every section of this repo's sources.

// ── RED-ARM SETUP ────────────────────────────────────────────────────────────────────────────

// ── OUTPUT AND I/O HELPERS ───────────────────────────────────────────────────────────────────

// ── WHERE THE PINS POINT ─────────────────────────────────────────────────────────────────────
//
// Everything this gate knows about WHERE the pins point lives in the consts at the top. Pin 1 is
// `SEED_REL` + `STARTER_NAME`; pin 2 is `SEED_ENTRY` against
// `crate::commands::db::operations::SEEDS`; pin 3 is `WAVE_REL` + `VERIFY_REL` + `WAVE_RUN_LINE`.
// Repointing any of them means changing the const and re-baselining the tests below.

#[cfg(test)]
#[path = "tests/faction_library_seeds/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_faction_library_seeds;

mod extract_fn_body;
use extract_fn_body::borrow;
use extract_fn_body::delete_first_wave_run;
use extract_fn_body::emit;
use extract_fn_body::emit_labelled;
use extract_fn_body::extract_fn_body;
use extract_fn_body::missing;
use extract_fn_body::py_repr;
use extract_fn_body::read_pair;
use extract_fn_body::red;
use extract_fn_body::seeds_without;

#[cfg(test)]
use source_audit::{assert_faction_library_pins, strip_hash_comments, strip_sql_comments};
