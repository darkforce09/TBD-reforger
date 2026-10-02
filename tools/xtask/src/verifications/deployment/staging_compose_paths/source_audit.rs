use super::*;

/// Entry point. `0` when the contract holds, `1` for every failure.
///
/// Deliberately NOT [`Verdict::into_exit`]'s three-way code: the wave gate and the `ci.yml`
/// `mod-gates-hosted` job both record pass/fail from this status, so a 2 for a broken checkout
/// would change what CI says. Widening the status is a decision for every gate at once.
///
/// The missing-source arm is the one place the output *shape* differs from every other failure:
/// it prints `FAIL: missing …` for each absent source and **no** summary line, because there was
/// nothing to summarise.
pub fn verify_staging_compose_paths(repo_root: &Path) -> Result<u8> {
    // A missing source is `FAIL: missing <path>`, then exit 1.
    //
    // Hand-built rather than leaning on `gate::require`'s missing-target rendering, so the message
    // names this gate's subject rather than a generic pin. The CAUSE is still the typed one, so a
    // caller matching the verdict sees `DidNotRun`, not a violation.
    let mut blind = false;
    for source in [WEBSITE_DEPLOY_SOURCE, STAGING_DEPLOY_PIPELINE] {
        let source_path = repo_root.join(source);
        if !source_path.is_file() {
            let absent = Verdict::DidNotRun(
                NotRun::TargetMissing(source_path.clone()),
                Finding {
                    headline: format!("missing {}", source_path.display()),
                    detail: Vec::new(),
                },
            );
            println!("{absent}");
            blind = true;
        }
    }
    if blind {
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

/// Every check, in the order the report prints them: the website deploy's compose pins and its
/// `cd` ban, the game server deploy's compose ban per source, then the two on-disk file checks.
///
/// Split out from [`verify_staging_compose_paths`] so the contract is testable against a scratch
/// tree without capturing stdout, and returning a LIST rather than stopping at the first failure:
/// an operator who has moved the compose file wants every place the move was missed in one run,
/// not one more place per re-run.
pub(super) fn audit(repo_root: &Path) -> Result<Vec<Verdict>> {
    let mut out = Vec::new();

    match read_source(repo_root, WEBSITE_DEPLOY_SOURCE) {
        Ok(source) => {
            let stripped = strip_comments(&source);
            out.extend(pin_compose_lines(&stripped)?);
            out.push(ban_cd_into_api(&stripped)?);
        }
        Err(unread) => out.push(unread),
    }
    match staging_deploy_sources(repo_root) {
        Ok(sources) => {
            for (source, text) in sources {
                let stripped = strip_comments(&text);
                out.push(ban_compose_in_the_game_server_deploy(&source, &stripped)?);
            }
        }
        Err(unread) => out.push(unread),
    }
    // Checked whatever happened above: the operator should still learn whether the compose file
    // is where it belongs.
    out.extend(compose_files_on_disk(repo_root));
    Ok(out)
}

/// Every production source of the game server deploy, repo-relative and in path order, with its
/// text: `<module>.rs` when present and each `.rs` file under the module folder outside a `tests/`
/// folder. An absent [`STAGING_DEPLOY_PIPELINE`] or an unreadable file is the "did not run" verdict
/// that names it, so a moved deploy never reads as a deploy without compose.
pub(super) fn staging_deploy_sources(repo_root: &Path) -> Result<Vec<(String, String)>, Verdict> {
    let pipeline = repo_root.join(STAGING_DEPLOY_PIPELINE);
    if !pipeline.is_file() {
        return Err(Verdict::did_not_run(
            format!("missing {}", pipeline.display()),
            Kind::Pin,
            NotRun::TargetMissing(pipeline),
        ));
    }
    let unlisted = |folder: &Path, cause: std::io::Error| {
        Verdict::did_not_run(
            format!("cannot list {}", folder.display()),
            Kind::Pin,
            NotRun::Unreadable {
                path: folder.to_path_buf(),
                source: cause,
            },
        )
    };
    let mut sources = Vec::new();
    let module_file = format!("{STAGING_DEPLOY_MODULE}.rs");
    if repo_root.join(&module_file).is_file() {
        sources.push(module_file);
    }
    let mut folders = vec![repo_root.join(STAGING_DEPLOY_MODULE)];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).map_err(|cause| unlisted(&folder, cause))? {
            let path = entry.map_err(|cause| unlisted(&folder, cause))?.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "tests") {
                    folders.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && let Ok(relative) = path.strip_prefix(repo_root)
            {
                sources.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    sources.sort();
    sources
        .into_iter()
        .map(|source| read_source(repo_root, &source).map(|text| (source, text)))
        .collect()
}

/// One audited source's text, or the "did not run" verdict that names why it could not be read.
///
/// An unreadable source is a NAMED cause printed with the report, never a bare non-zero status:
/// a gate whose subject it could not open must not read as either a pass or an unexplained
/// failure. Status is unchanged (still 1).
fn read_source(repo_root: &Path, source: &str) -> Result<String, Verdict> {
    let source_path = repo_root.join(source);
    std::fs::read_to_string(&source_path).map_err(|cause| {
        Verdict::did_not_run(
            format!("cannot read {}", source_path.display()),
            Kind::Pin,
            NotRun::Unreadable {
                path: source_path,
                source: cause,
            },
        )
    })
}

/// The comment stripper every compose check runs on.
///
/// WHY IT EXISTS: matching the raw source for the good path counts a *comment* mentioning it as
/// presence, and a comment naming the compose file typically sits two lines above the real
/// invocation. A gate that cannot tell the contract being honoured from the contract being
/// *described* is not a gate. Everything downstream runs on the stripped text.
///
/// It reads BOTH comment syntaxes — `//` for the Rust sources, `#` for the remote shell text those
/// sources embed — from one pass, so a comment in either layer is stripped.
///
/// ODDITIES CARRIED ON PURPOSE — this is neither a Rust nor a shell parser and must not become
/// one:
///
/// * **`#` opens a comment anywhere outside quotes**, not only at a word boundary. Real `sh` reads
///   `foo#bar` as one word; this eats `#bar`. Harmless: the machine only ever *removes* text, so
///   it can only make a pin stricter and a ban looser by what a comment held.
/// * **`//` opens a comment outside quotes**, so a `//` comment in the audited source is stripped
///   too. The hazard it brings is an unquoted URL: `https://host/x` loses everything from `//`
///   on. Unreachable while the audited sources quote their URLs; worth knowing before one is
///   added.
/// * **A backslash escapes inside single quotes.** POSIX says it does not — `'a\'` is the two-char
///   string `a\`. This consumes `\'` as a pair, stays `in_squote`, and therefore stops stripping
///   comments for the whole rest of the file, which would let a `#`-commented good path count as
///   presence again: exactly the hole this stripper exists to close. **Latent bug, carried
///   knowingly**, pinned by
///   `tests::a_backslash_before_a_closing_single_quote_swallows_the_rest` so a fix is a deliberate
///   act with a red test rather than an accident.
/// * **No heredoc, `$'…'`, raw-string or lifetime awareness.** A heredoc block is walked as
///   ordinary text, and a Rust lifetime's lone `'` opens a quote; the audited sources hold
///   neither.
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

/// Every line of the stripped source that runs compose ([`COMPOSE_COMMAND`]), trimmed, in order.
pub(super) fn compose_lines(stripped: &str) -> Result<Vec<&str>> {
    // Matched per line, so `Pattern`'s multi-line anchoring is moot here — it is used for the
    // compiled-in matcher, not the anchors.
    let compose = Pattern::regex(COMPOSE_COMMAND)?;
    let mut lines = Vec::new();
    for raw in stripped.lines() {
        let line = raw.trim();
        // `probe_str`, not a bare `is_match`: the `?` stops a future fallible matcher from
        // silently reading as "no match".
        if gate::probe_str(&compose, line).map_err(|cause| anyhow::anyhow!("{cause:?}"))? {
            lines.push(line);
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
/// ODDITY CARRIED KNOWINGLY: the search is not tied to the compose occurrence, so it takes the
/// FIRST `-f <arg>` anywhere on the line — `rm -f /tmp/x && docker compose -f good.yml` is judged
/// on `/tmp/x`. Unreachable on the audited source today, pinned in `tests`.
pub(super) fn f_path<'a>(re: &Regex, line: &'a str) -> Option<&'a str> {
    let caps = re.captures(line)?;
    (1..=3).find_map(|g| caps.get(g)).map(|m| m.as_str())
}

/// The pin proper over the website deploy's source: at least one compose command, and every one
/// names [`GOOD_PATH`] after `-f` and [`BAD_PATH`] nowhere.
///
/// A line gets every message that applies to it, in this order: no parseable `-f`, then the wrong
/// path, then the reference to the API folder's file.
pub(super) fn pin_compose_lines(stripped: &str) -> Result<Vec<Verdict>> {
    let lines = compose_lines(stripped)?;
    let f_re = f_regex()?;
    let base = source_basename(WEBSITE_DEPLOY_SOURCE);
    let mut out = Vec::new();

    // An absent compose command is a failure, never a vacuous pass: a gate that goes quiet
    // because the thing it audits was deleted checks nothing.
    if lines.is_empty() {
        out.push(Verdict::failed(format!(
            "no compose command in {base} after comment strip"
        )));
    }

    let bad = Pattern::literal(BAD_PATH);
    for line in lines {
        match f_path(&f_re, line) {
            // A compose command whose `-f` has no argument, or no `-f` at all. Kept distinct from
            // "wrong path" because the fix differs: a truncated edit, not a wrong destination.
            None => out.push(detailed(
                "compose line has no parseable -f path:",
                vec![line.to_string()],
            )),
            // THE HEADLINE CONTRACT. Equality, not a substring test: a relative
            // `docker-compose.staging.yml`, a `../`-prefixed one and the API folder's file are all
            // simply "not GOOD_PATH". Each reports the value actually found, so the operator can
            // see which edit went wrong.
            Some(path) if path != GOOD_PATH => out.push(Verdict::failed(format!(
                "compose -f path must be {GOOD_PATH} (got: {path})"
            ))),
            Some(_) => {}
        }
        // Belt and braces over the equality check above, which compares the extracted `-f`
        // argument; this rejects the API folder's file ANYWHERE on the line: in an `--env-file`,
        // in a second `-f` (compose accepts overlays, and the later file wins for conflicting
        // keys), or in a `cd` sharing the line. `gate::ban_str` so the decision stays in the
        // library and only the prose is local.
        out.push(with_detail(
            gate::ban_str(
                &format!("compose line still references {BAD_PATH}"),
                &bad,
                line,
            ),
            vec![line.to_string()],
        ));
    }

    Ok(out)
}

/// The ban on a `cd` into `apps/website/api_v2` in the website deploy's source ([`CD_INTO_API`]).
///
/// Not redundant with the `-f` pin: `cd api && docker compose -f docker-compose.staging.yml` puts
/// a plausible-looking relative filename in front of the wrong directory. The `-f` argument alone
/// cannot tell you which file compose opens; only the pair can.
pub(super) fn ban_cd_into_api(stripped: &str) -> Result<Verdict> {
    Ok(gate::ban_str(
        &format!(
            "{} cds into apps/website/api_v2 (compose runs from the checkout root)",
            source_basename(WEBSITE_DEPLOY_SOURCE)
        ),
        &Pattern::regex(CD_INTO_API)?,
        stripped,
    ))
}

/// The game server deploy runs no compose command: every one in the stripped text of `source`, a
/// repo-relative path, is reported, each on its own continuation line.
pub(super) fn ban_compose_in_the_game_server_deploy(
    source: &str,
    stripped: &str,
) -> Result<Verdict> {
    let lines = compose_lines(stripped)?;
    if lines.is_empty() {
        return Ok(Verdict::Held);
    }
    Ok(detailed(
        &format!(
            "{source} runs compose; the staging compose stack belongs to cargo xtask deploy website:"
        ),
        lines.into_iter().map(str::to_string).collect(),
    ))
}

/// The two on-disk assertions: the good compose file exists, the API folder's one does not.
///
/// Both `Failed`, never `DidNotRun`: here the file's *existence* IS the assertion, so a missing
/// compose file is a check that ran and found a violation. (Contrast a missing deploy source in
/// [`verify_staging_compose_paths`], where absence blinds the gate and "did not run" is the honest
/// answer.) The API folder's path is tested for ANY entry, not just a file: a *directory* left
/// there is just as much a second compose location, and the wider test suits a ban.
pub(super) fn compose_files_on_disk(repo_root: &Path) -> Vec<Verdict> {
    let mut out = Vec::new();
    if !repo_root.join(GOOD_PATH).is_file() {
        out.push(Verdict::failed(format!("missing {GOOD_PATH}")));
    }
    // `symlink_metadata`, not `exists()`: `exists()` follows symlinks, so a *dangling* symlink at
    // the API folder's path would report absent. A symlink there is exactly how someone points an
    // old command line at the new file, so it must trip the ban. This cannot change the verdict
    // on any tree where the path is a real file or genuinely absent.
    if repo_root.join(BAD_PATH).symlink_metadata().is_ok() {
        out.push(Verdict::failed(format!(
            "unexpected {BAD_PATH}: the staging compose file is {GOOD_PATH} alone"
        )));
    }
    out
}

/// An audited source's filename, as the messages quote it. Derived from the path constant so a
/// repoint cannot leave the prose naming a file that no longer exists.
pub(super) fn source_basename(source: &'static str) -> &'static str {
    match source.rsplit_once('/') {
        Some((_, base)) => base,
        None => source,
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
