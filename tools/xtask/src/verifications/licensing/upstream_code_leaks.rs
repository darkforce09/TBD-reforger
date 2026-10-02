//! The upstream-code leak gate, `cargo xtask verify no-crf-leak`: no code and no asset GUID of a
//! licensed reference lane may reach a shipped addon.
//!
//! **Role:** Scans our addons (`apps/mod/tbd-framework`, `apps/mod/tbd-export`) for two kinds of
//! leak from the reference lanes in `apps/mod/References/`: a lane's identifier prefix (`CRF_`,
//! `PS_`) used as code, and an asset GUID a lane declares in its own `UI/` or `Prefabs/` folders
//! reused in our files while no vanilla game `.pak` holds it. `crf_framework` is Arma Public
//! License (read, cite and design-mirror, never copy); `playable_selector` carries no licence at
//! all, so default copyright applies and nothing of it may be copied, adapted or redistributed.
//!
//! **Position:** Called by `cargo xtask verify no-crf-leak` and by the mod wave gate, which invoke
//! it by that name. Reads the paths [`Lanes`] resolves (the two addons, both lanes, and the
//! vanilla `.pak` folder of the local game install), prints a transcript and returns the exit code.
//!
//! **Signals & state:** None across runs. One run reads every input once; the vanilla probe runs
//! one scoped thread per `.pak`. [`Log`] retains every printed line so tests assert the transcript.
//!
//! **Invariants:**
//! - Exit 0 only when every step ran and found nothing; exit 1 lists the findings; exit 2 (did not
//!   run) when a lane, an addon tree or a file cannot be read. A lane that is absent or holds no
//!   `UI/` or `Prefabs/` folder is exit 2 naming the lane and `apps/mod/References/README.md`.
//! - A finding is a licence decision, never an exemption: a reported GUID is resolved by
//!   re-authoring the reference from vanilla or by recording an attribution, never by an
//!   allow-list or a narrowed pattern.
//! - The identifier pattern is anchored on a preceding character that is neither an identifier
//!   character nor `@` ([`identifier_pattern`]): a short prefix never matches inside a longer word
//!   (`PS_` inside `MAPS_`, `GROUPS_`, `OPS_`, `TIPS_`), and `@CRF_Framework`, the Workshop name a
//!   mission uses for a dependency, is not code.
//! - A line that is only a comment is not judged ([`COMMENT_RE`] over the rendered
//!   `path:line:text`): citing the reference that was design-mirrored is the wanted practice.
//! - The identifier step prints at most [`HEAD`] hits and skips `EnfusionMCP` folders (injected
//!   bridge tooling); the GUID step scans everything and prints, under each reported GUID, the
//!   `path:line` of every reference in our addons.
//! - Files are read the way GNU grep reads them ([`verify_crf_leak::grep_visible`]): a NUL in the
//!   first 32 KiB makes a file binary and invisible, a later NUL hides the rest of the file.
//! - Lanes are walked through symlinks at every level, because in a slice worktree each lane is a
//!   symlink; asset folders are found up to depth 2, since CRF nests `UI/` at depth 1 and
//!   PlayableSelector at depth 2 (`PlayableSelector/UI`).
//! - A shared GUID whose bytes occur in any vanilla `.pak` is an engine fact, not a leak. One pass
//!   over the paks answers every shared GUID of both lanes ([`vanilla_pak_probe`]). With no `.pak`
//!   present every shared GUID is reported: without vanilla data the gate cannot tell an engine
//!   fact from a leak, and it reports rather than exempts.
//! - Hits are printed in path order, then line order.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use verification_core::{Kind, NotRun, Pattern, Verdict, scan};

use crate::core::repository_layout::{
    CRF_FRAMEWORK_REFERENCE, PLAYABLE_SELECTOR_OVERRIDE_ENV, PLAYABLE_SELECTOR_REFERENCE,
    REFERENCES_DIR,
};

/// The shipping addon whose scripts and assets must stay free of upstream code.
const MOD_REL: &str = "apps/mod/tbd-framework";
/// The Workbench export addon, ours as well (tbd-emcp is third-party enfusion-mcp code, not ours).
const EXPORT_REL: &str = "apps/mod/tbd-export";
/// Where the vanilla `.pak` files live, relative to `$HOME`.
const VANILLA_HOME_REL: &str = ".local/share/Steam/steamapps/common/Arma Reforger/addons/data";
/// Injected dev-only tooling, gitignored; excluded from the identifier step, not the GUID step.
const EXCLUDE_DIR: &str = "EnfusionMCP";
/// Found rather than hardcoded because the lanes nest differently: `crf_framework/UI` vs
/// `playable_selector/PlayableSelector/UI`.
const ASSET_DIR_NAMES: &[&str] = &["UI", "Prefabs"];
/// The most identifier hits one step prints.
const HEAD: usize = 20;
/// GNU grep's initial read buffer, the window its up-front binary test looks at.
const GREP_BUF: usize = 32 * 1024;
/// The Enfusion asset GUID: sixteen uppercase hex digits in braces.
const GUID_RE: &str = r"\{[0-9A-F]{16}\}";
/// The comment filter, applied to the rendered `path:line:text`. `[^:]+` spans the path, so a
/// source file with a colon in its name has its comment lines judged as code.
const COMMENT_RE: &str = r"^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*|#)";
/// The epilogue line for the PlayableSelector lane.
const EPILOGUE_PS: &str =
    "  PlayableSelector — NO LICENCE; default copyright, so no permission to copy at all.";
/// The identifier step's two lanes, in print order. The label is prose for the banner.
const IDENT_LANES: &[(&str, &str)] = &[
    ("CRF, Arma Public License", "CRF_"),
    ("PlayableSelector, NO LICENCE", "PS_"),
];

/// The identifier pattern for one lane prefix: the prefix at the start of a line or after a
/// character that is neither an identifier character nor `@`.
fn identifier_pattern(prefix: &str) -> String {
    format!("(^|[^A-Za-z0-9_@]){prefix}")
}

/// The paths the gate resolves before it checks anything.
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
    /// The lanes of the checkout at `repo_root`, read from `HOME` and `TBD_PS_ORACLE`.
    fn from_env(repo_root: &Path) -> Lanes {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let ps_override = std::env::var_os(PLAYABLE_SELECTOR_OVERRIDE_ENV);
        Lanes::resolve(repo_root, &home, ps_override)
    }

    /// The lanes of the checkout at `repo_root`. A non-empty `ps_override` names the
    /// PlayableSelector lane in place of its folder in the references folder.
    fn resolve(repo_root: &Path, home: &Path, ps_override: Option<std::ffi::OsString>) -> Lanes {
        let ps = match ps_override {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => repo_root.join(PLAYABLE_SELECTOR_REFERENCE),
        };
        Lanes {
            mod_dir: repo_root.join(MOD_REL),
            export_dir: repo_root.join(EXPORT_REL),
            crf: repo_root.join(CRF_FRAMEWORK_REFERENCE),
            ps,
            vanilla: home.join(VANILLA_HOME_REL),
        }
    }

    /// The two reference lanes with their transcript labels, in check order.
    fn references(&self) -> [(&'static str, &Path); 2] {
        [("CRF", &self.crf), ("PlayableSelector", &self.ps)]
    }
}

/// Every line the gate prints, streamed *and* retained.
///
/// The transcript is the gate's contract — the tests assert exact text rather than a boolean.
/// Retaining is what makes that possible; streaming keeps a long run visibly alive.
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

#[cfg(test)]
#[path = "tests/upstream_code_leaks/tests.rs"]
mod tests;

mod asset_guid_reuse;
mod vanilla_pak_probe;
mod verify_crf_leak;
pub use verify_crf_leak::verify_crf_leak;

#[cfg(test)]
use asset_guid_reuse::{asset_dirs, guids_under};
#[cfg(test)]
use vanilla_pak_probe::present_in_paks;
#[cfg(test)]
use verify_crf_leak::{grep_visible, numbered, pattern, run};
