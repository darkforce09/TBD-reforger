//! T-456 / T-460 — OnBackendFetchSuccess must refuse oversized REST bodies before
//! ParseMissionJson, using the same `MISSION_FILE_MAX_BYTES` ceiling as LoadFromProfileFile
//! (T-853 / T-881 port of `scripts/mod/verify-t456-mission-rest-size-gate.sh`).
//!
//! T-460 (Wave 22 adversarial): prior Class-R was false-green —
//!   (1) a `//` comment containing `MISSION_FILE_MAX_BYTES` counted as the size check
//!       before ParseMissionJson;
//!   (2) only the IsMissionBodyWithinCap signature was required — `return true;` greens.
//! This gate strips comments before the order assert, requires a live
//! `IsMissionBodyWithinCap(` call before `ParseMissionJson(`, and pins the helper body to
//! `Length() <= MISSION_FILE_MAX_BYTES`.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`, entirely — four call sites.** One comment stripper and three RED setup
//!    transforms. The script was on `scripts/python-inventory.txt` solely for those; the
//!    inventory line goes with them (same commit as T-468).
//! 2. **`2>/dev/null`-shaped fail-opens on compound probes.** Bash `gate_probe_str` statuses
//!    above 1 are mapped to an explicit DidNotRun-style FAIL. In-process [`gate::probe_str`]
//!    cannot return a tool error; the message arm is retained so a future fallible probe keeps
//!    the fail-closed contract.
//! 3. **`mktemp` scribble.** RED arms are in-memory string transforms; live files are never
//!    written.
//!
//! Output + binary 0/1 status are a contract (`wave.sh` tails failures; T-853 diffs stdout).

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use tbd_gate::{Pattern, Verdict, gate};

const FILE_REL: &str = "apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c";

/// Entry point. `0` when live pins hold and every RED proof bit; `1` on any failure; `2` when a
/// RED arm cannot be set up (`sys.exit(2)` under bash `set -e`).
pub fn verify_t456(repo_root: &Path) -> Result<u8> {
    let file = repo_root.join(FILE_REL);
    if !file.is_file() {
        println!("FAIL: missing {}", file.display());
        return Ok(1);
    }

    let live = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            println!("FAIL: cannot read {}: {e}", file.display());
            return Ok(1);
        }
    };

    let mut failed = false;

    if !assert_rest_size_gate(&live, "live")? {
        failed = true;
    }

    // ── RED 1: comment-only "size check" (comment mentions the constant; live call removed) ──
    let red1 = match red1_strip_cap_call(&live) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("{msg}");
            return Ok(2);
        }
    };
    if assert_rest_size_gate(&red1, "RED-comment-only")? {
        println!(
            "FAIL: RED comment-only still passed — order pin ignores comments? or call not required"
        );
        failed = true;
    } else {
        println!(
            "RED proof: comment-only MISSION_FILE_MAX_BYTES (no live IsMissionBodyWithinCap) → FAIL (expected)"
        );
    }

    // ── RED 2: post-parse check (call moved after ParseMissionJson) ──
    let red2 = match red2_relocate_after_parse(&live) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("{msg}");
            return Ok(2);
        }
    };
    if assert_rest_size_gate(&red2, "RED-post-parse")? {
        println!("FAIL: RED post-parse still passed — order pin is not discriminating");
        failed = true;
    } else {
        println!("RED proof: IsMissionBodyWithinCap after ParseMissionJson → FAIL (expected)");
    }

    // ── RED 3: helper stubbed to `return true;` ──
    let red3 = match red3_stub_return_true(&live) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("{msg}");
            return Ok(2);
        }
    };
    if assert_rest_size_gate(&red3, "RED-return-true")? {
        println!("FAIL: RED return-true helper still passed — body pin is not discriminating");
        failed = true;
    } else {
        println!("RED proof: IsMissionBodyWithinCap return true → FAIL (expected)");
    }

    // Live file must still PASS after all RED perturbations (in-memory only; FILE untouched).
    // Re-read from disk so a concurrent edit is still caught.
    let restored = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            println!("FAIL: cannot re-read {}: {e}", file.display());
            failed = true;
            String::new()
        }
    };
    if restored.is_empty() {
        // already marked failed
    } else if assert_rest_size_gate(&restored, "live-restore")? {
        println!(
            "GREEN proof: live IsMissionBodyWithinCap before ParseMissionJson + Length() compare → PASS"
        );
    } else {
        println!("FAIL: live file no longer passes after RED proofs (FILE should be untouched)");
        failed = true;
    }

    if failed {
        println!("verify-t456-mission-rest-size-gate: FAIL");
        return Ok(1);
    }
    println!("verify-t456-mission-rest-size-gate: PASS");
    Ok(0)
}

/// Port of bash `assert_rest_size_gate`. Returns `true` when every pin held.
fn assert_rest_size_gate(src: &str, label: &str) -> Result<bool> {
    let raw_body = extract_success(src);
    if raw_body.is_empty() {
        println!("FAIL ({label}): could not extract OnBackendFetchSuccess");
        return Ok(false);
    }

    let stripped = strip_c_comments(&raw_body);

    // Cap constant must appear in the success handler (not only in the profile path).
    let v = gate::require_str(
        &format!(
            "({label}) OnBackendFetchSuccess has no non-comment MISSION_FILE_MAX_BYTES reference"
        ),
        &Pattern::literal("MISSION_FILE_MAX_BYTES"),
        &stripped,
    );
    if !held_or_print(v) {
        return Ok(false);
    }

    // T-460: size check must be a live IsMissionBodyWithinCap( call before ParseMissionJson(
    let check_line = first_line_matching(&stripped, "IsMissionBodyWithinCap(");
    let parse_line = first_line_matching(&stripped, "ParseMissionJson(");
    match (check_line, parse_line) {
        (None, _) | (_, None) => {
            println!(
                "FAIL ({label}): missing IsMissionBodyWithinCap( and/or ParseMissionJson( in OnBackendFetchSuccess (non-comment)"
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
    let has_true = match gate::probe_str(&pat_true, &helper) {
        Ok(b) => b,
        Err(_) => {
            // Closed fail-open: a probe that cannot run must not green the stub check.
            println!(
                "FAIL ({label}): always-true stub probe did not execute (grep exited err / err)."
            );
            println!("      Refusing to report OK on a check that never compared anything.");
            return Ok(false);
        }
    };
    let has_real = match gate::probe_str(&pat_real, &helper) {
        Ok(b) => b,
        Err(_) => {
            println!(
                "FAIL ({label}): always-true stub probe did not execute (grep exited err / err)."
            );
            println!("      Refusing to report OK on a check that never compared anything.");
            return Ok(false);
        }
    };
    if has_true && !has_real {
        println!(
            "FAIL ({label}): IsMissionBodyWithinCap returns true without the Length() <= MISSION_FILE_MAX_BYTES return"
        );
        return Ok(false);
    }

    let cap_const =
        Pattern::regex(r"MISSION_FILE_MAX_BYTES = 8 \* 1024 \* 1024").expect("cap const");
    let v = gate::require_str(
        &format!("({label}) MISSION_FILE_MAX_BYTES is not the pinned 8*1024*1024"),
        &cap_const,
        src,
    );
    if !held_or_print(v) {
        return Ok(false);
    }

    let v = gate::require_str(
        &format!(
            "({label}) LoadFromProfileFile no longer compares fileSize to MISSION_FILE_MAX_BYTES"
        ),
        &Pattern::literal("fileSize > MISSION_FILE_MAX_BYTES"),
        src,
    );
    if !held_or_print(v) {
        return Ok(false);
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

/// Extract OnBackendFetchSuccess body (from its signature through the next method).
fn extract_success(src: &str) -> String {
    let start = Regex::new(
        r"(?m)^[[:space:]]*protected static void OnBackendFetchSuccess\(RestCallback cb\)",
    )
    .expect("success start");
    let end = Regex::new(
        r"(?m)^[[:space:]]*protected static void OnBackendFetchError\(RestCallback cb\)",
    )
    .expect("success end");
    extract_until(src, &start, &end)
}

/// Extract IsMissionBodyWithinCap method body (signature through ParseMissionJson).
fn extract_helper(src: &str) -> String {
    let start =
        Regex::new(r"(?m)^[[:space:]]*protected static bool IsMissionBodyWithinCap\(string data\)")
            .expect("helper start");
    let end = Regex::new(r"(?m)^[[:space:]]*protected static bool ParseMissionJson\(string data\)")
        .expect("helper end");
    extract_until(src, &start, &end)
}

fn extract_until(src: &str, start: &Regex, end: &Regex) -> String {
    let Some(m) = start.find(src) else {
        return String::new();
    };
    let rest = &src[m.start()..];
    let mut out = String::new();
    for line in rest.lines() {
        out.push_str(line);
        out.push('\n');
        if end.is_match(line) {
            break;
        }
    }
    out
}

/// Python `strip_c_comments`: drop `//` and `/* */`, keep newlines inside block comments.
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

fn red1_strip_cap_call(src: &str) -> std::result::Result<String, String> {
    let pat = Regex::new(r"(?s)\n\t\tif \(!IsMissionBodyWithinCap\(data\)\)\n\t\t\{.*?\n\t\t\}\n")
        .expect("red1");
    let Some(m) = pat.find(src) else {
        return Err(
            "RED1 setup failed: could not strip IsMissionBodyWithinCap call (n=0)".to_string(),
        );
    };
    // Count must be exactly 1 — refuse to prove on a tree with zero/many matches.
    if pat.find_iter(src).count() != 1 {
        let n = pat.find_iter(src).count();
        return Err(format!(
            "RED1 setup failed: could not strip IsMissionBodyWithinCap call (n={n})"
        ));
    }
    let mut out = String::with_capacity(src.len());
    out.push_str(&src[..m.start()]);
    out.push('\n');
    out.push_str(&src[m.end()..]);
    Ok(out)
}

fn red2_relocate_after_parse(src: &str) -> std::result::Result<String, String> {
    let block_re = Regex::new(
        r"(?s)\n\t\t// T-456 — REST path must honour the same MISSION_FILE_MAX_BYTES ceiling as profile load\..*?\n\t\t\}\n",
    )
    .expect("red2 block");
    let Some(m) = block_re.find(src) else {
        return Err("RED2 setup failed: could not find T-456 REST size-gate block".to_string());
    };
    let gate_block = m.as_str().to_string();
    let src_wo = format!("{}\n{}", &src[..m.start()], &src[m.end()..]);
    let parse_re =
        Regex::new(r"(?s)(if \(!ParseMissionJson\(data\)\)\n\t\t\{.*?\n\t\t\}\n)").expect("parse");
    let Some(caps) = parse_re.captures(&src_wo) else {
        return Err(
            "RED2 setup failed: could not relocate gate after ParseMissionJson (n=0)".to_string(),
        );
    };
    let n = parse_re.find_iter(&src_wo).count();
    if n != 1 {
        return Err(format!(
            "RED2 setup failed: could not relocate gate after ParseMissionJson (n={n})"
        ));
    }
    let first = caps.get(0).unwrap();
    let body = caps.get(1).unwrap().as_str();
    let mut out = String::new();
    out.push_str(&src_wo[..first.start()]);
    out.push_str(body);
    out.push_str(&gate_block);
    out.push_str(&src_wo[first.end()..]);
    Ok(out)
}

fn red3_stub_return_true(src: &str) -> std::result::Result<String, String> {
    let pat = Regex::new(
        r"(protected static bool IsMissionBodyWithinCap\(string data\)\n\t\{\n\t\t)return data\.Length\(\) <= MISSION_FILE_MAX_BYTES;",
    )
    .expect("red3");
    let Some(caps) = pat.captures(src) else {
        return Err("RED3 setup failed: could not stub IsMissionBodyWithinCap (n=0)".to_string());
    };
    if pat.find_iter(src).count() != 1 {
        let n = pat.find_iter(src).count();
        return Err(format!(
            "RED3 setup failed: could not stub IsMissionBodyWithinCap (n={n})"
        ));
    }
    let m = caps.get(0).unwrap();
    let prefix = caps.get(1).unwrap().as_str();
    let mut out = String::new();
    out.push_str(&src[..m.start()]);
    out.push_str(prefix);
    out.push_str("return true;");
    out.push_str(&src[m.end()..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_c_comments_drops_line_and_block() {
        assert_eq!(strip_c_comments("a // x\nb"), "a \nb");
        assert_eq!(strip_c_comments("a /* x\ny */ b"), "a \n b");
    }

    #[test]
    fn live_shaped_helper_passes_order_and_length() {
        let src = r#"
	protected static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024;
	protected static void OnBackendFetchSuccess(RestCallback cb)
	{
		if (!IsMissionBodyWithinCap(data))
		{
			Print(string.Format("too big %1 > %2", data.Length(), MISSION_FILE_MAX_BYTES), LogLevel.ERROR);
			return;
		}
		if (!ParseMissionJson(data))
		{
			return;
		}
	}
	protected static void OnBackendFetchError(RestCallback cb)
	{
	}
	protected static bool IsMissionBodyWithinCap(string data)
	{
		return data.Length() <= MISSION_FILE_MAX_BYTES;
	}
	protected static bool ParseMissionJson(string data)
	{
		return true;
	}
	protected static bool LoadFromProfileFile(string missionId)
	{
		if (fileSize > MISSION_FILE_MAX_BYTES)
		{
			return false;
		}
		return true;
	}
"#;
        assert!(assert_rest_size_gate(src, "fixture").unwrap());
    }

    #[test]
    fn return_true_stub_fails() {
        let src = r#"
	protected static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024;
	protected static void OnBackendFetchSuccess(RestCallback cb)
	{
		if (!IsMissionBodyWithinCap(data))
		{
			return;
		}
		if (!ParseMissionJson(data))
		{
			return;
		}
	}
	protected static void OnBackendFetchError(RestCallback cb)
	{
	}
	protected static bool IsMissionBodyWithinCap(string data)
	{
		return true;
	}
	protected static bool ParseMissionJson(string data)
	{
		return true;
	}
	protected static bool LoadFromProfileFile(string missionId)
	{
		if (fileSize > MISSION_FILE_MAX_BYTES)
		{
			return false;
		}
		return true;
	}
"#;
        assert!(!assert_rest_size_gate(src, "stub").unwrap());
    }
}
