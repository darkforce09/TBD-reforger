//! T-181.47 UI-layout gate — the T-853 port of `scripts/mod/verify-ui-layouts.sh`.
//!
//! ── WHY THIS EXISTS (carried over from the script) ───────────────────────────────────────────
//!
//! A `.layout` only loads when a menu opens, which needs a connected client. `compile.sh` never
//! reads it and `world-boot.sh` boots with zero players, so a broken layout ships silently and the
//! first symptom is a human staring at an unreadable screen. That is exactly how T-181.47
//! happened: the list rendered as a ~10px column of clipped text for a whole session.
//!
//! This gate cannot prove a layout *looks* right — only a client can. It proves the things that
//! were actually wrong, all of which are decidable from the text:
//!
//! | arm | invariant                                                                       |
//! |-----|---------------------------------------------------------------------------------|
//! | C1  | brace balance — a desynced parser reports every later keyword as unknown         |
//! | C2  | slot classes are attested — only names observed working in shipped layouts       |
//! | C3  | `FrameWidgetSlot` geometry — Position/Size must agree with the Offsets           |
//! | C4  | layout-container children must declare a slot, or they collapse                  |
//! | C5  | widget-name contract — every name `FindAnyWidget()` asks for must exist          |
//! | C6  | a container child's slot must say how it is aligned, or it collapses             |
//!
//! ── MEASURED FACTS THIS ENCODES (2026-07-25, from the script header) ─────────────────────────
//!
//! * A `FrameWidgetSlot` rect is `left = parentW*Anchor[0] + OffsetLeft`,
//!   `right = parentW*Anchor[2] - OffsetRight` (same for Y). Workbench *also* writes `PositionX/Y`
//!   and `SizeX/Y`, which mirror the same rect as `PositionX = OffsetLeft` and
//!   `SizeX = -(OffsetLeft + OffsetRight)`. Where the two disagree the Offsets win — proven by a
//!   shipped, visible reference widget with `PositionX 0` / `OffsetLeft 3` that renders with a 3px
//!   inset. C3 removes the ambiguity by requiring them to agree, so it cannot matter which one the
//!   engine reads.
//! * Alignment is `LayoutHorizontalAlign { Left=0, Center=1, Right=2, Stretch=3 }` —
//!   `apps/mod/vanilla_reference/Scripts/Core/generated/UI/LayoutHorizontalAlign.c`.
//! * `ButtonSlot` / `OverlaySlot` / `SizeLayoutSlot` / `ScrollLayoutSlot` all derive from
//!   `AlignableSlot` and accept only `HorizontalAlign` / `VerticalAlign` / `Padding`. `Anchor`,
//!   `PositionX` and `Offset*` belong to `FrameWidgetSlot` ALONE — putting them on a
//!   `ButtonWidgetSlot` is what produced `GUI (E): Unknown keyword/data`.
//!
//! ── WHERE THE OTHER HALF LIVES ───────────────────────────────────────────────────────────────
//!
//! The script is two programs wearing one shebang. Arms C1/C2/C3/C4/C6 are a ~90-line `awk` state
//! machine over `.layout` syntax; arm C5 is a `grep`/`sort`/`comm` pipeline over the scripts. The
//! awk half is [`crate::gate_ui_layouts_awk`] — a pure `text -> Vec<String>` function with no
//! knowledge of paths, I/O or `tbd-gate`. Read it for the awk semantics, the four bash oddities
//! carried over deliberately (including a latent bug in the script this port reproduces rather
//! than fixes), and the GUID-versus-brace-counter defect that made the script's own first cut pass
//! the known-broken files. THIS file owns the filesystem, arm C5, the exit contract and the
//! reporting.
//!
//! The split happened at 1113 lines, past SIZE-3's >1000 hard fail; the seam is the one place the
//! two halves never touch.
//!
//! ── WHAT `tbd-gate` BUYS, AND WHICH FAIL-OPEN SHAPES ARE CLOSED ──────────────────────────────
//!
//! 1. `shopt -s nullglob; layouts=("$UI_DIR"/*.layout)` yields an empty array both when the
//!    directory is empty **and when it does not exist**, and the script then prints "no .layout
//!    files under X" — a true statement with a false cause, pointing the reader at the wrong fix.
//!    [`layout_files`] uses [`scan::walk_files`], whose missing-root check is
//!    [`NotRun::TargetMissing`], so a renamed directory reads as DID NOT RUN (exit 2) rather than
//!    as an empty glob (exit 1). This is one of the port's two behavioural deviations (the other is
//!    the ordering note below) and it is reachable only on a tree where the bash message was
//!    already misleading about its own cause.
//! 2. `names=$(... | grep -vx 'FocusAnchor' || true)` — `|| true` swallows grep's exit 2 as well as
//!    its exit 1, and an empty `names` then prints
//!    `OK  widget-name contract (0 names bound by script, all declared)`: a gate reporting OK over
//!    zero inputs, the T-556 signature defect. No subprocess can fail here, and [`C5_FLOOR`] pins
//!    that the bound set never shrinks below the seeds.
//! 3. `grep -rhoE ... "$SCRIPT_DIR"` under `set -euo pipefail` **abandons the run with no verdict**
//!    if that directory is gone: grep exits 2, pipefail propagates it out of the command
//!    substitution, errexit fires, and the script stops between C4 and C5. Measured — the operator
//!    gets the two `OK  <layout>` lines, a raw `grep: …: No such file or directory` on stderr, and
//!    `rc=1`, with no `FAIL:` line and no `==> UI layout gate FAILED` trailer to say which check
//!    was skipped or that the run was incomplete. [`bound_widget_names`] returns [`NotRun`], which
//!    names the path, says the pin did not run, and exits 2 rather than 1 so CI can tell a broken
//!    checkout from a real violation.
//! 4. Exit 127. The script shells out to `grep`, `comm`, `sort`, `tr`, `awk` and `basename`; none
//!    of them are reachable states here — the matcher is [`Pattern`], compiled in.
//!
//! The port's **second** deviation is the C4/C6 emission order: the script iterates two awk arrays
//! in hash order, this port iterates them in line order. It is measured, argued and pinned in
//! [`crate::gate_ui_layouts_awk`]'s "MEASURED DEVIATION" section, because that is where the code
//! is. Every injected-defect arm other than a deliberate two-C6 probe is byte-identical to the
//! script, rc included.
//!
//! Exit 0 = clean. Exit 1 = a check failed. Exit 2 = a check did not run.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tbd_gate::Pattern;
use tbd_gate::scan::{self, Hit};
use tbd_gate::verdict::{Kind, NotRun, Verdict};

// The awk BEGIN block's `ok_slot[]`, `container[]` and the geometry-key alternation moved with the
// state machine that reads them — they are awk program data, not gate configuration.
use crate::gate_ui_layouts_awk::Analyzer;

/// The `[Attribute]`-driven lookups in `TBD_ListBox` have these defaults, so they are bound by
/// script even though no `Find*("…")` literal names them.
const LISTBOX_ATTRIBUTE_DEFAULTS: &[&str] = &["Content", "EmptyState"];

/// `FocusAnchor` is documented as optional — `TBD_MenuBase` falls back to the root widget.
const OPTIONAL_NAMES: &[&str] = &["FocusAnchor"];

/// Anti-vacuity floor for C5. The seeds above alone guarantee this many names, so a smaller set
/// means the extraction broke, not that the code stopped binding widgets.
const C5_FLOOR: usize = 2;

/// The four accessors whose string literal is a widget-name contract with the layouts.
///
/// Ordered longest-first so the scan reproduces POSIX ERE **leftmost-longest** alternation: at a
/// given offset `grep -E '(FindAnyWidget|Find|…)'` prefers the longest alternative, and trying
/// `Find` before `FindAnyWidget` would truncate every hit.
const FINDERS: &[&str] = &["FindAnyWidget", "FindHandlerOn", "FindText", "Find"];

pub fn verify_ui_layouts(repo_root: &Path) -> Result<u8> {
    let ui_dir = repo_root.join("apps/mod/tbd-framework/UI/layouts");
    let script_dir = repo_root.join("apps/mod/tbd-framework/Scripts/Game/TBD/UI");

    let mut fail = false;

    let layouts = match layout_files(&ui_dir) {
        Ok(v) => v,
        Err(cause) => return Ok(did_not_run("UI layout gate", cause)),
    };
    if layouts.is_empty() {
        // Verbatim from the script. Reached only when the directory EXISTS and holds no `*.layout`
        // — `walk_files` already split off the "directory is gone" case above.
        println!("FAIL: no .layout files under {}", ui_dir.display());
        return Ok(1);
    }

    println!("==> verifying {} TBD layout(s)", layouts.len());

    // ── C1/C2/C3/C4/C6 — one awk pass per file ──────────────────────────────────────────────
    for path in &layouts {
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(source) => {
                let cause = NotRun::Unreadable {
                    path: path.clone(),
                    source,
                };
                return Ok(did_not_run(&format!("layout {fname}"), cause));
            }
        };
        let text = String::from_utf8_lossy(&bytes);
        let findings = Analyzer::new(&fname).run(&text);
        if findings.is_empty() {
            // `note()` in bash: `printf '    %s\n'`. The two spaces after "OK" are load-bearing.
            println!("    OK  {fname}  (braces, slot classes, geometry, container slots)");
        } else {
            // `bad()`: `printf 'FAIL: %s\n'`, which is exactly `Finding`'s Display.
            for line in findings {
                println!("{}", Verdict::failed(line));
                fail = true;
            }
        }
    }

    // ── C5: widget-name contract ────────────────────────────────────────────────────────────
    // Every literal a script hands to FindAnyWidget()/Find()/FindText()/FindHandlerOn(), plus the
    // TBD_ListBox attribute defaults, must be a Name in some layout. A missing one is not an error
    // at runtime — it is a null the code politely tolerates and a feature that never appears.
    let bound = match bound_widget_names(&script_dir) {
        Ok(v) => v,
        Err(cause) => return Ok(did_not_run("C5 widget-name contract", cause)),
    };
    if bound.len() < C5_FLOOR {
        println!(
            "{}",
            Verdict::failed(format!(
                "C5 bound-name extraction returned {} names (< {C5_FLOOR}) — the scan is broken, \
                 not the code",
                bound.len()
            ))
        );
        println!("==> UI layout gate FAILED");
        return Ok(1);
    }
    let declared = match declared_widget_names(&layouts) {
        Ok(v) => v,
        Err(cause) => return Ok(did_not_run("C5 declared-name scan", cause)),
    };

    // `comm -23 <(names) <(declared)`. Both bash sides are `sort -u` and both sides here are a
    // `BTreeSet`, so the SET is identical; the ORDER is byte order rather than the ambient locale's
    // collation, deliberately — a gate's output must not depend on `LANG`.
    let missing: Vec<&String> = bound.difference(&declared).collect();
    if missing.is_empty() {
        // `grep -c .` over the bash `names` list counts non-empty lines; every element here is a
        // non-empty identifier, so `len()` is the same number.
        println!(
            "    OK  widget-name contract ({} names bound by script, all declared)",
            bound.len()
        );
    } else {
        for name in missing {
            println!(
                "{}",
                Verdict::failed(format!(
                    "C5 script binds widget \"{name}\" but no layout declares it"
                ))
            );
            fail = true;
        }
    }

    if fail {
        println!("==> UI layout gate FAILED");
        return Ok(1);
    }
    println!("==> UI layout gate PASSED");
    Ok(0)
}

/// Render a [`NotRun`] the way `tbd-gate` does and yield exit 2.
///
/// Factored out so every "did not run" path in this file is one line and cannot drift into a
/// `return Ok(0)` during a later edit.
fn did_not_run(what: &str, cause: NotRun) -> u8 {
    println!("{}", Verdict::did_not_run(what, Kind::Pin, cause));
    println!("==> UI layout gate FAILED");
    2
}

/// `shopt -s nullglob; layouts=("$UI_DIR"/*.layout)`, fail-closed.
///
/// The glob is NON-recursive, hence the `parent()` test. The `.layout.meta` siblings drop out for
/// free: `Path::extension` on `TBD_ListRow.layout.meta` is `meta`. `walk_files` sorts, so report
/// order never depends on readdir order — the shell glob sorted by locale collation, which for
/// these ASCII names is the same order.
fn layout_files(ui_dir: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let root = ui_dir.to_path_buf();
    scan::walk_files(&[ui_dir], move |p| {
        p.parent() == Some(root.as_path())
            && p.extension().and_then(|e| e.to_str()) == Some("layout")
    })
}

/// The C5 left-hand side: names bound by script, seeded and filtered exactly as the bash does.
fn bound_widget_names(script_dir: &Path) -> Result<BTreeSet<String>, NotRun> {
    // `grep -r` with no `--include`: every file under the tree, not just `.c`.
    let files = scan::walk_files(&[script_dir], |_| true)?;
    // The outer `grep -rhoE` as a line filter; [`finder_names`] is the `grep -o` half, which must
    // find EVERY occurrence on a line and not just the first.
    let finder_line =
        Pattern::regex(r#"(FindAnyWidget|Find|FindText|FindHandlerOn)\("[A-Za-z_][A-Za-z0-9_]*""#)
            .expect("static pattern compiles");
    let hits: Vec<Hit> = scan::grep_lines(&finder_line, &files)?;

    let mut names: BTreeSet<String> = BTreeSet::new();
    for hit in &hits {
        names.extend(finder_names(&hit.line));
    }
    names.extend(LISTBOX_ATTRIBUTE_DEFAULTS.iter().map(|s| s.to_string()));
    for opt in OPTIONAL_NAMES {
        names.remove(*opt);
    }
    Ok(names)
}

/// The C5 right-hand side: `grep -rhoE '^[ \t]*Name "…"' "$UI_DIR"/*.layout`.
fn declared_widget_names(layouts: &[PathBuf]) -> Result<BTreeSet<String>, NotRun> {
    let name_line =
        Pattern::regex(r#"^[ \t]*Name "[A-Za-z_][A-Za-z0-9_]*""#).expect("static pattern compiles");
    let hits = scan::grep_lines(&name_line, layouts)?;
    Ok(hits.iter().filter_map(|h| declared_name(&h.line)).collect())
}

/// Every `Find*("Ident"` occurrence on one line, in source order.
///
/// Hand-rolled rather than `Regex::find_iter` because `grep -o` semantics are exactly "leftmost,
/// longest, non-overlapping", and spelling that out makes the alternation-order requirement on
/// [`FINDERS`] impossible to break silently.
fn finder_names(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    'outer: while i < line.len() {
        if line.is_char_boundary(i) {
            for f in FINDERS {
                if !line[i..].starts_with(f) {
                    continue;
                }
                // `grep -o` has no word boundary: `MyFind("X"` really does match the `Find` arm.
                if let Some((ident, end)) = paren_quoted_ident(line, i + f.len()) {
                    out.push(ident);
                    i = end; // non-overlapping: resume past the whole match
                    continue 'outer;
                }
            }
        }
        i += 1;
    }
    out
}

/// Parse `("Ident"` at `at`, returning the identifier and the offset just past the closing quote.
fn paren_quoted_ident(line: &str, at: usize) -> Option<(String, usize)> {
    let rest = line.get(at..)?.strip_prefix('(')?.strip_prefix('"')?;
    let ident = leading_ident(rest)?;
    let end = at + 2 + ident.len();
    // The bash pattern ends with a literal `"`, so an unterminated literal is not a match.
    if line.as_bytes().get(end) != Some(&b'"') {
        return None;
    }
    Some((ident.to_string(), end + 1))
}

/// `^[ \t]*Name "Ident"` on an already-filtered line → `Ident`.
fn declared_name(line: &str) -> Option<String> {
    // The leading-blank trim is spelled out rather than borrowed from the awk module: keeping this
    // side free of that dependency is what makes the seam a seam.
    let rest = line
        .trim_start_matches([' ', '\t'])
        .strip_prefix("Name")?
        .strip_prefix(' ')?
        .strip_prefix('"')?;
    leading_ident(rest).map(str::to_string)
}

/// `[A-Za-z_][A-Za-z0-9_]*` anchored at the start of `s`.
fn leading_ident(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    if b.is_empty() || !(b[0].is_ascii_alphabetic() || b[0] == b'_') {
        return None;
    }
    let mut n = 1;
    while n < b.len() && (b[n].is_ascii_alphanumeric() || b[n] == b'_') {
        n += 1;
    }
    Some(&s[..n])
}

#[cfg(test)]
mod tests {
    use super::*;

    // The C1/C2/C3/C4/C6 tests live next to the state machine they exercise, in
    // `gate_ui_layouts_awk.rs`. What remains here is the C5 half: the two `grep -o` extractions
    // that turn a line into widget names.

    #[test]
    fn finder_names_reads_every_occurrence_leftmost_longest() {
        assert_eq!(
            finder_names(r#"a = FindAnyWidget("Title"); b = w.FindText("Detail");"#),
            vec!["Title".to_string(), "Detail".to_string()]
        );
        // `grep -o` has no word boundary — the `Find` arm really does match inside `MyFind(`.
        assert_eq!(finder_names(r#"MyFind("X")"#), vec!["X".to_string()]);
        // FindAnyWidget must not be truncated to the `Find` arm.
        assert_eq!(finder_names(r#"FindAnyWidget("Q")"#), vec!["Q".to_string()]);
        // Not matches: no `("` immediately after, and `FindTextWidget` is neither alternative.
        assert!(finder_names(r#"FindAnyWidget(name)"#).is_empty());
        assert!(finder_names(r#"FindTextWidget("Z")"#).is_empty());
    }

    #[test]
    fn declared_name_extraction_is_line_anchored() {
        assert_eq!(declared_name(" Name \"RowBody\""), Some("RowBody".into()));
        assert_eq!(declared_name(" m_sName \"RowBody\""), None);
    }
}
