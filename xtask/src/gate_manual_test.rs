//! T-859 — port of `scripts/mod/manual-test.sh` → `cargo xtask mod manual-test`.
//!
//! Path pins mirror `scripts/mod/lib/paths.sh` (do **not** delete paths.sh — T-879):
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`, `SCHEMA=packages/tbd-schema`, `WEB=apps/website/api`.
//!
//! PASS/FAIL/SKIP accounting and `== section ==` banners match bash byte-for-byte.
//! On the live tree this gate ships **red** (legacy Go restspike + npm schema + missing
//! missions / mcp.json / GAME_SERVER_TOKENS) — acceptance is the bash/port diff, not green.
//!
//! Fail-opens closed vs bash:
//! - `.cursor/mcp.json` + absent `jq` used to PASS (`! command -v jq || jq …`). We always
//!   parse with `serde_json` when the file exists (intent: "valid JSON").
//! - Mission JSON used `node -e … 2>/dev/null`, collapsing node-absent into "missing fields".
//!   In-process `serde_json` checks the same three fields; no silent tool-absent pass.
//! - `npm run validate >/dev/null 2>&1` still maps any non-zero (incl. absent npm / no
//!   package.json) to FAIL — pinned, not upgraded to DidNotRun, so stdout stays aligned.
//!
//! Preserved oddity: restspike build failure does `fail` then **`exit 1`** with no Summary
//! banner (bash lines 72 + set -e path). Full-server / docs arms are unreachable after that.
//!
//! Live REST + full-server HTTP arms live in [`live`] (SIZE-1 split).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Child;

use anyhow::Result;
use serde_json::Value;
use tbd_gate::proc::Run;

#[path = "gate_manual_test_live.rs"]
mod live;

/// Entry for `xtask mod manual-test`.
pub fn run(repo_root: &Path) -> Result<u8> {
    Ok(run_suite(repo_root))
}

pub(crate) struct Paths {
    pub mono_root: PathBuf,
    pub mod_root: PathBuf,
    pub schema: PathBuf,
    pub web: PathBuf,
}

impl Paths {
    /// Reproduce `scripts/mod/lib/paths.sh` against an already-resolved monorepo root.
    fn from_root(root: &Path) -> Self {
        Self {
            mono_root: root.to_path_buf(),
            mod_root: root.join("apps/mod"),
            schema: root.join("packages/tbd-schema"),
            web: root.join("apps/website/api"),
        }
    }
}

pub(crate) struct Acc {
    pass: u32,
    fail: u32,
    skip: u32,
}

impl Acc {
    fn new() -> Self {
        Self {
            pass: 0,
            fail: 0,
            skip: 0,
        }
    }

    pub(crate) fn pass(&mut self, msg: &str) {
        println!("  PASS  {msg}");
        self.pass += 1;
    }

    pub(crate) fn fail(&mut self, msg: &str) {
        println!("  FAIL  {msg}");
        self.fail += 1;
    }

    pub(crate) fn skip(&mut self, msg: &str) {
        println!("  SKIP  {msg}");
        self.skip += 1;
    }
}

fn section(title: &str) {
    println!();
    println!("== {title} ==");
}

fn run_suite(root: &Path) -> u8 {
    let p = Paths::from_root(root);
    let mut a = Acc::new();

    // --- 1. tbd-schema ---
    section("packages/tbd-schema validation");
    check_npm_validate(&p, &mut a);
    check_schema_artifacts(&p, &mut a);

    // --- 3. Config / env ---
    section("Config");
    check_env_example(&p, &mut a);
    check_mcp_json(&p, &mut a);

    // --- 4. Mission files on disk ---
    section("Compiled missions");
    check_missions(&p, &mut a);

    // --- 5. Live REST spike (restspike harness) ---
    section("Live game-server REST API");
    if !build_restspike(&p, &mut a) {
        // bash: `|| { fail "build restspike"; exit 1; }` — no Summary.
        return 1;
    }
    if let Err(code) = live::run_restspike_suite(&p, &mut a) {
        return code;
    }

    // --- 6. Full server (needs Postgres) ---
    section("Full website API (public, no Discord)");
    live::run_full_server_suite(&p, &mut a);

    // --- 7. Docs / milestones ---
    section("Documentation");
    check_docs(&p, &mut a);

    // --- Summary ---
    section("Summary");
    println!(
        "Passed: {}  Failed: {}  Skipped: {}",
        a.pass, a.fail, a.skip
    );
    if a.fail > 0 {
        return 1;
    }
    println!("All runnable manual tests passed.");
    0
}

fn check_npm_validate(p: &Paths, a: &mut Acc) {
    // Pinned: any failure (absent npm, no package.json, validate red) → FAIL, matching
    // `(cd "$SCHEMA" && npm run validate >/dev/null 2>&1)`.
    let ok = match Run::new("npm")
        .arg("run")
        .arg("validate")
        .cwd(&p.schema)
        .merged_output()
    {
        Ok(m) => m.code == 0,
        Err(_) => false,
    };
    if ok {
        a.pass("npm run validate (9 artifacts)");
    } else {
        a.fail("npm run validate");
    }
}

fn check_schema_artifacts(p: &Paths, a: &mut Acc) {
    let ok = p.schema.join("schema/mission.schema.json").is_file()
        && p.schema.join("bridge/bridge-contract.md").is_file()
        && p.schema
            .join("golden-missions/bridgehead-at-levie.json")
            .is_file();
    if ok {
        a.pass("schema + bridge + golden mission files exist");
    } else {
        a.fail("missing packages/tbd-schema artifacts");
    }
}

fn check_env_example(p: &Paths, a: &mut Acc) {
    let path = p.web.join(".env.example");
    // bash: `grep -q 'GAME_SERVER_TOKENS' "$WEB/.env.example"` — missing file still
    // prints `grep: <path>: No such file or directory` on stderr before the FAIL line.
    let ok = match fs::read_to_string(&path) {
        Ok(t) => t.contains("GAME_SERVER_TOKENS"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("grep: {}: No such file or directory", path.display());
            false
        }
        Err(_) => false,
    };
    if ok {
        a.pass(".env.example documents GAME_SERVER_TOKENS");
    } else {
        a.fail(".env.example missing GAME_SERVER_TOKENS");
    }
}

fn check_mcp_json(p: &Paths, a: &mut Acc) {
    let path = p.mono_root.join(".cursor/mcp.json");
    // Closed fail-open: bash passed when jq was absent; we always parse.
    let ok = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Value>(&text).is_ok(),
        Err(_) => false,
    };
    if ok {
        a.pass(".cursor/mcp.json is valid JSON");
    } else {
        a.fail(".cursor/mcp.json invalid or missing");
    }
}

fn mission_fields_ok(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    if v.get("schemaVersion").and_then(|x| x.as_str()) != Some("1.0") {
        return false;
    }
    let name_ok = v
        .pointer("/meta/name")
        .and_then(|x| x.as_str())
        .is_some_and(|s| !s.is_empty());
    let nets_ok = v
        .pointer("/radioPlan/nets")
        .and_then(|x| x.as_array())
        .is_some_and(|a| !a.is_empty());
    name_ok && nets_ok
}

fn check_missions(p: &Paths, a: &mut Acc) {
    for id in ["msn_8f3a2c", "msn_2d91be"] {
        let path = p.web.join(format!("missions/{id}.json"));
        if !path.is_file() {
            a.fail(&format!("missions/{id}.json missing"));
            continue;
        }
        // Closed fail-open: was `node -e … 2>/dev/null`.
        if mission_fields_ok(&path) {
            a.pass(&format!(
                "missions/{id}.json parses (schemaVersion, meta, radioPlan)"
            ));
        } else {
            a.fail(&format!("missions/{id}.json missing required fields"));
        }
    }
}

fn build_restspike(p: &Paths, a: &mut Acc) -> bool {
    let out = PathBuf::from("/tmp/tbd-restspike");
    let ok = match Run::new("go")
        .arg("build")
        .arg("-o")
        .arg(&out)
        .arg("./cmd/restspike")
        .cwd(&p.web)
        .merged_output()
    {
        Ok(m) => m.code == 0,
        Err(_) => false,
    };
    if !ok {
        a.fail("build restspike");
        return false;
    }
    true
}

pub(crate) struct ChildGuard(pub(crate) Option<Child>);

impl ChildGuard {
    pub(crate) fn take(&mut self) -> Option<Child> {
        self.0.take()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn check_docs(p: &Paths, a: &mut Acc) {
    for f in ["CLAUDE-CONTINUATION.md", "MILESTONES.md"] {
        if p.mod_root.join(f).is_file() {
            a.pass(&format!("{f} exists"));
        } else {
            a.fail(&format!("{f} missing"));
        }
    }
    if p.schema.join("spikes/rest-spike-0.1.md").is_file() {
        a.pass("rest-spike doc exists");
    } else {
        a.fail("rest-spike doc missing");
    }
}
