use super::*;

pub fn verify_crf_leak(repo_root: &Path) -> Result<u8> {
    let mut log = Log {
        lines: Vec::new(),
        echo: true,
    };
    Ok(run(&Lanes::from_env(repo_root), &mut log))
}

/// The script body: four checks, then the epilogue-and-exit-1 or the PASS line.
pub(super) fn run(lanes: &Lanes, log: &mut Log) -> u8 {
    let mut fail = false;
    // One vanilla answer per bare GUID for the whole run. The two lanes share 13 GUIDs (measured)
    // and bash re-greps each; the paks cannot change mid-run, so memoising is provably the same
    // answer for a fraction of the I/O — and a *miss* costs 20 GB.
    let mut memo: HashMap<String, bool> = HashMap::new();

    for (label, prefix) in IDENT_LANES {
        let ours = [lanes.mod_dir.as_path(), lanes.export_dir.as_path()];
        match check_identifier_leak(log, &ours, label, prefix) {
            Ok(hit) => fail |= hit,
            Err(cause) => return refuse(log, cause),
        }
    }
    for (label, oracle) in [("CRF", &lanes.crf), ("PlayableSelector", &lanes.ps)] {
        match check_guid_leak(log, lanes, label, oracle, &mut memo) {
            Ok(hit) => fail |= hit,
            Err(cause) => return refuse(log, cause),
        }
    }

    if fail {
        log.say("");
        log.say("Oracles are reference-only. Design-mirror them; never copy them.");
        log.say("  CRF              — Arma Public License; read, cite, do not vendor.");
        log.say(EPILOGUE_PS);
        log.say(format!(
            "See {} §2 and {} §Oracle lanes.",
            crate::core::repository_layout::documentation::MOD_DESIGN,
            crate::core::repository_layout::documentation::SLICE_WORKFLOW_RUNBOOK
        ));
        return 1;
    }
    log.say("no-oracle-leak: PASS (CRF + PlayableSelector)");
    0
}

/// Exit **2**, not the script's 1: "the tree is dirty" and "I never read the tree" are different
/// operator actions. `wave.sh` tests `rc -eq 0`, so any nonzero is still FAIL there.
pub(super) fn refuse(log: &mut Log, cause: NotRun) -> u8 {
    let msg = "no-oracle-leak could not examine the trees it was pointed at";
    log.say(Verdict::did_not_run(msg, Kind::Ban, cause).to_string());
    2
}

pub(super) fn check_identifier_leak(
    log: &mut Log,
    roots: &[&Path],
    label: &str,
    prefix: &str,
) -> Result<bool, NotRun> {
    log.say(format!(
        "==> {prefix} identifiers in tbd-framework + tbd-export code ({label})"
    ));
    let ident = pattern(&format!("(^|[^A-Za-z0-9_]){prefix}"))?;
    let comment = pattern(COMMENT_RE)?;

    let mut hits: Vec<String> = Vec::new();
    for file in scan::walk_files(roots, outside_excluded_dir)? {
        let bytes = read(&file)?;
        for (line_no, line) in numbered(grep_visible(&bytes)) {
            if !ident.is_match(&line) {
                continue;
            }
            // Build `grep -rn`'s exact rendering first: the comment filter is anchored on it, not
            // on the source line, and `$MOD` was absolute so these paths are absolute too.
            let rendered = format!("{}:{line_no}:{line}", file.display());
            if !comment.is_match(&rendered) {
                hits.push(rendered);
            }
        }
    }

    if hits.is_empty() {
        log.say("  OK (none)");
        return Ok(false);
    }
    log.say(format!(
        "FAIL: {prefix} symbols found in our mod trees (tbd-framework, tbd-export):"
    ));
    for hit in hits.iter().take(HEAD) {
        log.say(hit.clone());
    }
    Ok(true)
}

/// `--exclude-dir=EnfusionMCP`. grep prunes the directory; [`scan::walk_files`] filters files, so
/// we still descend into it and discard — same output, a few stat calls more.
pub(super) fn outside_excluded_dir(path: &Path) -> bool {
    !path.components().any(|c| c.as_os_str() == EXCLUDE_DIR)
}

pub(super) fn check_guid_leak(
    log: &mut Log,
    lanes: &Lanes,
    label: &str,
    oracle: &Path,
    memo: &mut HashMap<String, bool>,
) -> Result<bool, NotRun> {
    log.say(format!(
        "==> {label} layout/prefab GUIDs reused in tbd-framework or tbd-export"
    ));
    // `[ -d ]` follows symlinks, and so does `is_dir`.
    if !oracle.is_dir() {
        log.say(format!("  SKIP — {label} {SKIP_TAIL}"));
        return Ok(false);
    }
    let dirs = asset_dirs(oracle)?;
    if dirs.is_empty() {
        // Deliberately NOT worded as OK: reaching here means we compared nothing, which is how the
        // `find -L` symlink bug hid itself.
        log.say(format!(
            "  SKIP — no UI/ or Prefabs/ dirs under {}; NO GUID comparison was made",
            oracle.display()
        ));
        return Ok(false);
    }

    let guid = Regex::new(GUID_RE).map_err(|e| broken_pattern(GUID_RE, e))?;
    let refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
    let oracle_guids = guids_under(&guid, &refs)?;
    // Recomputed per lane, as in the script. 143 files; the repeat costs nothing.
    let ours = guids_under(
        &guid,
        &[lanes.mod_dir.as_path(), lanes.export_dir.as_path()],
    )?;
    if oracle_guids.is_empty() || ours.is_empty() {
        log.say("  OK (nothing to compare)");
        return Ok(false);
    }

    // bash `comm -12` over two `sort -u` streams. A `BTreeSet` intersection is byte order, which
    // for these fixed-shape `{16 uppercase hex}` strings is exactly what en_AU.UTF-8 collation
    // produces — verified against the script's own output ordering.
    let mut leaks: Vec<&String> = Vec::new();
    for g in oracle_guids.intersection(&ours) {
        if in_vanilla(&lanes.vanilla, &g.replace(['{', '}'], ""), memo)? {
            continue; // present in vanilla -> engine fact, not an oracle leak
        }
        leaks.push(g);
    }

    if leaks.is_empty() {
        log.say("  OK (shared GUIDs are all vanilla engine facts)");
        return Ok(false);
    }
    log.say(format!(
        "FAIL: {label}-only asset GUIDs reused (not present in vanilla):"
    ));
    for g in &leaks {
        log.say(format!("  {g}"));
    }
    Ok(true)
}

/// `find -L "$oracle" -maxdepth 2 -type d \( -name UI -o -name Prefabs \)`.
///
/// Depth 0 is included because `find` includes the start point. Symlinks are followed at every
/// level — see the module docs; `maxdepth 2` is also what bounds a symlink cycle.
pub(super) fn asset_dirs(oracle: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let mut out = Vec::new();
    if is_asset_dir(oracle) {
        out.push(oracle.to_path_buf());
    }
    for one in children(oracle)? {
        if !one.is_dir() {
            continue;
        }
        if is_asset_dir(&one) {
            out.push(one.clone());
        }
        for two in children(&one)? {
            if is_asset_dir(&two) {
                out.push(two);
            }
        }
    }
    out.sort();
    Ok(out)
}

pub(super) fn is_asset_dir(path: &Path) -> bool {
    let named = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| ASSET_DIR_NAMES.contains(&n));
    named && path.is_dir()
}

/// One directory level. `find` would print a suppressed error and carry on; refusing instead is
/// the anti-fail-open choice — a lane we could not read must not report "nothing to compare".
pub(super) fn children(dir: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let bad = |source: std::io::Error| NotRun::Unreadable {
        path: dir.to_path_buf(),
        source,
    };
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(bad)? {
        out.push(entry.map_err(bad)?.path());
    }
    Ok(out)
}

/// `grep -rhoE '\{[0-9A-F]{16}\}' … | sort -u`.
///
/// `Regex` directly rather than [`Pattern`], which exposes only `is_match` while `-o` needs every
/// match. Scanning whole text instead of line by line is identical here: the pattern holds no `.`
/// and no newline, so a match can never span a line break.
pub(super) fn guids_under(guid: &Regex, roots: &[&Path]) -> Result<BTreeSet<String>, NotRun> {
    let mut out = BTreeSet::new();
    for file in scan::walk_files(roots, |_| true)? {
        let bytes = read(&file)?;
        for m in guid.find_iter(&String::from_utf8_lossy(grep_visible(&bytes))) {
            out.insert(m.as_str().to_string());
        }
    }
    Ok(out)
}

/// `[ -d "$game" ] && grep -qla "$bare" "$game"/*.pak 2>/dev/null`.
///
/// Every non-zero status is "not in vanilla", exactly as the `&&` chain read it: 1 is a clean
/// miss, 2 is grep erroring on an unexpanded glob. A tool that never ran is *not* folded in — it
/// propagates as `NotRun` and exits 2, where bash would have called 127 a leak, 92 times over.
pub(super) fn in_vanilla(
    dir: &Path,
    bare: &str,
    memo: &mut HashMap<String, bool>,
) -> Result<bool, NotRun> {
    if let Some(hit) = memo.get(bare) {
        return Ok(*hit);
    }
    let paks = paks(dir);
    let hit = if paks.is_empty() {
        false // No Steam install, or a glob that matched nothing. Module docs oddity 7.
    } else {
        let probe = Run::new("grep").arg("-qla").arg(bare).args(&paks);
        probe.status()? == 0
    };
    memo.insert(bare.to_string(), hit);
    Ok(hit)
}

/// The shell glob `"$game"/*.pak`: sorted, dotfiles excluded, no type test — a directory named
/// `*.pak` would be handed to grep here exactly as bash hands it over.
pub(super) fn paks(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let glob = |n: &str| n.ends_with(".pak") && !n.starts_with('.');
    let named = |p: &PathBuf| p.file_name().and_then(|n| n.to_str()).is_some_and(glob);
    let mut out: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(named).collect();
    out.sort();
    out
}

pub(super) fn read(path: &Path) -> Result<Vec<u8>, NotRun> {
    std::fs::read(path).map_err(|source| NotRun::Unreadable {
        path: path.to_path_buf(),
        source,
    })
}

/// The bytes GNU grep 3.8 would actually produce output from.
///
/// Measured against `/usr/bin/grep` 3.8: a NUL inside the first read buffer flags the whole file
/// binary and it emits nothing at all; a NUL that arrives later lets the matches *before* it print
/// and swallows the rest. Both `--binary-files=without-match` (arm 1) and the default mode with
/// `-o` (arm 2) behave this way on stdout, which is all the script captures — it pipes stdout and
/// ends with `|| true`, so grep's exit status never reaches a decision.
pub(super) fn grep_visible(bytes: &[u8]) -> &[u8] {
    if bytes[..bytes.len().min(GREP_BUF)].contains(&0) {
        return &[];
    }
    match bytes.iter().position(|b| *b == 0) {
        Some(cut) => &bytes[..cut],
        None => bytes,
    }
}

/// `grep -n`'s numbering: 1-based, split on `\n` only.
///
/// Not [`scan::grep_lines`], which uses `str::lines` and therefore **strips a trailing `\r`**.
/// grep keeps it, and `tbd-framework` does hold a CRLF file the Workbench MCP bridge wrote, so the
/// difference is one commit away from being observable.
pub(super) fn numbered(bytes: &[u8]) -> Vec<(usize, String)> {
    let mut lines: Vec<&[u8]> = bytes.split(|b| *b == b'\n').collect();
    // `split` yields a trailing empty piece for a file that ends in a newline; grep does not
    // count that as a line. A file NOT ending in one still has its last partial line counted.
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let n = |(i, l): (usize, &&[u8])| (i + 1, String::from_utf8_lossy(l).into_owned());
    lines.iter().enumerate().map(n).collect()
}

/// A pattern constant that will not compile is a bug in THIS file; it must not read as "no hits".
pub(super) fn pattern(src: &str) -> Result<Pattern, NotRun> {
    Pattern::regex(src).map_err(|e| broken_pattern(src, e))
}

pub(super) fn broken_pattern(src: &str, e: regex::Error) -> NotRun {
    NotRun::ToolError {
        tool: "regex".into(),
        status: 1,
        stderr: format!("{src}: {e}"),
    }
}
