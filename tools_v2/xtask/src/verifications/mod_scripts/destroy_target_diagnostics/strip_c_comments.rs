use super::*;

/// Python `strip_c_comments`: drop `//` and `/* */`, keep newlines inside block comments.
pub(super) fn strip_c_comments(src: &str) -> String {
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

pub(super) fn assert_other_pins(
    root: &Path,
    file: &Path,
    pins: &[&str],
    failed: &mut bool,
) -> bool {
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

/// FAIL-OPEN CLOSED: bash `if scan … 2>/dev/null` treated any non-zero (crash, 127) as "expected
/// FAIL". Here only [`Verdict::Failed`]-equivalent (dirty scan) counts as the proof biting;
/// a clean scan is "still passed"; we never swallow a DidNotRun because the subject is in hand.
pub(super) fn red_scan(
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

pub(super) fn red_registry(
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

pub(super) fn inject_paraphrase_lie(src: &str) -> Option<String> {
    if !src.contains("ArmDestroyTargets") {
        eprintln!("RED1 setup failed: ArmDestroyTargets missing");
        return None;
    }
    let lie = "\t//! entities[] are never placed on today's build (struct ignores them)\n";
    let out = src.replacen(ARM_SIG, &format!("{lie}{ARM_SIG}"), 1);
    if out == src { None } else { Some(out) }
}

pub(super) fn collapse_diagnose_returns(src: &str) -> Option<String> {
    // Exact Python `re.sub` needle: literal spaces (not `\\s+`), `.*?` under DOTALL, group 2 = `\\n\\t}`.
    let pat = Regex::new(
        r"(?s)(protected static string DiagnoseEmptyDestroyTargets\(notnull TBD_Objective objective\)\n\t\{).*?(\n\t\})",
    )
    .ok()?;
    let mut count = 0;
    let out = pat.replace(src, |caps: &regex::Captures| {
        count += 1;
        format!(
            "{}\n\t\t//! Distinguishes missing/skipped spawn vs out-of-zone placement (comment only).\n\t\treturn \"destroy targets empty — no matches in zone\";\n\t{}",
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

pub(super) fn rename_diagnose_fn(src: &str) -> Option<String> {
    let mut src2 = src.replacen(
        DIAG_SIG,
        "\t//! DiagnoseEmptyDestroyTargets — renamed; name kept in comment only.\n\
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

pub(super) fn comment_only_registry_pin(src: &str) -> Option<String> {
    if !src.contains(REG_FORMAT_OLD) {
        return None;
    }
    let new = "\t\t\t//! not in the registry, so there is no prefab to look for (comment only)\n\
               \t\t\tobjective.m_sInertReason = DiagnoseEmptyDestroyTargets(objective);";
    Some(src.replacen(REG_FORMAT_OLD, new, 1))
}

pub(super) fn inject_historical_lie(src: &str) -> Option<String> {
    let lie = "\t//! This build does not spawn the mission document `entities[]` — \
               TBD_MissionDocumentStruct does not model them\n";
    let out = src.replacen(ARM_SIG, &format!("{lie}{ARM_SIG}"), 1);
    if out == src { None } else { Some(out) }
}
