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

use verification_core::Pattern;

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
/// whole sentence. `-F` was originally chosen because `[` is a character class to both ugrep
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
    // Two mission lines explain a boot that ran no platform mission. Matched on tag and the
    // first structural words, not whole sentences: the tails are prose that gets reworded.
    if has_re(&t, r"\[TBD\]\[Mission\].*NO MISSION YET") {
        eprintln!(
            "    NOTE: '[TBD][Mission] NO MISSION YET' means the mod read no deployment and has no"
        );
        eprintln!(
            "          cached artifact: the profile's machineCredential is unset or refused, or the"
        );
        eprintln!(
            "          server has no deployment. It has nothing to do with room registration."
        );
    }
    if has_re(
        &t,
        r"\[TBD\]\[Mission\].*RUNNING THE LAST VERIFIED ARTIFACT",
    ) {
        eprintln!(
            "    NOTE: '[TBD][Mission] RUNNING THE LAST VERIFIED ARTIFACT' is the offline path: the"
        );
        eprintln!(
            "          platform gave no deployment answer and the cached artifact booted instead —"
        );
        eprintln!("          --artifact-file working as designed.");
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
#[path = "tests/logread/tests.rs"]
mod tests;
