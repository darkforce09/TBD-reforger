use super::*;

/// bash `usage_fail` — rc **2**.
pub(super) fn usage_fail(msg: &str) -> u8 {
    eprintln!("ERROR: {msg}");
    eprintln!("{USAGE_LINE}");
    2
}

/// bash `env_fail` — rc **3**.
///
/// ENVIRONMENT, not code — the world was never booted, so a 3 says NOTHING about the mod. Same
/// split `world-boot.sh:355` established; keep the two readable side by side.
pub(super) fn env_fail(msg: &str, hint: &str) -> u8 {
    eprintln!();
    eprintln!("ENVIRONMENT: {msg}");
    if !hint.is_empty() {
        eprintln!("  {hint}");
    }
    3
}

/// bash's `for arg in "$@"` loop.
///
/// PRESERVED ODDITY — POSITION MATTERS, AND IT IS NOT A BUG TO FIX. The loop acts on each token in
/// turn, so `--help` *exits at the point it is reached*: `--bogus --help` is rc 2 (the unknown token
/// is seen first) while `--help --bogus` is rc 0. Baselines `a05` / `a06` pin both directions.
/// A conventional parser that collected everything before deciding would quietly change both.
///
/// PRESERVED ODDITY — `--admin=` and `--mission-id=` accept an EMPTY value, and the empty string is
/// carried forward rather than ignored. `--admin=` therefore fails admin validation with
/// `--admin='' is neither an identityId nor a SteamID` (baseline `a15`) rather than being skipped,
/// and `--mission-id=` fails the required check (baseline `a08`).
pub(super) fn parse(args: &[String], home: &str) -> Parsed {
    let mut o = Opts::defaults(home);
    // bash `${arg#*=}` — strip through the FIRST `=`. A value may itself contain `=`.
    fn val(a: &str) -> String {
        a.split_once('=').map(|x| x.1).unwrap_or("").to_string()
    }
    for arg in args {
        match arg.as_str() {
            a if a.starts_with("--mission-id=") => o.mission_id = val(a),
            a if a.starts_with("--mission-file=") => o.mission_file = val(a),
            a if a.starts_with("--event-id=") => o.event_id = val(a),
            a if a.starts_with("--backend-url=") => o.backend_url = val(a),
            a if a.starts_with("--token=") => o.token = val(a),
            a if a.starts_with("--admin=") => o.admins.push(val(a)),
            a if a.starts_with("--name=") => o.server_name = val(a),
            a if a.starts_with("--scenario=") => o.scenario = val(a),
            a if a.starts_with("--port=") => o.game_port = val(a),
            a if a.starts_with("--a2s-port=") => o.a2s_port = val(a),
            a if a.starts_with("--max-players=") => o.max_players = val(a),
            a if a.starts_with("--run-dir=") => o.run_dir = val(a),
            a if a.starts_with("--timeout=") => o.run_timeout = val(a),
            "--dry-run" => o.dry_run = true,
            "--selftest" => o.selftest = true,
            "-h" | "--help" => return Parsed::Help,
            other => return Parsed::Unknown(other.to_string()),
        }
    }
    Parsed::Opts(Box::new(o))
}

/// Is this string one of the two shapes the ENGINE's own schema accepts for `game.admins[]`?
///
/// Both patterns are copied verbatim out of the engine's rejection of a bad value (1.7.0.54):
///
/// ```text
///   BACKEND (E): RegEx Pattern: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
///   BACKEND (E): RegEx Pattern: "^[0-9]{17}$"
/// ```
///
/// A bad entry is a HARD FATAL at boot ("There are errors in server config!" -> "Unable to
/// initialize the game"), and the engine reports it ~90 s in, after a full script compile. Failing
/// here costs a millisecond and names the value.
///
/// NOTE ON THE ANCHORS. bash ran `printf '%s' "$a" | grep -qE '^…$'`, and `grep` anchors per LINE.
/// [`verification_core::Pattern`] sets `multi_line(true)` unconditionally for exactly that reason, so an
/// admin value containing an embedded newline is accepted here iff bash accepted it — which it did.
/// That widening is pinned by `admin_newline_widening_is_preserved` rather than silently "fixed":
/// the engine gets the value as one JSON string either way, and changing it would be a behaviour
/// change wearing a bugfix's clothes.
pub(super) fn admin_id_is_valid(a: &str) -> bool {
    use verification_core::Pattern;
    // Compiled per call. Measured cost is irrelevant next to a 90-second engine boot, and the
    // alternative (a lazy static) buys nothing a human reading this file would thank us for.
    let uuid = Pattern::regex("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
        .expect("static pattern");
    let steam = Pattern::regex("^[0-9]{17}$").expect("static pattern");
    uuid.is_match(a) || steam.is_match(a)
}

/// bash `grep -oE '<pat>'` over `text`: every non-overlapping match on every line, in order.
///
/// Two of the preflight reads (`ADDON_GUID`, `SCENARIO`) are `grep -o | grep -o` pipelines, and
/// their exact behaviour on a malformed input is load-bearing: the emptiness of the result is what
/// produces the rc 1 arms `c01` / `c02`.
pub(super) fn grep_o(pattern: &str, text: &str) -> Vec<String> {
    let re = match regex::RegexBuilder::new(pattern).multi_line(true).build() {
        Ok(r) => r,
        // A pattern that will not compile is a programming error here — every call site passes a
        // literal. Returning empty keeps this infallible for callers without inventing a match.
        Err(_) => return Vec::new(),
    };
    re.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

/// bash `VAR="$(cmd)"` — command substitution strips trailing newlines but keeps interior ones.
pub(super) fn substitution(lines: Vec<String>) -> String {
    lines.join("\n").trim_end_matches('\n').to_string()
}

/// The gproj GUID read, as a pipeline: `grep -oE '^\s*GUID\s+"…"' | grep -oE '[0-9A-Fa-f]{8,}'`.
pub(super) fn read_addon_guid(gproj_text: &str) -> String {
    substitution(
        grep_o(r#"^[[:space:]]*GUID[[:space:]]+"[0-9A-Fa-f]+""#, gproj_text)
            .iter()
            .flat_map(|line| grep_o("[0-9A-Fa-f]{8,}", line))
            .collect(),
    )
}

/// The scenarioId read: `grep -oE '"scenarioId"[^,]*' | grep -oE '\{[^}]+\}[^"]*'`.
pub(super) fn read_scenario(dev_text: &str) -> String {
    substitution(
        grep_o(r#""scenarioId"[^,]*"#, dev_text)
            .iter()
            .flat_map(|m| grep_o("\\{[^}]+\\}[^\"]*", m))
            .collect(),
    )
}

/// CLI entry: `cargo xtask mod playtest -- <args>`.
pub fn run(args: &[String]) -> Result<u8> {
    let home = std::env::var("HOME").unwrap_or_default();
    let opts = match parse(args, &home) {
        Parsed::Help => {
            print!("{HELP}");
            return Ok(0);
        }
        Parsed::Unknown(a) => return Ok(usage_fail(&format!("unknown argument: {a}"))),
        Parsed::Opts(o) => *o,
    };

    // ROOT. bash derived it from `$0`'s directory (`dirname $0/../..`); we walk up from the cwd for
    // `.ai/tickets/registry.json`, which is how every other ported gate finds it. Both land on the
    // checkout the operator is standing in.
    let root = find_repo_root()?;
    let host = Host::detect();
    Ok(main_with(&root, &home, &host, opts))
}

/// The body, with the environment injected so tests can drive it.
pub(super) fn main_with(root: &Path, home: &str, host: &Host, o: Opts) -> u8 {
    let mod_src = root.join("apps/mod/tbd-framework");
    let server_dir = format!("{home}/.local/share/Steam/steamapps/common/Arma Reforger Server");
    let server_bin = PathBuf::from(&server_dir).join("ArmaReforgerServer");
    let dev_config = root.join(crate::core::repository_layout::DEV_SERVER_PROFILE);

    // ═══ KILL DISCIPLINE, THE LIVENESS PROBE, AND THE RUN LOCK ══════════════════════════════
    // Reached this early on purpose, ahead of every other check, for two reasons: `--selftest`
    // has to be able to get here without a mission id, and `assert_no_live_server` has to run
    // BEFORE staging rewrites server.json underneath a server that is still running.
    let paths = lifecycle::RunPaths::new(&o.run_dir);

    // ── --selftest: prove the kill path can FAIL, and cannot lie ─────────────────────────────
    // A gate nobody has watched fail is not a gate, and `kill_run` only ever reports a stop it
    // did not achieve when the bridge flakes — which no green boot exercises. Boots no game
    // server.
    //
    // ORDERING ODDITY, PRESERVED: this runs before the `--mission-id` check and before port
    // validation, so `--selftest --port=1 --a2s-port=1` still selftests (baseline `f03`).
    if o.selftest {
        return lifecycle::selftest(host);
    }

    if o.mission_id.is_empty() {
        return usage_fail("--mission-id is required — it is what the mod loads");
    }

    // `a2sPort` and `bindPort` are separate UDP sockets. Equal ports make the engine log
    // `NETWORK (E): Unable to start replication` and exit **status 0**, so nothing downstream
    // notices (docs/mod/STAGING-SERVER.md). Refuse here instead of at boot.
    if o.game_port == o.a2s_port {
        return usage_fail(&format!(
            "--port and --a2s-port must differ (got {} for both); standard layout is 2001 game / 17777 A2S",
            o.game_port
        ));
    }

    for a in &o.admins {
        if !admin_id_is_valid(a) {
            eprintln!("ERROR: --admin='{a}' is neither an identityId nor a SteamID.");
            eprintln!("  identityId: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx  (lowercase hex)");
            eprintln!("  SteamID:    17 digits");
            eprintln!(
                "  The engine rejects anything else and refuses to start; this is its schema, not ours."
            );
            return 2;
        }
    }

    if !host.require_host() {
        return env_fail(
            "no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine",
            "See tools_v2/xtask/src/core/host_execution.rs: the container has an older glibc, so the game binary cannot run in here at all.",
        );
    }
    if !is_executable(&server_bin) {
        return env_fail(
            &format!("server binary not found at {}", server_bin.display()),
            "Install it from Steam (appid 1890870):  steam steam://install/1890870",
        );
    }
    if !dev_config.is_file() {
        return env_fail(
            &format!("dev config not found at {}", dev_config.display()),
            "The checkout does not look like this repo — verify the working tree before blaming the mod.",
        );
    }
    if !mod_src.is_dir() {
        return env_fail(
            &format!("mod source not found at {}", mod_src.display()),
            "",
        );
    }

    // GUID read out of addon.gproj, never hardcoded — `world-boot.sh:376` does the same, for the same
    // reason: a literal here would drift from the gproj silently and the mod would stop resolving.
    let gproj = mod_src.join("addon.gproj");
    let addon_guid = read_addon_guid(&std::fs::read_to_string(&gproj).unwrap_or_default());
    if addon_guid.is_empty() {
        eprintln!("ERROR: could not read GUID from {}", gproj.display());
        return 1;
    }

    let scenario = if o.scenario.is_empty() {
        read_scenario(&std::fs::read_to_string(&dev_config).unwrap_or_default())
    } else {
        o.scenario.clone()
    };
    if scenario.is_empty() {
        eprintln!(
            "ERROR: could not read scenarioId from {}",
            dev_config.display()
        );
        return 1;
    }

    // The LAN address the room advertises. Resolved on the HOST: inside a container the default
    // route can belong to the podman bridge, and a room registered on 10.88.x.x is unreachable from
    // the friend's machine while looking perfectly healthy in the log.
    let lan_ip = host.capture_trimmed(&[
        "sh",
        "-c",
        "ip route get 1.1.1.1 2>/dev/null | awk '{print $7; exit}'",
    ]);
    if lan_ip.is_empty() {
        return env_fail(
            "could not determine this machine's LAN IP",
            &format!(
                "Pass it by hand: edit publicAddress in {}/server.json after a --dry-run.",
                o.run_dir
            ),
        );
    }

    let server_name = if o.server_name.is_empty() {
        format!("TBD Playtest ({})", o.mission_id)
    } else {
        o.server_name.clone()
    };

    println!("==> staging {}", o.run_dir);

    // BEFORE a single byte of this run dir is rewritten. Both guards can exit; both say what to do.
    //
    // The lock is an RAII guard rather than bash's `trap release_lock EXIT`. Every path below
    // RETURNS a code instead of calling `process::exit`, precisely so this Drop runs — an `exit()`
    // anywhere under here would leak the lock dir and turn every later invocation into the "taking
    // over a stale lock" arm.
    let _lock = match lifecycle::claim_lock(&paths, &o) {
        Ok(g) => g,
        Err(code) => return code,
    };
    if let Err(code) = lifecycle::assert_no_live_server(&paths, host, &o) {
        return code;
    }
    if let Err(e) = std::fs::create_dir_all(format!("{}/addons", o.run_dir)) {
        // bash's bare `mkdir -p` would have printed the shell's own diagnostic and carried on to
        // fail at the next step. Naming it here is strictly more informative and cannot mask a
        // success — there is no success to mask.
        return env_fail(&format!("cannot create {}/addons: {e}", o.run_dir), "");
    }

    // ── profile ──────────────────────────────────────────────────────────────────────────────
    // `$profile:` resolves to <-profile-arg>/profile/, NOT <-profile-arg>/ (`world-boot.sh:383`).
    // `cargo xtask setup server-profile` already knows that; do not seed one level up.
    if let Err(code) = render::setup_server_profile(root, &o.run_dir) {
        return code;
    }
    let backend_cfg = format!("{}/profile/profile/TBD_BackendConfig.json", o.run_dir);
    if !Path::new(&backend_cfg).is_file() {
        return env_fail(
            &format!("setup server-profile did not produce {backend_cfg}"),
            "",
        );
    }

    // Token: explicit flag wins; otherwise `setup server-profile` already substituted the one from
    // `apps/website/api_v2/.env` and we leave its work alone. (former python3 site 1 of 3)
    if let Err(e) = render::patch_backend_config(&backend_cfg, &o) {
        // bash printed python's traceback on stderr and then this exact line. The cause keeps its
        // own line so the `ERROR:` line stays byte-identical to the baseline.
        eprintln!("{e}");
        eprintln!("ERROR: could not patch {backend_cfg}");
        return 1;
    }

    if !o.mission_file.is_empty() {
        if !Path::new(&o.mission_file).is_file() {
            return usage_fail(&format!("--mission-file={} does not exist", o.mission_file));
        }
        // `TBD_MissionLoader.LoadFromProfileFile` reads `$profile:missions/<missionId>.json`, so the
        // file on disk must be named for the ID, not for the golden it came from (`cargo xtask setup
        // server-profile` carries the same note). Copy rather than re-serialise: the mod must parse
        // these exact bytes.
        let missions = format!("{}/profile/profile/missions", o.run_dir);
        let _ = std::fs::create_dir_all(&missions);
        let dst = format!("{missions}/{}.json", o.mission_id);
        match std::fs::copy(&o.mission_file, &dst) {
            Ok(n) => println!(
                "    staged {n} bytes as the on-disk fallback for {}",
                o.mission_id
            ),
            Err(e) => {
                eprintln!("cp: cannot copy '{}' to '{dst}': {e}", o.mission_file);
                return 1;
            }
        }
    }

    // ── addon staging dir ────────────────────────────────────────────────────────────────────
    // A symlink to the live checkout, exactly like `deploy-staging.sh:1100`. This is the copy that
    // must win at load time; `assert_local_addon_won` below proves it did.
    let link = format!("{}/addons/tbd-framework", o.run_dir);
    // bash `ln -sfn`: replace the LINK, never follow it into the target directory.
    if std::fs::symlink_metadata(&link).is_ok() {
        let _ = std::fs::remove_file(&link);
    }
    if let Err(e) = std::os::unix::fs::symlink(&mod_src, &link) {
        eprintln!(
            "ln: failed to create symbolic link '{link}' -> '{}': {e}",
            mod_src.display()
        );
        return 1;
    }

    // ── server config ────────────────────────────────────────────────────────────────────────
    // (former python3 sites 2 and 3 of 3)
    let server_json = format!("{}/server.json", o.run_dir);
    let admins_json = render::admins_json(&o.admins);
    if let Err(e) = render::render_server_json(&render::ServerJson {
        src: &dev_config,
        dst: Path::new(&server_json),
        ip: &lan_ip,
        port: &o.game_port,
        a2s: &o.a2s_port,
        max_players: &o.max_players,
        guid: &addon_guid,
        scenario: &scenario,
        name: &server_name,
        admins: &o.admins,
    }) {
        // Same split as the backend-config patch above: cause first, then bash's exact line.
        eprintln!("{e}");
        eprintln!("ERROR: could not render {server_json}");
        return 1;
    }
    println!("    rendered {server_json} (mods=[{addon_guid}] admins={admins_json})");

    if o.admins.is_empty() {
        println!();
        println!(
            "  NOTE: no --admin given, so game.admins[] is empty. TBD_AdminService.IsAdmin() resolves"
        );
        println!(
            "        from vanilla's SCR_PlayerListedAdminManagerComponent, which is populated ONLY"
        );
        println!(
            "        from game.admins[]. With none, every '#tbd' command answers 'TBD: admin only.'"
        );
        println!(
            "        and T-181.16's admin-respawn item cannot be reached. The 'passwordAdmin' field"
        );
        println!("        is a DIFFERENT mechanism and does not feed that list.");
        println!();
    }

    let cmd_display = format!(
        "./ArmaReforgerServer -addonsDir {}/addons -config {server_json} -profile {}/profile -maxFPS 60 -logStats 30000 -nothrow",
        o.run_dir, o.run_dir
    );

    if o.dry_run {
        println!();
        println!("[dry-run] cd \"{server_dir}\" && {cmd_display}");
        println!("[dry-run] would advertise: {lan_ip}:{}", o.game_port);
        return 0;
    }

    boot::boot_and_wait(&boot::BootCtx {
        host,
        paths: &paths,
        opts: &o,
        server_dir: &server_dir,
        cmd_display: &cmd_display,
        addon_guid: &addon_guid,
        lan_ip: &lan_ip,
        scenario: &scenario,
    })
}

/// bash `[ -x PATH ]`: a regular file with any execute bit.
pub(super) fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match path.metadata() {
        Ok(m) if m.is_file() => m.permissions().mode() & 0o111 != 0,
        _ => false,
    }
}
