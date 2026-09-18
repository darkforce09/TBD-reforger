use super::*;

/// The `{ … }` body of a shell function, by brace counting.
///
/// The one place a raw [`regex::Regex`] is used instead of [`Pattern`]: this needs the match
/// *offset*, and `Pattern` deliberately exposes only `is_match`. `(?m)^name\(\)` is a line anchor,
/// which is what `Pattern` builds with anyway.
///
/// Brace counting ignores quotes and heredocs, exactly as the script did. In `wave.sh` that holds
/// because its function bodies are balanced; a `"}"` inside a string would truncate the body early,
/// which fails CLOSED — a shorter body cannot contain the invocation.
pub(super) fn extract_fn_body<'a>(src: &'a str, fn_name: &str) -> Result<Option<&'a str>> {
    let opener = regex::Regex::new(&format!(
        r"(?m)^(?:pub\s+)?fn {}\s*\(",
        regex::escape(fn_name)
    ))?;
    let Some(m) = opener.find(src) else {
        return Ok(None);
    };
    // T-902: rust `pub fn name(` — signature may span lines (`-> u8 {`). Find the body brace.
    let Some(brace) = src[m.start()..].find('{') else {
        return Ok(None);
    };
    let start = m.start() + brace;
    let mut depth = 0i32;
    for (offset, ch) in src[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(Some(&src[start..start + offset + 1]));
                }
            }
            _ => {}
        }
    }
    Ok(None) // Unbalanced — reported as "missing `name()`", which is what the script did.
}

/// [`SEEDS`] minus `entry` — the RED-2 perturbation, DERIVED from the live const.
///
/// `None` means the const no longer contains the entry the arm removes, i.e. the *proof* is
/// broken rather than the tree — the live pin will already have said so, and a RED arm that
/// perturbs nothing must not print "→ FAIL (expected)". The script wrote the same sentence to
/// stderr and `sys.exit(2)`; that status is preserved.
pub(super) fn seeds_without(
    seeds: &[&'static str],
    entry: &str,
    arm: &str,
) -> Option<Vec<&'static str>> {
    if !seeds.contains(&entry) {
        eprintln!("{arm} setup failed: {entry} is not in {RECIPE_CONST} to begin with");
        return None;
    }
    Some(seeds.iter().copied().filter(|s| *s != entry).collect())
}

/// `&[&'static str]` → `&[&str]`, so a perturbed `Vec` can be handed to `run_pins`.
pub(super) fn borrow<'a>(seeds: &'a [&'static str]) -> Vec<&'a str> {
    seeds.to_vec()
}

/// Delete the VERIFY_STEPS t440 row. T-902: one shared table, so zero copies remain.
///
/// The count check is the arm's own integrity test: if the row is still present, nothing was
/// actually removed; if it was never present, the proof would pass for the wrong reason.
pub(super) fn delete_first_wave_run(wave: &str) -> Option<String> {
    let Some(idx) = wave.find(WAVE_RUN_LINE) else {
        eprintln!("RED3 setup failed: gate.rs T-440 VERIFY_STEPS row not found");
        return None;
    };
    let out = format!("{}{}", &wave[..idx], &wave[idx + WAVE_RUN_LINE.len()..]);
    let left = out.matches(WAVE_RUN_LINE).count();
    if left != 0 {
        eprintln!("RED3 setup failed: expected 0 remaining t440 rows, got {left}");
        return None;
    }
    Some(out)
}

/// A RED arm's verdict-about-a-verdict. `passed` is the *pin set's* result, so a RED arm that
/// "passes" means the pin failed to bite — which is itself a gate failure.
pub(super) fn red(passed: bool, still_passed: &str, expected: &str, failed: &mut bool) {
    if passed {
        println!("{still_passed}");
        *failed = true;
    } else {
        println!("{expected}");
    }
}

/// bash's `FAIL: missing $PATH` + a six-space hint, with a typed cause behind it.
pub(super) fn missing(path: &Path, hint: String) -> Verdict {
    Verdict::DidNotRun(
        NotRun::TargetMissing(path.to_path_buf()),
        Finding {
            headline: format!("missing {}", path.display()),
            detail: vec![hint],
        },
    )
}

/// Print an unlabelled verdict (the pre-flights) and return the script's `exit 1`.
pub(super) fn emit(verdict: Verdict) -> u8 {
    println!("{verdict}");
    u8::try_from(verdict.into_exit_legacy_binary()).unwrap_or(1)
}

/// Print a verdict in the heredoc's labelled form: `FAIL (label): headline`, then six-space detail.
///
/// [`Finding`]'s own `Display` writes a bare `FAIL:`, so the label forces a hand-rolled render —
/// but the `Verdict` stays the carrier, so a `DidNotRun` cannot be printed as if the pin had run.
pub(super) fn emit_labelled(verdict: &Verdict, label: &str) {
    match verdict {
        Verdict::Held => {}
        Verdict::Failed(finding) | Verdict::DidNotRun(_, finding) => {
            println!("FAIL ({label}): {}", finding.headline);
            for line in &finding.detail {
                println!("      {line}");
            }
        }
    }
}

/// Read the seed and linked wave implementations, naming unreadable or disconnected inputs.
pub(super) fn read_pair(seed: &Path, wave: &Path) -> Result<(String, String), Verdict> {
    use crate::verifications::architecture::wave_gate_sources::{
        WAVE_CHILDREN, wave_children_are_linked,
    };

    let seed_text = read_py(seed)?;
    let mut wave_text = read_py(wave)?;
    if !wave_children_are_linked(&wave_text) {
        return Err(Verdict::failed(
            "gate.rs must declare and export both gate_slice and cmd_gate implementation modules",
        ));
    }
    for (module_name, _) in WAVE_CHILDREN {
        let child = wave.with_extension("").join(format!("{module_name}.rs"));
        wave_text.push('\n');
        wave_text.push_str(&read_py(&child)?);
    }
    Ok((seed_text, wave_text))
}

/// Read a file the way Python's text mode did, or name why not.
///
/// Universal newlines (`\r\n` and lone `\r` → `\n`) is not pedantry here: without it a CRLF input
/// would put a literal `\r` into every `py_repr` in the evidence dump, and `trim_end` would then
/// disagree with `.rstrip()` about where a line ends. `.editorconfig` forbids CRLF in this repo,
/// so it is belt-and-braces for a tree checked out on Windows.
pub(super) fn read_py(path: &Path) -> Result<String, Verdict> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.contains('\r') => Ok(text.replace("\r\n", "\n").replace('\r', "\n")),
        Ok(text) => Ok(text),
        Err(source) => Err(Verdict::did_not_run(
            format!("cannot read {}", path.display()),
            Kind::Pin,
            NotRun::Unreadable {
                path: path.to_path_buf(),
                source,
            },
        )),
    }
}

/// CPython `repr()` of a `str`, which the RED-2b evidence dump emits verbatim.
///
/// Rust's `{:?}` is NOT a substitute: it escapes `'` (giving `"…\'…"`), always picks double quotes,
/// and renders control characters as `\u{7f}`. CPython picks `'` unless the string contains one and
/// no `"`, and escapes control characters as `\xNN`.
///
/// ASCII is exact. Above U+009F this treats every code point as printable, where CPython consults
/// `str.isprintable()` (Unicode categories Cc/Cf/Cs/Co/Cn/Zl/Zp/Zs). Closing that gap means
/// shipping Unicode category tables for a case that requires an unprintable non-ASCII code point
/// inside a seed FILE NAME on a list that has ALREADY failed the membership pin. Noted, not
/// implemented.
pub(super) fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        let cp = c as u32;
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            _ if cp < 0x20 || cp == 0x7f || (0x80..=0x9f).contains(&cp) => {
                out.push_str(&format!("\\x{cp:02x}"));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}
