use super::*;

/// The closure gate: (1) zero tracked `.mjs`/`.cjs` outside `apps/mod`; (2) no `node `/`npx `
/// invocations under [`SCAN_DIRS`] / [`SCAN_FILES`] outside the enfusion-mcp floor
/// (`xtask mcp call` `.js` runner tiers in gate_mcp_call.rs); (3) zero `actions/setup-node` in CI.
///
/// ── T-897: WHY THE DECLARED LIST FAILS CLOSED ────────────────────────────────────────────────
///
/// Check (2) used to open its subjects with `let Ok(text) = read_to_string(path) else { return }`
/// and hardcode `scan(root.join("Makefile"))`. Deleting the Makefile would therefore have REMOVED
/// one third of the gate's reach while it went on printing `OK (none)` — the defect class T-853
/// exists to kill: a check whose subject is a file, where deleting the file retires the check
/// instead of failing it. Subjects are now DECLARED ([`SCAN_FILES`] / [`SCAN_DIRS`]) and a
/// declared subject that cannot be read is reported as a failure with its cause.
pub fn verify_no_node() -> Result<u8> {
    let root = repo_root()?;
    let mut fails = 0u64;

    println!("==> git ls-files '*.mjs' '*.cjs' (excl apps/mod)");
    let out = std::process::Command::new("git")
        .args(["ls-files", "*.mjs", "*.cjs"])
        .current_dir(&root)
        .output()?;
    let tracked: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.starts_with("apps/mod/"))
        .map(str::to_string)
        .collect();
    if tracked.is_empty() {
        println!("  OK (none)");
    } else {
        println!("FAIL: tracked Node scripts remain:");
        for t in &tracked {
            println!("  {t}");
        }
        fails += 1;
    }

    let declared: String = SCAN_FILES
        .iter()
        .copied()
        .chain(SCAN_DIRS.iter().copied())
        .collect::<Vec<&str>>()
        .join(" + ");
    println!("==> node/npx invocations in {declared} (allowlist: enfusion-mcp floor)");
    // Files allowed to invoke node/npx: the enfusion-mcp runner tiers only.
    // Floor moved to tools_v2/xtask/src/commands/mcp/call.rs (not scanned here — SCAN_DIRS/SCAN_FILES only).
    let allow_files: &[&str] = &[];
    let mut offenders: Vec<String> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    fn walk_scripts(dir: &Path, acc: &mut Vec<PathBuf>, unreadable: &mut Vec<String>) {
        let rd = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) => {
                unreadable.push(format!("{}: {e}", dir.display()));
                return;
            }
        };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if name != "node_modules" && name != "__pycache__" {
                    walk_scripts(&p, acc, unreadable);
                }
            } else if name.ends_with(".sh") || name.ends_with(".yml") || name.ends_with(".yaml") {
                acc.push(p);
            }
        }
    }

    // Resolve every declared subject FIRST. Absent is a failure, not a smaller scan.
    let mut subjects: Vec<PathBuf> = Vec::new();
    for f in SCAN_FILES {
        let p = root.join(f);
        if p.is_file() {
            subjects.push(p);
        } else {
            missing.push((*f).to_string());
        }
    }
    let mut targets: Vec<PathBuf> = Vec::new();
    for d in SCAN_DIRS {
        let p = root.join(d);
        if p.is_dir() {
            walk_scripts(&p, &mut targets, &mut unreadable);
        } else {
            missing.push((*d).to_string());
        }
    }
    subjects.extend(targets.iter().cloned());

    for path in &subjects {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                // NOT a silent `continue`. A subject this gate claims to scan and cannot read is a
                // check that did not happen, and a check that did not happen is not a pass.
                unreadable.push(format!("{rel}: {e}"));
                continue;
            }
        };
        if allow_files.contains(&rel.as_str()) {
            continue;
        }
        for (i, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with('#') || t.starts_with("//") {
                continue; // comments may reference the floor
            }
            // invocation shapes only: `node <arg>` / `npx <arg>` in command position
            // (drop inline `##` help text first — a task's help line may NAME the ban).
            let code = line.split("##").next().unwrap_or(line);
            let hit = code.split(&['|', ';', '&', '(', ')'][..]).any(|seg| {
                let seg = seg.trim_start();
                seg.starts_with("node ") || seg.starts_with("npx ")
            });
            if hit {
                offenders.push(format!("{rel}:{} {}", i + 1, line.trim()));
            }
        }
    }
    if !missing.is_empty() {
        println!("FAIL: declared scan subject(s) absent — the gate would have narrowed silently:");
        for m in &missing {
            println!("  {m}");
        }
        println!(
            "      Restore the path, or delete it from SCAN_FILES/SCAN_DIRS in tools_v2/xtask/src/verifications/language_bans/node_and_file_limits.rs"
        );
        println!("      in the SAME commit that deletes the file. (T-897)");
        fails += 1;
    }
    if !unreadable.is_empty() {
        println!("FAIL: declared scan subject(s) unreadable — those bytes were never examined:");
        for u in &unreadable {
            println!("  {u}");
        }
        fails += 1;
    }
    if offenders.is_empty() {
        println!("  OK (none)");
    } else {
        println!("FAIL: node/npx invocations outside the enfusion-mcp floor:");
        for o in &offenders {
            println!("  {o}");
        }
        fails += 1;
    }

    println!("==> actions/setup-node in workflows");
    let mut setup_node = Vec::new();
    for t in &targets {
        if t.to_string_lossy().contains(".github")
            && std::fs::read_to_string(t).is_ok_and(|s| s.contains("actions/setup-node"))
        {
            setup_node.push(
                t.strip_prefix(&root)
                    .unwrap_or(t)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    if setup_node.is_empty() {
        println!("  OK (none)");
    } else {
        println!("FAIL: setup-node steps remain: {}", setup_node.join(", "));
        fails += 1;
    }

    if fails > 0 {
        eprintln!("\nverify-no-node: FAIL ({fails})");
        return Ok(1);
    }
    println!("\nverify-no-node: OK — Node exists solely as the enfusion-mcp runtime");
    Ok(0)
}
