//! Every mission document the mod loads is held to `MISSION_FILE_MAX_BYTES` (8 MiB) before it is
//! parsed. The document arrives from the platform's artifact route (REST) or from the mod's
//! artifact cache, and three places enforce the ceiling:
//!
//! * `TBD_MissionLoader.LoadDocument` calls `IsMissionBodyWithinCap(data)` before
//!   `ParseMissionJson(data)`, and the helper compares `data.Length() <= MISSION_FILE_MAX_BYTES`;
//! * `TBD_MissionArtifactVerification` refuses a received document longer than the ceiling;
//! * `TBD_MissionArtifactCache` refuses a cached file outside `1..MISSION_FILE_MAX_BYTES` before
//!   reading it.
//!
//! Two false-green shapes this gate is built to refuse:
//!   (1) a `//` comment containing `MISSION_FILE_MAX_BYTES` counting as the size check before
//!       `ParseMissionJson`;
//!   (2) requiring only the `IsMissionBodyWithinCap` signature, which a `return true;` satisfies.
//!
//! So the gate strips comments before the order assert, requires a live `IsMissionBodyWithinCap(`
//! call before `ParseMissionJson(`, and pins the helper body to
//! `Length() <= MISSION_FILE_MAX_BYTES`.
//!
//! ── RED ARMS CANNOT FAIL OPEN ────────────────────────────────────────────────────────────────
//!
//! The RED arms are in-memory string transforms of the loader; live files are never written. A
//! probe that cannot answer is an explicit fail, never a pass: [`gate::probe_str`] cannot return a
//! tool error today, and the arm that would report one is kept so a future fallible probe stays
//! fail-closed.
//!
//! Output and the binary 0/1 status are a contract: the wave gate prints the last 15 lines of a
//! failed step.

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use verification_core::{Pattern, Verdict, gate};

const LOADERS_REL: &str = "apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders";
const LOADER_FILE: &str = "TBD_MissionLoader.c";
const VERIFICATION_FILE: &str = "TBD_MissionArtifactVerification.c";
const CACHE_FILE: &str = "TBD_MissionArtifactCache.c";

/// The three sources the ceiling lives in.
struct Sources {
    loader: String,
    verification: String,
    cache: String,
}

/// Entry point. `0` when live pins hold and every RED proof bit; `1` on any failure; `2` when a
/// RED arm cannot be set up, which is a gate that did not run rather than one that found nothing.
pub fn verify_mission_rest_size_limits(repo_root: &Path) -> Result<u8> {
    let Some(live) = read_sources(repo_root) else {
        return Ok(1);
    };

    let mut failed = false;
    if !assert_rest_size_gate(&live, "live")? {
        failed = true;
    }

    type RedArm = fn(&str) -> std::result::Result<String, String>;
    let arms: [(&str, RedArm, &str); 3] = [
        (
            "RED-comment-only",
            red1_strip_cap_call,
            "comment-only MISSION_FILE_MAX_BYTES (no live IsMissionBodyWithinCap)",
        ),
        (
            "RED-post-parse",
            red2_relocate_after_parse,
            "IsMissionBodyWithinCap after ParseMissionJson",
        ),
        (
            "RED-return-true",
            red3_stub_return_true,
            "IsMissionBodyWithinCap return true",
        ),
    ];
    for (label, arm, description) in arms {
        let perturbed = match arm(&live.loader) {
            Ok(s) => s,
            Err(msg) => {
                eprintln!("{msg}");
                return Ok(2);
            }
        };
        let red = Sources {
            loader: perturbed,
            verification: live.verification.clone(),
            cache: live.cache.clone(),
        };
        if assert_rest_size_gate(&red, label)? {
            println!("FAIL: {label} still passed — the pins are not discriminating");
            failed = true;
        } else {
            println!("RED proof: {description} → FAIL (expected)");
        }
    }

    // The live files must still PASS after all RED perturbations (in memory only; files
    // untouched). Re-read from disk so a concurrent edit is still caught.
    match read_sources(repo_root) {
        None => failed = true,
        Some(restored) if assert_rest_size_gate(&restored, "live-restore")? => println!(
            "GREEN proof: live IsMissionBodyWithinCap before ParseMissionJson + Length() compare → PASS"
        ),
        Some(_) => {
            println!(
                "FAIL: live files no longer pass after RED proofs (files should be untouched)"
            );
            failed = true;
        }
    }

    if failed {
        println!("mission-rest-size-limits: FAIL");
        return Ok(1);
    }
    println!("mission-rest-size-limits: PASS");
    Ok(0)
}

fn read_sources(repo_root: &Path) -> Option<Sources> {
    let directory = repo_root.join(LOADERS_REL);
    let read = |name: &str| {
        let path = directory.join(name);
        match std::fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(e) => {
                println!("FAIL: cannot read {}: {e}", path.display());
                None
            }
        }
    };
    Some(Sources {
        loader: read(LOADER_FILE)?,
        verification: read(VERIFICATION_FILE)?,
        cache: read(CACHE_FILE)?,
    })
}

/// Runs every pin over one set of sources. Returns `true` when all of them held.
fn assert_rest_size_gate(sources: &Sources, label: &str) -> Result<bool> {
    let src = &sources.loader;
    let raw_body = extract_load_document(src);
    if raw_body.is_empty() {
        println!("FAIL ({label}): could not extract LoadDocument");
        return Ok(false);
    }
    let stripped = strip_c_comments(&raw_body);

    // The size check must be a live IsMissionBodyWithinCap( call before ParseMissionJson(
    let check_line = first_line_matching(&stripped, "IsMissionBodyWithinCap(");
    let parse_line = first_line_matching(&stripped, "ParseMissionJson(");
    match (check_line, parse_line) {
        (None, _) | (_, None) => {
            println!(
                "FAIL ({label}): missing IsMissionBodyWithinCap( and/or ParseMissionJson( in LoadDocument (non-comment)"
            );
            return Ok(false);
        }
        (Some(c), Some(p)) if c >= p => {
            println!(
                "FAIL ({label}): IsMissionBodyWithinCap( (line {c}) is not before ParseMissionJson( (line {p})"
            );
            return Ok(false);
        }
        (Some(_), Some(_)) => {}
    }

    let helper_sig = Pattern::regex(r"protected static bool IsMissionBodyWithinCap\(string data\)")
        .expect("helper sig regex");
    let v = gate::require_str(
        &format!("({label}) missing IsMissionBodyWithinCap(string) helper"),
        &helper_sig,
        src,
    );
    if !held_or_print(v) {
        return Ok(false);
    }

    let helper = strip_c_comments(&extract_helper(src));
    let length_cmp = Pattern::regex(r"Length\(\)[[:space:]]*<=[[:space:]]*MISSION_FILE_MAX_BYTES")
        .expect("length cmp regex");
    let v = gate::require_str(
        &format!(
            "({label}) IsMissionBodyWithinCap body does not compare Length() <= MISSION_FILE_MAX_BYTES"
        ),
        &length_cmp,
        &helper,
    );
    if !held_or_print(v) {
        return Ok(false);
    }

    // Reject an always-true stub that also happens to mention the compare in a dead branch.
    // COMPOUND: has `return true;` AND lacks the real return — neither half is a failure alone.
    let pat_true = Pattern::regex(r"return[[:space:]]+true[[:space:]]*;").expect("return true");
    let pat_real = Pattern::regex(
        r"return[[:space:]]+data\.Length\(\)[[:space:]]*<=[[:space:]]*MISSION_FILE_MAX_BYTES[[:space:]]*;",
    )
    .expect("return real");
    let (Ok(has_true), Ok(has_real)) = (
        gate::probe_str(&pat_true, &helper),
        gate::probe_str(&pat_real, &helper),
    ) else {
        // Closed fail-open: a probe that cannot run must not green the stub check.
        println!("FAIL ({label}): always-true stub probe did not execute.");
        println!("      Refusing to report OK on a check that never compared anything.");
        return Ok(false);
    };
    if has_true && !has_real {
        println!(
            "FAIL ({label}): IsMissionBodyWithinCap returns true without the Length() <= MISSION_FILE_MAX_BYTES return"
        );
        return Ok(false);
    }

    let pins = [
        (
            "MISSION_FILE_MAX_BYTES is not the pinned 8*1024*1024",
            Pattern::regex(r"MISSION_FILE_MAX_BYTES = 8 \* 1024 \* 1024").expect("cap const"),
            src.as_str(),
        ),
        (
            "the artifact verification no longer refuses a document over MISSION_FILE_MAX_BYTES",
            Pattern::literal("document.Length() > TBD_MissionLoader.MISSION_FILE_MAX_BYTES"),
            sources.verification.as_str(),
        ),
        (
            "the artifact cache no longer refuses a file over MISSION_FILE_MAX_BYTES before reading it",
            Pattern::literal("byteCount > TBD_MissionLoader.MISSION_FILE_MAX_BYTES"),
            sources.cache.as_str(),
        ),
    ];
    for (message, pattern, text) in pins {
        let live = strip_c_comments(text);
        if !held_or_print(gate::require_str(
            &format!("({label}) {message}"),
            &pattern,
            &live,
        )) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn held_or_print(v: Verdict) -> bool {
    match &v {
        Verdict::Held => true,
        Verdict::Failed(_) | Verdict::DidNotRun(_, _) => {
            println!("{v}");
            false
        }
    }
}

fn first_line_matching(text: &str, needle: &str) -> Option<usize> {
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .map(|(i, _)| i + 1)
}

/// The closing brace of a method, which sits at one tab of indentation.
fn method_end() -> Regex {
    Regex::new(r"^\t\}[[:space:]]*$").expect("method end")
}

/// `LoadDocument`, from its signature through its closing brace.
fn extract_load_document(src: &str) -> String {
    let start =
        Regex::new(r"(?m)^[[:space:]]*static bool LoadDocument\(string data, string source\)")
            .expect("load start");
    extract_until(src, &start, &method_end())
}

/// `IsMissionBodyWithinCap`, from its signature through its closing brace.
fn extract_helper(src: &str) -> String {
    let start =
        Regex::new(r"(?m)^[[:space:]]*protected static bool IsMissionBodyWithinCap\(string data\)")
            .expect("helper start");
    extract_until(src, &start, &method_end())
}

fn extract_until(src: &str, start: &Regex, end: &Regex) -> String {
    let Some(m) = start.find(src) else {
        return String::new();
    };
    let mut out = String::new();
    for line in src[m.start()..].lines() {
        out.push_str(line);
        out.push('\n');
        if end.is_match(line) {
            break;
        }
    }
    out
}

/// Drop `//` and `/* */` comments, keeping the newlines inside block comments.
fn strip_c_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < n {
        if chars[i] == '/' && i + 1 < n && chars[i + 1] == '/' {
            i += 2;
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if chars[i] == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i = (i + 2).min(n);
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// The cap check in `LoadDocument`, exactly once.
fn cap_check_block() -> Regex {
    Regex::new(r"(?s)\n\t\tif \(!IsMissionBodyWithinCap\(data\)\)\n\t\t\{.*?\n\t\t\}\n")
        .expect("cap block")
}

fn exactly_one(
    pattern: &Regex,
    src: &str,
    what: &str,
) -> std::result::Result<(usize, usize), String> {
    let found: Vec<_> = pattern.find_iter(src).collect();
    match found.as_slice() {
        [only] => Ok((only.start(), only.end())),
        _ => Err(format!("{what} setup failed (n={})", found.len())),
    }
}

fn red1_strip_cap_call(src: &str) -> std::result::Result<String, String> {
    let (start, end) = exactly_one(
        &cap_check_block(),
        src,
        "RED1: strip IsMissionBodyWithinCap call",
    )?;
    Ok(format!("{}\n{}", &src[..start], &src[end..]))
}

fn red2_relocate_after_parse(src: &str) -> std::result::Result<String, String> {
    let (start, end) = exactly_one(&cap_check_block(), src, "RED2: find the size-gate block")?;
    let gate_block = src[start..end].to_string();
    let without = format!("{}\n{}", &src[..start], &src[end..]);
    let parse = Regex::new(r"(?s)\t\tif \(!ParseMissionJson\(data\)\)\n\t\t\treturn false;\n")
        .expect("parse");
    let (parse_start, parse_end) = exactly_one(
        &parse,
        &without,
        "RED2: relocate the gate after ParseMissionJson",
    )?;
    Ok(format!(
        "{}{}{}{}",
        &without[..parse_start],
        &without[parse_start..parse_end],
        gate_block,
        &without[parse_end..]
    ))
}

fn red3_stub_return_true(src: &str) -> std::result::Result<String, String> {
    let pattern = Regex::new(
        r"(protected static bool IsMissionBodyWithinCap\(string data\)\n\t\{\n\t\t)return data\.Length\(\) <= MISSION_FILE_MAX_BYTES;",
    )
    .expect("red3");
    let (start, end) = exactly_one(&pattern, src, "RED3: stub IsMissionBodyWithinCap")?;
    let prefix = pattern
        .captures(&src[start..end])
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();
    Ok(format!(
        "{}{prefix}return true;{}",
        &src[..start],
        &src[end..]
    ))
}

#[cfg(test)]
#[path = "tests/mission_rest_size_limits/tests.rs"]
mod tests;
