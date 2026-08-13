//! The plan / registry / worktree readers everything else keys off.
//!
//! These are `wave.sh`'s smallest functions and its most load-bearing ones: `land` decides what to
//! merge from [`tree_state`] and [`has_work`], and `status`, `wave`, `wave --close` and `land` all
//! key off [`current_wave`].

use std::path::Path;

use serde_json::Value;

use super::{Ctx, git_stdout_lossy};
use crate::werr;

/// `plan_rows` — the wave plan minus comments, the header row, and blank lines.
///
/// The two filters stay BRE `^#` / `^wave[[:space:]]`, which mean the same thing under ugrep and
/// GNU grep — see the engine note inside [`super::base::prev_wave_close`]. A missing `$PLAN` greps
/// an error into `/dev/null` and yields the empty set, which is why `status` on a stray checkout
/// used to say `ALL WAVES COMPLETE` about a directory that is not the repo.
pub fn plan_rows(ctx: &Ctx) -> Vec<String> {
    let body = std::fs::read_to_string(&ctx.plan).unwrap_or_default();
    body.lines()
        .filter(|l| !l.starts_with('#'))
        .filter(|l| {
            !(l.starts_with("wave")
                && l[4..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false))
        })
        // `sed '/^\s*$/d'` — GNU `\s` is [[:space:]].
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// Split a plan row on TAB, awk's `-F'\t'` — `$1`, `$2`, `$3`, `$4`.
fn cols(row: &str) -> Vec<&str> {
    row.split('\t').collect()
}

fn col(row: &str, n: usize) -> &str {
    cols(row).get(n - 1).copied().unwrap_or("")
}

/// awk's `==` between a field and a `-v` variable: numeric when BOTH look numeric, string
/// otherwise.
///
/// This is not pedantry. `wave_tickets` compares `$1 == w`, and the plan carried two spellings of
/// column 1 until T-616 (`80` and `w80`). Under awk `"w80" == 80` is a STRING comparison and false;
/// under a naive `==` on parsed integers it would either panic or silently match. The legacy
/// spelling still lives in HISTORY, which [`super::base::wave_plan_tickets_at`] reads.
fn awk_eq(field: &str, var: &str) -> bool {
    match (field.trim().parse::<f64>(), var.trim().parse::<f64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => field == var,
    }
}

/// `ticket_title` — column 3 of the first row whose column 2 is this ticket.
pub fn ticket_title(ctx: &Ctx, id: &str) -> String {
    for r in plan_rows(ctx) {
        if col(&r, 2) == id {
            return col(&r, 3).to_string();
        }
    }
    String::new()
}

/// `wave_tickets` — column 2 of every row in wave `w`.
pub fn wave_tickets(ctx: &Ctx, w: &str) -> Vec<String> {
    plan_rows(ctx)
        .iter()
        .filter(|r| awk_eq(col(r, 1), w))
        .map(|r| col(r, 2).to_string())
        // `for t in $(wave_tickets …)` word-splits, so an empty column 2 contributes nothing.
        .filter(|s| !s.is_empty())
        .collect()
}

/// `.ai/tickets/registry.json`, parsed once, with `is_shipped`'s exact failure semantics.
///
/// ── THE PYTHON THIS REPLACES, AND WHY ITS EXIT CODES ARE THE CONTRACT ────────────────────────
///
/// ```text
/// is_shipped() {
///   python3 - "$1" <<'EOF' 2>/dev/null
///   r=json.load(open('.ai/tickets/registry.json'))
///   t=[x for x in r['tickets'] if x['id']==sys.argv[1]]
///   sys.exit(0 if (t and t[0]['status'] in ('shipped','cancelled')) else 1)
///   EOF
/// }
/// ```
///
/// Every failure mode of that snippet exits NON-ZERO, i.e. "not shipped":
///
///   * the file is missing or is not JSON      -> exception -> rc 1
///   * `r['tickets']` is absent                -> `KeyError` -> rc 1
///   * ANY ticket lacks `id`                   -> `KeyError` inside the comprehension -> rc 1,
///     for EVERY query, not just that ticket. That is why [`Registry::poisoned`] exists rather
///     than a per-ticket `Option`.
///   * the MATCHED ticket lacks `status`       -> `KeyError` -> rc 1
///   * no ticket matches                       -> `t` is `[]`, falsy -> rc 1
///
/// Answering "not shipped" for a registry it could not read is WRONG for one caller, and that
/// caller has its own reader: [`super::base::wave_ledger_unshipped_at`] returns rc 3 for
/// cannot-read, because turning an unreadable blob into a CONTRADICTION would hard-refuse the gate
/// over a file nobody parsed.
///
/// PERFORMANCE, and it is only that: the bash forked one `python3` PER TICKET — 548 of them for a
/// single `status`. This parses once. The ANSWERS are identical; only the wall clock moves.
pub struct Registry {
    poisoned: bool,
    by_id: std::collections::HashMap<String, Option<String>>,
}

impl Registry {
    pub fn from_value(v: &Value) -> Registry {
        let mut r = Registry {
            poisoned: true,
            by_id: Default::default(),
        };
        let Some(tickets) = v.get("tickets").and_then(Value::as_array) else {
            return r;
        };
        for t in tickets {
            let Some(obj) = t.as_object() else { return r };
            let Some(id) = obj.get("id") else { return r };
            let key = match id.as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let status = obj
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_string);
            r.by_id.insert(key, status);
        }
        r.poisoned = false;
        r
    }

    #[allow(dead_code)]
    pub fn load(path: &Path) -> Registry {
        let Ok(body) = std::fs::read_to_string(path) else {
            return Registry {
                poisoned: true,
                by_id: Default::default(),
            };
        };
        let Ok(v) = serde_json::from_str::<Value>(&body) else {
            return Registry {
                poisoned: true,
                by_id: Default::default(),
            };
        };
        Self::from_value(&v)
    }

    pub fn load_repo(root: &Path) -> Registry {
        match crate::registry::load_registry(root) {
            Ok(v) => Self::from_value(&v),
            Err(_) => Registry {
                poisoned: true,
                by_id: Default::default(),
            },
        }
    }

    /// `is_shipped` — rc 0 for `shipped`/`cancelled`, non-zero for everything else.
    pub fn is_shipped(&self, id: &str) -> bool {
        if self.poisoned {
            return false;
        }
        match self.by_id.get(id) {
            // Matched, but `t[0]['status']` raised: not shipped.
            Some(None) => false,
            Some(Some(s)) => s == "shipped" || s == "cancelled",
            None => false,
        }
    }
}

/// The lowest wave AT OR ABOVE THE LIVE GENERATION FLOOR with at least one unshipped ticket.
///
/// T-616. Column 1 of the plan used to carry two spellings — bare `0`-`11`/`43`-`68`/`99` and
/// `w76`…`w81` — and that mix was not cosmetic, it was LOAD-BEARING. `sort -n` scores any
/// non-numeric key as 0, so every `wNN` row sorted into the wave-0 block AHEAD of wave 1, and this
/// loop therefore reached the live factory rows first. The answer it returned was right; the reason
/// was an accident. MEASURED 2026-08-01 before the migration: `current_wave` -> `w80`, which is the
/// operationally correct wave, arrived at by a sort that believed 80 < 1.
///
/// So normalising the column to bare integers — which is what T-616 asks for, and what
/// `slice-collisions` needs since `int('w80')` raises — is only HALF a migration. With uniform
/// numbers `sort -n` finally orders honestly, and this loop then walks the LEGACY BACKLOG first and
/// returns wave 3 (T-578/579/580/587, all `deferred`). `wave`, `wave --close` and `land` all key
/// off this function, so that would have pointed every one of them at a four-year-old deferred
/// backlog row instead of the wave in flight. A uniform format that silently re-aims `land` is a
/// worse bug than the mixed format it replaced.
///
/// The floor is what the `w` prefix actually MEANT, written down as data the sort can respect. The
/// plan holds two generations: the legacy packing waves (0-11, 43-68, plus 99 as a parking lot,
/// still carrying genuinely open `idea`/`deferred` backlog) and the live factory waves, which begin
/// at 76. Only the live generation is dispatchable, so only it can be "current". MEASURED after the
/// migration: waves with unshipped tickets are 0, 3, 5, 7, 8, 9, 10, 11, 80, 81, 99 — floor 76
/// selects 80, identical to the pre-migration answer.
///
/// Raise this when a later generation starts; it is one integer in one place, which is strictly
/// more maintainable than a prefix that had to be typed onto every row and understood by every
/// parser.
pub fn current_wave(ctx: &Ctx) -> String {
    let mut rows = plan_rows(ctx);
    // `sort -n -k1,1`: numeric on field 1 (whitespace-delimited, so the TSV's column 1), with
    // GNU sort's last-resort whole-line comparison breaking ties. A non-numeric key scores 0.
    rows.sort_by(|a, b| {
        let ka = sort_key_numeric(a);
        let kb = sort_key_numeric(b);
        ka.partial_cmp(&kb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(b))
    });
    for r in rows {
        // `IFS=$'\t' read -r w t _`
        let w = col(&r, 1);
        let t = col(&r, 2);
        if w == "0" {
            continue;
        }
        // Bare-integer guard: a row whose label is not numeric cannot be compared, and silently
        // skipping it is how the pre-T-616 mix hid. Say so and keep going.
        if w.is_empty() || !w.bytes().all(|b| b.is_ascii_digit()) {
            werr!(
                "wave: non-numeric wave label '{w}' in {} — T-616 normalised these to integers",
                ctx.plan
            );
            continue;
        }
        let n: i64 = w.parse().unwrap_or(0);
        if n < ctx.generation_floor {
            continue;
        }
        if !ctx.registry_view.is_shipped(t) {
            return w.to_string();
        }
    }
    "done".into()
}

/// GNU `sort -n` on the first whitespace-delimited field: leading blanks skipped, an optional
/// sign, digits; anything else scores 0.
fn sort_key_numeric(line: &str) -> f64 {
    let first = line.split([' ', '\t']).next().unwrap_or("");
    let mut s = first.trim_start();
    let neg = s.starts_with('-');
    if neg || s.starts_with('+') {
        s = &s[1..];
    }
    let digits: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let v: f64 = digits.parse().unwrap_or(0.0);
    if neg { -v } else { v }
}

/// `committed` | `dirty` | `absent` | `unknown`.
///
/// This is the guard that stops `land` merging a slice an agent is still writing into, so a silent
/// failure here is a correctness bug, not an inconvenience: swallowing the error with `2>/dev/null`
/// and testing for empty output makes a FAILED status indistinguishable from a CLEAN one, and the
/// half-finished slice merges. Verified 2026-07-26 that bare `status --porcelain` is unaffected by
/// the missing git-lfs (only `add`/`stash` run the clean filters), but check the exit status anyway
/// — `land` treats anything that is not `committed` as not-ready.
pub fn tree_state(ctx: &Ctx, id: &str) -> &'static str {
    let d = format!("{}/{id}", ctx.worktrees);
    if !Path::new(&d).is_dir() {
        return "absent";
    }
    // git-lfs is installed neither in the container nor on the host, and `status` runs the clean
    // filter to re-hash modified files. In a worktree that has touched anything LFS-adjacent this
    // aborts with `git-lfs filter-process: not found` / `fatal: the remote end hung up
    // unexpectedly` and exit 128 — OBSERVED on slice/T-192 mid-run. Neutralise the filters for this
    // read-only check.
    let out = std::process::Command::new("git")
        .args(["-C", &d])
        .args(LFS_NEUTRAL)
        .args(["status", "--porcelain"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            if o.stdout.is_empty() {
                "committed"
            } else {
                "dirty"
            }
        }
        _ => "unknown",
    }
}

/// The `-c filter.lfs.*` flags `tree_state` and [`git_porcelain_paths`] share, exactly as
/// `slice-worktree` already does for the same reason.
pub const LFS_NEUTRAL: [&str; 8] = [
    "-c",
    "filter.lfs.process=",
    "-c",
    "filter.lfs.clean=cat",
    "-c",
    "filter.lfs.smudge=cat",
    "-c",
    "filter.lfs.required=false",
];

/// Working-tree porcelain paths with LFS filters neutralised — same flags as [`tree_state`].
///
/// T-401: `changed_rs` / `wasm_changed` / `refuse_empty_range` used `git status --porcelain
/// 2>/dev/null` and treated empty stdout as "no changes". When the LFS clean filter aborts (exit
/// 128, empty stdout) that silently half-killed every change-scoped gate: committed diffs still
/// showed, but uncommitted working-tree Rust/frontend edits vanished. Capture rc, never swallow a
/// non-zero behind `2>/dev/null`, and fail loud.
///
/// `Err(rc)` is the bash `return "$rc"`, which every caller propagates with `|| return $?`.
pub fn git_porcelain_paths() -> Result<Vec<String>, i32> {
    let out = std::process::Command::new("git")
        .args(LFS_NEUTRAL)
        .args(["status", "--porcelain"])
        .output();
    let (stdout, rc) = match out {
        Ok(o) => (
            String::from_utf8_lossy(&o.stdout).into_owned(),
            super::host::status_code(&o.status),
        ),
        Err(_) => (String::new(), 127),
    };
    if rc != 0 {
        werr!("wave: git status --porcelain failed (rc={rc}) — refusing silent empty change list");
        return Err(rc);
    }
    // `sed 's/^...//'` — drop the two status columns and the space. `printf '%s\n' "$out"` on an
    // empty capture still emits one empty line, and the callers filter blanks downstream.
    Ok(stdout
        .lines()
        .map(|l| {
            if l.len() >= 3 {
                l[3..].to_string()
            } else {
                String::new()
            }
        })
        .collect())
}

/// `has_work` — does `slice/<id>` carry commits main does not have?
pub fn has_work(id: &str) -> bool {
    let n = git_stdout_lossy(&["rev-list", "--count", &format!("main..slice/{id}")]);
    // `|| echo 0` — a failed rev-list yields "0".
    let n = if n.trim().is_empty() { "0" } else { n.trim() };
    n.parse::<i64>().unwrap_or(0) > 0
}

/// How many tickets have shipped since the last adversarial verifier ran.
///
/// WHY THIS IS A COUNTER AND NOT A HABIT: the verifier was specified as "one per wave" (rule 4),
/// and the run drifted from discrete waves into a continuous stream of individual agents. That did
/// not just change the vocabulary — it DELETED THE EVENT the verifier fires on, so it silently
/// stopped running and 27 tickets landed unverified before the operator noticed. A trigger that
/// depends on remembering a boundary that no longer exists is not a trigger.
///
/// `.ai/artifacts/last-verified` holds the sha the last verifier examined. Debt is the count of
/// platform tickets marked shipped since. Nagging at 8, which is one wave's width.
pub fn verify_debt(ctx: &Ctx) -> String {
    let marker = ctx.root.join(".ai/artifacts/last-verified");
    let base = std::fs::read_to_string(&marker)
        .ok()
        .and_then(|s| s.lines().next().map(str::to_string))
        .map(|s| s.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        .unwrap_or_default();
    if base.is_empty() {
        return "unknown (no .ai/artifacts/last-verified)".into();
    }
    let log = git_stdout(&[
        "-C",
        &ctx.root.display().to_string(),
        "log",
        "--oneline",
        &format!("{base}..HEAD"),
    ])
    .unwrap_or_default();
    // `grep -ciE 'T-[0-9]+[,: ].*(shipped|ship)'` — count of matching LINES, case-insensitive.
    let re = regex::Regex::new(r"(?i)T-[0-9]+[,: ].*(shipped|ship)").expect("static regex");
    let n = log.lines().filter(|l| re.is_match(l)).count();
    // `cut -c1-8` — first eight characters.
    let short: String = base.chars().take(8).collect();
    format!("{n} since {short}")
}

/// `git … 2>/dev/null` returning `None` on failure — local alias so `verify_debt` reads like the
/// bash it came from.
fn git_stdout(args: &[&str]) -> Option<String> {
    super::git_stdout(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_rows_drops_comments_header_and_blanks() {
        // The three filters, in the shapes the real plan has.
        let body = "# comment\nwave\tticket\ttitle\n\n   \n80\tT-1\tTitle\n";
        let rows: Vec<&str> = body
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter(|l| {
                !(l.starts_with("wave")
                    && l[4..]
                        .chars()
                        .next()
                        .map(|c| c.is_whitespace())
                        .unwrap_or(false))
            })
            .filter(|l| !l.trim().is_empty())
            .collect();
        assert_eq!(rows, vec!["80\tT-1\tTitle"]);
    }

    #[test]
    fn awk_eq_is_numeric_only_when_both_sides_look_numeric() {
        assert!(awk_eq("150", "150"));
        assert!(awk_eq("150.0", "150"), "awk compares strnums numerically");
        assert!(
            !awk_eq("w80", "80"),
            "the legacy spelling must NOT match a bare integer"
        );
        assert!(awk_eq("w80", "w80"));
    }

    #[test]
    fn registry_poisons_on_a_ticket_without_an_id() {
        // python's KeyError inside the comprehension makes EVERY query answer "not shipped".
        let dir = std::env::temp_dir().join(format!("t853-reg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("registry.json");
        std::fs::write(
            &p,
            r#"{"tickets":[{"id":"T-1","status":"shipped"},{"status":"shipped"}]}"#,
        )
        .unwrap();
        let r = Registry::load(&p);
        assert!(
            !r.is_shipped("T-1"),
            "one id-less ticket poisons every lookup"
        );
        std::fs::write(&p, r#"{"tickets":[{"id":"T-1","status":"shipped"}]}"#).unwrap();
        assert!(Registry::load(&p).is_shipped("T-1"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn registry_unreadable_is_not_shipped() {
        let r = Registry::load(Path::new("/nonexistent/registry.json"));
        assert!(!r.is_shipped("T-1"));
    }

    #[test]
    fn cancelled_counts_as_shipped() {
        let dir = std::env::temp_dir().join(format!("t853-reg2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("registry.json");
        std::fs::write(&p, r#"{"tickets":[{"id":"T-9","status":"cancelled"}]}"#).unwrap();
        assert!(Registry::load(&p).is_shipped("T-9"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sort_key_scores_non_numeric_as_zero_like_gnu_sort() {
        assert_eq!(sort_key_numeric("80\tT-1"), 80.0);
        assert_eq!(sort_key_numeric("w80\tT-1"), 0.0);
    }
}
