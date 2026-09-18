//! T-440 / T-478 — the faction-library seed pin (T-853 port of
//! `scripts/mod/verify-t440-faction-library-seed.sh`).
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Class-R. Three facts must hold together, and the script's own header says why each is there:
//!
//! > Wave 10 / residual adversarial: cold/schema gates validate faction-library.sample.json but
//! > never pin that `make seed` applies apps/website/api/seeds/faction_library.sql. Deleting that
//! > Makefile seed line still greens the cold gate.
//!
//! So: (1) the seed file carries a **live** `INSERT INTO user_factions` naming the starter BLUFOR
//! faction `'US Army 1980s'` (T-256), (2) the seeder **applies** it, and (3) `wave.sh` invokes
//! this gate from **both** of its gate paths.
//!
//! ── T-897: PIN 2'S SUBJECT MOVED OFF THE MAKEFILE ────────────────────────────────────────────
//!
//! Pin 2 used to parse the `Makefile` `seed:` recipe and require a real shell redirect
//! (`< seeds/faction_library.sql`), because a recipe is TEXT and text can name a file it never
//! applies — hence RED 2 (`echo …path… >/dev/null`) and RED 2b (the path inside a psql `-c` SQL
//! comment). T-897 deleted the Makefile; the successor is [`crate::commands::db::operations::SEEDS`], the const
//! `cargo xtask db seed` iterates. That retires both smuggles as a CLASS: a `&[&str]` has no
//! commented-out members and no echo form, so an entry is either applied or absent. The RED arms
//! move with the subject — they now perturb the LIST (drop the entry; park a look-alike beside
//! it) so the gate still proves it bites on every run rather than asserting that it would.
//!
//! ── WHY EACH PIN IS SHAPED THE WAY IT IS (T-478, wave 29 THIS-WAVE BLOCKER) ───────────────────
//!
//! The first version of this gate was false-green. Carried verbatim from the script it replaces:
//!
//! > (1) raw `grep 'US Army 1980s'` PASS'd `-- US Army 1980s` + `SELECT 1;`;
//! > (2) path substring on any non-# recipe line PASS'd `echo seeds/faction_library.sql
//! >     >/dev/null` and psql `-c` comment smuggles;
//! > (3) script never pinned wave.sh cold (`cmd_gate`) + slice (`gate_slice`) wiring.
//! > Cure: strip SQL `--` / `/* */` before name pin; require live INSERT INTO user_factions that
//! > includes `'US Army 1980s'` as a string literal; require a recipe line with shell redirect
//! > `< seeds/faction_library.sql` (reject echo); pin wave.sh both gate paths invoke this script.
//! > RED→GREEN on perturbations.
//!
//! That last sentence is why this file is twice the size of an ordinary pin: the gate does not
//! merely assert, it **proves it bites** on every run. [`verify_t440`] runs the whole pin set six
//! times — once against the live tree, once against each of four deliberately-broken variants
//! that must FAIL, and once more against the live tree to show nothing was clobbered. T-556 calls
//! this the anti-vacuity discipline; T-462's registry note records the defect class it exists to
//! kill ("deleting the seed line or emptying the SQL still greens the cold gate").
//!
//! OWNS WIDEN (carried from the script): wave_plan T-440/T-478 lists `Makefile` + `wave.sh` +
//! `faction_library.sql`; this is the Class-R perturbation guard, same spirit as T-437/T-444/T-472.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`, entirely — four call sites.** One heredoc implementing the pins, three more
//!    doing RED setup by string-replacing a file. The script is on `scripts/python-inventory.txt`
//!    solely for those. Everything they did is `regex` + `str` here, so the interpreter dependency
//!    is gone and the inventory line goes with it. (Nothing else in the script shelled out: no
//!    `grep`, no `awk`, no `sed`.)
//! 2. **Two `2>/dev/null` fail-opens on the RED arms.** Each RED proof read
//!    `if assert_t440_pins … 2>/dev/null; then "still passed" else "FAIL (expected)"`. A *crash*
//!    inside the heredoc — unreadable file, `SyntaxError` after an edit, absent `python3` (127) —
//!    exits non-zero and is therefore indistinguishable from "the pin correctly rejected the
//!    perturbation", with the traceback that would have explained it swallowed by the redirect.
//!    On a machine with no `python3` every RED proof printed "→ FAIL (expected)" and the gate
//!    exited 0. Here the pins are a function returning [`Verdict`]s, so "the check reported a
//!    violation" and "the check could not run" are different values and cannot be confused.
//! 3. **Six re-reads of three files, and the `mktemp -d` + `trap` that fed them.** The
//!    perturbations are string transforms of text already in hand, so the port never writes to the
//!    filesystem at all — and the script's standing risk, a temp-path bug scribbling on the live
//!    tree (exactly what its GREEN arm was watching for), stops existing. The GREEN arm still
//!    re-reads from disk, because catching a *concurrent* edit is the other half of its job.
//! 4. **`set -e` turning an unreadable input into a bare status.** An I/O error under the heredoc
//!    aborted with no gate output at all; those are typed `NotRun` causes now.
//!
//! What it does NOT remove: two dead branches and a defeated blank-line filter, documented at
//! their sites. They are bugs, but they are *this gate's* bugs, and a port whose acceptance
//! criterion is a byte-for-byte stdout diff is the wrong commit in which to fix them. See
//! `seed_pin`, `seed_list_pin` and `wave_pin`.
//!
//! ── OUTPUT AND STATUS ARE A CONTRACT ─────────────────────────────────────────────────────────
//!
//! `wave.sh`'s `run()` captures `"$@" 2>&1` and prints `tail -15` of a failed step, so every line
//! emitted below is operator-facing evidence, not decoration. Acceptance for this port was a
//! byte-for-byte stdout+stderr+status diff against the script on a clean tree and on four broken
//! ones. That includes the Python `repr()` of the recipe lines in the RED-2b arm, which is why
//! `py_repr` exists rather than `{:?}` — Rust's `Debug` for `str` escapes `'` and would differ.

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
const SEED_REL: &str = "apps/website/api/seeds/faction_library.sql";
/// The wave driver whose two gate paths must both invoke this gate.
/// T-902 deleted `scripts/platform/wave.sh`; both paths now live in `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`
/// as `VERIFY_STEPS` iterated by `gate_slice` and `cmd_gate`.
const WAVE_REL: &str = "tools_v2/xtask/src/commands/platform/wave_execution/gate.rs";
/// How the rust driver names this gate in `VERIFY_STEPS`. Const + call sites are one atomic
/// change — this gate READS those call sites.
const VERIFY_REL: &str = r#"("T-440 faction library seed", "t440")"#;
/// The starter BLUFOR faction (T-256). Pinned as a SQL *string literal*, not a bare substring.
const STARTER_NAME: &str = "US Army 1980s";

/// RED 2b — a look-alike entry parked beside the real one's absence. The post-T-897 analog of the
/// `echo`/`psql -c` smuggles: it is the only way left to have the seed's NAME in the list without
/// the seed being applied, and equality matching is what refuses it.
const SEED_LOOKALIKE: &str = "faction_library.sql.bak";
/// RED 3 — the `gate_slice` invocation, deleted to prove the dual-path pin is really dual. The
/// trailing newline is part of the needle: the deletion must not leave a blank line behind.
const WAVE_RUN_LINE: &str = "    (\"T-440 faction library seed\", \"t440\"),\n";
const VERIFY_LOOP: &str = "for (label, name) in VERIFY_STEPS";

// ── THE PIN SET ──────────────────────────────────────────────────────────────────────────────

// ── COMMENT STRIPPERS ────────────────────────────────────────────────────────────────────────
//
// Both are transcribed index-for-index from the heredoc, over `Vec<char>` rather than `&[u8]`,
// because Python indexes `str` by code point and a UTF-8 byte walk would land mid-character on the
// box-drawing runs that head every section of this repo's Makefile and scripts.

// ── RED-ARM SETUP ────────────────────────────────────────────────────────────────────────────

// ── OUTPUT AND I/O HELPERS ───────────────────────────────────────────────────────────────────

// ── WHERE THE PINS POINT ─────────────────────────────────────────────────────────────────────
//
// Everything this gate knows about WHERE the pins point lives in the consts at the top. Pin 1 is
// `SEED_REL` + `STARTER_NAME`; pin 2 is `SEED_ENTRY` against `crate::commands::db::operations::SEEDS` (T-897 — it was the
// Makefile `seed:` recipe until then); pin 3 is `WAVE_REL` + `VERIFY_REL` + `WAVE_RUN_LINE`.
// Repointing any of them means changing the const and re-baselining the tests below; nothing in
// this file parses a build file any more.

#[cfg(test)]
#[path = "tests/faction_library_seeds/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_t440;

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
use source_audit::{assert_t440_pins, strip_hash_comments, strip_sql_comments};
