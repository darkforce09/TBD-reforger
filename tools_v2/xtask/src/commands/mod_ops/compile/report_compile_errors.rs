use super::*;

pub(super) fn report_compile_errors(
    mod_src: &Path,
    run_dir: &Path,
    errlog: &Path,
) -> io::Result<u8> {
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

/// The Game module's file count is the vanilla baseline plus every `.c` under
/// `tbd-framework/Scripts/Game` — one addon, one term (since 2026-09-12; before that the count was
/// the union of the two mirror trees, which could not tell "framework lost a file" from "fine",
/// T-946.24). The check is `loaded >= vanilla + ours`, not `==`: the baseline is calibrated once
/// per machine and only ever goes stale LOW as the vanilla game grows (this box: 5633 calibrated
/// 2026-07; green runs already reported 5761), and `--probe` adds files of its own. Anything below
/// the sum means the engine skipped part or all of the addon — the stale-rdb failure this guard
/// exists for, now including the partial case the union count could never see.
pub(super) fn load_count_guard(
    root: &Path,
    mod_src: &Path,
    server_dir: &Path,
    console: &Path,
) -> io::Result<Option<u8>> {
    let loaded = last_num(console, r"Module: Game; loaded ([0-9]*)x files").unwrap_or(0);
    let ours = count_game_scripts(mod_src)?;
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
    if vanilla > 0 && loaded < vanilla + ours {
        let what = if loaded <= vanilla {
            "NONE of tbd-framework's scripts were compiled — the engine skipped the loose addon entirely".to_string()
        } else {
            format!(
                "only {} of tbd-framework's scripts were compiled — the engine skipped part of the loose addon",
                loaded - vanilla
            )
        };
        return Ok(Some(env_fail(
            &format!(
                "the Game module loaded {loaded} files; vanilla-only is {vanilla} and tbd-framework holds {ours} Scripts/Game .c files, so at least {} were expected: {what}",
                vanilla + ours
            ),
            Some(
                "Almost always a stale or unreadable apps/mod/tbd-framework/resourceDatabase.rdb (it IS committed, but the engine rejects it once it drifts from the script tree). Fix: open apps/mod/tbd-export/addon.gproj in Workbench (it loads tbd-framework as a dependency and regenerates the rdb), then re-run. If the vanilla game shrank instead, delete .compile-vanilla-baseline to recalibrate.",
            ),
        )));
    }
    if vanilla > 0 {
        println!(
            "    Game module: {loaded} files = {vanilla} vanilla + {ours} tbd-framework (+{} beyond the sum)",
            loaded - vanilla - ours
        );
    }
    Ok(None)
}

pub(super) fn parse_args(args: &[String]) -> std::result::Result<Parse, String> {
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

pub(super) fn env_fail(msg: &str, hint: Option<&str>) -> u8 {
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

pub(super) fn latest_logs_dir(logs: &Path) -> Option<PathBuf> {
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

pub(super) fn file_contains(path: &Path, needle: &str) -> bool {
    fs::read_to_string(path)
        .map(|t| t.contains(needle))
        .unwrap_or(false)
}

pub(super) fn last_re(path: &Path, pat: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    Regex::new(pat)
        .ok()?
        .find_iter(&text)
        .last()
        .map(|m| m.as_str().to_string())
}

pub(super) fn last_num(path: &Path, pat: &str) -> Option<u64> {
    let text = fs::read_to_string(path).ok()?;
    let caps = Regex::new(pat).ok()?.captures_iter(&text).last()?;
    caps.get(1)?.as_str().parse().ok()
}

pub(super) fn count_tbd_warnings(errlog: &Path) -> usize {
    fs::read_to_string(errlog)
        .map(|t| {
            t.lines()
                .filter(|l| l.contains("SCRIPT    (W): @\"Scripts/Game/TBD/"))
                .count()
        })
        .unwrap_or(0)
}
