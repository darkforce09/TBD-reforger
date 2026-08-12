//! T-438 / T-461 — the `deploy-staging.sh` compose-path pin (T-853 port of
//! `scripts/mod/verify-t438-deploy-staging-compose-path.sh`).
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Staging deploys must point `docker compose -f` at `apps/website/docker-compose.staging.yml`
//! (T-251), never at the `apps/website/api/` sibling that used to live there. Getting it wrong
//! does not fail loudly — compose happily starts *a* stack from *a* file, so the deploy goes green
//! and staging quietly runs the wrong topology. Hence a static pin rather than a smoke test.
//!
//! T-461 (wave 23 adversarial) found the previous Class-R false-green, in the script's own words:
//!
//! > (1) a `//` / `#` comment containing the good path counted as presence;
//! > (2) only one exact `cd '$TBD_REMOTE_DIR/apps/website/api'` string was banned, so live could
//! >     use api/compose while dry-run stayed good (or good path lived only in a comment).
//!
//! So the gate strips comments first, then requires the good `-f` path on **both** the dry-run
//! `echo` and the live `ssh_cmd` line, requires those two to agree with each other, and separately
//! rejects the stale api/ path on either. "Dry-run says one thing, live does another" is the
//! failure this shape exists to catch: a dry run is the only rehearsal anyone gets.
//!
//! OWNS WIDEN (from the script): wave_plan T-438 lists only `scripts/mod/deploy-staging.sh`; this
//! is the Class-R perturbation guard for that path contract. T-461 owns the script hardening.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`.** The bash delegated comment-stripping and quote-safe `-f` extraction to a
//!    heredoc'd Python program — the single reason this gate is on `scripts/python-inventory.txt`.
//!    [`strip_shell_comments`] and [`f_path`] are that program in Rust, unit-tested, which a
//!    heredoc never could be. **That inventory line can go**; nothing here shells out at all.
//! 2. **A fail-quiet that printed nothing.** `pin_out="$(… python3 …)" || pin_rc=$?` captured only
//!    stdout, so every way Python could die before its own `print`s — absent interpreter (127),
//!    non-UTF-8 input, an unreadable file — left `pin_out` EMPTY: the operator got `verify-…:
//!    FAIL` with **zero diagnosis on stdout** and a traceback on stderr that `wave.sh`'s `tail
//!    -15` may or may not have reached. Closed on *status*, the important half, but mute. Those
//!    states are named [`NotRun`] causes now, printed with the rest of the report.
//! 3. **`pin_rc` observable only as "non-zero".** The script header claims `2 = internal error`,
//!    but the Python never exits 2 — an unhandled exception exits 1 — so "found a violation" and
//!    "never examined the file" were indistinguishable. They are [`Verdict::Failed`] and
//!    [`Verdict::DidNotRun`] here, and the distinction survives even though the status stays 0/1.
//!
//! It does NOT remove any `2>/dev/null` or `|| true`: this script had none, and it stats `$FILE`
//! before reading it. Those fail-open shapes were already absent; claiming otherwise is theatre.
//!
//! ── OUTPUT AND STATUS ARE A CONTRACT ─────────────────────────────────────────────────────────
//!
//! `wave.sh`'s `run()` captures `"$@" 2>&1` and prints `tail -15` of a failed step, so every line
//! below is operator-facing evidence. Acceptance was a byte-for-byte stdout+stderr+rc diff against
//! the script on a clean tree and on 13 perturbed throwaway roots (see `tests::bites`). Status
//! stays bash's binary 0/1 — see [`verify_t438`].
//!
//! ── WHEN `deploy-staging.sh` ITSELF IS PORTED ────────────────────────────────────────────────
//!
//! T-853 ports `scripts/mod/deploy-staging.sh` later in the same program, and the file this gate
//! reads then stops being shell. **Everything about *what* is pinned lives in the consts below** —
//! [`DEPLOY_SCRIPT`], [`GOOD_PATH`], [`BAD_PATH`], [`CD_INTO_API_SQ`], [`CD_INTO_API_DQ`],
//! [`DRY_RUN_KEY`], [`LIVE_KEY`] — and every message is `format!`ed from them. Repointing:
//! change [`DEPLOY_SCRIPT`]; if the new host is not shell, replace [`strip_shell_comments`] and
//! the two `cd` bans — a Rust deploy driver has no `cd` line to ban, it has a
//! `Command::current_dir`, and the ban should follow it there rather than be deleted.

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use tbd_gate::{Finding, Kind, NotRun, Pattern, Verdict, gate};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// Printed on the PASS/FAIL summary line: the script's filename stem verbatim, because operator
/// logs and `cargo xtask verify t438` transcripts are grepped for exactly this string.
const GATE_NAME: &str = "verify-t438-deploy-staging-compose-path";

/// The deploy driver under inspection, repo-relative.
///
/// T-853 REPOINTED THIS, which is what the module docs below said would happen and why every
/// message is `format!`ed from a const. `scripts/mod/deploy-staging.sh` became
/// `cargo xtask deploy staging`; the two `docker compose -f` invocations that carry the T-438
/// contract live in the transport module. The gate went RED on the deletion first — that was the
/// intended alarm, and it fired.
const DEPLOY_SCRIPT: &str = "xtask/src/deploy_staging/remote.rs";

/// The one true compose file (T-251). Double duty exactly as in the bash: the string that must
/// follow `-f`, **and** — joined onto the repo root — the file that must exist. The script spelled
/// it twice (`$COMPOSE`, `$GOOD_PATH`); two spellings of one contract drift, so there is one here.
const GOOD_PATH: &str = "apps/website/docker-compose.staging.yml";

/// The stale pre-T-251 location. Must appear on neither compose line and must not exist on disk —
/// a file left there is what makes the wrong `-f` path a *plausible* edit rather than an obvious
/// typo, so the gate removes the temptation as well as the reference.
const BAD_PATH: &str = "apps/website/api/docker-compose.staging.yml";

/// Banned outright: `cd`-ing the remote shell into `api/` before compose. Both quotings, because
/// T-461's finding was that banning one exact string is banning nothing.
const CD_INTO_API_SQ: &str = "cd '$TBD_REMOTE_DIR/apps/website/api'";
/// The double-quoted twin of [`CD_INTO_API_SQ`].
const CD_INTO_API_DQ: &str = r#"cd "$TBD_REMOTE_DIR/apps/website/api""#;

/// How the dry-run compose line is recognised: the script prints its plan with this prefix.
const DRY_RUN_KEY: &str = "[dry-run]";
/// How the live compose line is recognised: the remote-exec helper it is handed to.
/// Names the live dispatch in operator-facing messages only — it is NOT a matcher any more.
///
/// T-853: was `ssh_cmd`, bash's one-line ssh wrapper, and `line.contains(LIVE_KEY)` was how the
/// live compose invocation was found. The Rust equivalent is [`Runner::ssh_ok`], whose call spans
/// lines, so classification moved to "does this compose line carry the dry-run marker". This const
/// survives so the failure text still tells the reader WHICH invocation is wrong.
const LIVE_KEY: &str = "ssh_ok";

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
fn audit(repo_root: &Path) -> Result<Vec<Verdict>> {
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
fn strip_shell_comments(text: &str) -> String {
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

/// The two compose invocations, classified out of the stripped source.
///
/// `None` is itself a finding: the script treated a *missing* dry-run or live compose line as a
/// failure, not as vacuously satisfied. T-556 anti-vacuity applied to the gate's own inputs — a
/// gate that goes quiet because the thing it audits was deleted checks nothing.
struct ComposeLines<'a> {
    dry: Option<&'a str>,
    live: Option<&'a str>,
}

/// python: the `for raw in stripped.splitlines()` classification loop.
///
/// LAST ONE WINS, faithfully: `dry_line`/`live_line` were plain assignments, so a second dry-run
/// compose line later in the file silently replaced the first. Also faithful: a `docker compose -f`
/// line mentioning neither key is ignored, so a *third* invocation could use the stale path
/// unchallenged. Both are real holes; neither is this port's to close, because closing them
/// changes the verdict on trees the baseline calls clean. Flagged for whoever owns T-438 next.
fn classify(stripped: &str) -> Result<ComposeLines<'_>> {
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
fn f_regex() -> Result<Regex> {
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
fn f_path<'a>(re: &Regex, line: &'a str) -> Option<&'a str> {
    let caps = re.captures(line)?;
    (1..=3).find_map(|g| caps.get(g)).map(|m| m.as_str())
}

/// The Python pin proper: every message it could print, in its exact order.
///
/// Order is load-bearing — each message assumes the ones above it are shown too. "The paths
/// diverge" only reads correctly beside the two "must be …" lines, and a tree where dry-run and
/// live are each wrong in a different way emits all three.
fn pin_compose_lines(stripped: &str) -> Result<Vec<Verdict>> {
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
fn ban_cd_into_api(stripped: &str) -> Vec<Verdict> {
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
fn compose_files_on_disk(repo_root: &Path) -> Vec<Verdict> {
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
fn script_basename() -> &'static str {
    match DEPLOY_SCRIPT.rsplit_once('/') {
        Some((_, base)) => base,
        None => DEPLOY_SCRIPT,
    }
}

/// A multi-line finding: headline plus six-space-indented continuations, which is both
/// [`Finding`]'s rendering and the Python's `print(f"      {line}")`.
fn detailed(headline: &str, detail: Vec<String>) -> Verdict {
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
fn with_detail(verdict: Verdict, detail: Vec<String>) -> Verdict {
    match verdict {
        Verdict::Held => Verdict::Held,
        Verdict::Failed(mut finding) => {
            finding.detail = detail;
            Verdict::Failed(finding)
        }
        Verdict::DidNotRun(cause, finding) => Verdict::DidNotRun(cause, finding),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A scratch repo root that cleans itself up. Same shape as `gate_t444`'s, and for the same
    /// reason: a handful of tests do not justify a dev-dependency. `good()` lays down a tree that
    /// satisfies the contract, so every test below perturbs exactly one thing.
    struct Tree(PathBuf);

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl Tree {
        fn new(name: &str) -> Tree {
            let p = std::env::temp_dir().join(format!("t438-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Tree(p)
        }

        fn good(name: &str) -> Tree {
            let t = Tree::new(name);
            t.write(DEPLOY_SCRIPT, GOOD_SCRIPT);
            t.write(GOOD_PATH, "services: {}\n");
            t
        }

        fn write(&self, rel: &str, body: &str) {
            let path = self.0.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }

        /// Exactly what [`verify_t438`] prints above its summary line.
        fn text(&self) -> String {
            audit(&self.0)
                .unwrap()
                .iter()
                .filter(|v| !matches!(v, Verdict::Held))
                .map(|v| format!("{v}\n"))
                .collect()
        }
    }

    /// The live `deploy-staging.sh` compose block, comment included — the comment is the point: it
    /// names the good path, so a gate grepping the raw file would pass on the comment alone.
    const GOOD_SCRIPT: &str = r#"echo "==> docker compose (API + Postgres)"
# T-438: compose file lives at apps/website/docker-compose.staging.yml (T-251),
# not under apps/website/api/. Match `cargo xtask deploy website`.
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] cd \$TBD_REMOTE_DIR && docker compose -f apps/website/docker-compose.staging.yml up -d --build"
else
  ssh_cmd "cd '$TBD_REMOTE_DIR' && docker compose -f apps/website/docker-compose.staging.yml up -d --build"
fi
"#;

    /// Rewrite the deploy script in a good tree, then require every `want` substring in the report
    /// AND bash's exit 1.
    ///
    /// T-556 anti-vacuity: one green line is no evidence, so the only thing that makes a Class-R
    /// gate worth having is that each arm still bites. Every arm below was ALSO diffed
    /// byte-for-byte (stdout+stderr+rc) against the bash script it replaces, on a throwaway root.
    fn bites(name: &str, script: &str, want: &[&str]) {
        let t = Tree::good(name);
        t.write(DEPLOY_SCRIPT, script);
        let text = t.text();
        for w in want {
            assert!(text.contains(w), "[{name}] missing {w:?} in:\n{text}");
        }
        assert_eq!(verify_t438(&t.0).unwrap(), 1, "[{name}] must exit 1");
    }

    /// The live tree must satisfy the gate. When T-853 ports `deploy-staging.sh` this goes red
    /// first, which is the intended alarm: the pin needs repointing, not deleting.
    #[test]
    fn the_live_deploy_script_holds() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert_eq!(verify_t438(root).unwrap(), 0);
    }

    #[test]
    fn a_correct_script_holds() {
        assert_eq!(Tree::good("ok").text(), "");
        assert_eq!(verify_t438(&Tree::good("ok2").0).unwrap(), 0);
    }

    /// Six script-level perturbations, each of which the bash gate catches and so must this one.
    #[test]
    fn every_script_perturbation_bites() {
        // The compose path goes relative — the perturbation the gate's title names.
        bites(
            "relative",
            &GOOD_SCRIPT.replace(GOOD_PATH, "docker-compose.staging.yml"),
            &[
                "FAIL: dry-run -f path must be apps/website/docker-compose.staging.yml \
                 (got: docker-compose.staging.yml)",
                &format!(
                    "FAIL: live {LIVE_KEY} -f path must be apps/website/docker-compose.staging.yml \
                 (got: docker-compose.staging.yml)"
                ),
            ],
        );
        // T-461's exact finding: live regresses to api/ while the dry-run stays clean. Three
        // separate messages must fire — wrong path, divergence, stale reference.
        let live = format!("ssh_cmd \"cd '$TBD_REMOTE_DIR' && docker compose -f {GOOD_PATH}");
        bites(
            "live-api",
            &GOOD_SCRIPT.replace(&live, &live.replace(GOOD_PATH, BAD_PATH)),
            &[
                &format!(
                    "FAIL: live {LIVE_KEY} -f path must be apps/website/docker-compose.staging.yml \
                 (got: apps/website/api/docker-compose.staging.yml)"
                ),
                "FAIL: dry-run and live compose -f paths diverge:",
                "FAIL: live compose line still references \
                 apps/website/api/docker-compose.staging.yml",
            ],
        );
        // The wave-23 false-green: the good path present ONLY in `#` and `//` comments.
        bites(
            "comment-only",
            &format!("# docker compose -f {GOOD_PATH}\n// docker compose -f {GOOD_PATH}\n"),
            &[
                "FAIL: no dry-run docker compose -f line after comment strip",
                &format!("FAIL: no live {LIVE_KEY} docker compose -f line after comment strip"),
            ],
        );
        // The banned `cd`, each quoting on its own — T-461 hole (2).
        bites(
            "cd-sq",
            &format!("{GOOD_SCRIPT}{CD_INTO_API_SQ}\n"),
            &[&format!(
                "FAIL: {} still cds into apps/website/api (compose must not)",
                script_basename()
            )],
        );
        bites(
            "cd-dq",
            &format!("{GOOD_SCRIPT}{CD_INTO_API_DQ}\n"),
            &[&format!(
                "FAIL: {} still cds into apps/website/api (double-quoted form)",
                script_basename()
            )],
        );
        // `-f` with no argument: its own message, echoing the line at the Python's six-space indent.
        bites(
            "no-arg",
            "  echo \"[dry-run] docker compose -f\"\n  ssh_cmd \"docker compose -f\"\n",
            &[
                "FAIL: dry-run compose line has no parseable -f path:\n      \
                 echo \"[dry-run] docker compose -f\"",
                "FAIL: live compose line has no parseable -f path:\n      \
                 ssh_cmd \"docker compose -f\"",
            ],
        );
    }

    /// The on-disk pair: the compose file moved away (`-f`), and a leftover restored at the stale
    /// path (`-e`). Neither uses [`bites`] — one needs a tree that is deliberately not good.
    #[test]
    fn the_on_disk_compose_pair_bites() {
        let gone = Tree::new("no-compose");
        gone.write(DEPLOY_SCRIPT, GOOD_SCRIPT);
        assert_eq!(gone.text(), format!("FAIL: missing {GOOD_PATH}\n"));
        assert_eq!(verify_t438(&gone.0).unwrap(), 1);

        let stale = Tree::good("stale");
        stale.write(BAD_PATH, "services: {}\n");
        assert_eq!(
            stale.text(),
            format!("FAIL: unexpected {BAD_PATH} (stale path)\n")
        );
    }

    /// A missing `deploy-staging.sh` is a check that did not run — never a pass — and prints
    /// WITHOUT the trailing summary line, as bash's early `exit 1` did.
    #[test]
    fn a_missing_deploy_script_does_not_read_as_pass() {
        assert_eq!(verify_t438(&Tree::new("no-script").0).unwrap(), 1);
        assert_eq!(verify_t438(Path::new("/nonexistent/tbd-t438")).unwrap(), 1);
    }

    #[test]
    fn quoted_dash_f_arguments_lose_their_quotes() {
        let re = f_regex().unwrap();
        for (line, want) in [
            ("docker compose -f 'a b.yml' up", Some("a b.yml")),
            ("docker compose -f \"a b.yml\" up", Some("a b.yml")),
            ("docker compose -f a.yml up", Some("a.yml")),
            ("docker compose -f", None),
            // ODDITY PIN, not an endorsement (see [`f_path`]): the first `-f` on the line wins,
            // even when it belongs to some other command entirely.
            ("rm -f /tmp/x && docker compose -f good.yml", Some("/tmp/x")),
        ] {
            assert_eq!(f_path(&re, line), want, "{line}");
        }
    }

    #[test]
    fn comments_go_and_quoted_hashes_stay() {
        assert_eq!(strip_shell_comments("a # b\nc\n"), "a \nc\n");
        assert_eq!(strip_shell_comments("e '# no'\n"), "e '# no'\n");
        assert_eq!(strip_shell_comments("e \"# no\"\n"), "e \"# no\"\n");
        // The C-style arm — deliberate, and the unquoted-URL hazard it implies.
        assert_eq!(strip_shell_comments("x // y\nz\n"), "x \nz\n");
        assert_eq!(strip_shell_comments("curl https://h/p\n"), "curl https:\n");
    }

    /// ODDITY PIN, not an endorsement. See [`strip_shell_comments`]: POSIX says a backslash inside
    /// single quotes is literal, so the quote closes and `# gone` should be stripped. This machine
    /// believes the quote is still open and strips nothing after it. A future fix turns this red.
    #[test]
    fn a_backslash_before_a_closing_single_quote_swallows_the_rest() {
        assert_eq!(strip_shell_comments("a='x\\' # gone\n"), "a='x\\' # gone\n");
    }
}
