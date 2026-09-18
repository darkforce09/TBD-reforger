use super::*;

/// Entry point. `0` when the contract holds, `1` for every failure — bash's binary status.
///
/// Deliberately NOT [`Verdict::into_exit`]'s three-way code. The script `exit 1`-ed for a missing
/// `deploy-staging.sh` just as it did for a wrong `-f` path, and `wave.sh`, `cargo xtask verify t438` and
/// `ci.yml mod-gates-hosted` all record pass/fail from that; returning 2 for a broken checkout
/// would change what CI says in the commit that was supposed to change nothing. Widening it is
/// T-853 Phase 7's call, made once for all gates.
///
/// The missing-script arm is the one place the output *shape* differs from every other failure:
/// bash `exit 1`-ed there before ever setting `FAIL`, so it printed `FAIL: missing …` and **no**
/// `verify-…: FAIL` summary. Faithfully reproduced.
pub fn verify_t438(repo_root: &Path) -> Result<u8> {
    let script = repo_root.join(DEPLOY_SCRIPT);

    // bash: `if [[ ! -f "$FILE" ]]; then echo "FAIL: missing $FILE"; exit 1; fi`
    //
    // Hand-built rather than leaning on `gate::require`'s missing-target rendering: the library's
    // text ("— target file missing: … / The pin could not run.") is better prose but is not what
    // the script printed, and byte-identical output is the acceptance criterion. The CAUSE is
    // still the typed one, so a caller matching the verdict sees `DidNotRun`, not a violation.
    if !script.is_file() {
        let absent = Verdict::DidNotRun(
            NotRun::TargetMissing(script.clone()),
            Finding {
                headline: format!("missing {}", script.display()),
                detail: Vec::new(),
            },
        );
        println!("{absent}");
        return Ok(1);
    }

    let mut failed = false;
    for verdict in &audit(repo_root)? {
        // `Verdict::Held` renders as the empty string; printing it would emit a blank line the
        // bash never did. Skipped explicitly rather than relying on Display happening to be empty.
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

/// Every check, in the order the script printed them: the Python pin's findings, then the two
/// on-disk file checks.
///
/// Split out from [`verify_t438`] so the contract is testable against a scratch tree without
/// capturing stdout — and returning a list rather than a first-failure because the script
/// accumulated. That is deliberate on its part: an operator who has moved the compose file wants
/// every place the move was missed in one run, not one more place per re-run.
pub(super) fn audit(repo_root: &Path) -> Result<Vec<Verdict>> {
    let script = repo_root.join(DEPLOY_SCRIPT);
    let mut out = Vec::new();

    // python: `src = open(path, encoding="utf-8").read()` — THE FAIL-QUIET THIS CLOSES. In the
    // script this sat inside a command substitution, so any exception produced a traceback on
    // stderr, an EMPTY `pin_out`, and a bare `verify-…: FAIL` with nothing on stdout explaining
    // it. Here it is a named cause printed with the report. Status unchanged (still 1).
    let source = match std::fs::read_to_string(&script) {
        Ok(text) => text,
        Err(source) => {
            out.push(Verdict::did_not_run(
                format!("cannot read {}", script.display()),
                Kind::Pin,
                NotRun::Unreadable {
                    path: script,
                    source,
                },
            ));
            // Fall through to the disk checks: bash ran them after a failed pin too, and the
            // operator should still learn whether the compose file is where it belongs.
            out.extend(compose_files_on_disk(repo_root));
            return Ok(out);
        }
    };

    let stripped = strip_shell_comments(&source);
    out.extend(pin_compose_lines(&stripped)?);
    out.extend(ban_cd_into_api(&stripped));
    out.extend(compose_files_on_disk(repo_root));
    Ok(out)
}

/// The Python `strip_shell_comments` state machine, transcribed character for character.
///
/// WHY IT EXISTS: T-461 hole (1). Grepping the raw file for the good path counted a *comment*
/// mentioning it as presence — and `deploy-staging.sh:1606` is exactly such a comment, two lines
/// above the real invocation ("T-438: compose file lives at apps/website/docker-compose.staging.yml
/// (T-251)"). The gate could not tell the contract being honoured from the contract being
/// *described*. Everything downstream runs on the stripped text.
///
/// ODDITIES PRESERVED ON PURPOSE — this is not a shell parser and must not become one, because its
/// output is load-bearing for the byte-for-byte diff:
///
/// * **`#` opens a comment anywhere outside quotes**, not only at a word boundary. Real `sh` reads
///   `foo#bar` as one word; this eats `#bar`. Harmless (the machine only ever *removes* text, so
///   it can only make a pin stricter) and it is what the baseline does.
/// * **`//` opens a comment outside quotes** — C syntax, not shell, put there because T-461's
///   finding mentioned `//` comments. The live hazard in a shell script is an unquoted URL:
///   `https://host/x` loses everything from `//` on. Unreachable in today's `deploy-staging.sh`
///   (its URLs are quoted); worth knowing before someone adds one.
/// * **A backslash escapes inside single quotes.** POSIX says it does not — `'a\'` is the two-char
///   string `a\`. This consumes `\'` as a pair, stays `in_squote`, and therefore stops stripping
///   comments for the whole rest of the file, which would let a `#`-commented good path count as
///   presence again: the very hole T-461 closed. **Latent bug, carried knowingly**, pinned by
///   `tests::a_backslash_before_a_closing_single_quote_swallows_the_rest` so a fix is a deliberate
///   act with a red test rather than an accident.
/// * **No heredoc, `$'…'` or line-continuation awareness.** `deploy-staging.sh` has several
///   `<<'EOF'` blocks, walked as ordinary text. Worst case a compose line hidden in a heredoc goes
///   unseen — and a compose line in a heredoc is not one this gate pins.
pub(super) fn strip_shell_comments(text: &str) -> String {
    // Indexed by code point, as Python's `text[i]` is — not by byte. The two agree on where the
    // ASCII delimiters are, but transcribing the indices faithfully keeps the equivalence obvious.
    let src: Vec<char> = text.chars().collect();
    let n = src.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    // Never both true: the only place either is set clears the other, so the merged in-quote arm
    // below is exactly the Python's two separate `if in_squote:` / `if in_dquote:` blocks.
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

/// python: the `for raw in stripped.splitlines()` classification loop.
///
/// LAST ONE WINS, faithfully: `dry_line`/`live_line` were plain assignments, so a second dry-run
/// compose line later in the file silently replaced the first. Also faithful: a `docker compose -f`
/// line mentioning neither key is ignored, so a *third* invocation could use the stale path
/// unchallenged. Both are real holes; neither is this port's to close, because closing them
/// changes the verdict on trees the baseline calls clean. Flagged for whoever owns T-438 next.
pub(super) fn classify(stripped: &str) -> Result<ComposeLines<'_>> {
    // `grep -E 'docker\s+compose\s+-f'` in engine form. Matched per line, so `Pattern`'s
    // multi-line anchoring is moot here — it is used for the compiled-in matcher, not the anchors.
    let compose = Pattern::regex(r"docker\s+compose\s+-f")?;
    let mut lines = ComposeLines {
        dry: None,
        live: None,
    };
    for raw in stripped.lines() {
        let line = raw.trim();
        // `probe_str`, not a bare `is_match`: this is a compound condition (match, THEN classify),
        // exactly the shape `gate_probe_str` existed for, and the `?` stops a future fallible
        // matcher from silently reading as "no match".
        if !gate::probe_str(&compose, line).map_err(|cause| anyhow::anyhow!("{cause:?}"))? {
            continue;
        }
        // T-853: classify by the PRESENCE or ABSENCE of the dry-run marker, not by a transport
        // spelling. bash put the whole thing on one line — `ssh_cmd "… docker compose -f …"` — so
        // `line.contains("ssh_cmd")` identified the live one. The Rust call spans several lines:
        // `runner.ssh_ok(` is on one, the composed command string on another. A per-line marker
        // therefore matches NEITHER, and the live line would have gone unclassified — a gate
        // silently checking one of the two paths it exists to check.
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

/// The quote-safe `-f` argument extractor —
/// python: `re.compile(r"""-f\s+(?:'([^']+)'|"([^"]+)"|(\S+))""")`.
///
/// Handles `-f 'a b.yml'`, `-f "a b.yml"` and bare `-f a.yml`, which is why the script reached for
/// Python rather than `awk`/`cut`: a path containing a space truncates under field splitting.
pub(super) fn f_regex() -> Result<Regex> {
    Ok(Regex::new(r#"-f\s+(?:'([^']+)'|"([^"]+)"|(\S+))"#)?)
}

/// python: `f_re.search(line)` + `next(g for g in m.groups() if g is not None)`.
///
/// `regex::Regex` directly rather than [`Pattern`]: this needs *captures*, and `Pattern` exposes
/// only `is_match`. No anchors are involved, so the `^`/`$` line-semantics trap `Pattern` exists
/// to prevent cannot arise here.
///
/// ODDITY PRESERVED: the search is not tied to the `docker compose` occurrence, so it takes the
/// FIRST `-f <arg>` anywhere on the line — `rm -f /tmp/x && docker compose -f good.yml` is judged
/// on `/tmp/x`. Latent, unreachable in today's script, pinned in `tests`.
pub(super) fn f_path<'a>(re: &Regex, line: &'a str) -> Option<&'a str> {
    let caps = re.captures(line)?;
    (1..=3).find_map(|g| caps.get(g)).map(|m| m.as_str())
}

/// The Python pin proper: every message it could print, in its exact order.
///
/// Order is load-bearing — each message assumes the ones above it are shown too. "The paths
/// diverge" only reads correctly beside the two "must be …" lines, and a tree where dry-run and
/// live are each wrong in a different way emits all three.
pub(super) fn pin_compose_lines(stripped: &str) -> Result<Vec<Verdict>> {
    let lines = classify(stripped)?;
    let f_re = f_regex()?;
    let mut out = Vec::new();

    // python: `if dry_line is None` / `if live_line is None`
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

    // python: "…has no parseable -f path:" — a line that matched `docker compose -f` but whose
    // `-f` has no argument. Kept distinct from "wrong path" because the fix differs: a truncated
    // edit, not a wrong destination.
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

    // python: `if dry_path != good` / `if live_path != good` — THE HEADLINE CONTRACT, and the
    // reason it is equality and not a substring test: a RELATIVE-ised `docker-compose.staging.yml`,
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

    // python: the divergence check — T-461 hole (2) in one comparison. Even if a future edit
    // relaxes what GOOD_PATH may be, the rehearsal and the real thing must never disagree: a dry
    // run describing a deploy nobody is about to perform is worse than no dry run at all.
    if let (Some(dry), Some(live)) = (dry_path, live_path)
        && dry != live
    {
        out.push(detailed(
            "dry-run and live compose -f paths diverge:",
            vec![format!("dry-run: {dry}"), format!("live:    {live}")],
        ));
    }

    // python: `if line and bad in line` — belt and braces over the equality checks above. Those
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

/// python: the two `cd '$TBD_REMOTE_DIR/apps/website/api'` bans over the stripped source.
///
/// Not redundant with the `-f` pin: `cd api && docker compose -f docker-compose.staging.yml` puts
/// a plausible-looking relative filename in front of the wrong directory. The `-f` argument alone
/// cannot tell you which file compose opens; only the pair can. Two literals, because T-461's
/// finding was that pinning one quoting style pins nothing.
pub(super) fn ban_cd_into_api(stripped: &str) -> Vec<Verdict> {
    let base = script_basename();
    vec![
        gate::ban_str(
            &format!("{base} still cds into apps/website/api (compose must not)"),
            &Pattern::literal(CD_INTO_API_SQ),
            stripped,
        ),
        gate::ban_str(
            &format!("{base} still cds into apps/website/api (double-quoted form)"),
            &Pattern::literal(CD_INTO_API_DQ),
            stripped,
        ),
    ]
}

/// bash: the `[[ ! -f "$COMPOSE" ]]` / `[[ -e "$STALE" ]]` pair.
///
/// Both `Failed`, never `DidNotRun`: here the file's *existence* IS the assertion, so a missing
/// compose file is a check that ran and found a violation. (Contrast the missing
/// `deploy-staging.sh` in [`verify_t438`], where absence blinds the gate and "did not run" is the
/// honest answer.) Note bash used `-e`, not `-f`, for the stale path — carried over deliberately:
/// a *directory* left at that path is just as much a leftover, and the wider test suits a ban.
pub(super) fn compose_files_on_disk(repo_root: &Path) -> Vec<Verdict> {
    let mut out = Vec::new();
    if !repo_root.join(GOOD_PATH).is_file() {
        out.push(Verdict::failed(format!("missing {GOOD_PATH}")));
    }
    // `symlink_metadata`, not `exists()`: `exists()` follows symlinks, so a *dangling* symlink at
    // the stale path would report absent. Symlinking the old location back is exactly how someone
    // unblocks a one-off deploy, so it must trip the ban. (bash's `-e` follows symlinks too; this
    // is the port's one deliberate strengthening, and it cannot change the verdict on any tree
    // where the path is a real file or genuinely absent.)
    if repo_root.join(BAD_PATH).symlink_metadata().is_ok() {
        out.push(Verdict::failed(format!(
            "unexpected {BAD_PATH} (stale path)"
        )));
    }
    out
}

/// The script's filename, as the two `cd` messages quote it. Derived from [`DEPLOY_SCRIPT`] so a
/// repoint cannot leave the prose naming a file that no longer exists.
pub(super) fn script_basename() -> &'static str {
    match DEPLOY_SCRIPT.rsplit_once('/') {
        Some((_, base)) => base,
        None => DEPLOY_SCRIPT,
    }
}

/// A multi-line finding: headline plus six-space-indented continuations, which is both
/// [`Finding`]'s rendering and the Python's `print(f"      {line}")`.
pub(super) fn detailed(headline: &str, detail: Vec<String>) -> Verdict {
    Verdict::Failed(Finding {
        headline: headline.to_string(),
        detail,
    })
}

/// Attach the Python's continuation lines to a verdict the library decided. Keeping the DECISION
/// in `gate::*` and only the PROSE here is the point — a hand-rolled `if line.contains(BAD_PATH)`
/// would re-open exactly the hole `tbd-gate` exists to close. `DidNotRun` passes through
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
