//! T-468 / T-471 / T-472 / T-476 / T-486 / T-489 — CI schema job must stay on
//! `make ci-local-schema`, and Makefile recipe bodies for the Class-R verifies must
//! stay real (T-853 / T-881 port of `scripts/mod/verify-t468-ci-schema-parity.sh`).
//!
//! Without this tripwire, someone can revert the CI schema job to bare
//! `cargo run -p xtask -- schema validate` + citations and reopen the map-object-enums
//! hole while CI stays green. Recipe-body pins close hollow `@true` / `echo PASS` /
//! `#fake` smuggles on `ci-local-schema`, `verify-t456`, and this gate's own
//! `verify-t468` target.
//!
//! ── T-853 MUTUAL PIN ─────────────────────────────────────────────────────────────────────────
//!
//! Pre-port, `bash_pins` required `^\t@?bash <exact-script-path>` for both verify-t456 and
//! verify-t468. Porting either alone fails the other on a CORRECT tree. Both bash pins are
//! dropped here and re-pinned at the cargo spelling in ONE atomic change with the Makefile /
//! wave.sh / ci.yml call sites. [`VERIFY_T456`] / [`VERIFY_T468`] are the single source; test
//! fixtures derive from them (gate_t440 precedent).
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`, entirely.** The script was a heredoc owning YAML comment strip + recipe
//!    pins. Ported in-process; the `scripts/python-inventory.txt` line dies with it.
//! 2. **`pin_out="$(…)" || pin_rc=$?` swallow.** A Python crash still surfaced via non-zero
//!    status, but stderr was discarded when the capture only kept stdout. Failures print
//!    directly here.
//!
//! T-489 circularity (preserved): CI / ci-local / wave invoke this gate *directly* (cargo),
//! not via `make verify-t468`. A hollow make target must not green those callers. Human
//! `make verify-t468` remains a convenience that still runs the gate.

use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use regex::Regex;

/// Cargo spelling pinned into Makefile `verify-t456:` (tab + optional `@` + this exact line).
/// Const + call sites are one atomic change — see module docs.
const VERIFY_T456: &str = "cargo run -q -p xtask -- verify t456";
/// Cargo spelling pinned into Makefile `verify-t468:` (self-pin).
const VERIFY_T468: &str = "cargo run -q -p xtask -- verify t468";

const CI_REL: &str = ".github/workflows/ci.yml";
const MAKE_REL: &str = "Makefile";
const GOOD_RUN: &str = "make ci-local-schema";

/// Entry point. Bash collapsed any Python non-zero to exit 1 after printing FAIL — same here.
pub fn verify_t468(repo_root: &Path) -> Result<u8> {
    let ci_path = repo_root.join(CI_REL);
    if !ci_path.is_file() {
        println!("FAIL: missing {}", ci_path.display());
        return Ok(1);
    }

    let src = match std::fs::read_to_string(&ci_path) {
        Ok(s) => s,
        Err(e) => {
            println!("FAIL: cannot read {}: {e}", ci_path.display());
            return Ok(1);
        }
    };

    let makefile_path = repo_root.join(MAKE_REL);
    let makefile = if makefile_path.is_file() {
        match std::fs::read_to_string(&makefile_path) {
            Ok(s) => Some(s),
            Err(e) => {
                println!("FAIL: cannot read {}: {e}", makefile_path.display());
                return Ok(1);
            }
        }
    } else {
        None
    };

    let fail = run_pins(&src, makefile.as_deref(), makefile_path.as_path());
    if fail != 0 {
        println!("verify-t468-ci-schema-parity: FAIL");
        return Ok(1);
    }
    println!("verify-t468-ci-schema-parity: PASS");
    Ok(0)
}

/// Port of the Python pin block. Returns `0` when clean, `1` when any pin failed.
fn run_pins(ci_src: &str, makefile: Option<&str>, makefile_path: &Path) -> i32 {
    let stripped = strip_yaml_hash_comments(ci_src);
    let lines: Vec<&str> = stripped.lines().collect();

    let schema_job_re = Regex::new(r"^  schema:\s*$").expect("schema job");
    let schema_start = lines.iter().position(|line| schema_job_re.is_match(line));
    let Some(schema_start) = schema_start else {
        println!("FAIL: no top-level `schema:` job in .github/workflows/ci.yml");
        return 1;
    };

    let job_key = Regex::new(r"^  [A-Za-z0-9_-]+:\s*$").expect("job key");
    let mut schema_lines: Vec<&str> = Vec::new();
    for line in &lines[schema_start + 1..] {
        if job_key.is_match(line) {
            break;
        }
        schema_lines.push(line);
    }
    if schema_lines.is_empty() {
        println!("FAIL: schema job has empty body");
        return 1;
    }

    let run_re = Regex::new(r"^\s+-\s+run:\s*(.+?)\s*$|^\s+run:\s*(.+?)\s*$").expect("run step");
    let mut runs: Vec<String> = Vec::new();
    for line in &schema_lines {
        let Some(caps) = run_re.captures(line) else {
            continue;
        };
        let mut cmd = caps
            .get(1)
            .or_else(|| caps.get(2))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        if (cmd.starts_with('\'') && cmd.ends_with('\''))
            || (cmd.starts_with('"') && cmd.ends_with('"'))
        {
            cmd = cmd[1..cmd.len() - 1].to_string();
        }
        runs.push(cmd);
    }

    let mut fail = 0;

    if runs.is_empty() {
        println!("FAIL: schema job has no `run:` steps after comment strip");
        fail = 1;
    }

    let make_ws = Regex::new(r"^make\s+ci-local-schema$").expect("make ws");
    let has_good = runs.iter().any(|r| r == GOOD_RUN || make_ws.is_match(r));

    if !has_good {
        println!("FAIL: schema job must run `make ci-local-schema` (full gate set)");
        println!("      found run steps:");
        for r in &runs {
            println!("        - {r}");
        }
        println!("      Pre-T-434 hole: validate + citations alone misses map-object-enums.");
        fail = 1;
    }

    let narrow_re = Regex::new(r"schema\s+validate\b").expect("narrow");
    let narrow: Vec<&String> = runs
        .iter()
        .filter(|r| {
            narrow_re.is_match(r)
                && !r.contains("ci-local-schema")
                && !r.contains("schema-validate")
        })
        .collect();
    if !narrow.is_empty() && !has_good {
        println!("FAIL: schema job uses narrow `schema validate` without ci-local-schema:");
        for r in &narrow {
            println!("      {r}");
        }
        fail = 1;
    }

    let Some(mf) = makefile else {
        println!("FAIL: missing Makefile at {}", makefile_path.display());
        return 1;
    };

    if !Regex::new(r"(?m)^ci-local-schema:")
        .expect("ci-local-schema target")
        .is_match(mf)
    {
        println!("FAIL: Makefile missing `ci-local-schema:` target (CI pin would be vacuous)");
        fail = 1;
    } else {
        let live = extract_recipe_live(mf, "ci-local-schema");
        match live {
            None => {
                // extract returns None only when target missing — already checked above.
                println!(
                    "FAIL: Makefile missing `ci-local-schema:` target (CI pin would be vacuous)"
                );
                fail = 1;
            }
            Some(live) if live.is_empty() => {
                println!("FAIL: Makefile `ci-local-schema:` has no tab-indented recipe body");
                println!(
                    "      hollow target names still green CI — require schema-validate + verify-citations"
                );
                fail = 1;
            }
            Some(live) => {
                let need = ["schema-validate", "verify-citations"];
                let goal_line_re =
                    Regex::new(r"^\t@?(?:\$\(MAKE\)|make)\s+(.+)$").expect("goal line");
                let mut invoked: HashSet<String> = HashSet::new();
                for ln in &live {
                    let Some(caps) = goal_line_re.captures(ln) else {
                        continue;
                    };
                    for tok in caps[1].split_whitespace() {
                        if tok.starts_with('-') {
                            continue;
                        }
                        invoked.insert(tok.to_string());
                    }
                }
                let missing: Vec<&str> = need
                    .iter()
                    .copied()
                    .filter(|p| !invoked.contains(*p))
                    .collect();
                if !missing.is_empty() {
                    println!(
                        "FAIL: Makefile `ci-local-schema:` recipe must invoke: {}",
                        missing.join(", ")
                    );
                    println!("      found recipe lines (comments stripped):");
                    for ln in &live {
                        println!("        '{}'", py_repr_ascii(ln));
                    }
                    println!(
                        "      T-472: require tab + optional @ + $(MAKE)|make + exact \
                         target schema-validate / verify-citations (not echo/true, \
                         not -fake suffix, not # comment smuggle)."
                    );
                    fail = 1;
                }
            }
        }
    }

    // T-853: bash_pins dropped; cargo_pins re-pin both recipes at the VERIFY_* consts.
    let cargo_pins = [("verify-t456", VERIFY_T456), ("verify-t468", VERIFY_T468)];
    for (target, cmd) in cargo_pins {
        let live = extract_recipe_live(mf, target);
        match live {
            None => {
                println!("FAIL: Makefile missing `{target}:` target (T-467/T-476 pin vacuous)");
                fail = 1;
            }
            Some(live) if live.is_empty() => {
                println!("FAIL: Makefile `{target}:` has no tab-indented recipe body");
                println!("      hollow `@true` / echo greens make — require `{cmd}`");
                fail = 1;
            }
            Some(live) => {
                // Exact recipe goal: ^\t @? <VERIFY_*> (whitespace|EOL)
                let cargo_goal_re = Regex::new(&format!(r"^\t@?{}\s*(?:\s|$)", regex::escape(cmd)))
                    .expect("cargo goal");
                // The escape of spaces makes `\ ` in the pattern; also accept the literal
                // spaced form via a looser startswith check after optional @.
                let ok = live
                    .iter()
                    .any(|ln| cargo_goal_re.is_match(ln) || recipe_invokes_cargo(ln, cmd));
                if !ok {
                    println!("FAIL: Makefile `{target}:` recipe must invoke: {cmd}");
                    println!("      found recipe lines (comments stripped):");
                    for ln in &live {
                        // Match Python `!r` for ASCII recipe lines (single quotes, `\t` escapes).
                        println!("        '{}'", py_repr_ascii(ln));
                    }
                    println!(
                        "      T-476/T-486: require tab + optional @ + bash + exact script path \
                         (not @true/echo, not # comment smuggle, not -fake suffix)."
                    );
                    fail = 1;
                }
            }
        }
    }

    fail
}

/// `^\t@?<cmd>(?:\s|$)` after comment strip — token-exact like the old bash path pin.
fn recipe_invokes_cargo(line: &str, cmd: &str) -> bool {
    let rest = if let Some(r) = line.strip_prefix("\t@") {
        r
    } else if let Some(r) = line.strip_prefix('\t') {
        r
    } else {
        return false;
    };
    rest == cmd || rest.starts_with(&format!("{cmd} "))
}

/// Tab recipe lines under `target:` with `#` comments stripped; `None` if missing.
fn extract_recipe_live(mf: &str, target: &str) -> Option<Vec<String>> {
    let target_re = Regex::new(&format!(r"^{}:", regex::escape(target))).expect("target");
    let next_target = Regex::new(r"^[^\s#]").expect("next target");
    let blank_recipe = Regex::new(r"^\t\s*$").expect("blank");
    let mut recipe_lines: Vec<String> = Vec::new();
    let mut in_target = false;
    for line in mf.lines() {
        if target_re.is_match(line) {
            in_target = true;
            continue;
        }
        if !in_target {
            continue;
        }
        if next_target.is_match(line) && !line.starts_with('\t') {
            break;
        }
        if line.starts_with('\t') {
            recipe_lines.push(line.to_string());
        }
    }
    if !in_target {
        return None;
    }
    let mut live = Vec::new();
    for ln in recipe_lines {
        let cleaned = strip_recipe_hash_comment(&ln);
        if blank_recipe.is_match(&cleaned) || cleaned == "\t" {
            continue;
        }
        live.push(cleaned);
    }
    Some(live)
}

/// Python `!r` for the ASCII recipe lines this gate dumps (tab → `\t`, single quotes).
fn py_repr_ascii(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\'', "\\'")
}

fn strip_recipe_hash_comment(line: &str) -> String {
    if !line.starts_with('\t') {
        return line.to_string();
    }
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut out = String::from("\t");
    let mut i = 1;
    let mut in_squote = false;
    let mut in_dquote = false;
    while i < n {
        let c = chars[i];
        if in_squote {
            out.push(c);
            if c == '\'' && !(i + 1 < n && chars[i + 1] == '\'') {
                in_squote = false;
            } else if c == '\'' && i + 1 < n && chars[i + 1] == '\'' {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_dquote {
            out.push(c);
            if c == '\\' && i + 1 < n {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_dquote = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_squote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_dquote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '#' {
            break;
        }
        out.push(c);
        i += 1;
    }
    out.trim_end().to_string()
}

/// Strip `#` comments outside quotes. Preserves newlines for line structure.
fn strip_yaml_hash_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_squote = false;
    let mut in_dquote = false;
    while i < n {
        let c = chars[i];
        if in_squote {
            out.push(c);
            if c == '\'' && !(i + 1 < n && chars[i + 1] == '\'') {
                in_squote = false;
            } else if c == '\'' && i + 1 < n && chars[i + 1] == '\'' {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_dquote {
            out.push(c);
            if c == '\\' && i + 1 < n {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_dquote = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_squote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_dquote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal CI schema job that satisfies the run-step pin.
    fn ci_ok() -> String {
        "jobs:\n  schema:\n    steps:\n      - run: make ci-local-schema\n  other:\n    steps:\n      - run: true\n".to_string()
    }

    /// Makefile fixtures derive VERIFY_* from the consts so they cannot drift (gate_t440).
    fn make_ok() -> String {
        format!(
            "ci-local-schema: ## schema\n\t$(MAKE) schema-validate\n\t$(MAKE) verify-citations\n\
verify-t456: ## t456\n\t@{VERIFY_T456}\n\
verify-t468: ## t468\n\t@{VERIFY_T468}\n"
        )
    }

    #[test]
    fn live_shaped_pins_hold() {
        assert_eq!(
            run_pins(&ci_ok(), Some(&make_ok()), Path::new("Makefile")),
            0
        );
    }

    #[test]
    fn hollow_t456_recipe_fails() {
        let mf = make_ok().replace(&format!("\t@{VERIFY_T456}"), "\t@true");
        assert_ne!(run_pins(&ci_ok(), Some(&mf), Path::new("Makefile")), 0);
    }

    #[test]
    fn hollow_t468_recipe_fails() {
        let mf = make_ok().replace(&format!("\t@{VERIFY_T468}"), "\t@true");
        assert_ne!(run_pins(&ci_ok(), Some(&mf), Path::new("Makefile")), 0);
    }

    #[test]
    fn bash_spelling_no_longer_satisfies_pin() {
        let mf = make_ok().replace(
            VERIFY_T456,
            "bash scripts/mod/verify-t456-mission-rest-size-gate.sh",
        );
        assert_ne!(run_pins(&ci_ok(), Some(&mf), Path::new("Makefile")), 0);
    }

    #[test]
    fn schema_job_without_ci_local_schema_fails() {
        let ci = "jobs:\n  schema:\n    steps:\n      - run: cargo run -p xtask -- schema validate\n  other:\n    steps:\n      - run: true\n";
        assert_ne!(run_pins(ci, Some(&make_ok()), Path::new("Makefile")), 0);
    }

    #[test]
    fn verify_consts_are_the_cargo_spelling() {
        assert_eq!(VERIFY_T456, "cargo run -q -p xtask -- verify t456");
        assert_eq!(VERIFY_T468, "cargo run -q -p xtask -- verify t468");
    }
}
