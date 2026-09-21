use super::*;

/// Flush stdout. Call before writing to stderr and before spawning an inheriting child — see the
/// STREAM ORDERING note in the module header.
pub fn flush() {
    let _ = std::io::stdout().flush();
}

/// Write to the innermost active step capture, or to real stdout when no step is running.
pub fn emit(s: &str) {
    SINK.with(|c| {
        let mut b = c.borrow_mut();
        match b.last_mut() {
            Some(buf) => buf.push_str(s),
            None => {
                drop(b);
                print!("{s}");
            }
        }
    });
}

/// Write to the innermost active step capture, or to real stderr when no step is running.
pub fn emit_err(s: &str) {
    SINK.with(|c| {
        let mut b = c.borrow_mut();
        match b.last_mut() {
            Some(buf) => buf.push_str(s),
            None => {
                drop(b);
                flush();
                eprint!("{s}");
            }
        }
    });
}

/// `out="$(step 2>&1)"; rc=$?` — run `f` with its output captured.
pub fn capture_step<R>(f: impl FnOnce() -> R) -> (String, R) {
    SINK.with(|c| c.borrow_mut().push(String::new()));
    let r = f();
    let out = SINK.with(|c| c.borrow_mut().pop().unwrap_or_default());
    (out, r)
}

/// `git … 2>/dev/null` capturing trimmed stdout, `None` on any failure.
///
/// One helper rather than a `Command` at every site: the bash reached for git roughly ninety
/// times and swallowed stderr at nearly all of them, and the places where it did NOT swallow are
/// the interesting ones ([`ledger::git_porcelain_paths`], T-401).
pub fn git_stdout(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string(),
    )
}

/// As [`git_stdout`], but keeps the output even when git exited non-zero — the `|| true` shape.
pub fn git_stdout_lossy(args: &[&str]) -> String {
    match std::process::Command::new("git").args(args).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string(),
        Err(_) => String::new(),
    }
}

/// `git rev-parse --short <rev>`, or the input unchanged when git cannot resolve it.
///
/// An unresolvable rev yields `""`, so a refusal message interpolating it renders with an empty
/// span rather than failing: the message still names which rev it could not resolve by position.
pub fn short(rev: &str) -> String {
    git_stdout(&["rev-parse", "--short", rev]).unwrap_or_default()
}

/// `git log -1 --format=%s <rev>`, empty when git cannot resolve it (the `2>/dev/null` shape).
pub fn subject(rev: &str) -> String {
    git_stdout(&["log", "-1", "--format=%s", rev]).unwrap_or_default()
}

/// `$K` from the environment, empty treated as unset — `make`'s `?=` semantics, the same rule
/// [`Ctx::enter`] applies to `CARGO_TARGET_DIR`.
pub(super) fn env_nonempty(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|s| !s.is_empty())
}

/// `<cargo_target_dir>/run-main` — the whole formula, in one expression, so the driver and
/// `platform preflight` cannot answer this question differently.
///
/// A subdirectory of the shared cache, not a sibling: `platform wave reclaim` already spares the
/// shared dir wholesale, so a run target outside it is a 40 GB tree no sweep knows about. It is
/// also what makes the disk claim checkable — one `run-main`, whatever the worktree count.
pub fn run_target_dir_for(cargo_target_dir: &str) -> String {
    Path::new(cargo_target_dir)
        .join(RUN_TARGET_SUBDIR)
        .display()
        .to_string()
}

/// The run target for a caller with no [`Ctx`] — `platform preflight`, which must not build one
/// (`Ctx::enter` chdirs and prints the host banner).
///
/// Policy, in precedence order, and identical to the one [`Ctx::enter`] applies:
/// `$TBD_RUN_TARGET_DIR` (the driver exports it, so a child inherits the parent's answer), then
/// `$CARGO_TARGET_DIR/run-main`, then `<main checkout>/target/run-main`.
///
/// THE ASYMMETRY, AND WHY IT IS RIGHT. [`disown_ambient_target_dir`] strips `CARGO_TARGET_DIR`
/// before [`Ctx::enter`], so the DRIVER's run target is `<main checkout>/target/run-main` whatever
/// the shell exported; `platform preflight` does not disown, so standalone it answers with the
/// operator's cache — right, because its job is to describe the environment the operator's next
/// `cargo run` will use. Each caller gets the dir its own lanes write, from one formula.
pub fn resolve_run_target_dir(main_root: &Path) -> String {
    if let Some(v) = env_nonempty("TBD_RUN_TARGET_DIR") {
        return v;
    }
    let base = env_nonempty("CARGO_TARGET_DIR")
        .unwrap_or_else(|| main_root.join("target").display().to_string());
    run_target_dir_for(&base)
}

/// `<bin_dir>/tbd-built-from` — the stamp sits beside the binaries it describes, so deleting the
/// profile directory deletes the claim with it and cannot leave a stale one behind.
pub fn run_stamp_path(bin_dir: &Path) -> PathBuf {
    bin_dir.join(RUN_STAMP_FILE)
}

/// Write the stamp, creating the directory if cargo has not yet.
pub fn write_run_stamp(bin_dir: &Path, stamp: &RunStamp) -> std::io::Result<()> {
    std::fs::create_dir_all(bin_dir)?;
    std::fs::write(run_stamp_path(bin_dir), stamp.render())
}

/// Read the stamp. `None` covers BOTH "absent" and "unreadable as a stamp" — see
/// [`RunStamp::parse`]; the caller must treat that as unknown provenance, never as agreement.
pub fn read_run_stamp(bin_dir: &Path) -> Option<RunStamp> {
    RunStamp::parse(&std::fs::read_to_string(run_stamp_path(bin_dir)).ok()?)
}

/// `cargo xtask platform wave run <cargo args…> [-- <run args…>]` — the run lane.
///
/// Build, stamp, launch — all into [`Ctx::run_target_dir`], the stamp between the two so it
/// describes a build that succeeded and exists before the server it labels does. Argv left of
/// `--` goes to BOTH cargo verbs (`-p`, `--bin`, `--release` are accepted by each); right of it
/// is the program's and reaches only `run`. That is `ci_editor_api`'s own build-then-run shape,
/// which is the lane this exists to make safe.
///
/// NOT through [`host::Host::hostrun_argv`], and not for style: `hostrun` wraps every command in
/// `timeout $GATE_TIMEOUT` — a run lane's whole point is a process that outlives the gate's
/// 1200 s — and pipes both streams, which would hide a server's log until it exits. Cargo is
/// spawned directly with inherited stdio, as [`crate::commands::build::recipes`] does.
pub fn cmd_run(ctx: &Ctx, args: &[String]) -> u8 {
    if let Some(lines) = run_lane_refusal(&ctx.root, &ctx.main_root, &ctx.run_target_dir, args) {
        for l in &lines {
            werr!("{l}");
        }
        return 1;
    }
    // The two-glibc guard, reused rather than re-derived: a container-built run-main read back by
    // host cargo is `GLIBC_2.xx not found`, which reads as a broken checkout (T-853).
    if let Err(msg) = crate::core::cargo_target_directory::abi_guard(Path::new(&ctx.run_target_dir))
    {
        werr!("run: {msg}");
        return 1;
    }

    let (build_args, run_args) = split_run_args(args);
    let profile = if build_args.iter().any(|a| a == "--release") {
        "release"
    } else {
        "debug"
    };

    wprintln!("run: CARGO_TARGET_DIR={}", ctx.run_target_dir);
    let rc = spawn_cargo(ctx, "build", &build_args, &[]);
    if rc != 0 {
        werr!("run: build failed (rc {rc}) — nothing stamped, nothing launched.");
        return rc;
    }

    let bin_dir = Path::new(&ctx.run_target_dir).join(profile);
    let stamp = RunStamp {
        sha: git_stdout(&["rev-parse", "HEAD"]).unwrap_or_default(),
        checkout: ctx.root.display().to_string(),
    };
    if stamp.sha.len() != 40 {
        // Refuse rather than write a stamp preflight will reject anyway: an unresolvable HEAD
        // means we cannot say what this binary is, and launching an unlabelled binary is the
        // state this whole ticket exists to end.
        werr!("run: REFUSING — `git rev-parse HEAD` did not resolve; cannot stamp the build.");
        return 1;
    }
    if let Err(e) = write_run_stamp(&bin_dir, &stamp) {
        werr!(
            "run: REFUSING — could not write {}: {e}",
            run_stamp_path(&bin_dir).display()
        );
        return 1;
    }
    wprintln!(
        "run: {} {} {}",
        RUN_STAMP_FILE,
        short(&stamp.sha),
        stamp.checkout
    );

    spawn_cargo(ctx, "run", &build_args, &run_args)
}

/// The three reasons a run lane must not proceed, rendered as the stderr it prints, or `None`.
///
/// A free function over paths rather than a method on [`Ctx`]: building a `Ctx` chdirs and
/// detects the host bridge, so the refusals could only be tested by the thing they prevent.
/// Returning the LINES keeps the message under test — a refusal naming the wrong path is a
/// refusal nobody acts on.
pub(super) fn run_lane_refusal(
    root: &Path,
    main_root: &Path,
    run_target_dir: &str,
    args: &[String],
) -> Option<Vec<String>> {
    // R1 — no argv builds the whole workspace and stamps it as if a run binary existed.
    if args.is_empty() {
        return Some(vec![
            "run: REFUSING — no cargo arguments.".into(),
            "     usage: cargo xtask platform wave run -p website-api --bin api".into(),
            "            cargo xtask platform wave run -p developer-tools --bin world -- reclassify"
                .into(),
        ]);
    }
    // R2 — a caller naming its own target dir has opted out and would still get a stamp.
    if let Some(bad) = args
        .iter()
        .find(|a| a.starts_with("--target-dir") || a.starts_with("CARGO_TARGET_DIR="))
    {
        return Some(vec![
            format!("run: REFUSING — `{bad}` overrides the run target this lane exists to pin."),
            "     Drop it, or use `cargo xtask platform wave test --slice <T-xxx>` for a".into(),
            "     private per-slice dir (T-742).".into(),
        ]);
    }
    // R3 — THE MAIN GOAL, at the only place it can be enforced. `run-main` is MAIN's; a
    // worktree writing it is the wave-1 incident again, one layer down. A slice that needs a
    // live server takes a private dir instead: disk, but it cannot poison anybody.
    if root != main_root {
        return Some(vec![
            "run: REFUSING — this is a worktree, and the run target belongs to the main checkout."
                .into(),
            format!("     worktree      = {}", root.display()),
            format!("     main checkout = {}", main_root.display()),
            format!("     run target    = {run_target_dir}"),
            "     A worktree build here would be executed by main's next run lane (T-300).".into(),
            "     Use CARGO_TARGET_DIR=<main_root>/target-<slice> for a private server dir.".into(),
        ]);
    }
    None
}

/// Split a run-lane argv at the first bare `--`: left goes to both cargo verbs, right only to
/// `run`. A missing `--` means everything is a cargo argument.
pub(super) fn split_run_args(args: &[String]) -> (Vec<String>, Vec<String>) {
    match args.iter().position(|a| a == "--") {
        Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
        None => (args.to_vec(), Vec::new()),
    }
}

/// `cargo <verb> <cargo_args…> [-- <run_args…>]` in the run target, stdio inherited.
pub(super) fn spawn_cargo(ctx: &Ctx, verb: &str, cargo_args: &[String], run_args: &[String]) -> u8 {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg(verb)
        .args(cargo_args)
        .current_dir(&ctx.root)
        .env("CARGO_TARGET_DIR", &ctx.run_target_dir);
    if !run_args.is_empty() {
        cmd.arg("--").args(run_args);
    }
    // Flush before the child inherits stdout — the STREAM ORDERING contract in the header.
    flush();
    match cmd.status() {
        // A signalled child has NO exit code; `128+n` is bash's fiction and [`host::capture`]
        // reproduces it deliberately, so this does too. The OOM killer is a routine visitor here.
        Ok(s) => match s.code() {
            Some(code) => (code & 0xff) as u8,
            None => {
                let sig = std::os::unix::process::ExitStatusExt::signal(&s).unwrap_or(0);
                werr!("run: cargo {verb} was killed by signal {sig} — not a build failure.");
                128u8.saturating_add(sig as u8)
            }
        },
        // 127 is "not installed", not "it ran and failed" (`NotRun::ToolAbsent`).
        Err(e) => {
            werr!("run: failed to spawn cargo {verb}: {e}");
            127
        }
    }
}

/// Entry for `cargo xtask platform wave [args…]`.
///
/// The bash tail was a `case "${1:-status}"`, so no argument means `status`. Exit codes are the
/// command's own; the unknown arm prints the header and exits 1.
/// Strip an inherited `CARGO_TARGET_DIR` before anything spawns.
///
/// ── WHY THIS IS AT THE ENTRY POINT AND NOT AT EACH CALL SITE ─────────────────────────────────
///
/// This driver CHOOSES target dirs; it does not take one. It has three, each for a measured
/// reason: `GATE_CHECK_TARGET` so the gate's artifacts are written by the gate alone (that is what
/// makes one fingerprint invalidation hold for every step under it), a per-slice private dir for
/// ad-hoc `cargo test` (T-742, so a slice cannot read a sibling's binary), and the shared warm
/// cache derived from `git rev-parse --git-common-dir` so every linked worktree points at the
/// PRIMARY repo's `target/` instead of cold-building a 609-crate workspace eight times.
///
/// An ambient `CARGO_TARGET_DIR` can only fight all three. MEASURED 2026-08-12, and this is what
/// motivated the guard: the driver was invoked with `CARGO_TARGET_DIR=<repo>/target-container`
/// exported. Steps that cross the bridge run cargo ON THE HOST, so host cargo (glibc 2.43) wrote
/// host binaries into the container's target dir, and the next in-container `cargo run` died with
/// `GLIBC_2.39 not found` — a link error that reads exactly like a broken checkout and is not one.
/// That is the same two-glibc trap `scripts/lib/hostrun.sh` was written for, arriving through an
/// environment variable instead of a compiler.
///
/// Removing it is right rather than merely convenient: there is no value a caller could supply
/// that this driver should honour. Announced, never silent — a tool that quietly edits its own
/// environment is the thing that makes the next failure unexplainable.
pub(super) fn disown_ambient_target_dir() {
    if let Ok(v) = std::env::var("CARGO_TARGET_DIR") {
        if !v.is_empty() {
            eprintln!("wave: ignoring inherited CARGO_TARGET_DIR={v}");
            eprintln!(
                "      This driver picks its own (gate-check, per-slice private, shared warm cache)."
            );
            eprintln!(
                "      An inherited one crosses the container/host bridge and poisons the cache it names."
            );
            // SAFETY: single-threaded entry, before any child is spawned or thread started.
            unsafe { std::env::remove_var("CARGO_TARGET_DIR") };
        }
    }
}

pub fn run(args: &[String]) -> Result<u8> {
    disown_ambient_target_dir();
    let ctx = Ctx::enter()?;
    let cmd = args.first().map(String::as_str).unwrap_or("status");
    let rest: Vec<String> = args.iter().skip(1).cloned().collect();

    let rc = match cmd {
        "status" => status::cmd_status(&ctx),
        "prep" => status::cmd_prep(&ctx),
        "test" => test_cmd::cmd_test(&ctx, &rest),
        // T-300. Sibling of `test`, and for the same reason one layer over: `test` keeps a slice's
        // cargo test off the shared cache, `run` keeps a launched binary off it.
        "run" => cmd_run(&ctx, &rest),
        "gate" => match rest.first().map(String::as_str) {
            Some("--slice") => {
                gate::gate_slice(&ctx, rest.get(1).map(String::as_str).unwrap_or(""))
            }
            // `advance` writes the shared persist DB, so it takes the same lock the wave gate
            // holds when it calls this. GATE_LOCK_HELD is deliberately not settable from the
            // environment (it is reset at load), so there is no way to skip this by exporting a
            // variable — here, a `GateLock` has no public constructor at all (T-406).
            Some("--migrate-persist") => {
                let mode = rest.get(1).map(String::as_str).unwrap_or("audit");
                let mut state = lock::GateState::new();
                if mode == "advance" {
                    match state.take(&ctx, "migrate-persist advance") {
                        0 => {}
                        n => return Ok(n),
                    }
                }
                migrate::gate_db_migrate_persist(&ctx, &state, mode)
            }
            other => gate::cmd_gate(&ctx, other.unwrap_or("")),
        },
        "wave" => {
            if rest.first().map(String::as_str) == Some("--close") {
                // T-923: everything after `--close` belongs to the close ceremony
                // (`--summary <text>`, `--dry-run`) and is allowlist-parsed there.
                land::cmd_wave_close(&ctx, &rest[1..])
            } else {
                status::cmd_wave(&ctx)
            }
        }
        "verified" => land::cmd_verified(&ctx, rest.first().map(String::as_str).unwrap_or("")),
        "reclaim" => reclaim::cmd_reclaim(&ctx, &rest),
        "land" => land::cmd_land(&ctx, &rest),
        "revert" => land::cmd_revert(&ctx, rest.first().map(String::as_str).unwrap_or("")),
        "push" => push::cmd_push(&ctx),
        _ => {
            println!("{UNKNOWN_HELP}");
            1
        }
    };
    flush();
    Ok(rc)
}
