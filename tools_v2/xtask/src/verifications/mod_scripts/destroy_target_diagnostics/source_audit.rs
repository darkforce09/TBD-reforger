use super::*;

/// Entry point. `0` when live pins hold and every RED proof bit; `1` on any failure; `2` when a
/// RED arm cannot be set up (`sys.exit(2)` under bash `set -e`).
pub fn verify_destroy_target_diagnostics(repo_root: &Path) -> Result<u8> {
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
    // alphanumeric suffix keeps the path-normaliser (`/tmp/tmp.[A-Za-z0-9]+`) happy.
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
        println!("destroy-target-diagnostics: FAIL");
        return Ok(1);
    }
    println!("destroy-target-diagnostics: PASS");
    Ok(0)
}

pub(super) fn read_text(path: &Path) -> Result<String, Verdict> {
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

pub(super) fn emit(v: Verdict) -> u8 {
    println!("{v}");
    u8::try_from(v.into_binary_exit_code()).unwrap_or(1)
}

pub(super) fn emit_labelled(v: &Verdict, label: &str) {
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

/// Returns `true` when the file is clean (bash exit 0). Prints FAIL lines and sets `failed` on
/// violations. `DidNotRun` is not used here — the subject is already in hand.
pub(super) fn scan_forbidden(path: &Path, text: &str, failed: &mut bool) -> Result<bool> {
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

pub(super) fn py_repr_str(s: &str) -> String {
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

/// Returns `true` when all pins hold. Accumulates (bash `fail_msg` + continue) except where the
/// heredoc `sys.exit(fail)`'d early after a missing body.
pub(super) fn assert_registry_pins(src: &str, label: &str, failed: &mut bool) -> Result<bool> {
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

pub(super) fn fail_msg(label: &str, msg: &str, failed: &mut bool) {
    println!("FAIL ({label}): {msg}");
    *failed = true;
}

pub(super) fn brace_end(s: &str, start: usize) -> Option<usize> {
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
