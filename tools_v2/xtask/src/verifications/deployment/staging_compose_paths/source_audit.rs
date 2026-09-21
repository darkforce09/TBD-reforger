use super::*;

/// Entry point. `0` when the contract holds, `1` for every failure.
///
/// Deliberately NOT [`Verdict::into_exit`]'s three-way code: the wave gate and the `ci.yml`
/// `mod-gates-hosted` job both record pass/fail from this status, so a 2 for a broken checkout
/// would change what CI says. Widening the status is a decision for every gate at once.
///
/// The missing-source arm is the one place the output *shape* differs from every other failure:
/// it prints `FAIL: missing …` and **no** summary line, because there was nothing to summarise.
pub fn verify_staging_compose_paths(repo_root: &Path) -> Result<u8> {
    let source_path = repo_root.join(DEPLOY_SOURCE);

    // A missing render source is `FAIL: missing <path>`, then exit 1.
    //
    // Hand-built rather than leaning on `gate::require`'s missing-target rendering, so the message
    // names this gate's subject rather than a generic pin. The CAUSE is still the typed one, so a
    // caller matching the verdict sees `DidNotRun`, not a violation.
    if !source_path.is_file() {
        let absent = Verdict::DidNotRun(
            NotRun::TargetMissing(source_path.clone()),
            Finding {
                headline: format!("missing {}", source_path.display()),
                detail: Vec::new(),
            },
        );
        println!("{absent}");
        return Ok(1);
    }

    let mut failed = false;
    for verdict in &audit(repo_root)? {
        // `Verdict::Held` renders as the empty string; printing it would emit a blank line.
        // Skipped explicitly rather than relying on Display happening to be empty.
        if matches!(verdict, Verdict::Held) {
            continue;
        }
        println!("{verdict}");
        failed = true;
    }

    if failed {
        println!("{GATE_NAME}: FAIL");
        return Ok(1);
    }
    println!("{GATE_NAME}: PASS");
    Ok(0)
}

/// Every check, in the order the report prints them: the compose-line pins, then the two on-disk
/// file checks.
///
/// Split out from [`verify_staging_compose_paths`] so the contract is testable against a scratch
/// tree without capturing stdout, and returning a LIST rather than stopping at the first failure:
/// an operator who has moved the compose file wants every place the move was missed in one run,
/// not one more place per re-run.
pub(super) fn audit(repo_root: &Path) -> Result<Vec<Verdict>> {
    let source_path = repo_root.join(DEPLOY_SOURCE);
    let mut out = Vec::new();

    // An unreadable source is a NAMED cause printed with the report, never a bare non-zero status:
    // a gate whose subject it could not open must not read as either a pass or an unexplained
    // failure. Status is unchanged (still 1).
    let source = match std::fs::read_to_string(&source_path) {
        Ok(text) => text,
        Err(cause) => {
            out.push(Verdict::did_not_run(
                format!("cannot read {}", source_path.display()),
                Kind::Pin,
                NotRun::Unreadable {
                    path: source_path,
                    source: cause,
                },
            ));
            // Fall through to the disk checks: the operator should still learn whether the
            // compose file is where it belongs.
            out.extend(compose_files_on_disk(repo_root));
            return Ok(out);
        }
    };

    let stripped = strip_comments(&source);
    out.extend(pin_compose_lines(&stripped)?);
    out.extend(ban_cd_into_api(&stripped));
    out.extend(compose_files_on_disk(repo_root));
    Ok(out)
}

/// The comment stripper the compose-line pins run on.
///
/// WHY IT EXISTS: matching the raw source for the good path counts a *comment* mentioning it as
/// presence, and a comment naming the compose file typically sits two lines above the real
/// invocation. A gate that cannot tell the contract being honoured from the contract being
/// *described* is not a gate. Everything downstream runs on the stripped text.
///
/// It reads BOTH comment syntaxes — `//` for the Rust render source, `#` for the remote shell
/// text that source embeds — from one pass, so a comment in either layer is stripped.
///
/// ODDITIES CARRIED ON PURPOSE — this is neither a Rust nor a shell parser and must not become
/// one:
///
/// * **`#` opens a comment anywhere outside quotes**, not only at a word boundary. Real `sh` reads
///   `foo#bar` as one word; this eats `#bar`. Harmless: the machine only ever *removes* text, so
///   it can only make a pin stricter.
/// * **`//` opens a comment outside quotes**, so a `//` comment in the audited source is stripped
///   too. The hazard it brings is an unquoted URL: `https://host/x` loses everything from `//`
///   on. Unreachable while the audited source quotes its URLs; worth knowing before one is added.
/// * **A backslash escapes inside single quotes.** POSIX says it does not — `'a\'` is the two-char
///   string `a\`. This consumes `\'` as a pair, stays `in_squote`, and therefore stops stripping
///   comments for the whole rest of the file, which would let a `#`-commented good path count as
///   presence again: exactly the hole this stripper exists to close. **Latent bug, carried
///   knowingly**, pinned by
///   `tests::a_backslash_before_a_closing_single_quote_swallows_the_rest` so a fix is a deliberate
///   act with a red test rather than an accident.
/// * **No heredoc, `$'…'` or line-continuation awareness.** A heredoc block is walked as ordinary
///   text. Worst case a compose line hidden in one goes unseen — and a compose line in a heredoc
///   is not one this gate pins.
pub(super) fn strip_comments(text: &str) -> String {
    // Indexed by code point, not by byte, so an index never lands mid-character.
    let src: Vec<char> = text.chars().collect();
    let n = src.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    // Never both true: the only place either is set clears the other, which is what lets the
    // in-quote arm below be one merged branch rather than two.
    let mut in_squote = false;
    let mut in_dquote = false;

    while i < n {
        let c = src[i];
        if in_squote || in_dquote {
            out.push(c);
            if c == '\\' && i + 1 < n {
                out.push(src[i + 1]);
                i += 2;
                continue;
            }
            if in_squote && c == '\'' {
                in_squote = false;
            } else if in_dquote && c == '"' {
                in_dquote = false;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' | '"' => {
                in_squote = c == '\'';
                in_dquote = c == '"';
                out.push(c);
                i += 1;
            }
            // Drop to end of line, leaving the newline itself for the next iteration to copy.
            '#' => {
                while i < n && src[i] != '\n' {
                    i += 1;
                }
            }
            '/' if i + 1 < n && src[i + 1] == '/' => {
                i += 2;
                while i < n && src[i] != '\n' {
                    i += 1;
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Split the stripped source into its dry-run compose line and its live one.
///
/// LAST ONE WINS: each field is a plain assignment, so a second dry-run compose line later in the
/// file replaces the first. Widening either to a list changes the verdict on trees this gate
/// calls clean today, so it is a deliberate decision rather than a tidy-up.
pub(super) fn classify(stripped: &str) -> Result<ComposeLines<'_>> {
    // Matched per line, so `Pattern`'s multi-line anchoring is moot here — it is used for the
    // compiled-in matcher, not the anchors.
    let compose = Pattern::regex(r"docker\s+compose\s+-f")?;
    let mut lines = ComposeLines {
        dry: None,
        live: None,
    };
    for raw in stripped.lines() {
        let line = raw.trim();
        // `probe_str`, not a bare `is_match`: this is a compound condition (match, THEN classify),
        // and the `?` stops a future fallible matcher from silently reading as "no match".
        if !gate::probe_str(&compose, line).map_err(|cause| anyhow::anyhow!("{cause:?}"))? {
            continue;
        }
        // Classify by the PRESENCE or ABSENCE of the dry-run marker, not by a transport spelling:
        // the live call spans several lines — `runner.ssh_ok(` on one, the composed command string
        // on another — so a per-line transport marker matches NEITHER and the live line goes
        // unclassified, leaving a gate that silently checks one of the two paths it exists for.
        //
        // Absence is exactly equivalent here and does not depend on how the command is dispatched:
        // there are two compose invocations, one guarded by `--dry-run` and one not. The
        // "no live line at all" arm below still fires, because an absent live invocation leaves
        // `live: None` either way.
        if line.contains(DRY_RUN_KEY) {
            lines.dry = Some(line);
        } else {
            lines.live = Some(line);
        }
    }
    Ok(lines)
}

/// The quote-safe `-f` argument extractor.
///
/// Handles `-f 'a b.yml'`, `-f "a b.yml"` and bare `-f a.yml`. The quoted alternatives are what
/// make it quote-safe: a compose path containing a space truncates under plain field splitting,
/// and a truncated path compares unequal to [`GOOD_PATH`] for the wrong reason.
pub(super) fn f_regex() -> Result<Regex> {
    Ok(Regex::new(r#"-f\s+(?:'([^']+)'|"([^"]+)"|(\S+))"#)?)
}

/// The first non-empty capture group of [`f_regex`] — the `-f` argument, whatever its quoting.
///
/// `regex::Regex` directly rather than [`Pattern`]: this needs *captures*, and `Pattern` exposes
/// only `is_match`. No anchors are involved, so the `^`/`$` line-semantics trap `Pattern` exists
/// to prevent cannot arise here.
///
/// ODDITY CARRIED KNOWINGLY: the search is not tied to the `docker compose` occurrence, so it
/// takes the FIRST `-f <arg>` anywhere on the line — `rm -f /tmp/x && docker compose -f good.yml`
/// is judged on `/tmp/x`. Unreachable on the audited source today, pinned in `tests`.
pub(super) fn f_path<'a>(re: &Regex, line: &'a str) -> Option<&'a str> {
    let caps = re.captures(line)?;
    (1..=3).find_map(|g| caps.get(g)).map(|m| m.as_str())
}

/// The pin proper: every message it can print, in the order it prints them.
///
/// Order is load-bearing — each message assumes the ones above it are shown too. "The paths
/// diverge" only reads correctly beside the two "must be …" lines, and a tree where dry-run and
/// live are each wrong in a different way emits all three.
pub(super) fn pin_compose_lines(stripped: &str) -> Result<Vec<Verdict>> {
    let lines = classify(stripped)?;
    let f_re = f_regex()?;
    let mut out = Vec::new();

    // An absent compose line is a failure, never a vacuous pass.
    if lines.dry.is_none() {
        out.push(Verdict::failed(
            "no dry-run docker compose -f line after comment strip",
        ));
    }
    if lines.live.is_none() {
        out.push(Verdict::failed(format!(
            "no live {LIVE_KEY} docker compose -f line after comment strip"
        )));
    }

    let dry_path = lines.dry.and_then(|line| f_path(&f_re, line));
    let live_path = lines.live.and_then(|line| f_path(&f_re, line));

    // A line that matched `docker compose -f` but whose `-f` has no argument. Kept distinct from
    // "wrong path" because the fix differs: a truncated edit, not a wrong destination.
    if let Some(line) = lines.dry
        && dry_path.is_none()
    {
        out.push(detailed(
            "dry-run compose line has no parseable -f path:",
            vec![line.to_string()],
        ));
    }
    if let Some(line) = lines.live
        && live_path.is_none()
    {
        out.push(detailed(
            "live compose line has no parseable -f path:",
            vec![line.to_string()],
        ));
    }

    // THE HEADLINE CONTRACT. Equality, not a substring test: a RELATIVE-ised
    // `docker-compose.staging.yml`,
    // a `../`-prefixed one and the api/ sibling are all simply "not GOOD_PATH". Each reports the
    // value actually found, so the operator can see which edit went wrong.
    if let Some(path) = dry_path
        && path != GOOD_PATH
    {
        out.push(Verdict::failed(format!(
            "dry-run -f path must be {GOOD_PATH} (got: {path})"
        )));
    }
    if let Some(path) = live_path
        && path != GOOD_PATH
    {
        out.push(Verdict::failed(format!(
            "live {LIVE_KEY} -f path must be {GOOD_PATH} (got: {path})"
        )));
    }

    // The divergence check, in one comparison. Even if a future edit relaxes what GOOD_PATH may
    // be, the rehearsal and the real thing must never disagree: a dry
    // run describing a deploy nobody is about to perform is worse than no dry run at all.
    if let (Some(dry), Some(live)) = (dry_path, live_path)
        && dry != live
    {
        out.push(detailed(
            "dry-run and live compose -f paths diverge:",
            vec![format!("dry-run: {dry}"), format!("live:    {live}")],
        ));
    }

    // Belt and braces over the equality checks above. Those
    // compare the extracted `-f` argument; this rejects the stale path ANYWHERE on the line: in an
    // `--env-file`, in a second `-f` (compose accepts overlays, and the later file wins for
    // conflicting keys), or in a `cd` sharing the line. `gate::ban_str` so the decision stays in
    // the library and only the prose is local.
    let bad = Pattern::literal(BAD_PATH);
    for (label, line) in [("dry-run", lines.dry), ("live", lines.live)] {
        let Some(line) = line else { continue };
        out.push(with_detail(
            gate::ban_str(
                &format!("{label} compose line still references {BAD_PATH}"),
                &bad,
                line,
            ),
            vec![line.to_string()],
        ));
    }

    Ok(out)
}

/// The two `cd '$TBD_REMOTE_DIR/apps/website/api_v2'` bans over the stripped source.
///
/// Not redundant with the `-f` pin: `cd api && docker compose -f docker-compose.staging.yml` puts
/// a plausible-looking relative filename in front of the wrong directory. The `-f` argument alone
/// cannot tell you which file compose opens; only the pair can. Two literals, because pinning
/// one quoting style pins nothing.
pub(super) fn ban_cd_into_api(stripped: &str) -> Vec<Verdict> {
    let base = source_basename();
    vec![
        gate::ban_str(
            &format!("{base} still cds into apps/website/api_v2 (compose must not)"),
            &Pattern::literal(CD_INTO_API_SQ),
            stripped,
        ),
        gate::ban_str(
            &format!("{base} still cds into apps/website/api_v2 (double-quoted form)"),
            &Pattern::literal(CD_INTO_API_DQ),
            stripped,
        ),
    ]
}

/// The two on-disk assertions: the good compose file exists, the stale one does not.
///
/// Both `Failed`, never `DidNotRun`: here the file's *existence* IS the assertion, so a missing
/// compose file is a check that ran and found a violation. (Contrast the missing deploy driver in
/// [`verify_staging_compose_paths`], where absence blinds the gate and "did not run" is the honest
/// answer.) The stale path is tested for ANY entry, not just a file: a *directory* left there is
/// just as much a leftover, and the wider test suits a ban.
pub(super) fn compose_files_on_disk(repo_root: &Path) -> Vec<Verdict> {
    let mut out = Vec::new();
    if !repo_root.join(GOOD_PATH).is_file() {
        out.push(Verdict::failed(format!("missing {GOOD_PATH}")));
    }
    // `symlink_metadata`, not `exists()`: `exists()` follows symlinks, so a *dangling* symlink at
    // the stale path would report absent. Symlinking the old location back is exactly how someone
    // unblocks a one-off deploy, so it must trip the ban. This cannot change the verdict on any
    // tree where the path is a real file or genuinely absent.
    if repo_root.join(BAD_PATH).symlink_metadata().is_ok() {
        out.push(Verdict::failed(format!(
            "unexpected {BAD_PATH} (stale path)"
        )));
    }
    out
}

/// The audited source's filename, as the two `cd` messages quote it. Derived from
/// [`DEPLOY_SOURCE`] so a
/// repoint cannot leave the prose naming a file that no longer exists.
pub(super) fn source_basename() -> &'static str {
    match DEPLOY_SOURCE.rsplit_once('/') {
        Some((_, base)) => base,
        None => DEPLOY_SOURCE,
    }
}

/// A multi-line finding: headline plus six-space-indented continuations, which is how
/// [`Finding`] renders.
pub(super) fn detailed(headline: &str, detail: Vec<String>) -> Verdict {
    Verdict::Failed(Finding {
        headline: headline.to_string(),
        detail,
    })
}

/// Attach continuation lines to a verdict the library decided. Keeping the DECISION in `gate::*`
/// and only the PROSE here is the point — a hand-rolled `if line.contains(BAD_PATH)` would
/// re-open exactly the fail-quiet hole `verification_core` exists to close. `DidNotRun` passes through
/// untouched: its detail already names the cause, and a hint about the compose line would mislead
/// when nothing was read.
pub(super) fn with_detail(verdict: Verdict, detail: Vec<String>) -> Verdict {
    match verdict {
        Verdict::Held => Verdict::Held,
        Verdict::Failed(mut finding) => {
            finding.detail = detail;
            Verdict::Failed(finding)
        }
        Verdict::DidNotRun(cause, finding) => Verdict::DidNotRun(cause, finding),
    }
}
