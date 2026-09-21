use super::*;

/// Entry for `xtask mod compile [flags…]`.
pub fn run(args: &[String]) -> Result<u8> {
    match parse_args(args) {
        Err(a) => {
            eprintln!("mod compile: unknown arg '{a}'");
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
    fail
}

/// Testable entry (no root walk).
pub fn run_with_root(root: &Path, opts: &Opts) -> u8 {
    if require_host().is_err() {
        return env_fail(
            "no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine",
            Some(
                "See tools_v2/xtask/src/core/host_execution.rs: the container has no C toolchain and an older glibc, so the game binary cannot run in here at all.",
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
    if let Some(probe) = &opts.probe_dir
        && !probe.is_dir()
    {
        eprintln!("mod compile: --probe dir not found: {}", probe.display());
        return 2;
    }

    let max_wait: u64 = std::env::var("TBD_COMPILE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180);

    let run_dir = match mktemp_dir("tbd-compile") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("mod compile: could not create run dir: {e}");
            return 2;
        }
    };

    let session = Session::install(run_dir.clone(), opts.keep_logs);
    let code = match compile_inner(root, &mod_src, &server_dir, &run_dir, opts, max_wait) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("mod compile: internal error: {e}");
            2
        }
    };
    session.finish();
    code
}

pub(super) fn compile_inner(
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
    // ONE ADDON. The gate compiles what ships: a dedicated server loads only `TBD_Framework`.
    // `apps/mod/tbd-export` is a standalone addon that depends on vanilla Reforger and on
    // tbd-emcp; both are Workbench tooling a dedicated server never reads
    // (`Scripts/WorkbenchGame`, see the help text), so they compile inside Workbench. The one
    // thing tbd-export holds that this gate could compile is its five
    // `Scripts/Game/TBD/Export/*.c` road-exporter scripts — to cover them, symlink
    // `apps/mod/tbd-export` beside the framework link above and append `,TBD_Export` here.
    let mut addons = String::from("TBD_Framework");

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
            eprintln!("mod compile: no .c files in {}", probe.display());
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

    println!("==> compiling tbd-framework (native headless server, no Workbench)");

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
    if let Some(code) = load_count_guard(root, mod_src, server_dir, &console)? {
        return Ok(code);
    }

    let stray = workbench_tooling_guard(mod_src);
    if !stray.is_empty() {
        println!();
        println!("FAIL: tbd-framework carries Workbench tooling");
        for l in &stray {
            println!("  {l}");
        }
        println!("      The shipping mod has no Scripts/WorkbenchGame by design (2026-09-12): the");
        println!(
            "      map-export plugins live in apps/mod/tbd-export and the enfusion-mcp handlers"
        );
        println!(
            "      in apps/mod/tbd-emcp. An injected EnfusionMCP/ copy (wb_launch gprojPath=…)"
        );
        println!("      lands here too — delete it; the server cannot compile any of it.");
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

/// Every file under `base`, sorted; `.git` pruned. A missing `base` is an `Err`, never an empty
/// list — a tree that is not there must not read as "nothing to check".
pub(super) fn walk_files(base: &Path) -> io::Result<Vec<PathBuf>> {
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
}

/// How many `.c` files `addon/Scripts/Game` holds — the number of files the engine adds to the
/// Game module for this addon, and so the term [`load_count_guard`] adds to the vanilla baseline.
/// Only `.c` counts: the tree also carries `README.md` placeholders the engine never loads. A
/// missing `Scripts/Game` is an `Err` (rc 2 upstream), never a silent 0 — a tree without its
/// scripts must not pass as "nothing expected".
pub(super) fn count_game_scripts(addon: &Path) -> io::Result<u64> {
    Ok(walk_files(&addon.join("Scripts/Game"))?
        .iter()
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("c"))
        .count() as u64)
}

/// 2026-09-12 — the shipping mod carries no Workbench tooling, and this is what keeps it so.
///
/// `Scripts/WorkbenchGame` left tbd-framework when tbd-export stopped being a mirror of it: the
/// map-export plugins live in `apps/mod/tbd-export`, the enfusion-mcp bridge handlers in
/// `apps/mod/tbd-emcp`, and tbd-export depends on both. The dedicated server never reads that
/// module, so nothing in this gate can compile it — this guard is the only thing that notices it
/// coming back. It does come back: the MCP's `wb_launch gprojPath=…` copies 19 `EMCP_WB_*.c` into
/// whichever addon was opened, and a second copy beside tbd-emcp's is a Workbench "Multiple
/// declaration" that kills the bridge.
///
/// Returns one line per finding — the directory, then up to five of its files so the report says
/// what landed there; empty means clean.
pub(super) fn workbench_tooling_guard(mod_src: &Path) -> Vec<String> {
    let dir = mod_src.join("Scripts/WorkbenchGame");
    if !dir.exists() {
        return Vec::new();
    }
    let mut out = vec![format!("{}: must not exist", dir.display())];
    if let Ok(files) = walk_files(&dir) {
        for p in files.iter().take(5) {
            out.push(format!("  {}", p.display()));
        }
        if files.len() > 5 {
            out.push(format!("  … {} more", files.len() - 5));
        }
    }
    out
}
