//! T-437 / T-474 — Destroy-target inert diagnostics must not claim entities[] never spawn
//! (T-853 / T-854 port of `scripts/mod/verify-t437-destroy-inert-diagnostics.sh`).
//!
//! After T-254, `TBD_MissionDocumentStruct` models `entities[]` and `SpawnMissionEntities` places
//! resolvable rows. Operator-facing strings that still blame a build that "does not spawn/model
//! entities[]" are lies. T-474 closed four false-green classes (paraphrased lies, collapsed
//! DiagnoseEmpty returns with pins only in comments, renamed fn with name only in a comment,
//! unresolved-alias registry pin moved to a comment). This gate strips `//` / `/* */` before
//! structural pins, requires a live fn definition + three return-string arms, broadens forbidden
//! paraphrases, and RED→GREEN-proves each attack on every run.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`, entirely — seven call sites.** Two heredocs (scan + registry pins) and five RED
//!    setup transforms. The script was on `scripts/python-inventory.txt` solely for those; the
//!    inventory line goes with them.
//! 2. **Five `2>/dev/null` fail-opens on the RED arms.** Each RED proof read
//!    `if scan_forbidden_file|assert_registry_pins … 2>/dev/null; then "still passed" else
//!    "FAIL (expected)"`. A crash / unreadable TMP / absent `python3` (127) exited non-zero and
//!    was indistinguishable from "the pin correctly rejected the perturbation", with the traceback
//!    swallowed. Here checks return [`Verdict`]: Held / Failed / DidNotRun cannot be confused, and
//!    a DidNotRun on a RED arm fails the gate with a distinct message (not "expected").
//! 3. **`mktemp` + `trap` scribble risk.** Perturbations are in-memory string transforms; FAIL
//!    lines that named `$TMP` still print a `/tmp/tmp.*` display path so clean-tree acceptance
//!    normalises the same way bash-vs-bash did. Live files are never written.
//!
//! Output + binary 0/1 status are a contract (`wave.sh` tails failures; T-853 diffs stdout).

use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use tbd_gate::{Finding, NotRun, Pattern, Verdict, gate};

const REG_REL: &str = "apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectiveRegistry.c";
const COMP_REL: &str =
    "apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c";
const RULES_REL: &str = "apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectiveRules.c";
const SCHEMA_REL: &str = "packages/tbd-schema/schema/mission.schema.json";
const VALIDATOR_REL: &str =
    "apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c";

const EXACT_LIES: &[&str] = &[
    "This build does not spawn the mission document",
    "does not spawn the mission document",
    "nothing spawns the mission document",
    "nothing spawns mission `entities[]`",
    "on today's build nothing spawns mission",
    "TBD_MissionDocumentStruct does not model them",
    "TBD_MissionDocumentStruct ignore `entities[]`",
    "does not spawn mission entities",
    "this build cannot create it",
];

const PARAPHRASES: &[&str] = &[
    r"are never placed",
    r"never placed",
    r"struct ignores",
    r"ignores them",
    r"ignores entities",
    r"does not spawn entities",
    r"does not model entities",
];

const ARM_SIG: &str = "\tstatic void ArmDestroyTargets(notnull TBD_Objective objective)";
const DIAG_SIG: &str =
    "\tprotected static string DiagnoseEmptyDestroyTargets(notnull TBD_Objective objective)";
const REG_PIN: &str = "not in the registry, so there is no prefab to look for";
const REG_FORMAT_OLD: &str = "\t\t\tobjective.m_sInertReason = string.Format(\
\"rules.targetAlias '%1' is not in the registry, so there is no prefab to look for\", \
objective.m_sTargetAlias);";

/// Entry point. `0` when live pins hold and every RED proof bit; `1` on any failure; `2` when a
/// RED arm cannot be set up (`sys.exit(2)` under bash `set -e`).
pub fn verify_t437(repo_root: &Path) -> Result<u8> {
    let paths = Paths::resolve(repo_root);
    for p in paths.all() {
        if !p.is_file() {
            println!("FAIL: missing {}", p.display());
            return Ok(1);
        }
    }

    let texts = match paths.read_all() {
        Ok(t) => t,
        Err(v) => return Ok(emit(v)),
    };

    let mut failed = false;

    for (path, text) in [
        (paths.reg.as_path(), texts.reg.as_str()),
        (paths.comp.as_path(), texts.comp.as_str()),
        (paths.rules.as_path(), texts.rules.as_str()),
        (paths.schema.as_path(), texts.schema.as_str()),
        (paths.validator.as_path(), texts.validator.as_str()),
    ] {
        let _ = scan_forbidden(path, text, &mut failed)?;
    }
    let _ = assert_registry_pins(&texts.reg, "live", &mut failed)?;

    let root = repo_root;
    let _ = assert_other_pins(root, &paths.comp, &["SpawnMissionEntities"], &mut failed);
    let _ = assert_other_pins(
        root,
        &paths.rules,
        &["TBD_MissionDocumentStruct` models `entities[]`"],
        &mut failed,
    );
    let _ = assert_other_pins(
        root,
        &paths.schema,
        &["SpawnMissionEntities", "out-of-zone authorship"],
        &mut failed,
    );
    let _ = assert_other_pins(
        root,
        &paths.validator,
        &["entities[] is modeled + SpawnMissionEntities"],
        &mut failed,
    );

    // Display path for RED FAIL lines — bash used `mktemp` (`/tmp/tmp.XXXXXXXXXX`). A stable
    // alphanumeric suffix keeps the T-853 path-normaliser (`/tmp/tmp.[A-Za-z0-9]+`) happy.
    let tmp_display = PathBuf::from(format!("/tmp/tmp.t437{}", std::process::id()));

    // ── RED 1: paraphrased lie ───────────────────────────────────────────────────────────────
    let red1 = match inject_paraphrase_lie(&texts.reg) {
        Some(s) => s,
        None => {
            eprintln!("RED1 setup failed: could not inject paraphrase");
            return Ok(2);
        }
    };
    red_scan(
        &tmp_display,
        &red1,
        "FAIL: RED paraphrased lie still passed — forbidden paraphrases not discriminating",
        "RED proof: paraphrased 'never placed' / 'struct ignores them' lie → FAIL (expected)",
        &mut failed,
    )?;

    // ── RED 2: collapse DiagnoseEmpty returns ────────────────────────────────────────────────
    let red2 = match collapse_diagnose_returns(&texts.reg) {
        Some(s) => s,
        None => {
            eprintln!("RED2 setup failed: could not collapse DiagnoseEmpty returns (n=0)");
            return Ok(2);
        }
    };
    red_registry(
        &red2,
        "RED-collapse-returns",
        "FAIL: RED collapsed DiagnoseEmpty returns still passed — return-arm pins ignore comments?",
        "RED proof: collapsed DiagnoseEmpty returns (out-of-zone only in comment) → FAIL (expected)",
        &mut failed,
    )?;

    // ── RED 3: rename live fn ────────────────────────────────────────────────────────────────
    let red3 = match rename_diagnose_fn(&texts.reg) {
        Some(s) => s,
        None => {
            eprintln!("RED3 setup failed: live definition still present or rename missed");
            return Ok(2);
        }
    };
    red_registry(
        &red3,
        "RED-rename-fn",
        "FAIL: RED comment-only DiagnoseEmptyDestroyTargets still passed — definition pin weak",
        "RED proof: DiagnoseEmptyDestroyTargets definition renamed (name comment-only) → FAIL (expected)",
        &mut failed,
    )?;

    // ── RED 4: registry pin comment-only ─────────────────────────────────────────────────────
    let red4 = match comment_only_registry_pin(&texts.reg) {
        Some(s) => s,
        None => {
            eprintln!("RED4 setup failed: unresolved-alias Format line not found");
            return Ok(2);
        }
    };
    red_registry(
        &red4,
        "RED-registry-comment",
        "FAIL: RED comment-only registry pin still passed — pin search ignores comments?",
        "RED proof: unresolved-alias registry pin comment-only → FAIL (expected)",
        &mut failed,
    )?;

    // ── RED 5: exact historical lie ──────────────────────────────────────────────────────────
    let red5 = match inject_historical_lie(&texts.reg) {
        Some(s) => s,
        None => {
            eprintln!("RED5 setup failed: could not inject historical lie");
            return Ok(2);
        }
    };
    red_scan(
        &tmp_display,
        &red5,
        "FAIL: RED exact historical lie restore still passed",
        "RED proof: exact historical lie restore → FAIL (expected)",
        &mut failed,
    )?;

    // ── GREEN: re-read live REG ──────────────────────────────────────────────────────────────
    match read_text(&paths.reg) {
        Ok(reg_now) => {
            let mut green_failed = false;
            let held = assert_registry_pins(&reg_now, "live-restore", &mut green_failed)?;
            if held {
                println!(
                    "GREEN proof: live DiagnoseEmptyDestroyTargets arms + registry pin → PASS"
                );
            } else {
                println!(
                    "FAIL: live registry no longer passes after RED proofs (REG should be untouched)"
                );
                failed = true;
            }
        }
        Err(v) => {
            emit_labelled(&v, "live-restore");
            println!(
                "FAIL: live registry no longer passes after RED proofs (REG should be untouched)"
            );
            failed = true;
        }
    }

    if failed {
        println!("verify-t437-destroy-inert-diagnostics: FAIL");
        return Ok(1);
    }
    println!("verify-t437-destroy-inert-diagnostics: PASS");
    Ok(0)
}

// ── Paths / I/O ──────────────────────────────────────────────────────────────────────────────

struct Paths {
    reg: PathBuf,
    comp: PathBuf,
    rules: PathBuf,
    schema: PathBuf,
    validator: PathBuf,
}

struct Texts {
    reg: String,
    comp: String,
    rules: String,
    schema: String,
    validator: String,
}

impl Paths {
    fn resolve(root: &Path) -> Paths {
        Paths {
            reg: root.join(REG_REL),
            comp: root.join(COMP_REL),
            rules: root.join(RULES_REL),
            schema: root.join(SCHEMA_REL),
            validator: root.join(VALIDATOR_REL),
        }
    }

    fn all(&self) -> [&Path; 5] {
        [
            self.reg.as_path(),
            self.comp.as_path(),
            self.rules.as_path(),
            self.schema.as_path(),
            self.validator.as_path(),
        ]
    }

    fn read_all(&self) -> Result<Texts, Verdict> {
        Ok(Texts {
            reg: read_text(&self.reg)?,
            comp: read_text(&self.comp)?,
            rules: read_text(&self.rules)?,
            schema: read_text(&self.schema)?,
            validator: read_text(&self.validator)?,
        })
    }
}

fn read_text(path: &Path) -> Result<String, Verdict> {
    std::fs::read_to_string(path).map_err(|source| {
        Verdict::DidNotRun(
            NotRun::Unreadable {
                path: path.to_path_buf(),
                source,
            },
            Finding {
                headline: format!("cannot read {}", path.display()),
                detail: vec![
                    "The pin could not run. An unreadable input must not read as a clean result."
                        .into(),
                ],
            },
        )
    })
}

fn emit(v: Verdict) -> u8 {
    println!("{v}");
    u8::try_from(v.into_exit_legacy_binary()).unwrap_or(1)
}

fn emit_labelled(v: &Verdict, label: &str) {
    match v {
        Verdict::Held => {}
        Verdict::Failed(f) | Verdict::DidNotRun(_, f) => {
            println!("FAIL ({label}): {}", f.headline);
            for line in &f.detail {
                println!("      {line}");
            }
        }
    }
}

// ── scan_forbidden_file (Python → Rust) ──────────────────────────────────────────────────────

/// Returns `true` when the file is clean (bash exit 0). Prints FAIL lines and sets `failed` on
/// violations. `DidNotRun` is not used here — the subject is already in hand.
fn scan_forbidden(path: &Path, text: &str, failed: &mut bool) -> Result<bool> {
    let mut dirty = false;
    for needle in EXACT_LIES {
        if let Some(idx) = text.find(needle) {
            let line = text[..idx].bytes().filter(|&b| b == b'\n').count() + 1;
            println!("FAIL: forbidden lie in {}:", path.display());
            println!("  {line}: {needle}");
            dirty = true;
        }
    }

    let norm = Regex::new(r"\s+")?.replace_all(text, " ");
    let allow =
        Regex::new(r#"(?i)never a ['"]build does not spawn entities\[\]['"] (?:claim|lie)"#)?;
    let allow_spans: Vec<(usize, usize)> = allow
        .find_iter(&norm)
        .map(|m| (m.start(), m.end()))
        .collect();

    let spawn = Regex::new(r#"(?i)never a ['"]build does not spawn"#)?;
    'pats: for pat in PARAPHRASES {
        let re = Regex::new(&format!("(?i){pat}"))?;
        for m in re.find_iter(&norm) {
            if allow_spans
                .iter()
                .any(|&(s, e)| m.start() >= s && m.end() <= e)
            {
                continue;
            }
            let start = m.start().saturating_sub(24);
            let end = (m.end() + 24).min(norm.len());
            let window = &norm[start..end];
            // Match text varies per hit, so the wider allow-window regex cannot be hoisted.
            #[allow(clippy::regex_creation_in_loops)]
            let wider = Regex::new(&format!(
                r#"(?i)never a ['"][^'"]{{0,40}}{}"#,
                regex::escape(m.as_str())
            ))?;
            if wider.is_match(window) {
                continue;
            }
            if spawn.is_match(window) && m.as_str().to_ascii_lowercase().contains("entities") {
                continue;
            }
            println!("FAIL: forbidden paraphrase in {}:", path.display());
            println!("  /{pat}/ matched: {}", py_repr_str(m.as_str()));
            dirty = true;
            break 'pats;
        }
        if dirty {
            break;
        }
    }

    if dirty {
        *failed = true;
    }
    Ok(!dirty)
}

fn py_repr_str(s: &str) -> String {
    // Python `repr` for these ASCII needles: always single-quoted, escape `'` and `\`.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

// ── assert_registry_pins ─────────────────────────────────────────────────────────────────────

/// Returns `true` when all pins hold. Accumulates (bash `fail_msg` + continue) except where the
/// heredoc `sys.exit(fail)`'d early after a missing body.
fn assert_registry_pins(src: &str, label: &str, failed: &mut bool) -> Result<bool> {
    let stripped = strip_c_comments(src);
    let mut local_fail = false;

    let defn = Regex::new(r"protected\s+static\s+string\s+DiagnoseEmptyDestroyTargets\s*\(")?;
    if !defn.is_match(&stripped) {
        fail_msg(
            label,
            "missing live definition `protected static string DiagnoseEmptyDestroyTargets(` (non-comment)",
            &mut local_fail,
        );
    }

    let body_re = Regex::new(
        r"protected\s+static\s+string\s+DiagnoseEmptyDestroyTargets\s*\(notnull TBD_Objective objective\)\s*\{",
    )?;
    let Some(m) = body_re.find(&stripped) else {
        fail_msg(
            label,
            "could not locate DiagnoseEmptyDestroyTargets body after comment strip",
            &mut local_fail,
        );
        if local_fail {
            *failed = true;
        }
        return Ok(!local_fail);
    };

    let start = m.end() - 1;
    let Some(end) = brace_end(&stripped, start) else {
        fail_msg(
            label,
            "DiagnoseEmptyDestroyTargets body was not brace-closed",
            &mut local_fail,
        );
        if local_fail {
            *failed = true;
        }
        return Ok(!local_fail);
    };
    let body = &stripped[start..=end];

    let arms: &[(&str, &str)] = &[
        ("out-of-zone", "out-of-zone placement"),
        (
            "missing-row",
            "No `entities[]` row with that alias was authored",
        ),
        ("spawn-miss", "spawn likely skipped or failed"),
    ];
    for (name, needle) in arms {
        if !body.contains(needle) {
            fail_msg(
                label,
                &format!(
                    "DiagnoseEmptyDestroyTargets body missing live {name} arm pin: {}",
                    py_repr_str(needle)
                ),
                &mut local_fail,
            );
            continue;
        }
        let arm_re = Regex::new(&format!(
            r"(?s)return\s+string\.Format\([^;]*{}",
            regex::escape(needle)
        ))?;
        if !arm_re.is_match(body) {
            fail_msg(
                label,
                &format!(
                    "DiagnoseEmptyDestroyTargets {name} pin {} is not inside a \
                     `return string.Format(...)` arm",
                    py_repr_str(needle)
                ),
                &mut local_fail,
            );
        }
    }

    if !stripped.contains(REG_PIN) {
        fail_msg(
            label,
            &format!(
                "missing live registry pin (non-comment): {}",
                py_repr_str(REG_PIN)
            ),
            &mut local_fail,
        );
    } else {
        let fmt_re = Regex::new(&format!(
            r"(?s)string\.Format\([^;]*{}",
            regex::escape(REG_PIN)
        ))?;
        if !fmt_re.is_match(&stripped) {
            fail_msg(
                label,
                &format!(
                    "registry pin {} is not inside a live string.Format(...)",
                    py_repr_str(REG_PIN)
                ),
                &mut local_fail,
            );
        }
    }

    if !body.contains("SpawnMissionEntities") {
        fail_msg(
            label,
            "DiagnoseEmptyDestroyTargets body missing live SpawnMissionEntities mention",
            &mut local_fail,
        );
    }

    if local_fail {
        *failed = true;
    }
    Ok(!local_fail)
}

fn fail_msg(label: &str, msg: &str, failed: &mut bool) {
    println!("FAIL ({label}): {msg}");
    *failed = true;
}

fn brace_end(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
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

// ── assert_other_pins (gate_require) ─────────────────────────────────────────────────────────

fn assert_other_pins(root: &Path, file: &Path, pins: &[&str], failed: &mut bool) -> bool {
    let rel = file
        .strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| file.display().to_string());
    let mut ok = true;
    for pin in pins {
        let msg = format!("missing truth pin in {rel}: {pin}");
        let v = gate::require(&msg, &Pattern::literal(pin), &[file]);
        match &v {
            Verdict::Held => {}
            Verdict::Failed(_) | Verdict::DidNotRun(_, _) => {
                println!("{v}");
                ok = false;
                *failed = true;
            }
        }
    }
    ok
}

// ── RED helpers ──────────────────────────────────────────────────────────────────────────────

/// FAIL-OPEN CLOSED: bash `if scan … 2>/dev/null` treated any non-zero (crash, 127) as "expected
/// FAIL". Here only [`Verdict::Failed`]-equivalent (dirty scan) counts as the proof biting;
/// a clean scan is "still passed"; we never swallow a DidNotRun because the subject is in hand.
fn red_scan(
    path: &Path,
    text: &str,
    still_passed: &str,
    expected: &str,
    failed: &mut bool,
) -> Result<()> {
    let mut local = false;
    let clean = scan_forbidden(path, text, &mut local)?;
    if clean {
        println!("{still_passed}");
        *failed = true;
    } else {
        println!("{expected}");
        // scan_forbidden set local; do NOT propagate to outer `failed` — RED expected to be dirty.
        let _ = local;
    }
    Ok(())
}

fn red_registry(
    text: &str,
    label: &str,
    still_passed: &str,
    expected: &str,
    failed: &mut bool,
) -> Result<()> {
    let mut local = false;
    let held = assert_registry_pins(text, label, &mut local)?;
    if held {
        println!("{still_passed}");
        *failed = true;
    } else {
        println!("{expected}");
        let _ = local;
    }
    Ok(())
}

fn inject_paraphrase_lie(src: &str) -> Option<String> {
    if !src.contains("ArmDestroyTargets") {
        eprintln!("RED1 setup failed: ArmDestroyTargets missing");
        return None;
    }
    let lie = "\t//! entities[] are never placed on today's build (struct ignores them)\n";
    let out = src.replacen(ARM_SIG, &format!("{lie}{ARM_SIG}"), 1);
    if out == src { None } else { Some(out) }
}

fn collapse_diagnose_returns(src: &str) -> Option<String> {
    // Exact Python `re.sub` needle: literal spaces (not `\\s+`), `.*?` under DOTALL, group 2 = `\\n\\t}`.
    let pat = Regex::new(
        r"(?s)(protected static string DiagnoseEmptyDestroyTargets\(notnull TBD_Objective objective\)\n\t\{).*?(\n\t\})",
    )
    .ok()?;
    let mut count = 0;
    let out = pat.replace(src, |caps: &regex::Captures| {
        count += 1;
        format!(
            "{}\n\t\t//! Distinguishes missing/skipped spawn vs out-of-zone placement (comment only — T-474 RED).\n\t\treturn \"destroy targets empty — no matches in zone\";\n\t{}",
            caps.get(1).map(|m| m.as_str()).unwrap_or(""),
            caps.get(2).map(|m| m.as_str()).unwrap_or(""),
        )
    });
    if count != 1 {
        None
    } else {
        Some(out.into_owned())
    }
}

fn rename_diagnose_fn(src: &str) -> Option<String> {
    let mut src2 = src.replacen(
        DIAG_SIG,
        "\t//! DiagnoseEmptyDestroyTargets — renamed; name kept in comment only (T-474 RED).\n\
         \tprotected static string DiagnoseEmptyTargets(notnull TBD_Objective objective)",
        1,
    );
    src2 = src2.replacen(
        "DiagnoseEmptyDestroyTargets(objective)",
        "DiagnoseEmptyTargets(objective)",
        1,
    );
    if src2.contains("DiagnoseEmptyDestroyTargets(notnull") || src2 == src {
        None
    } else {
        Some(src2)
    }
}

fn comment_only_registry_pin(src: &str) -> Option<String> {
    if !src.contains(REG_FORMAT_OLD) {
        return None;
    }
    let new = "\t\t\t//! was: not in the registry, so there is no prefab to look for (T-474 RED comment-only)\n\
               \t\t\tobjective.m_sInertReason = DiagnoseEmptyDestroyTargets(objective);";
    Some(src.replacen(REG_FORMAT_OLD, new, 1))
}

fn inject_historical_lie(src: &str) -> Option<String> {
    let lie = "\t//! This build does not spawn the mission document `entities[]` — \
               TBD_MissionDocumentStruct does not model them\n";
    let out = src.replacen(ARM_SIG, &format!("{lie}{ARM_SIG}"), 1);
    if out == src { None } else { Some(out) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask parent")
            .to_path_buf()
    }

    #[test]
    fn live_tree_holds() {
        assert_eq!(verify_t437(&repo()).unwrap(), 0);
    }

    #[test]
    fn strip_keeps_block_newlines() {
        assert_eq!(strip_c_comments("a // x\nb"), "a \nb");
        assert_eq!(strip_c_comments("a /* x\ny */ b"), "a \n b");
    }

    #[test]
    fn paraphrase_injection_is_caught() {
        let reg = std::fs::read_to_string(repo().join(REG_REL)).unwrap();
        let dirty = inject_paraphrase_lie(&reg).expect("inject");
        let mut failed = false;
        let clean = scan_forbidden(Path::new("/tmp/tmp.test"), &dirty, &mut failed).unwrap();
        assert!(!clean);
        assert!(failed);
    }

    #[test]
    fn collapsed_returns_fail_registry_pins() {
        let reg = std::fs::read_to_string(repo().join(REG_REL)).unwrap();
        let dirty = collapse_diagnose_returns(&reg).expect("collapse");
        let mut failed = false;
        assert!(!assert_registry_pins(&dirty, "t", &mut failed).unwrap());
    }
}
