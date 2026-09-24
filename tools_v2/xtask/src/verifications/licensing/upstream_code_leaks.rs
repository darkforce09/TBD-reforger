//! The oracle-leak guard: no upstream reference source may reach the shipping mod.
//!
//! ── THIS GATE SHIPS **RED**, AND THAT IS THE CORRECT STATE ───────────────────────────────────
//!
//! On the tree as committed this exits **1** and prints, under the CRF GUID arm:
//!
//! ```text
//! FAIL: CRF-only asset GUIDs reused (not present in vanilla):
//!   {41174B59DA65659A}
//!   {8A239DEA19509B1B}
//! ```
//!
//! Both are referenced from `apps/mod/tbd-framework/Data/registry.json` (lines 1367 and 1747,
//! measured 2026-08-12) — a Radio virtual-arsenal slot and a Soviet grenade bandolier. Both also
//! appear in `apps/mod/crf_framework`'s own `.layout`/`.et` files, and neither is in any vanilla
//! `.pak`, so the "engine fact" exemption below does not cover them.
//!
//! **That is a real Arma Public License finding for a human to resolve, not a defect in this
//! port.** Do not exempt them, do not allow-list them, do not soften the arm to make CI green: the
//! port is accepted by diffing its stdout against the script's on this very failure, so a green
//! `xtask verify no-crf-leak` would mean the port is broken, not that the licence problem went
//! away. Resolving it is a licence decision — re-author the two prefab references from vanilla, or
//! record an attribution — and it belongs to whoever owns the mod's APL posture.
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Third-party frameworks live on disk as READ-ONLY oracles, and hundreds of files of working code
//! next to a thinner implementation makes copy-paste the path of least resistance — so the script
//! makes the leak a build failure: no `<PREFIX>` identifier in `apps/mod/tbd-framework/**` outside
//! comments, and no GUID an oracle declares in its own `UI/`/`Prefabs/` assets reused in ours.
//! `crf_framework` is Arma Public License — attribution-bearing, but read-never-copy for us: we
//! design-mirror and cite, we do not vendor. `playable_selector` has **NO LICENCE AT ALL**, which
//! is strictly *worse* than APL: with no grant, default copyright applies and there is no
//! permission to copy, adapt or redistribute any of it. The command keeps the too-narrow name
//! `no-crf-leak` because the gate steps, `documentation_v2/runbooks/mod_slice_workflow.md` and
//! `t181_event_mod_program.md` invoke it by that name; renaming drops it out of the wave runner.
//!
//! ── BASH ODDITIES PRESERVED ON PURPOSE ───────────────────────────────────────────────────────
//!
//! 1. **`find -L` is load-bearing.** In a slice worktree every oracle lane is a SYMLINK, and a
//!    bare `find <symlink>` does not descend — it reports the link, which is not `-type d`, so the
//!    search returns nothing. Measured: the gate then printed a cheerful "nothing to compare" for
//!    CRF while the real comparison never ran. `asset_dirs` follows links at every level.
//! 2. **The identifier pattern is anchored on a non-identifier char**, `(^|[^A-Za-z0-9_])`, so a
//!    short prefix cannot false-positive on a longer word — load-bearing for `PS_`, which a bare
//!    `grep` also finds inside `MAPS_`, `GROUPS_`, `OPS_` and `TIPS_`.
//! 3. **Comment-only lines are stripped before judging**: citing the oracle you design-mirrored is
//!    the practice we want. The filter runs over the *rendered* `path:line:text` — [`COMMENT_RE`].
//! 4. **A shared GUID present in a vanilla `.pak` is an ENGINE FACT, not a leak** — measured, all
//!    4 initial CRF hits were vanilla, and it is why only 2 of the 74 CRF-shared and 0 of the 18
//!    PS-shared GUIDs are reported.
//! 5. **`head -20`** on the hit list; `--exclude-dir=EnfusionMCP` and
//!    `--binary-files=without-match` on the identifier scan only — the GUID scan has neither.
//! 6. **Both SKIP wordings are deliberately NOT "OK"**, verbatim. Reaching the second means
//!    nothing was compared, which is how the symlink bug above hid itself.
//! 7. **No Steam install ⇒ every shared GUID is reported**, because `[ -d "$game" ] && grep …`
//!    short-circuits false. A false accusation in a fail-closed costume, but changing it changes
//!    what the gate prints on most CI runners: argue that separately, never inside a port.
//!
//! ── DELIBERATE DEVIATIONS, ALL UNREACHABLE ON THE LIVE TREE ──────────────────────────────────
//!
//! * **Hit order is sorted, not `readdir` order** — `grep -r`'s fts order is measured to be
//!   neither sorted nor stable across filesystems, so the script's own ordering is not
//!   reproducible. [`scan::walk_files`] sorts. Moot: both identifier arms are `OK (none)`.
//! * **A missing `tbd-framework` or `tbd-export`, or an absent `grep`, is exit 2 — not a green run.** In bash,
//!   `grep -rn … 2>/dev/null || true` over an absent tree prints `OK (none)` *and* `OK (nothing to
//!   compare)` and exits 0: the fail-open defect `tbd-gate` exists to remove. Here, a `NotRun`.
//! * **GNU grep's binary heuristic is approximated** by `grep_visible`; measured 2026-08-12,
//!   `tbd-framework` holds one binary file (`resourceDatabase.rdb`, NUL at byte 4) with zero
//!   matches of either pattern. grep also suppresses lines carrying encoding errors; no such text
//!   file exists in either tree, so that branch is left out rather than implemented wrong.
//!
//! Runtime is ~7m25s, almost all vanilla probe: a GUID that is a genuine miss reads all ~20 GB of
//! `data0*.pak`. No timeout — bash had none, and a deadline would turn a cold page cache into a
//! leak report. (`grep` is `/usr/bin/grep` 3.8; this shell's `ugrep` shim is a shell *function*,
//! so neither a shell nor [`Run`] ever sees it.)

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use verification_core::proc::Run;
use verification_core::{Kind, NotRun, Pattern, Verdict, scan};

// The script's `MOD` and `CRF`. `crf_framework` is gitignored, so it is absent on a fresh clone —
// which is what the advisory SKIP is for.
const MOD_REL: &str = "apps/mod/tbd-framework";
/// 2026-09-12: the map-export tooling left tbd-framework for this thin dependency addon, so the
/// identifier and GUID arms scan it too (tbd-emcp is third-party enfusion-mcp code, not ours).
const EXPORT_REL: &str = "apps/mod/tbd-export";
const CRF_REL: &str = "apps/mod/crf_framework";
/// bash line 39: the in-repo lane (a slice worktree's symlink) **overrides even `TBD_PS_ORACLE`**,
/// because that assignment is unconditional in the script. `${VAR:-…}`, so *empty* falls back too;
/// the fallback is the operator's own checkout, which no repo script provisions.
const PS_REPO_REL: &str = "apps/mod/playable_selector";
const PS_ENV: &str = "TBD_PS_ORACLE";
const PS_HOME_REL: &str = "Projects/Archive/Reforger_Lobby/PlayableSelector-main";
/// Where the vanilla `.pak` files live, relative to `$HOME`.
const VANILLA_HOME_REL: &str = ".local/share/Steam/steamapps/common/Arma Reforger/addons/data";
/// Injected dev-only tooling, gitignored; excluded from the identifier scan, not the GUID scan.
const EXCLUDE_DIR: &str = "EnfusionMCP";
/// Found rather than hardcoded because the lanes nest differently: `crf_framework/UI` vs
/// `PlayableSelector-main/PlayableSelector/UI`.
const ASSET_DIR_NAMES: &[&str] = &["UI", "Prefabs"];
/// bash `head -20`; GNU grep's initial read buffer, the window its up-front binary test looks at;
/// and the Enfusion asset GUID, uppercase hex only, exactly as the script spells it.
const HEAD: usize = 20;
const GREP_BUF: usize = 32 * 1024;
const GUID_RE: &str = r"\{[0-9A-F]{16}\}";
/// bash's `grep -vE` comment filter, applied to the rendered `path:line:text`.
///
/// `[^:]+` for the path is the script's, warts and all: a source file with a colon in its NAME
/// fails to match, so its comment lines would be reported as leaks. Zero such files exist
/// (measured 2026-08-12); reproduced rather than quietly repaired.
const COMMENT_RE: &str = r"^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*|#)";
/// Tail of the advisory SKIP, and the epilogue line whose padding is the script's. Both hoisted
/// only so their call sites fit the line budget; the wording is the script's, verbatim.
const SKIP_TAIL: &str =
    "not present locally (gitignored / out-of-repo); GUID check is advisory here";
const EPILOGUE_PS: &str =
    "  PlayableSelector — NO LICENCE; default copyright, so no permission to copy at all.";
/// The identifier arm's two calls, in the script's order. The label is prose for the banner.
const IDENT_LANES: &[(&str, &str)] = &[
    ("CRF, Arma Public License", "CRF_"),
    ("PlayableSelector, NO LICENCE", "PS_"),
];

/// The four paths the script resolves before it checks anything.
///
/// Split out from [`verify_crf_leak()`] so the tests can point every lane at a fixture without
/// mutating `HOME` — `std::env::set_var` is `unsafe` in edition 2024 and races other test threads.
struct Lanes {
    mod_dir: PathBuf,
    export_dir: PathBuf,
    crf: PathBuf,
    ps: PathBuf,
    vanilla: PathBuf,
}

impl Lanes {
    fn from_env(repo_root: &Path) -> Lanes {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        // `PS_SRC="${TBD_PS_ORACLE:-$HOME/…}"`, then an unconditional in-repo override.
        let mut ps = match std::env::var_os(PS_ENV) {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => home.join(PS_HOME_REL),
        };
        if repo_root.join(PS_REPO_REL).is_dir() {
            ps = repo_root.join(PS_REPO_REL);
        }
        Lanes {
            mod_dir: repo_root.join(MOD_REL),
            export_dir: repo_root.join(EXPORT_REL),
            crf: repo_root.join(CRF_REL),
            ps,
            vanilla: home.join(VANILLA_HOME_REL),
        }
    }
}

/// Every line the gate prints, streamed *and* retained.
///
/// This stdout is a contract — the wave driver scrapes it, and a port is accepted by diffing it
/// — so the tests assert exact text rather than a boolean. Retaining is what makes that possible;
/// streaming is what stops a 7-minute run looking hung.
struct Log {
    lines: Vec<String>,
    echo: bool,
}

impl Log {
    fn say(&mut self, line: impl Into<String>) {
        let line = line.into();
        if self.echo {
            println!("{line}");
        }
        self.lines.push(line);
    }
}

/* ───────────────────── arm 1: <prefix> identifiers in our own code ───────────────────── */

/* ───────────────────── arm 2: oracle-declared asset GUIDs ───────────────────── */

/* ───────────────────── grep-compatible reading ───────────────────── */

#[cfg(test)]
#[path = "tests/upstream_code_leaks/tests.rs"]
mod tests;

mod verify_crf_leak;
pub use verify_crf_leak::verify_crf_leak;

#[cfg(test)]
use verify_crf_leak::{asset_dirs, grep_visible, guids_under, numbered, pattern, run};
