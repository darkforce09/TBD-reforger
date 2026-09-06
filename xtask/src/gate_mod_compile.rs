//! T-891 — `scripts/mod/compile.sh` → `cargo xtask mod compile`.
//! Exit: **0** clean · **1** CODE · **2** no verdict · **3** ENV. `--selftest` must exit **1**.

use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use regex::Regex;

use crate::gate_mod_compile_host::{
    Session, hostrun, is_executable, kill_run, mktemp_dir, require_host,
};
use crate::root::find_repo_root;

/// bash `sed -n '2,44p' "$0"`.
const HELP: &str = include_str!("gate_mod_compile_help.txt");

const SELFTEST_GPROJ: &str = "\
GameProject {\n\
 ID \"TBD_CompileSelfTest\"\n\
 GUID \"C0FFEE0000000001\"\n\
 TITLE \"TBD Compile Self Test\"\n\
 Dependencies {\n\
  \"58D0FB3206B6F859\"\n\
 }\n\
 Configurations {\n\
  GameProjectConfig PC {\n\
  }\n\
  GameProjectConfig HEADLESS {\n\
  }\n\
 }\n\
}\n";

const SELFTEST_C: &str = "\
// Deliberately broken — proves the gate still detects compile errors.\n\
// NOTE: must be an undefined symbol; malformed punctuation compiles clean in Enfusion.\n\
class TBD_CompileSelfTest\n\
{\n\
\tvoid Broken()\n\
\t{\n\
\t\tTBD_ThisSymbolDoesNotExist_SelfTest();\n\
\t}\n\
}\n";

const PROBE_GPROJ: &str = "\
GameProject {\n\
 ID \"TBD_ApiProbe\"\n\
 GUID \"C0FFEE0000000002\"\n\
 TITLE \"TBD API Probe\"\n\
 Dependencies {\n\
  \"58D0FB3206B6F859\"\n\
 }\n\
 Configurations {\n\
  GameProjectConfig PC {\n\
  }\n\
  GameProjectConfig HEADLESS {\n\
  }\n\
 }\n\
}\n";

const LAUNCH_SH: &str = r#"
  echo $$ > "$1/server.pid"
  exec timeout "$2" ./ArmaReforgerServer \
    -addonsDir "$1/addons" -addons "$3" -profile "$1/profile" -maxFPS 15
"#;

const CAL_SH: &str = r#"
    echo $$ > "$1/server.pid"
    exec timeout 120 ./ArmaReforgerServer -addonsDir "$1/addons" -profile "$1/profile" -maxFPS 15
"#;

#[derive(Debug, Default)]
pub struct Opts {
    pub selftest: bool,
    pub keep_logs: bool,
    pub probe_dir: Option<PathBuf>,
}

enum Parse {
    Help,
    Run(Opts),
}

/// Entry for `xtask mod compile [flags…]`.
pub fn run(args: &[String]) -> Result<u8> {
    match parse_args(args) {
        Err(a) => {
            eprintln!("compile.sh: unknown arg '{a}'");
            Ok(2)
        }
        Ok(Parse::Help) => {
            print!("{HELP}");
            Ok(0)
        }
        Ok(Parse::Run(opts)) => Ok(run_with_root(&find_repo_root()?, &opts)),
    }
}

/// Entry for `xtask mod compile-selftest` — T-897's port of the Makefile's `mod-compile-selftest`.
///
/// THE INSTRUMENT BEFORE THE VERDICT. This check's entire job is to prove the absence of false
/// greens, so it must not be one. Only exit **1** — a real Enfusion rejection of the deliberately
/// broken `--selftest` addon — counts as a pass, per the contract at the top of this file:
/// 0 compiled clean · 1 real compile failure · 2 no verdict reached · 3 environment failure.
///
/// Until T-312 the check was `if compile --selftest; then FAIL else OK fi`, which read ANY
/// non-zero as "the gate correctly rejected broken source". On a machine with no dedicated server
/// and no host bridge the gate exits 3 without compiling a line, and that printed SELFTEST OK —
/// while `mod wave gate` called it and reported PASS for a check that never happened.
///
/// The classification lived in the Makefile recipe until T-897 (`Makefile:290-298`), where it had
/// to be a shell `case` **because GNU make flattens every failed recipe to its own status 2**,
/// destroying the 1-vs-3 distinction the whole check turns on. In-process there is no flattening:
/// `rc` below is this gate's own. Each branch still NAMES its failure mode, because a caller
/// should get the diagnosis from the text and not have to reconstruct it from `$?`.
pub fn run_selftest() -> Result<u8> {
    let opts = Opts {
        selftest: true,
        ..Opts::default()
    };
    let rc = run_with_root(&find_repo_root()?, &opts);
    Ok(match rc {
        1 => {
            println!("SELFTEST OK: gate correctly rejected broken source (exit 1)");
            0
        }
        0 => {
            println!(
                "SELFTEST FAIL: gate returned 0 on deliberately broken source — it is no longer \
                 detecting compile errors, so every green mod-compile since is suspect."
            );
            1
        }
        3 => {
            println!(
                "SELFTEST FAIL: ENVIRONMENT (exit 3) — the gate never ran. Read the ENV FAIL \
                 above: it is this machine, and it says NOTHING about tbd-framework. A check that \
                 did not happen is not a pass."
            );
            1
        }
        2 => {
            println!(
                "SELFTEST FAIL: no verdict reached (exit 2 — timeout, or a bad argument to mod \
                 compile). Inconclusive is not a pass."
            );
            1
        }
        other => {
            println!(
                "SELFTEST FAIL: mod compile --selftest exited {other}, outside its documented \
                 0/1/2/3 contract."
            );
            1
        }
    })
}

/// T-901: the mod-gates.yml preflight, in Rust. Missing server or empty rdb is a hard fail
/// (exit 1). A check that did not find the depot must not print SELFTEST OK — that is
/// `run_selftest`'s job, and it already refuses exit 0 / 3 as a pass.
pub fn run_preflight() -> Result<u8> {
    Ok(preflight_with_root(&find_repo_root()?))
}

pub fn preflight_with_root(root: &Path) -> u8 {
    let home = std::env::var("HOME").unwrap_or_default();
    let bin = PathBuf::from(format!(
        "{home}/.local/share/Steam/steamapps/common/Arma Reforger Server/ArmaReforgerServer"
    ));
    let rdb = root.join("apps/mod/tbd-framework/resourceDatabase.rdb");
    let mut fail = 0u8;
    if !is_executable(&bin) {
        eprintln!(
            "::error title=mod-gates runner not provisioned::No Arma Reforger dedicated server at '{}'. Install appid 1890870 for the runner's user, or run the runner as the user that already has it. This job cannot be made to pass without it and will not pretend otherwise.",
            bin.display()
        );
        fail = 1;
    } else {
        println!("dedicated server: {}", bin.display());
    }
    let rdb_ok = rdb.is_file() && rdb.metadata().map(|m| m.len() > 0).unwrap_or(false);
    if !rdb_ok {
        eprintln!(
            "::error title=mod-gates checkout incomplete::apps/mod/tbd-framework/resourceDatabase.rdb missing or empty. Without it the engine skips the loose addon and compiles none of the mod."
        );
        fail = 1;
    }
    let rdb_export = root.join("apps/mod/tbd-export/resourceDatabase.rdb");
    let rdb_export_ok =
        rdb_export.is_file() && rdb_export.metadata().map(|m| m.len() > 0).unwrap_or(false);
    if !rdb_export_ok {
        eprintln!(
            "::error title=mod-gates checkout incomplete::apps/mod/tbd-export/resourceDatabase.rdb missing or empty. Without it the engine skips the loose addon and compiles none of the mod."
        );
        fail = 1;
    }
    fail
}

/// Testable entry (no root walk).
pub fn run_with_root(root: &Path, opts: &Opts) -> u8 {
    if require_host().is_err() {
        return env_fail(
            "no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine",
            Some(
                "See xtask/src/hostrun.rs: the container has no C toolchain and an older glibc, so the game binary cannot run in here at all.",
            ),
        );
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let server_dir = PathBuf::from(format!(
        "{home}/.local/share/Steam/steamapps/common/Arma Reforger Server"
    ));
    let server_bin = server_dir.join("ArmaReforgerServer");
    let mod_src = root.join("apps/mod/tbd-framework");

    if !is_executable(&server_bin) {
        return env_fail(
            &format!("dedicated server not found at {}", server_bin.display()),
            Some("Install it from Steam (appid 1890870):  steam steam://install/1890870"),
        );
    }
    if !mod_src.join("addon.gproj").is_file() {
        return env_fail(
            &format!("no addon.gproj at {}", mod_src.display()),
            Some(
                "The checkout does not look like this repo — verify the working tree before blaming the mod.",
            ),
        );
    }
    let export_src = root.join("apps/mod/tbd-export");
    if !export_src.join("addon.gproj").is_file() {
        return env_fail(
            &format!("no addon.gproj at {}", export_src.display()),
            Some(
                "The checkout does not look like this repo — verify the working tree before blaming the mod.",
            ),
        );
    }

    if let Some(probe) = &opts.probe_dir
        && !probe.is_dir()
    {
        eprintln!("compile.sh: --probe dir not found: {}", probe.display());
        return 2;
    }

    let max_wait: u64 = std::env::var("TBD_COMPILE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180);

    let run_dir = match mktemp_dir("tbd-compile") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("compile.sh: could not create run dir: {e}");
            return 2;
        }
    };

    let session = Session::install(run_dir.clone(), opts.keep_logs);
    let code = match compile_inner(root, &mod_src, &server_dir, &run_dir, opts, max_wait) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("compile.sh: internal error: {e}");
            2
        }
    };
    session.finish();
    code
}

fn compile_inner(
    root: &Path,
    mod_src: &Path,
    server_dir: &Path,
    run_dir: &Path,
    opts: &Opts,
    max_wait: u64,
) -> io::Result<u8> {
    fs::create_dir_all(run_dir.join("addons"))?;
    fs::create_dir_all(run_dir.join("profile"))?;
    let link = run_dir.join("addons/tbd-framework");
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(mod_src, &link)?;
    let export_link = run_dir.join("addons/tbd-export");
    let _ = fs::remove_file(&export_link);
    std::os::unix::fs::symlink(root.join("apps/mod/tbd-export"), &export_link)?;

    // T-946.23 — EXPORT FIRST, FRAMEWORK LAST, AND THE ORDER IS THE WHOLE POINT.
    //
    // The Enfusion VFS overlays addons BY PATH and the LAST one wins, so with the old
    // `TBD_Framework,TBD_Export` order every one of the 139 script paths that exists in both trees
    // was compiled from tbd-EXPORT and the tbd-framework copy was never read. The shipping server
    // loads only `TBD_Framework` (`scripts/mod/tbd-staging-server.config.json`), so the gate was
    // green over code that does not ship and silent about the code that does. Measured 2026-09-06
    // by planting `void f(){ThisSymbolDoesNotExist_CC();}` in
    // `tbd-framework/.../TBD_MissionSlotStruct.c`: `OK: compiled clean`, exit 0. The same line in
    // the tbd-export copy fails the gate. Found by the T-674.2 slice agent, reproduced here before
    // anything was changed.
    //
    // Framework last means the SHIPPING copies are the ones the engine reads. The export mirrors
    // are then covered by [`mirror_lockstep`] instead of by compilation, which is the honest trade:
    // the two trees are line-for-line identical by design, so a divergence is itself the defect.
    let mut addons = String::from("TBD_Export,TBD_Framework");

    if opts.selftest {
        let st = run_dir.join("addons/tbd-selftest");
        fs::create_dir_all(st.join("Scripts/Game"))?;
        fs::write(st.join("addon.gproj"), SELFTEST_GPROJ)?;
        fs::write(st.join("Scripts/Game/TBD_CompileSelfTest.c"), SELFTEST_C)?;
        addons.push_str(",TBD_CompileSelfTest");
    }

    if let Some(probe) = &opts.probe_dir {
        let mut cs: Vec<PathBuf> = fs::read_dir(probe)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("c"))
            .collect();
        cs.sort();
        if cs.is_empty() {
            eprintln!("compile.sh: no .c files in {}", probe.display());
            return Ok(2);
        }
        let pd = run_dir.join("addons/tbd-probe");
        fs::create_dir_all(pd.join("Scripts/Game"))?;
        fs::write(pd.join("addon.gproj"), PROBE_GPROJ)?;
        for f in &cs {
            fs::copy(f, pd.join("Scripts/Game").join(f.file_name().unwrap()))?;
        }
        addons.push_str(",TBD_ApiProbe");
        println!("    (probing from {})", probe.display());
        for f in &cs {
            println!(
                "      {}",
                f.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            );
        }
    }

    println!("==> compiling tbd-framework + tbd-export (native headless server, no Workbench)");

    let log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(run_dir.join("stdout.log"))?;
    let log2 = log.try_clone()?;
    let mut cmd = hostrun(&[
        "env",
        "-C",
        &server_dir.to_string_lossy(),
        "setsid",
        "sh",
        "-c",
        LAUNCH_SH,
        "_",
        &run_dir.to_string_lossy(),
        &max_wait.to_string(),
        &addons,
    ]);
    cmd.stdout(Stdio::from(log));
    cmd.stderr(Stdio::from(log2));
    let mut child = cmd.spawn()?;

    let deadline = Instant::now() + Duration::from_secs(max_wait);
    let mut console: Option<PathBuf> = None;
    let mut errlog: Option<PathBuf> = None;
    let mut verdict: Option<&str> = None;

    while Instant::now() < deadline {
        if console.is_none()
            && let Some(d) = latest_logs_dir(&run_dir.join("profile/logs"))
        {
            console = Some(d.join("console.log"));
            errlog = Some(d.join("error.log"));
        }
        if let (Some(c), Some(e)) = (&console, &errlog)
            && c.is_file()
        {
            if file_contains(c, "Game successfully created") {
                verdict = Some("ok");
                break;
            }
            if e.is_file() && file_contains(e, "SCRIPT    (E):") {
                verdict = Some("fail");
                break;
            }
        }
        thread::sleep(Duration::from_millis(300));
    }

    kill_run(run_dir);
    for _ in 0..10 {
        if child.try_wait()?.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    let _ = child.wait();

    let Some(verdict) = verdict else {
        eprintln!("FAIL: timed out after {max_wait}s with no compile verdict.");
        eprintln!("      (rerun with --keep-logs to inspect)");
        return Ok(2);
    };

    let errlog = errlog.unwrap_or_else(|| run_dir.join("missing-error.log"));
    let has_e = errlog.is_file() && file_contains(&errlog, "SCRIPT    (E):");
    if verdict == "fail" || has_e {
        return report_compile_errors(mod_src, run_dir, &errlog);
    }

    let console = console.ok_or_else(|| io::Error::other("missing console"))?;
    if let Some(code) = load_count_guard(root, server_dir, &console)? {
        return Ok(code);
    }

    let drift = mirror_lockstep(root)?;
    if !drift.is_empty() {
        println!();
        println!("FAIL: tbd-framework and tbd-export are not in lockstep");
        println!(
            "      (scripts compared as code + string literals, with the ASCII rule's punctuation"
        );
        println!("       folded away; every other shared path compared byte-for-byte)");
        for l in &drift {
            println!("  {l}");
        }
        println!(
            "      The engine compiles the tbd-framework copy (T-946.23) and the mirror ships too,"
        );
        println!(
            "      so a divergence here is code that no gate reads. Make the two copies agree."
        );
        return Ok(1);
    }

    let ascii_bad = ascii_check_export(&root.join("apps/mod/tbd-export"))?;
    if !ascii_bad.is_empty() {
        println!();
        println!("FAIL: non-ASCII bytes in tbd-export scripts (first offending byte per file)");
        println!("------------------------------------------------------------");
        for l in &ascii_bad {
            println!("{l}");
        }
        println!("------------------------------------------------------------");
        println!("Workbench's lexer rejects these, and the headless server never reads the");
        println!(
            "WorkbenchGame module — this scan is the only pre-restart guard. Transliterate to ASCII."
        );
        return Ok(1);
    }

    let files = last_re(
        &console,
        r"Module: Game; loaded [0-9]*x files; [0-9]*x classes",
    );
    let took = last_re(&console, r"Compiling Game scripts took: [0-9.]* ms");
    let warn = count_tbd_warnings(&errlog);
    println!("OK: compiled clean");
    if let Some(f) = files {
        println!("    {f}");
    }
    if let Some(t) = took {
        println!("    {t}");
    }
    println!("    {warn} warning(s) in TBD sources");
    Ok(0)
}

/// Scripts that live in tbd-export and have NO tbd-framework twin, on purpose.
///
/// T-946.24: this list is what makes the missing-twin case detectable. The engine reads the
/// framework copy of a shared path and falls through to export only when framework HAS no copy, so
/// "framework lost a file" and "this file is legitimately export-only" look identical to the
/// compiler — and the first one means the shipping mod is missing a script while the gate stays
/// green on export's. Measured 2026-09-06 by the wave-242 verifier: moving
/// `tbd-framework/Scripts/Game/TBD/Core/TBD_Log.c` aside left `OK: compiled clean` with the file
/// count UNCHANGED, because export's copy stepped into its place. Naming the legitimate cases is
/// the only way to tell the two apart.
const EXPORT_ONLY_SCRIPTS: &[&str] = &[
    // The road-export runtime: exists to run inside Workbench's Game module, not on a server.
    "Scripts/Game/TBD/Export/TBD_RoadClassifier.c",
    "Scripts/Game/TBD/Export/TBD_RoadExportComponent.c",
    "Scripts/Game/TBD/Export/TBD_RoadExportJson.c",
    "Scripts/Game/TBD/Export/TBD_RoadExportPaths.c",
    "Scripts/Game/TBD/Export/TBD_RoadRecords.c",
    // Workbench-only export plugins.
    "Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_TbdBlueprint.c",
    "Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BlueprintReconPlugin.c",
    "Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BuildingArchitectExtractor.c",
    "Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BuildingTraceExtract.c",
    "Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BuildingTraceScanner.c",
    "Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BuildingVoxelDump.c",
    "Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BuildingsExportPlugin.c",
    "Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_MapExportBuildings.c",
];

/// Shared NON-script paths the two addons are allowed to differ on, and why.
///
/// T-946.24: the addon order decides which body of a shared path the engine resolves, and that is
/// not only true of scripts. These three differ byte-for-byte while carrying the SAME resource
/// GUID, so flipping the order flipped which one wins — measured by the wave-242 verifier, which is
/// how the list came to exist. Each is a file an addon must own; anything else that differs is
/// drift, and the check below refuses it.
const MIRROR_DIVERGENT_ASSETS: &[(&str, &str)] = &[
    (
        "Prefabs/Systems/TBD_GameMode.et",
        "the export game mode carries TBD_RoadExportComponent; the shipping one must not",
    ),
    (
        "addon.gproj",
        "each addon declares its own GUID, name and dependencies",
    ),
    (
        "resourceDatabase.rdb",
        "each addon indexes its own resources",
    ),
];

/// The punctuation the pure-ASCII rule forces tbd-export to spell differently, folded so the two
/// trees can be compared on meaning. Deliberately small and explicit: an unfolded character simply
/// makes the comparison fail, which is a "make the two copies agree" message, not a silent pass.
const ASCII_FOLD: &[(char, &str)] = &[
    ('\u{2014}', "-"),
    ('\u{2013}', "-"),
    ('\u{2011}', "-"),
    ('\u{2018}', "'"),
    ('\u{2019}', "'"),
    ('\u{201c}', "\""),
    ('\u{201d}', "\""),
    ('\u{2026}', "..."),
    ('\u{00b7}', "."),
    ('\u{00d7}', "x"),
    ('\u{2192}', "->"),
    ('\u{2190}', "<-"),
    ('\u{2264}', "<="),
    ('\u{2265}', ">="),
    ('\u{00b2}', "^2"),
    ('\u{00b3}', "^3"),
    ('\u{00b0}', "deg"),
    ('\u{00b1}', "+/-"),
    ('\u{2248}', "~"),
    ('\u{2022}', "*"),
    ('\u{00a7}', "S"),
    ('\u{2260}', "!="),
    ('\u{2208}', "in"),
];

/// Strip comments while KEEPING string literals, then fold and collapse whitespace.
///
/// T-946.24 — two corrections to the first version of this check, both found by the wave-242
/// verifier and both silent:
///
///   * it reused `schema_gates`' stripper, which finds `//` BEFORE blanking literals, so everything
///     after a `"http://…"` on a line was discarded and any divergence there was invisible. Proved
///     with a mirror whose export copy called an undefined function after such a literal: `OK:
///     compiled clean`.
///   * it ERASED literal contents. A resource GUID is a string literal and it decides which layout
///     an addon instantiates, so two mirrors could load different UI and pass. Literals are kept
///     and folded instead, which is all the ASCII rule actually requires of them.
fn mirror_normalise(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let b: Vec<char> = src.chars().collect();
    let (mut i, n) = (0usize, b.len());
    let (mut in_str, mut in_line, mut in_block) = (false, false, false);
    while i < n {
        let c = b[i];
        let next = if i + 1 < n { b[i + 1] } else { '\0' };
        if in_line {
            if c == '\n' {
                in_line = false;
                out.push(c);
            }
            i += 1;
        } else if in_block {
            if c == '*' && next == '/' {
                in_block = false;
                out.push(' ');
                i += 2;
            } else {
                i += 1;
            }
        } else if in_str {
            out.push(c);
            if c == '\\' && next != '\0' {
                out.push(next);
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
            i += 1;
        } else if c == '/' && next == '/' {
            in_line = true;
            i += 2;
        } else if c == '/' && next == '*' {
            in_block = true;
            i += 2;
        } else {
            if c == '"' {
                in_str = true;
            }
            out.push(c);
            i += 1;
        }
    }
    let mut folded = String::with_capacity(out.len());
    for ch in out.chars() {
        match ASCII_FOLD.iter().find(|(k, _)| *k == ch) {
            Some((_, v)) => folded.push_str(v),
            None => folded.push(ch),
        }
    }
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// T-946.23 / T-946.24 — the two mod trees must agree on everything the addon order can swap.
///
/// The compile reads the tbd-framework copy of a shared path (see the addon order above), so the
/// tbd-export mirrors are checked here instead of by the compiler. Three questions, because the
/// verifier showed the first version answered only one of them:
///
///   1. does export hold a script framework does not, that is not on [`EXPORT_ONLY_SCRIPTS`]? Then
///      the shipping tree has lost a file and export's copy is silently standing in for it.
///   2. do two mirrors of the same script differ in CODE OR IN A STRING LITERAL, once the ASCII
///      rule's punctuation is folded away?
///   3. do two mirrors of the same NON-script path differ, outside [`MIRROR_DIVERGENT_ASSETS`]?
///
/// Returns one line per problem; empty means lockstep.
fn mirror_lockstep(root: &Path) -> io::Result<Vec<String>> {
    let fw = root.join("apps/mod/tbd-framework");
    let ex = root.join("apps/mod/tbd-export");
    if !fw.join("Scripts").is_dir() || !ex.join("Scripts").is_dir() {
        return Err(io::Error::other(
            "mirror lockstep: one of apps/mod/{tbd-framework,tbd-export}/Scripts is missing — \
             refusing to report lockstep over a tree that is not there",
        ));
    }
    let mut out = Vec::new();
    let every = |base: &Path| -> io::Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        let mut stack = vec![base.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for e in fs::read_dir(&dir)? {
                let p = e?.path();
                if p.is_dir() {
                    if p.file_name().and_then(|n| n.to_str()) == Some(".git") {
                        continue;
                    }
                    stack.push(p);
                } else {
                    found.push(p);
                }
            }
        }
        found.sort();
        Ok(found)
    };

    for p in every(&ex)? {
        let Ok(rel) = p.strip_prefix(&ex) else {
            continue;
        };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        let twin = fw.join(rel);
        let is_script = rel_s.starts_with("Scripts/") && rel_s.ends_with(".c");

        if !twin.is_file() {
            if is_script && !EXPORT_ONLY_SCRIPTS.contains(&rel_s.as_str()) {
                out.push(format!(
                    "{rel_s}: in tbd-export only. Either tbd-framework lost it — in which case the \
                     mod that SHIPS is missing this script and the gate compiled export's copy in \
                     its place — or it is deliberately export-only and belongs in \
                     EXPORT_ONLY_SCRIPTS with a reason."
                ));
            }
            continue;
        }

        if is_script {
            if mirror_normalise(&fs::read_to_string(&p)?)
                != mirror_normalise(&fs::read_to_string(&twin)?)
            {
                out.push(format!(
                    "{rel_s}: the two copies differ in code or in a string literal"
                ));
            }
        } else if fs::read(&p)? != fs::read(&twin)?
            && !MIRROR_DIVERGENT_ASSETS.iter().any(|(k, _)| *k == rel_s)
        {
            out.push(format!(
                "{rel_s}: the two copies differ byte-for-byte, and the addon order decides which \
                 one the engine resolves. Make them agree, or add it to MIRROR_DIVERGENT_ASSETS \
                 with a reason."
            ));
        }
    }
    out.sort();
    Ok(out)
}

/// The dedicated server compiles only the Game module: `Scripts/WorkbenchGame` sources are never
/// read headless — a planted `Undefined function` there sails through (probed 2026-08-28), and
/// their Workbench API symbols do not exist server-side, so no flag can compile them here.
/// Workbench itself DOES lex them and hard-rejects non-ASCII punctuation, which makes this byte
/// scan the only pre-restart guard for that module. Runs after the engine verdict so
/// `compile-selftest`'s exit-1 still means a real Enfusion rejection. tbd-export is kept pure
/// ASCII; tbd-framework predates the rule (non-ASCII comments, engine-green) and is exempt.
fn ascii_check_export(export_src: &Path) -> io::Result<Vec<String>> {
    let mut bad = Vec::new();
    let mut stack = vec![export_src.join("Scripts")];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir)? {
            let p = e?.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some("c") {
                let bytes = fs::read(&p)?;
                let mut line = 1usize;
                for &b in &bytes {
                    if b == b'\n' {
                        line += 1;
                    } else if b > 0x7F {
                        bad.push(format!("{}:{line}: non-ASCII byte 0x{b:02X}", p.display()));
                        break;
                    }
                }
            }
        }
    }
    bad.sort();
    Ok(bad)
}

fn report_compile_errors(mod_src: &Path, run_dir: &Path, errlog: &Path) -> io::Result<u8> {
    let text = fs::read_to_string(errlog).unwrap_or_default();
    let re = Regex::new(r#".*SCRIPT    \(E\): @"([^"]*),([0-9]*)": (.*)"#).unwrap();
    let mut all: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let c = re.captures(line)?;
            Some(format!("{}:{}: {}", &c[1], &c[2], &c[3]))
        })
        .collect();
    all.sort();
    all.dedup();

    let mut ours = Vec::new();
    let mut cascade = Vec::new();
    for line in &all {
        let p = line.split(':').next().unwrap_or("");
        if mod_src.join(p).exists()
            || run_dir.join("addons/tbd-export").join(p).exists()
            || run_dir.join("addons/tbd-selftest").join(p).exists()
            || run_dir.join("addons/tbd-probe").join(p).exists()
        {
            ours.push(line.clone());
        } else {
            cascade.push(line.clone());
        }
    }

    println!();
    println!("FAIL: Enfusion compile errors");
    println!("------------------------------------------------------------");
    if ours.is_empty() {
        println!("(none in TBD sources — see cascade below; the root cause may be a");
        println!(" missing dependency or a vanilla API that moved)");
    } else {
        for l in &ours {
            println!("{l}");
        }
    }
    println!("------------------------------------------------------------");
    println!(
        "{} error(s) in TBD sources, {} cascaded into vanilla.",
        ours.len(),
        cascade.len()
    );
    if !cascade.is_empty() {
        println!("Cascade (fix the TBD errors first; these usually vanish):");
        for l in cascade.iter().take(10) {
            println!("  {l}");
        }
        if cascade.len() > 10 {
            println!("  … {} more", cascade.len() - 10);
        }
    }
    Ok(1)
}

/// T-946.24 — THE COUNT IS THE UNION OF BOTH ADDONS, so this guard can only catch "the engine
/// skipped the loose addons entirely", never "tbd-framework's copy of a file was missing and
/// tbd-export's stood in for it" — the count is unchanged in that case, measured by the wave-242
/// verifier. The message used to name tbd-framework specifically and could not have detected the
/// thing it named. [`mirror_lockstep`]'s export-only check is what covers the substitution.
fn load_count_guard(root: &Path, server_dir: &Path, console: &Path) -> io::Result<Option<u8>> {
    let loaded = last_num(console, r"Module: Game; loaded ([0-9]*)x files").unwrap_or(0);
    let baseline_file = root.join(".compile-vanilla-baseline");
    if !baseline_file.is_file() || fs::metadata(&baseline_file)?.len() == 0 {
        println!("    (calibrating vanilla-only baseline, one time)");
        let cal_dir = mktemp_dir("tbd-cal")?;
        Session::set_cal(Some(cal_dir.clone()));
        fs::create_dir_all(cal_dir.join("addons"))?;
        fs::create_dir_all(cal_dir.join("profile"))?;

        let mut cmd = hostrun(&[
            "env",
            "-C",
            &server_dir.to_string_lossy(),
            "setsid",
            "sh",
            "-c",
            CAL_SH,
            "_",
            &cal_dir.to_string_lossy(),
        ]);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        let mut cal_child = cmd.spawn()?;

        let deadline = Instant::now() + Duration::from_secs(120);
        let mut cal_console: Option<PathBuf> = None;
        while Instant::now() < deadline {
            if cal_console.is_none()
                && let Some(d) = latest_logs_dir(&cal_dir.join("profile/logs"))
            {
                cal_console = Some(d.join("console.log"));
            }
            if let Some(ref c) = cal_console
                && file_contains(c, "Module: Game; loaded")
            {
                break;
            }
            thread::sleep(Duration::from_millis(300));
        }
        let cal_n = cal_console
            .as_ref()
            .and_then(|c| last_num(c, r"Module: Game; loaded ([0-9]*)x files"))
            .unwrap_or(0);
        if cal_n > 0 {
            fs::write(&baseline_file, format!("{cal_n}\n"))?;
        }
        if let Ok(pgid) = fs::read_to_string(cal_dir.join("server.pid")) {
            let pgid = pgid.trim();
            if !pgid.is_empty() {
                let _ = hostrun(&["kill", "-9", "--", &format!("-{pgid}")])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
        let _ = cal_child.kill();
        let _ = cal_child.wait();
        let _ = fs::remove_dir_all(&cal_dir);
        Session::set_cal(None);
    }

    let vanilla: u64 = fs::read_to_string(&baseline_file)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    if vanilla > 0 && loaded <= vanilla {
        return Ok(Some(env_fail(
            &format!(
                "the Game module loaded {loaded} files and vanilla-only is {vanilla}, so NEITHER loose addon's scripts were compiled — the engine skipped them entirely"
            ),
            Some(
                "Almost always a stale or unreadable apps/mod/tbd-framework/resourceDatabase.rdb (it IS committed, but the engine rejects it once it drifts from the script tree). Fix: open apps/mod/tbd-framework in Workbench once so it regenerates the rdb, then re-run.",
            ),
        )));
    }
    Ok(None)
}

fn parse_args(args: &[String]) -> std::result::Result<Parse, String> {
    let mut opts = Opts::default();
    for a in args {
        match a.as_str() {
            "--selftest" => opts.selftest = true,
            "--keep-logs" => opts.keep_logs = true,
            "-h" | "--help" => return Ok(Parse::Help),
            _ if a.starts_with("--probe=") => {
                opts.probe_dir = Some(PathBuf::from(&a["--probe=".len()..]));
            }
            _ => return Err(a.clone()),
        }
    }
    Ok(Parse::Run(opts))
}

fn env_fail(msg: &str, hint: Option<&str>) -> u8 {
    println!();
    println!("COMPILE GATE: ENV FAIL — {msg}");
    println!(
        "  This is the HARNESS's environment. The mod was never compiled, so this says NOTHING"
    );
    println!("  about tbd-framework — do not read it as a code failure.");
    if let Some(h) = hint {
        println!("  {h}");
    }
    3
}

fn latest_logs_dir(logs: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(logs)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("logs_"))
        })
        .collect();
    dirs.sort();
    dirs.pop()
}

fn file_contains(path: &Path, needle: &str) -> bool {
    fs::read_to_string(path)
        .map(|t| t.contains(needle))
        .unwrap_or(false)
}

fn last_re(path: &Path, pat: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    Regex::new(pat)
        .ok()?
        .find_iter(&text)
        .last()
        .map(|m| m.as_str().to_string())
}

fn last_num(path: &Path, pat: &str) -> Option<u64> {
    let text = fs::read_to_string(path).ok()?;
    let caps = Regex::new(pat).ok()?.captures_iter(&text).last()?;
    caps.get(1)?.as_str().parse().ok()
}

fn count_tbd_warnings(errlog: &Path) -> usize {
    fs::read_to_string(errlog)
        .map(|t| {
            t.lines()
                .filter(|l| l.contains("SCRIPT    (W): @\"Scripts/Game/TBD/"))
                .count()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn throwaway(tag: &str) -> PathBuf {
        let root = PathBuf::from(format!("/tmp/t853/compile/ut-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".ai/tickets")).unwrap();
        fs::create_dir_all(root.join("apps/mod/tbd-framework")).unwrap();
        fs::write(root.join(".ai/tickets/ROOT"), "{}\n").unwrap();
        root
    }

    fn with_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
        let old = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", home) };
        let out = f();
        match old {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        out
    }

    fn fake_server(home: &Path) {
        let server_dir = home.join(".local/share/Steam/steamapps/common/Arma Reforger Server");
        fs::create_dir_all(&server_dir).unwrap();
        let bin = server_dir.join("ArmaReforgerServer");
        fs::write(&bin, "#!/bin/true\n").unwrap();
        let mut perms = fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin, perms).unwrap();
    }

    #[test]
    fn no_addon_is_rc3() {
        let root = throwaway("noaddon");
        let home = root.join("home");
        fake_server(&home);
        let code = with_home(&home, || run_with_root(&root, &Opts::default()));
        assert_eq!(code, 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_server_is_rc3() {
        let root = throwaway("noserver");
        fs::write(root.join("apps/mod/tbd-framework/addon.gproj"), "x\n").unwrap();
        let home = root.join("empty-home");
        fs::create_dir_all(&home).unwrap();
        let code = with_home(&home, || run_with_root(&root, &Opts::default()));
        assert_eq!(code, 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_probe_is_rc2() {
        let root = throwaway("missprobe");
        let home = root.join("home");
        fake_server(&home);
        fs::write(root.join("apps/mod/tbd-framework/addon.gproj"), "x\n").unwrap();
        // tbd-export is gate-checked since the voxel-dump split — the fixture must satisfy the
        // env preconditions so the probe-arg check (the thing under test) is what fires.
        fs::create_dir_all(root.join("apps/mod/tbd-export")).unwrap();
        fs::write(root.join("apps/mod/tbd-export/addon.gproj"), "x\n").unwrap();
        let opts = Opts {
            probe_dir: Some(PathBuf::from("/tmp/t853/compile/no-such-probe-dir-ut")),
            ..Default::default()
        };
        let code = with_home(&home, || run_with_root(&root, &opts));
        assert_eq!(code, 2);
        let _ = fs::remove_dir_all(&root);
    }
}
