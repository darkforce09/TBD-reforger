//! Reading the engine's log — every `grep` the bash ran against `server.out`.
//!
//! Split out of `boot.rs` at 1005 lines, past SIZE-3's >1000 hard fail. The seam is a real one and
//! not an arbitrary cut: everything here is a PURE FUNCTION OF TEXT, so all of it is unit-testable
//! without an engine, a host bridge, or a process — which is exactly why it holds most of this
//! port's test coverage. `boot.rs` keeps the half that needs a live server: spawning it, polling it,
//! and stopping it.
//!
//! ── THE ONE RULE THESE FUNCTIONS SHARE ───────────────────────────────────────────────────────
//!
//! An unreadable or absent log reads as the EMPTY STRING, never as a match. Every question asked
//! here is "does the engine's output contain X", and over a log that does not exist the honest
//! answer is no. That is deliberately NOT the same fail-open as bash's `grep … 2>/dev/null`: the
//! callers in `boot.rs` never turn "no match" into a pass — a missing marker is what produces a
//! FAILED verdict, so an empty log fails closed at the call site rather than here.

use std::path::Path;

use tbd_gate::Pattern;

use super::Opts;
use super::lifecycle::RunPaths;

/// Read `server.out`. An unreadable log is the empty string, exactly as `grep … 2>/dev/null` was:
/// every caller's question is "does this text contain X", and the answer over a missing file is no.
pub(super) fn log(paths: &RunPaths) -> String {
    std::fs::read_to_string(&paths.srv_out).unwrap_or_default()
}

/// `grep -q <literal>`.
pub(super) fn has(text: &str, needle: &str) -> bool {
    text.contains(needle)
}

/// `grep -qE <ere>`. Compiled per call; the boot loop runs at 2 Hz and this is not measurable next
/// to a 90-second engine boot.
pub(super) fn has_re(text: &str, ere: &str) -> bool {
    Pattern::regex(ere)
        .map(|p| p.is_match(text))
        .unwrap_or(false)
}

/// The furthest milestone the log shows, newest first.
///
/// The TBD markers are matched as ESCAPED EREs (`\[TBD\]\[Stage\].*LOBBY`), not as `grep -F` on a
/// whole sentence (T-606). `-F` was originally chosen because `[` is a character class to both ugrep
/// and GNU grep — a real hazard, but the fix for it is to escape the brackets, not to pin the entire
/// English line. `-F '[TBD][Stage] LOADING -> LOBBY'` breaks if anyone changes the arrow, renames a
/// stage enum, or appends a clause; measured, changing `->` to `=>` alone drops this from 1 match to
/// 0, i.e. the launcher would report a server that IS in LOBBY as never having got there. `.*LOBBY`
/// also (correctly) still matches once the round advances past LOBBY.
///
/// The engine-owned markers below stay as plain strings: they are Bohemia's, not ours, and are far
/// more stable than anything we Print.
pub(super) fn boot_phase(paths: &RunPaths) -> String {
    if !Path::new(&paths.srv_out).is_file() {
        return "engine has not written anything yet".into();
    }
    let t = log(paths);
    if has_re(&t, r"\[TBD\]\[Stage\].*LOBBY") {
        "WORLD UP, mission already in LOBBY — the only thing missing is the backend room".into()
    } else if has(&t, "Starting RPL server, listening on address") {
        "WORLD UP, replication listening — waiting on the backend room registration".into()
    } else if has(&t, "Game::LoadEntities took") {
        "world entities loaded — waiting on replication, then the backend room".into()
    } else if has(&t, "GameProject load") {
        "loading the world".into()
    } else if has(&t, "Compiling Game scripts") {
        "compiling scripts".into()
    } else {
        "engine starting".into()
    }
}

/// True once the world is demonstrably up. Distinguishes "it never got there" from "it got there and
/// the registration hung", which need completely different diagnoses.
pub(super) fn world_is_up(paths: &RunPaths) -> bool {
    if !Path::new(&paths.srv_out).is_file() {
        return false;
    }
    let t = log(paths);
    has(&t, "Starting RPL server, listening on address")
        || has(&t, "Game::LoadEntities took")
        || has_re(&t, r"\[TBD\]\[Stage\].*LOBBY")
}

/// `grep -n <ere>` — 1-based line number, colon, the line, exactly as GNU grep prints it.
pub(super) fn grep_n(text: &str, ere: &str) -> Vec<String> {
    let p = match Pattern::regex(ere) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    text.lines()
        .enumerate()
        .filter(|(_, l)| p.is_match(l))
        .map(|(i, l)| format!("{}:{l}", i + 1))
        .collect()
}

/// `grep -c <ere>` — the number of LINES that match, not the number of matches.
pub(super) fn grep_c(text: &str, ere: &str) -> usize {
    match Pattern::regex(ere) {
        Ok(p) => text.lines().filter(|l| p.is_match(l)).count(),
        Err(_) => 0,
    }
}

/// Engine `(E)` lines, with the vanilla floor demoted rather than hidden.
///
/// Measured on a PASSING boot of this same config, 2026-07-31: 79 `(E)`/`(F)` lines in total, of
/// which 75 are the `DEFAULT`/`MATERIAL`/`RESOURCES` floor (70 of those are `DEFAULT (E): Trying to
/// register a signal …` on vanilla vehicles). Four lines survive the demotion. The old code dumped
/// the first 20 of the unsorted 79, which meant twenty lines of material and vehicle noise that are
/// present when everything works — and the actual cause appeared in none of them.
pub(super) fn dump_engine_errors(paths: &RunPaths) {
    let t = log(paths);
    // The inverse filter is applied to the `grep -n` OUTPUT, so its own anchor allows for the
    // `<lineno>:` prefix — `^[0-9]+:[[:space:]]*(DEFAULT|…)`.
    let noise_prefixed =
        Pattern::regex("^[0-9]+:[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\\(E\\):")
            .expect("static pattern");
    let signal: Vec<String> = grep_n(&t, r"\((E|F)\):")
        .into_iter()
        .filter(|l| !noise_prefixed.is_match(l))
        .take(20) // `| head -20`
        .collect();
    let noise_n = grep_c(
        &t,
        "^[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\\(E\\):",
    );

    if !signal.is_empty() {
        eprintln!("--- engine errors worth reading ---");
        for l in &signal {
            eprintln!("{l}");
        }
    } else {
        eprintln!("--- no engine error line names a cause ---");
    }
    if noise_n > 0 {
        eprintln!(
            "    ({noise_n} further vanilla DEFAULT/MATERIAL/RESOURCES (E) lines suppressed — a"
        );
        eprintln!(
            "     passing boot of this config carries ~79 of them; they are the floor, not a clue)"
        );
    }
    // This one is (E) and looks alarming and is not the problem. Say so where it will be read.
    // Matched on tag + the first structural words (`\[TBD\]\[Mission\].*backend refused`), not the
    // whole sentence: the tail carries `— http=%1 body=%2` and is prose that will be reworded
    // (TBD_MissionLoader.c:775). Missing this note only costs a diagnostic hint, but a hint that
    // silently stops appearing is how operators end up chasing a benign (E) line for an hour.
    if has_re(&t, r"\[TBD\]\[Mission\].*backend refused") {
        eprintln!(
            "    NOTE: '[TBD][Mission] backend refused the mission fetch — http=400' is BENIGN."
        );
        eprintln!(
            "          It means the mod could not fetch that id from the API and used the mission"
        );
        eprintln!(
            "          staged on disk instead — the --mission-file path working as designed. It is"
        );
        eprintln!(
            "          present on PASSING boots too. It has nothing to do with room registration."
        );
    }
}

/// `grep -A6 'Loaded addons:' | grep "guid: '<GUID>'" | tail -1`.
///
/// `grep -A` merges overlapping context windows and never repeats a line; the `--` group separators it
/// emits between non-adjacent windows are not reproduced because the very next stage filters for a
/// `guid: '…'` substring, which `--` can never match.
pub(super) fn loaded_addon_line(text: &str, guid: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut wanted = vec![false; lines.len()];
    for (i, l) in lines.iter().enumerate() {
        if l.contains("Loaded addons:") {
            for w in wanted.iter_mut().take((i + 7).min(lines.len())).skip(i) {
                *w = true;
            }
        }
    }
    let needle = format!("guid: '{guid}'");
    lines
        .iter()
        .enumerate()
        .filter(|(i, l)| wanted[*i] && l.contains(&needle))
        .map(|(_, l)| l.to_string())
        .next_back() // `| tail -1`
}

/// ── THE HARD GATE: did the LOCAL addon win, or the stale Workshop pak? ────────────────────────
///
/// See the module header of `playtest_server`. `-config` alone silently runs Workshop 1.0.1; if that
/// copy wins here, every line below this point would be a true statement about the WRONG code, which
/// is precisely the failure this program was written to make impossible.
pub(super) fn assert_local_addon_won(paths: &RunPaths, o: &Opts, addon_guid: &str) -> bool {
    let t = log(paths);
    let loaded = match loaded_addon_line(&t, addon_guid) {
        Some(l) => l,
        None => {
            eprintln!("FAILED: the engine never reported loading addon {addon_guid} at all.");
            for l in grep_n(&t, "Loaded addons:|Available addons:|gproj:")
                .into_iter()
                .take(20)
            {
                eprintln!("{l}");
            }
            return false;
        }
    };
    let wanted = format!("{}/addons/tbd-framework/addon.gproj", o.run_dir);
    if loaded.contains(&wanted) {
        return true;
    }
    eprintln!();
    eprintln!("FAILED: the STALE Workshop copy won, not your checkout.");
    eprintln!("  loaded: {loaded}");
    eprintln!("  wanted: {wanted}");
    eprintln!();
    eprintln!("  tbd-framework is published unlisted under the same id as the local gproj GUID,");
    eprintln!(
        "  so the engine can satisfy game.mods[] from the Workshop without ever touching your"
    );
    eprintln!(
        "  source. That build is version 1.0.1 and months old. Delete the cached copy and retry:"
    );
    eprintln!(
        "      rm -rf '{}/profile/addons/TBDFramework_{addon_guid}'",
        o.run_dir
    );
    false
}

/// `ls -1d "$LOGROOT"/logs_* 2>/dev/null | tail -1` + `/console.log`.
///
/// PRESERVED ODDITY: when the glob matches nothing the bash produced the literal string
/// `/console.log` (an empty substitution followed by the suffix), whose `[ -f ]` test then fails and
/// sends the tail to `server.out` instead. Reproduced exactly — including the fact that a host with a
/// readable `/console.log` would tail THAT. It is a one-in-a-million path and changing it would be a
/// behaviour change with no baseline.
pub(super) fn console_log_path(run_dir: &str) -> String {
    let logroot = format!("{run_dir}/profile/logs");
    let mut hits: Vec<String> = match std::fs::read_dir(&logroot) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("logs_"))
            .map(|n| format!("{logroot}/{n}"))
            .collect(),
        Err(_) => Vec::new(),
    };
    // `ls -1d` sorts; `tail -1` takes the last.
    hits.sort();
    let dir = hits.last().cloned().unwrap_or_default();
    format!("{dir}/console.log")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_log(tag: &str, body: &str) -> RunPaths {
        let d = std::env::temp_dir().join(format!("tbd-rps-boot-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = RunPaths::new(d.to_str().unwrap());
        std::fs::write(&p.srv_out, body).unwrap();
        p
    }
    #[test]
    fn boot_phase_reports_the_furthest_milestone_not_the_first() {
        // Order matters: a log that has BOTH must report LOBBY, the newest.
        let p = with_log(
            "phase",
            "Compiling Game scripts\nGameProject load\nGame::LoadEntities took 3s\n\
             NETWORK  : Starting RPL server, listening on address 0.0.0.0:2001\n\
             [TBD][Stage] LOADING -> LOBBY\n",
        );
        assert!(boot_phase(&p).starts_with("WORLD UP, mission already in LOBBY"));
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn the_lobby_marker_survives_a_reworded_arrow() {
        // T-606: `grep -F '[TBD][Stage] LOADING -> LOBBY'` dropped to ZERO matches when the arrow
        // changed, i.e. a server that WAS in LOBBY was reported as never having got there.
        let p = with_log("arrow", "[TBD][Stage] LOADING => LOBBY\n");
        assert!(boot_phase(&p).starts_with("WORLD UP, mission already in LOBBY"));
        assert!(world_is_up(&p));
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn an_absent_log_is_a_phase_not_a_crash() {
        let p = RunPaths::new("/nonexistent/run/dir");
        assert_eq!(boot_phase(&p), "engine has not written anything yet");
        assert!(!world_is_up(&p));
    }

    #[test]
    fn phases_below_the_world_do_not_claim_the_world_is_up() {
        let p = with_log("early", "Compiling Game scripts\n");
        assert_eq!(boot_phase(&p), "compiling scripts");
        assert!(!world_is_up(&p));
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn the_local_addon_must_win_or_the_gate_fails() {
        let mut o = Opts::defaults("/h");
        o.run_dir = "/home/u/tbd-playtest".into();
        let local = "ENGINE   : Loaded addons:\n\
             ENGINE   :   gproj: '/home/u/tbd-playtest/addons/tbd-framework/addon.gproj' guid: 'B2C3D4E5F6A78901'\n";
        let p = with_log("won", local);
        assert!(assert_local_addon_won(&p, &o, "B2C3D4E5F6A78901"));
        let _ = std::fs::remove_dir_all(&p.run_dir);

        // THE TRAP: the packed Workshop copy answering for the same GUID.
        let stale = "ENGINE   : Loaded addons:\n\
             ENGINE   :   gproj: '/home/u/tbd-playtest/profile/addons/TBDFramework_B2C3D4E5F6A78901/addon.gproj' guid: 'B2C3D4E5F6A78901'\n";
        let p = with_log("stale", stale);
        assert!(
            !assert_local_addon_won(&p, &o, "B2C3D4E5F6A78901"),
            "the stale Workshop pak winning must be a HARD failure, never a warning"
        );
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn a_guid_outside_the_loaded_addons_window_does_not_count() {
        // `grep -A6` bounds the search to six lines after the header. A `guid:` line further down
        // (the "Available addons:" list, say) must not satisfy the gate.
        let far = format!(
            "ENGINE   : Loaded addons:\n{}ENGINE   :   gproj: '/r/addons/tbd-framework/addon.gproj' guid: 'G1'\n",
            "filler\n".repeat(7)
        );
        assert!(loaded_addon_line(&far, "G1").is_none());
        let near = "ENGINE   : Loaded addons:\nfiller\nENGINE   :   gproj: '/r/x/addon.gproj' guid: 'G1'\n";
        assert!(loaded_addon_line(near, "G1").is_some());
    }

    #[test]
    fn loaded_addon_line_takes_the_last_match_like_tail_1() {
        let two = "Loaded addons:\n gproj: 'first' guid: 'G'\n gproj: 'second' guid: 'G'\n";
        assert!(loaded_addon_line(two, "G").unwrap().contains("second"));
    }

    #[test]
    fn the_hard_gate_holds_on_a_real_engine_boot() {
        // NOT a synthetic log. Captured verbatim from `ArmaReforgerServer` 1.7.0.54 booted BY THIS
        // PORT on 2026-08-12 (`--run-dir=/tmp/t853/w-play/live --port=2031`), lines 164-169 of its
        // `server.out`. Two details only a real boot supplies, and both are load-bearing:
        //
        //   * the engine's own leading-space indent grows by one per nesting level, so the `gproj:`
        //     lines start with two spaces and would not match a `^ENGINE` anchor;
        //   * the wanted entry is the THIRD gproj under the header, three lines down — inside
        //     `grep -A6`'s window, but only just, and it is preceded by two vanilla addons that
        //     `tail -1` must not select.
        //
        // This is the T-604 finding in evidence: `-addonsDir` + `-config` together, and the LOCAL
        // checkout wins over the unlisted Workshop 1.0.1 published under the same GUID.
        let real = "\
ENGINE       : GameProject load
 ENGINE       : Loaded addons:
  ENGINE       : gproj: './addons/core/core.gproj' guid: '5614BBCCBB55ED1C'
  ENGINE       : gproj: './addons/data/ArmaReforger.gproj' guid: '58D0FB3206B6F859'
  ENGINE       : gproj: '/tmp/t853/w-play/live/addons/tbd-framework/addon.gproj' guid: 'B2C3D4E5F6A78901'
GUI          : Using default language (en_us)
";
        let line = loaded_addon_line(real, "B2C3D4E5F6A78901")
            .expect("the real boot's addon line must be found");
        assert!(line.contains("/tmp/t853/w-play/live/addons/tbd-framework/addon.gproj"));
        // And the gate itself agrees, against the run dir that boot actually used.
        let mut o = Opts::defaults("/h");
        o.run_dir = "/tmp/t853/w-play/live".into();
        let p = with_log("realboot", real);
        assert!(assert_local_addon_won(&p, &o, "B2C3D4E5F6A78901"));
        // Flip the run dir and the same log must now FAIL — proving the gate reads the path and is
        // not merely finding the GUID somewhere.
        o.run_dir = "/somewhere/else".into();
        assert!(
            !assert_local_addon_won(&p, &o, "B2C3D4E5F6A78901"),
            "the gate must compare the PATH, not just the GUID"
        );
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn grep_n_numbers_from_one_and_grep_c_counts_lines() {
        let t = "a\nDEFAULT   (E): noise\nb\nBACKEND (E): real\n";
        assert_eq!(
            grep_n(t, r"\((E|F)\):"),
            vec![
                "2:DEFAULT   (E): noise".to_string(),
                "4:BACKEND (E): real".to_string()
            ]
        );
        assert_eq!(
            grep_c(
                t,
                "^[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\\(E\\):"
            ),
            1
        );
    }

    #[test]
    fn the_vanilla_error_floor_is_demoted_not_the_real_cause() {
        // The measured shape: 75 floor lines, four that matter. The floor must not crowd out the
        // signal inside the `head -20` window.
        let mut body = String::new();
        for i in 0..75 {
            body.push_str(&format!("DEFAULT   (E): Trying to register a signal {i}\n"));
        }
        body.push_str("BACKEND  (E): the actual cause\n");
        let p = with_log("floor", &body);
        let t = log(&p);
        let noise =
            Pattern::regex("^[0-9]+:[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\\(E\\):")
                .unwrap();
        let signal: Vec<String> = grep_n(&t, r"\((E|F)\):")
            .into_iter()
            .filter(|l| !noise.is_match(l))
            .take(20)
            .collect();
        assert_eq!(
            signal.len(),
            1,
            "the floor leaked into the signal: {signal:?}"
        );
        assert!(signal[0].ends_with("BACKEND  (E): the actual cause"));
        assert_eq!(
            grep_c(
                &t,
                "^[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\\(E\\):"
            ),
            75
        );
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn console_path_falls_back_to_the_literal_slash_console_log() {
        // PRESERVED ODDITY — see `console_log_path`.
        assert_eq!(console_log_path("/nonexistent/run"), "/console.log");
    }

    #[test]
    fn console_path_takes_the_newest_logs_dir() {
        let d = std::env::temp_dir().join(format!("tbd-rps-console-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("profile/logs/logs_2026-01-01_00-00-00")).unwrap();
        std::fs::create_dir_all(d.join("profile/logs/logs_2026-08-12_09-00-00")).unwrap();
        std::fs::create_dir_all(d.join("profile/logs/not-a-log")).unwrap();
        let got = console_log_path(d.to_str().unwrap());
        assert!(
            got.ends_with("logs_2026-08-12_09-00-00/console.log"),
            "{got}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_registered_marker_and_the_fatals_are_the_engines_own_strings() {
        let t = "BACKEND  : Server registered with address: 192.168.0.117:2001\n";
        assert!(has(t, "Server registered with address:"));
        for fatal in [
            "There are errors in server config!",
            "Unable to initialize the game",
            "NETWORK (E): Unable to start replication",
        ] {
            assert!(
                has_re(
                    fatal,
                    "There are errors in server config!|Unable to initialize the game|Unable to start replication"
                ),
                "{fatal}"
            );
        }
    }
}
