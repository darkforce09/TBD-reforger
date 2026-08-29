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

    let mut addons = String::from("TBD_Framework,TBD_Export");

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
                "the Game module loaded {loaded} files and vanilla-only is {vanilla}, so tbd-framework's scripts were NOT compiled — the engine skipped the loose addon entirely"
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
