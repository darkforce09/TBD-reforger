//! Reclaim orphan build caches. THIS IS NOT OPTIONAL HOUSEKEEPING — it is the failure that stopped
//! this program dead once.
//!
//! OBSERVED 2026-07-26: the disk hit **252 MB free of 952 GB** mid-wave. Two gate steps failed with
//! "No space left on device", which reads exactly like a build error. `/var/tmp` held ~116 GB of
//! agent target dirs from slices that had already SHIPPED — every agent is told to remove its own
//! and many either forgot or were killed by a session limit before they could.
//!
//! Skips any dir belonging to a slice whose worktree still exists, so a live agent's cache
//! survives.
//!
//! T-426: gate-private dirs (`target-gate-*`, `dist-gate-*`) live at MAIN_ROOT, not `/var/tmp` —
//! ~15 GB class, expensive to rebuild, warm is valuable (T-421 measured cold 23.4 s vs warm 9.3 s
//! slice gate). Default reclaim does NOT touch them; opt in with `--gate-dirs`. Optional
//! `--gate-dirs-older-than-days N` only removes gate dirs whose directory mtime is older than N
//! days (age-based sweep without nuking a cache that was used today).
//!
//! T-589: PER-SLICE private dirs (`target-<SLICE>`, `target-<SLICE>-api`) ALSO live at MAIN_ROOT,
//! and until T-589 nothing reaped them at all. See the block inside for why they are swept BY
//! DEFAULT while T-426's gate set stayed opt-in — the two look alike and are opposites.

use std::path::{Path, PathBuf};

use super::Ctx;
use crate::{werr, wprintln};

/// `du -sm <path> | cut -f1` — megabytes, or `None` when du could not answer.
fn du_mb(p: &Path) -> Option<u64> {
    let out = std::process::Command::new("du")
        .arg("-sm")
        .arg(p)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .split('\t')
        .next()?
        .trim()
        .parse()
        .ok()
}

/// `${sz:-?}` — the size, or a literal `?` when du said nothing.
fn sz_or_q(v: Option<u64>) -> String {
    v.map(|n| n.to_string()).unwrap_or_else(|| "?".into())
}

/// `basename | tr 'A-Z' 'a-z' | tr -d '-'` — the comparison key both sweeps use.
fn key_of(name: &str) -> String {
    name.to_ascii_lowercase().replace('-', "")
}

/// bash's glob ordering key — LOCALE COLLATION, not byte order.
///
/// FOUND BY THE DIFF HARNESS, and it is exactly the kind of thing a port gets wrong silently.
/// A first cut sorted the expansion with `Vec::<PathBuf>::sort()`, i.e. bytewise, and produced
///
/// ```text
///   target-T-888  target-T-999  target-T-999-api  target-ci  target-dev-api
/// ```
///
/// where the bash produced
///
/// ```text
///   target-ci  target-dev-api  target-T-888  target-T-999  target-T-999-api
/// ```
///
/// Bash sorts pathname expansions with `strcoll()` under the active locale, and glibc's
/// `en_US.UTF-8` gives punctuation and case NO primary weight — so `target-ci` sorts before
/// `target-T-888` because `ci` < `t888` once the hyphens and the case are ignored. Bytewise, `T`
/// (0x54) sorts before `c` (0x63) and the order inverts.
///
/// This changes only the ORDER OF THE REPORT, never which directory is removed — the decisions are
/// per-entry. It is fixed anyway because the report is the only record an operator has of an
/// `rm -rf`, and because a diff that has to be explained away is a diff nobody checks next time.
///
/// The key is an approximation of the collation, not an implementation of it: alphanumerics only,
/// lowercased, with the raw name as the tie-break (which is glibc's later-pass behaviour for names
/// that differ only in case or punctuation). It agrees with `LC_ALL=en_US.UTF-8` bash on every name
/// this function can see. Under `LC_ALL=C` bash WOULD sort bytewise and this would differ — the
/// factory does not run under `LC_ALL=C`, and pinning the locale here would be a behaviour change
/// rather than a port.
fn collate_key(name: &str) -> (String, String) {
    (
        name.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect(),
        name.to_string(),
    )
}

/// Expand one shell glob against the filesystem, in the order bash's pathname expansion produces.
/// A pattern that matches nothing yields nothing, which is where the bash relied on
/// `[ -e "$d" ] || continue` to drop the unexpanded literal.
fn glob_dir(dir: &str, matches: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .filter(|e| matches(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect();
    v.sort_by_key(|p| {
        collate_key(
            &p.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
        )
    });
    v
}

/// `/var/tmp/t[0-9]*-probe` and friends — `t`, a digit, anything, then the suffix.
fn t_digit_suffix(name: &str, suffix: &str) -> bool {
    let Some(rest) = name.strip_prefix('t') else {
        return false;
    };
    rest.chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
        && name.ends_with(suffix)
}

pub fn cmd_reclaim(ctx: &Ctx, args: &[String]) -> u8 {
    let mut gate_dirs = false;
    // NOT an Option: the bash initialises it to the STRING "0", which is non-empty, and the
    // `${gate_min_age_days:+…}` in the gate-dirs header therefore ALWAYS expands. See below.
    let mut gate_min_age_days: i64 = 0;
    let mut slice_dirs = true;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--gate-dirs" => gate_dirs = true,
            "--gate-dirs-older-than-days" => {
                gate_dirs = true;
                // `${2:-0}` — a missing value is 0. A non-numeric one makes the later
                // `[ "$x" -gt 0 ]` error to stderr and take the false branch, i.e. no age filter;
                // parsing to 0 is the same behaviour without the stray diagnostic.
                gate_min_age_days = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 1;
            }
            "--no-slice-dirs" => slice_dirs = false,
            other => {
                werr!(
                    "reclaim: refusing unknown argument '{other}' (expected --gate-dirs, --gate-dirs-older-than-days N and/or --no-slice-dirs)"
                );
                return 2;
            }
        }
        i += 1;
    }

    // THE SPARED SET IS LOAD-BEARING, SO ITS ABSENCE MUST BE DISTINGUISHABLE FROM ITS EMPTINESS.
    // `for w in $(git worktree list | ...)` cannot tell "no other worktrees" (legitimately empty)
    // from "git did not answer" (unknown) — both leave $live empty, and the second one turns every
    // live slice's dir into an apparent orphan. For /var/tmp that has always been the standing
    // risk; for the MAIN_ROOT sweep below it would delete a running agent's build cache, so capture
    // the exit status and let that sweep refuse rather than guess.
    let wt_out = std::process::Command::new("git")
        .args(["worktree", "list"])
        .output();
    let (wt_list, mut live_ok) = match wt_out {
        Ok(o) if o.status.success() => (String::from_utf8_lossy(&o.stdout).into_owned(), true),
        _ => (String::new(), false),
    };
    if wt_list.is_empty() {
        live_ok = false;
    }
    let mut live: Vec<String> = Vec::new();
    for line in wt_list.lines().skip(1) {
        let first = line.split_whitespace().next().unwrap_or("");
        if first.is_empty() {
            continue;
        }
        let base = Path::new(first)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        live.push(key_of(&base));
    }
    // `${live:- none}` — `live` accumulates with a LEADING space, so the non-empty rendering is
    // "…(spared): t702 t212" and the empty one falls back to the literal " none".
    if live.is_empty() {
        wprintln!("live slices (spared): none");
    } else {
        wprintln!("live slices (spared): {}", live.join(" "));
    }

    let mut freed: u64 = 0;

    // ── /var/tmp: the agent caches this command was written for ─────────────────────────────────
    let mut var_tmp: Vec<PathBuf> = Vec::new();
    var_tmp.extend(glob_dir("/var/tmp", |n| n.contains("target")));
    var_tmp.extend(glob_dir("/var/tmp", |n| n.starts_with("v2-")));
    var_tmp.extend(glob_dir("/var/tmp", |n| t_digit_suffix(n, "-probe")));
    var_tmp.extend(glob_dir("/var/tmp", |n| t_digit_suffix(n, "-dist")));
    for d in var_tmp {
        if !d.exists() {
            continue;
        }
        let base = d
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let key = key_of(&base);
        // `case "$key" in "$l"*)` — a PREFIX match here, unlike the exact match the MAIN_ROOT
        // sweep uses. Preserved: /var/tmp names carry suffixes the ticket-anchored pattern below
        // does not have to parse.
        let skip = live.iter().any(|l| key.starts_with(l.as_str()));
        if skip {
            wprintln!("  spared  {}", d.display());
            continue;
        }
        let sz = du_mb(&d);
        if std::fs::remove_dir_all(&d).is_ok() {
            freed += sz.unwrap_or(0);
            wprintln!("  removed {:<44} {} MB", d.display(), sz_or_q(sz));
        }
    }

    // T-589 — PER-SLICE PRIVATE TARGET DIRS AT MAIN_ROOT. Swept by DEFAULT. Here is why, since the
    // sibling set two blocks down is deliberately opt-in and the two are easy to confuse.
    //
    // T-426 made target-gate-* opt-in for one reason: it is a WARM SHARED cache. Every future slice
    // gate hits target-gate-api (24 GB today), and T-421 measured cold 23.4 s vs warm 9.3 s — so
    // deleting it bills work that has not happened yet, to everyone, invisibly. That argument does
    // not survive translation to a target-<SLICE> dir, which is the opposite on every axis: exactly
    // one slice ever hits it, that slice is gone, and nothing will ever hit it again. Its entire
    // remaining function is to occupy disk. `target-T-454` did that for weeks at 2.7 GB while this
    // very command printed "reclaimed 0 MB" standing next to it (measured 2026-07-31), and the
    // volume runs at 87%. Opt-in housekeeping that nobody opts into is not housekeeping.
    // `--no-slice-dirs` turns it off for the one operator who wants a look before a sweep.
    //
    // The leak is also SELF-INFLICTED and structural, which is what makes "just tell agents to
    // clean up" insufficient: PLATFORM_FACTORY's Known traps and the brief template now INSTRUCT
    // every slice agent to build its own runnable binary into target-<slice>-api (T-581/T-582 were
    // served each other's binaries out of the shared target/). Agents are told to delete it and
    // mostly do — T-585 reclaimed 8.0 GB itself — but "mostly" is the wrong verb for a slice that
    // gets parked, rate-limited or killed mid-run, and those are the ones that leave a dir behind.
    //
    // SELECTION IS POSITIVE IDENTIFICATION, NOT A BLOCKLIST. A dir is removed only when its own
    // name says which slice owns it: `target-<TICKET>` with the ticket FIRST, optional suffix
    // after. A blocklist here fails open — the one unlisted name is the one that gets deleted — and
    // this function's blast radius is `rm -rf` on a directory. Measured at MAIN_ROOT today, three
    // dirs that a looser rule would have eaten:
    //     target/                  67 GB  the shared CARGO_TARGET_DIR for every worktree
    //     target-dev-api          3.6 GB  the operator's live `make api` cache — no ticket in name
    //     target-gate-schema-T422 1.7 GB  a GATE dir that CONTAINS a ticket id
    // The last one is why the ticket must be the first component after `target-`: anchoring there
    // means no target-gate-* name can be read as a slice dir even if the explicit exclusion below
    // were deleted. A name the pattern cannot parse (target-ci, target-T-068.13-api) is SPARED, not
    // guessed at — and printed with its size, because a silent skip is the same defect as the
    // "0 MB" report that produced this ticket, just wearing a quieter hat.
    let main_root = ctx.main_root.display().to_string();
    if !slice_dirs {
        wprintln!("slice dirs at {main_root}: not swept (--no-slice-dirs)");
    } else if !live_ok {
        wprintln!(
            "slice dirs at {main_root}: REFUSED — 'git worktree list' did not answer, so the spared set"
        );
        wprintln!(
            "  is unknown and every dir here would look like an orphan. Nothing swept. Run from the repo."
        );
    } else {
        let mut unknown_mb: u64 = 0;
        let mut unknown_n: u64 = 0;
        wprintln!("slice dirs at {main_root}:");
        let shared = ctx.main_root.join("target");
        let sz = du_mb(&shared);
        wprintln!(
            "  spared  {:<44} {} MB  (shared CARGO_TARGET_DIR — never reclaimed)",
            shared.display(),
            sz_or_q(sz)
        );
        for sd in glob_dir(&main_root, |n| n.starts_with("target-")) {
            if !sd.is_dir() {
                continue;
            }
            let sbase = sd
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            // `target` cannot come out of a target-* glob and target-gate-* cannot parse as a slice
            // dir; both arms are asserted anyway rather than reasoned about, because the cost of
            // being wrong once is 67 GB or a red gate, and the cost of the arm is a string compare.
            if sbase == "target" || sbase.starts_with("target-gate-") {
                continue;
            }
            // NOT a safety arm — the ticket-first rule below already spares this, and did so on its
            // own in this sweep's first real run. It is a REPORTING arm: target-dev-api is the
            // operator's live `make api` cache (Makefile:134,196), permanent by design, and the
            // generic line below filed it under "unparseable" and advised RENAMING IT so it could
            // be reaped. Naming one known-permanent dir is cheaper than printing that about the
            // cache behind the API the operator is using right now.
            if sbase == "target-dev-api" {
                let sz = du_mb(&sd);
                wprintln!(
                    "  spared  {:<44} {} MB  (operator dev API cache — permanent, Makefile owns it)",
                    sd.display(),
                    sz_or_q(sz)
                );
                continue;
            }
            let Some(stok) = slice_token(&sbase) else {
                let sz = du_mb(&sd);
                unknown_mb += sz.unwrap_or(0);
                unknown_n += 1;
                wprintln!(
                    "  spared  {:<44} {} MB  (name carries no ticket id — no owner to check)",
                    sd.display(),
                    sz_or_q(sz)
                );
                continue;
            };
            let skey = key_of(&stok);
            // EXACT match here, not the prefix match /var/tmp uses.
            if live.contains(&skey) {
                wprintln!("  spared  {:<44} (live slice {stok})", sd.display());
                continue;
            }
            let sz = du_mb(&sd);
            if std::fs::remove_dir_all(&sd).is_ok() {
                freed += sz.unwrap_or(0);
                wprintln!("  removed {:<44} {} MB", sd.display(), sz_or_q(sz));
            }
        }
        if unknown_n > 0 {
            wprintln!(
                "  {unknown_mb} MB in {unknown_n} unattributed dir(s) NOT reclaimed — reclaim removes only what it can attribute to a slice"
            );
        }
    }

    // T-742 — orphan ad-hoc private dirs under `$HOME/.cache/tbd-target-T-*`. Swept by default with
    // the same live-slice spare set as MAIN_ROOT/target-T-*. The shared cache
    // (`$HOME/.cache/tbd-target` with no ticket suffix) is NEVER touched. Agents must still delete
    // their own dir before reporting; this is the parked/killed-agent half.
    let home = std::env::var("HOME").unwrap_or_default();
    if !slice_dirs {
        wprintln!("adhoc dirs at {home}/.cache: not swept (--no-slice-dirs)");
    } else if !live_ok {
        wprintln!("adhoc dirs at {home}/.cache: REFUSED — live-slice set unknown; nothing swept.");
    } else {
        wprintln!("adhoc dirs at {home}/.cache:");
        let shared = PathBuf::from(format!("{home}/.cache/tbd-target"));
        let sz = du_mb(&shared);
        wprintln!(
            "  spared  {:<44} {} MB  (shared agent cache — never reclaimed)",
            shared.display(),
            sz_or_q(sz)
        );
        for cd in glob_dir(&format!("{home}/.cache"), |n| {
            n.starts_with("tbd-target-T-")
        }) {
            if !cd.is_dir() {
                continue;
            }
            let cbase = cd
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let Some(ctok) = adhoc_token(&cbase) else {
                let sz = du_mb(&cd);
                wprintln!(
                    "  spared  {:<44} {} MB  (name carries no ticket id)",
                    cd.display(),
                    sz_or_q(sz)
                );
                continue;
            };
            let ckey = key_of(&ctok);
            if live.contains(&ckey) {
                wprintln!("  spared  {:<44} (live slice {ctok})", cd.display());
                continue;
            }
            let sz = du_mb(&cd);
            if std::fs::remove_dir_all(&cd).is_ok() {
                freed += sz.unwrap_or(0);
                wprintln!("  removed {:<44} {} MB", cd.display(), sz_or_q(sz));
            }
        }
    }

    // ── the T-426 gate set, opt-in ─────────────────────────────────────────────────────────────
    let gate_glob = |root: &str| -> Vec<PathBuf> {
        let mut v = glob_dir(root, |n| n.starts_with("target-gate-"));
        v.extend(glob_dir(root, |n| n.starts_with("dist-gate-")));
        v
    };
    if gate_dirs {
        // PRESERVED ODDITY: `${gate_min_age_days:+, min age …}` tests for NON-EMPTY, and the
        // variable defaults to the string "0", so a bare `--gate-dirs` still prints ", min age 0d".
        wprintln!("gate dirs (--gate-dirs, min age {gate_min_age_days}d):");
        for d in gate_glob(&main_root) {
            if !d.exists() {
                continue;
            }
            if gate_min_age_days > 0 {
                let age_days = dir_age_days(&d);
                if age_days < gate_min_age_days {
                    wprintln!(
                        "  spared (age {age_days}d < {gate_min_age_days}d) {}",
                        d.display()
                    );
                    continue;
                }
            }
            let sz = du_mb(&d);
            if std::fs::remove_dir_all(&d).is_ok() {
                freed += sz.unwrap_or(0);
                wprintln!("  removed {:<44} {} MB", d.display(), sz_or_q(sz));
            }
        }
    } else {
        let mut gate_sz: u64 = 0;
        for gd in gate_glob(&main_root) {
            if !gd.exists() {
                continue;
            }
            gate_sz += du_mb(&gd).unwrap_or(0);
        }
        if gate_sz > 0 {
            wprintln!(
                "gate dirs at {main_root}: {gate_sz} MB not reclaimed (pass --gate-dirs to opt in)"
            );
        }
    }

    wprintln!("reclaimed {freed} MB — {} free", df_avail(&ctx.root));
    0
}

/// `^target-([Tt]-?[0-9]+)(-.*)?$` — the ticket must be the FIRST component after `target-`.
fn slice_token(base: &str) -> Option<String> {
    let rest = base.strip_prefix("target-")?;
    let mut it = rest.chars();
    let t = it.next()?;
    if t != 'T' && t != 't' {
        return None;
    }
    let mut idx = 1usize;
    if rest[idx..].starts_with('-') {
        idx += 1;
    }
    let digits: String = rest[idx..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        return None;
    }
    let tok_end = idx + digits.len();
    // `(-.*)?$` — the remainder must be empty or start with `-`. `target-T-068.13-api` therefore
    // does NOT parse, and is SPARED rather than guessed at.
    let tail = &rest[tok_end..];
    if !tail.is_empty() && !tail.starts_with('-') {
        return None;
    }
    Some(rest[..tok_end].to_string())
}

/// `^tbd-target-(T-[0-9]+)(-.*)?$` — note this one is uppercase-`T`-only and requires the dash.
fn adhoc_token(base: &str) -> Option<String> {
    let rest = base.strip_prefix("tbd-target-T-")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let tail = &rest[digits.len()..];
    if !tail.is_empty() && !tail.starts_with('-') {
        return None;
    }
    Some(format!("T-{digits}"))
}

/// `(( $(date +%s) - $(stat -c %Y "$d") ) / 86400)` — integer division, as the bash did it.
fn dir_age_days(d: &Path) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mtime = std::fs::metadata(d)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    (now - mtime) / 86400
}

/// `df -h "$ROOT" | tail -1 | awk '{print $4}'`.
fn df_avail(root: &Path) -> String {
    let out = std::process::Command::new("df")
        .arg("-h")
        .arg(root)
        .output();
    let Ok(o) = out else { return String::new() };
    let body = String::from_utf8_lossy(&o.stdout).into_owned();
    body.lines()
        .next_back()
        .and_then(|l| l.split_whitespace().nth(3))
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_identification_spares_everything_it_cannot_parse() {
        // The three dirs a looser rule would have eaten, measured at MAIN_ROOT.
        assert_eq!(slice_token("target-T-454"), Some("T-454".into()));
        assert_eq!(slice_token("target-T-582-api"), Some("T-582".into()));
        assert_eq!(slice_token("target-t742"), Some("t742".into()));
        assert_eq!(slice_token("target-dev-api"), None);
        assert_eq!(slice_token("target-ci"), None);
        // A GATE dir that CONTAINS a ticket id — anchoring at the first component is what makes
        // this unparseable even without the explicit target-gate-* exclusion.
        assert_eq!(slice_token("target-gate-schema-T422"), None);
        // `target-T-068.13-api` is not `(-.*)?$` after the digits.
        assert_eq!(slice_token("target-T-068.13-api"), None);
    }

    #[test]
    fn adhoc_pattern_is_uppercase_t_with_a_dash_only() {
        assert_eq!(adhoc_token("tbd-target-T-742"), Some("T-742".into()));
        assert_eq!(adhoc_token("tbd-target-T-742-x"), Some("T-742".into()));
        assert_eq!(adhoc_token("tbd-target-t742"), None);
        assert_eq!(adhoc_token("tbd-target-wave138-verify"), None);
        // The shared cache must never parse as a slice dir.
        assert_eq!(adhoc_token("tbd-target"), None);
    }

    #[test]
    fn key_matches_the_tr_pipeline() {
        assert_eq!(key_of("T-702"), "t702");
        assert_eq!(key_of("v2-target-T-1"), "v2targett1");
    }
}
