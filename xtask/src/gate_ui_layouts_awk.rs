//! The `awk` half of the T-853 port of `scripts/mod/verify-ui-layouts.sh`.
//!
//! ── WHY THIS IS ITS OWN FILE ─────────────────────────────────────────────────────────────────
//!
//! The script is two programs wearing one shebang: a ~90-line `awk` state machine that parses
//! Enfusion `.layout` syntax (arms C1/C2/C3/C4/C6), and a pipeline of `grep`/`sort`/`comm` that
//! checks the widget-name contract (arm C5). The port kept them in one module until it reached
//! 1113 lines — past SIZE-3, which hard-fails above 1000.
//!
//! The seam is the one place the two halves do not touch: this module knows nothing about paths,
//! I/O, exit codes or `tbd-gate`. Its entire interface is
//!
//! ```ignore
//! Analyzer::new("TBD_ListRow.layout").run(&text) -> Vec<String>   // one finding per element
//! ```
//!
//! so it is a pure function from layout text to findings, and every test below is a string in and
//! a string out. [`crate::gate_ui_layouts`] owns everything else and is the front door — read its
//! header first for what the gate is for and why a broken layout ships silently.
//!
//! ── THE BUG THAT MADE THE FIRST CUT OF THIS GATE VACUOUS ─────────────────────────────────────
//!
//! Per the T-181.51 registry note the FIRST version of the script **passed the known-broken
//! files**. Every Workbench GUID is written as a quoted `"{7BD1A70000000750}"`, so counting `{`
//! and `}` over the raw line desyncs the depth counter; `owner[depth]` then points at the wrong
//! widget, the C4/C6 bookkeeping never matches, and the gate reports clean over a layout it was
//! written to reject. It was fixed by stripping quoted strings before counting — which is what
//! lands C6 on `TBD_ListRow.layout:13`, the exact empty slot block that collapsed every row to a
//! sliver for a whole session.
//!
//! [`strip_quoted`] is that fix, and it is the most load-bearing function in this file.
//! [`tests::guid_braces_do_not_desync_the_counter`] and [`tests::guid_desync_would_hide_a_c6`] pin
//! it; the second asserts the *unstripped* count is genuinely wrong (`{`: 2→1, `}`: 1→0), so a
//! `strip_quoted` that did nothing could not satisfy the pin.
//!
//! ── MEASURED DEVIATION: C4/C6 COME OUT IN LINE ORDER, THE SCRIPT USES HASH ORDER ─────────────
//!
//! The awk program ends with `for (l in needs_slot)` and `for (l in needs_align)`. POSIX leaves
//! that iteration order **unspecified**, and it is not incidental: measured on this machine
//! (`mawk 1.3.4 20200120`, which is what `/usr/bin/awk` and `/usr/bin/nawk` both resolve to here),
//! the key set `{9, 14, 29, 46, 54, 69}` iterates as `29 46 69 54 9 14` — not ascending, not
//! descending, not insertion order, and identical for a reversed insertion sequence. It is the
//! internal hash, so it is a function of the awk build rather than of the layout file.
//!
//! Injecting two C6 defects into `TBD_ListRow.layout` (lines 14 and 29) therefore makes the script
//! print 29 before 14, while [`Analyzer::end`] prints 14 before 29. **Every other injected arm —
//! C1 both ways, C2 on both files, C3 on both invariants, C4, C6 and C5 including a two-name C5 —
//! is byte-identical to the script including rc.**
//!
//! The deviation is kept, for the reason `tbd_gate::scan::walk_files` already sorts: *"a gate's
//! output must not depend on readdir ordering, or two runs over the same tree disagree and the
//! diff-based port acceptance becomes meaningless."* The same argument applies to a hash order
//! that varies by awk implementation. Nothing consumes the order: `scripts/mod/wave.sh:141` runs
//! this gate through a `run()` helper that branches on the exit status alone and, on failure,
//! prints `tail -12` of the merged output for a human. The finding SET, the finding TEXT and the
//! EXIT CODE are unchanged. [`tests::multiple_c6_findings_come_out_in_ascending_line_order`] pins
//! the choice so it stays a decision rather than an accident.

use std::collections::BTreeMap;

/// Slot classes observed working in shipped Enfusion layouts — awk's `ok_slot[]`, built in `BEGIN`.
///
/// Anything else is a guess, and a guess costs an unreadable screen that nobody sees until a client
/// connects. `ButtonSlot` / `OverlaySlot` / `SizeLayoutSlot` / `ScrollLayoutSlot` all derive from
/// `AlignableSlot` and accept only `HorizontalAlign` / `VerticalAlign` / `Padding`; `Anchor`,
/// `PositionX` and `Offset*` belong to `FrameWidgetSlot` ALONE, and putting them on a
/// `ButtonWidgetSlot` is what produced `GUI (E): Unknown keyword/data`.
const OK_SLOT: &[&str] = &[
    "FrameWidgetSlot",
    "OverlayWidgetSlot",
    "ButtonWidgetSlot",
    "LayoutSlot",
    "AlignableSlot",
];

/// awk's `container[]`: widgets that size children by layout rules, not by anchors.
///
/// A child of one of these MUST declare a slot, or it silently falls back to its desired size —
/// which for a `FrameWidgetClass` is ZERO. That is the T-181.47 defect exactly.
const CONTAINER: &[&str] = &[
    "OverlayWidgetClass",
    "SizeLayoutWidgetClass",
    "ScrollLayoutWidgetClass",
    "VerticalLayoutWidgetClass",
    "HorizontalLayoutWidgetClass",
    "ButtonWidgetClass",
];

/// The geometry keys C3 reasons about — the bash alternation, in the bash's order.
///
/// `Anchor` is matched by the same line predicate as the rest (so the alternation ports 1:1) and
/// then discarded by [`Analyzer::record`]: it is a 4-tuple, not a scalar that mirrors an Offset.
const GEOM_KEYS: &[&str] = &[
    "Anchor",
    "PositionX",
    "PositionY",
    "SizeX",
    "SizeY",
    "OffsetLeft",
    "OffsetTop",
    "OffsetRight",
    "OffsetBottom",
];

/// A transliteration of the script's `awk` program: one instance per layout file.
///
/// ── THE GEOMETRY RULE C3 ENFORCES (measured 2026-07-25) ──────────────────────────────────────
///
/// A `FrameWidgetSlot` rect is `left = parentW*Anchor[0] + OffsetLeft`,
/// `right = parentW*Anchor[2] - OffsetRight` (same for Y). Workbench *also* writes `PositionX/Y`
/// and `SizeX/Y`, which mirror the same rect as `PositionX = OffsetLeft` and
/// `SizeX = -(OffsetLeft + OffsetRight)`. Where the two disagree the Offsets win — proven by a
/// shipped, visible reference widget with `PositionX 0` / `OffsetLeft 3` that renders with a 3px
/// inset. C3 requires them to agree, so it cannot matter which one the engine reads.
///
/// ── ODDITIES CARRIED OVER DELIBERATELY ───────────────────────────────────────────────────────
///
/// 1. **`exit` on a stray `}` still runs `END`.** awk's `exit` inside a main rule jumps to the
///    `END` block rather than terminating, so "closing brace with no opener" is *followed* by
///    `C1 …: unbalanced braces (depth -1 at EOF)` plus whatever C4/C6 entries were pending. Two
///    findings from one defect reads like a bug; it is the behaviour of the script this replaces,
///    and the acceptance diff is against that. [`Analyzer::run`] reproduces it exactly, and
///    [`tests::c1_stray_close_also_reports_the_eof_line`] stops a "tidier" port dropping the
///    second line.
/// 2. **`align_owner` is sticky.** It is assigned only inside the `ol in needs_slot` branch and
///    cleared only when a brace closes while `in_slot`, so a `Slot` block that transfers nothing
///    inherits the previous owner. Harmless — `needs_align` no longer holds that key — but it is
///    not what the code reads like, and a "tidy-up" that reset it per slot would change nothing
///    today and could change something tomorrow.
/// 3. **LATENT BUG, PRESERVED:** [`Analyzer::flush_frame`] early-returns when `have_frame` is
///    false, so awk's `delete seen; delete val` never runs on that path. Geometry keys written
///    inside a NON-frame slot — an `OffsetLeft` on a `ButtonWidgetSlot`, which is precisely the
///    T-181.47 mistake — survive into the next `FrameWidgetSlot` and can raise a C3 against values
///    belonging to a different widget. This is a real defect in the script, found while porting;
///    it is reported rather than patched because fixing it changes what the gate prints on a
///    broken tree, and a port is not the place to argue a behaviour change.
///    [`tests::stale_geometry_leaks_across_slots`] pins the current behaviour so the eventual fix
///    is a deliberate edit to a failing test rather than a silent drift.
/// 4. **`for (l in needs_slot)` is UNORDERED, and this port deliberately differs.** See the
///    module-level "MEASURED DEVIATION" section — the one place the port does not print what the
///    script prints, and the one place it is the script that is wrong.
pub(crate) struct Analyzer<'a> {
    fname: &'a str,
    out: Vec<String>,
    /// Signed on purpose: the `depth < 0` test after the decrement is the C1 stray-brace arm.
    depth: i64,
    /// awk's `owner[]` / `owner_line[]`, indexed by depth. Index 0 is written by the closing-brace
    /// loop *before* the decrement, so the vectors run one past the maximum nesting.
    owner: Vec<String>,
    owner_line: Vec<usize>,
    pending_widget: String,
    /// awk spells "unset" as `""` for these; `NR` is 1-based, so `0` is the unambiguous stand-in.
    pending_line: usize,
    in_slot: bool,
    align_owner: usize,
    have_frame: bool,
    frame_line: usize,
    /// awk's `seen[]` and `val[]` merged: presence is `seen`, the value is `val`. Exact, because
    /// `seen[k]` is only ever assigned `1` on the same line that assigns `val[k]`.
    vals: BTreeMap<&'static str, f64>,
    needs_slot: BTreeMap<usize, String>,
    needs_align: BTreeMap<usize, String>,
}

impl<'a> Analyzer<'a> {
    /// `fname` is the BASENAME, because every finding interpolates it as awk's `FNAME`.
    pub(crate) fn new(fname: &'a str) -> Analyzer<'a> {
        Analyzer {
            fname,
            out: Vec::new(),
            depth: 0,
            owner: Vec::new(),
            owner_line: Vec::new(),
            pending_widget: String::new(),
            pending_line: 0,
            in_slot: false,
            align_owner: 0,
            have_frame: false,
            frame_line: 0,
            vals: BTreeMap::new(),
            needs_slot: BTreeMap::new(),
            needs_align: BTreeMap::new(),
        }
    }

    /// Run the whole awk program over one file's text. An empty result means the layout is clean.
    pub(crate) fn run(mut self, text: &str) -> Vec<String> {
        for (idx, line) in awk_records(text).into_iter().enumerate() {
            if !self.record(line, idx + 1) {
                break; // awk `exit`: falls through to END, it does not terminate
            }
        }
        self.end();
        self.out
    }

    /// One awk record. Returns `false` where the script calls `exit`.
    fn record(&mut self, line: &str, nr: usize) -> bool {
        // ── Widget declaration: remember what class owns the block we are about to enter. ──
        if is_widget_decl(line) {
            let wclass = cut_at_blank_or_brace(trim_leading_blank(line)).to_string();
            self.pending_widget.clear();
            self.pending_widget.push_str(&wclass);
            self.pending_line = nr;
            // C4: a container child with no slot block at all. `depth` is still the enclosing
            // block; scan down past anonymous `{ }` child-lists for the nearest owning widget.
            let mut parent = "";
            let mut j = self.depth;
            while j >= 1 {
                let o = self.owner_at(j);
                if !o.is_empty() {
                    parent = o;
                    break;
                }
                j -= 1;
            }
            if !parent.is_empty() && CONTAINER.contains(&parent) {
                self.needs_slot.insert(nr, format!("{parent} > {wclass}"));
            }
        }

        // ── Slot declaration ──
        if let Some(slot) = slot_class(line) {
            if !OK_SLOT.contains(&slot.as_str()) {
                self.push(format!(
                    "C2 {}:{nr} unattested slot class {slot}",
                    self.fname
                ));
            }
            // A Slot block sits inside its widget block, so the widget owning `depth` declared it.
            // For a container child, having a slot is not enough: C6 requires it to say how it is
            // aligned, because an empty `Slot ButtonWidgetSlot { }` leaves the child at its desired
            // size — a FrameWidget then reports 0 and the row collapses to a sliver.
            let ol = self.owner_line_at(self.depth);
            if ol != 0
                && let Some(entry) = self.needs_slot.remove(&ol)
            {
                self.needs_align.insert(ol, entry);
                self.align_owner = ol;
            }
            self.flush_frame();
            if slot == "FrameWidgetSlot" {
                self.have_frame = true;
                self.frame_line = nr;
            }
            self.in_slot = true;
        }

        if self.in_slot && self.align_owner != 0 && starts_with_keyword(line, "HorizontalAlign") {
            self.needs_align.remove(&self.align_owner);
        }

        if self.in_slot
            && let Some(key) = GEOM_KEYS.iter().find(|k| starts_with_keyword(line, k))
            && *key != "Anchor"
        {
            // `Anchor` is a 4-tuple; C3 only reasons about the scalars that mirror the Offsets.
            self.vals.insert(key, awk_to_number(keyword_value(line)));
        }

        // ── Brace tracking. Quoted strings are dropped FIRST — see the module docs. ──
        let bl = strip_quoted(line);
        let opens = bl.matches('{').count();
        let closes = bl.matches('}').count();
        for _ in 0..opens {
            self.depth += 1;
            let d = self.depth;
            self.set_owner(d, "", 0);
            if !self.pending_widget.is_empty() {
                let w = std::mem::take(&mut self.pending_widget);
                self.set_owner(d, &w, self.pending_line);
            }
        }
        for _ in 0..closes {
            if self.in_slot {
                self.in_slot = false;
                self.align_owner = 0;
                self.flush_frame();
            }
            let d = self.depth;
            self.set_owner(d, "", 0);
            self.depth -= 1;
            if self.depth < 0 {
                self.push(format!(
                    "C1 {}:{nr} closing brace with no opener",
                    self.fname
                ));
                return false;
            }
        }
        true
    }

    /// C3: Position/Size must mirror the Offsets of the same `FrameWidgetSlot`.
    fn flush_frame(&mut self) {
        if !self.have_frame {
            // NOTE the absent `self.vals.clear()` — oddity 3 on the type. Deliberate, not an
            // oversight: the bash leaks the same way and the acceptance diff is against the bash.
            return;
        }
        let (f, l) = (self.fname, self.frame_line);
        let g = |k: &str| self.vals.get(k).copied();
        let mut out = Vec::new();
        if let (Some(px), Some(ol)) = (g("PositionX"), g("OffsetLeft"))
            && px != ol
        {
            out.push(format!(
                "C3 {f}:{l} PositionX {} != OffsetLeft {}",
                awk_to_string(px),
                awk_to_string(ol)
            ));
        }
        if let (Some(py), Some(ot)) = (g("PositionY"), g("OffsetTop"))
            && py != ot
        {
            out.push(format!(
                "C3 {f}:{l} PositionY {} != OffsetTop {}",
                awk_to_string(py),
                awk_to_string(ot)
            ));
        }
        if let (Some(sx), Some(ol), Some(or)) = (g("SizeX"), g("OffsetLeft"), g("OffsetRight"))
            && sx != -(ol + or)
        {
            out.push(format!(
                "C3 {f}:{l} SizeX {} but -(OffsetLeft {} + OffsetRight {}) = {}",
                awk_to_string(sx),
                awk_to_string(ol),
                awk_to_string(or),
                awk_to_string(-(ol + or))
            ));
        }
        if let (Some(sy), Some(ot), Some(ob)) = (g("SizeY"), g("OffsetTop"), g("OffsetBottom"))
            && sy != -(ot + ob)
        {
            out.push(format!(
                "C3 {f}:{l} SizeY {} but -(OffsetTop {} + OffsetBottom {}) = {}",
                awk_to_string(sy),
                awk_to_string(ot),
                awk_to_string(ob),
                awk_to_string(-(ot + ob))
            ));
        }
        self.out.extend(out);
        self.have_frame = false;
        self.vals.clear();
    }

    /// The awk `END` block. See the module docs on why the two maps iterate in line order here and
    /// in hash order in the script.
    fn end(&mut self) {
        self.flush_frame();
        if self.depth != 0 {
            self.push(format!(
                "C1 {}: unbalanced braces (depth {} at EOF)",
                self.fname, self.depth
            ));
        }
        for (l, what) in std::mem::take(&mut self.needs_slot) {
            self.out.push(format!(
                "C4 {}:{l} {what} has no Slot block — it will collapse to its desired size",
                self.fname
            ));
        }
        for (l, what) in std::mem::take(&mut self.needs_align) {
            self.out.push(format!(
                "C6 {}:{l} {what} has a Slot block with no HorizontalAlign — it will collapse to \
                 its desired size",
                self.fname
            ));
        }
    }

    fn push(&mut self, s: String) {
        self.out.push(s);
    }

    fn owner_at(&self, d: i64) -> &str {
        usize::try_from(d)
            .ok()
            .and_then(|i| self.owner.get(i))
            .map_or("", String::as_str)
    }

    fn owner_line_at(&self, d: i64) -> usize {
        usize::try_from(d)
            .ok()
            .and_then(|i| self.owner_line.get(i))
            .copied()
            .unwrap_or(0)
    }

    fn set_owner(&mut self, d: i64, who: &str, line: usize) {
        let Ok(i) = usize::try_from(d) else { return };
        if self.owner.len() <= i {
            self.owner.resize(i + 1, String::new());
            self.owner_line.resize(i + 1, 0);
        }
        self.owner[i].clear();
        self.owner[i].push_str(who);
        self.owner_line[i] = line;
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────
// awk primitives
//
// Each of these is one awk construct, kept as a named function so the bash it replaces can be
// quoted directly above it. They are why a "tidy" rewrite of this file is dangerous: awk's
// string/number coercions and its `sub()` anchoring are not the obvious Rust equivalents, and
// every difference is observable in the gate's output.
// ─────────────────────────────────────────────────────────────────────────────────────────────

/// awk record splitting with `RS="\n"`.
///
/// NOT `str::lines()`: that strips a trailing `\r`, so on a CRLF layout the line content would be
/// silently altered and a `SizeX 5\r` would print differently from what awk prints.
fn awk_records(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut recs: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        recs.pop();
    }
    recs
}

/// `/^[ \t]*[A-Za-z_]+WidgetClass[ \t{]/`
fn is_widget_decl(line: &str) -> bool {
    let s = trim_leading_blank(line);
    let b = s.as_bytes();
    let mut n = 0;
    while n < b.len() && (b[n].is_ascii_alphabetic() || b[n] == b'_') {
        n += 1;
    }
    // `[A-Za-z_]+` needs at least one character BEFORE the literal, so a bare `WidgetClass {` is
    // not a declaration. The identifier run is greedy, so the suffix test applies to the whole run.
    if n < "WidgetClass".len() + 1 || !s[..n].ends_with("WidgetClass") {
        return false;
    }
    matches!(b.get(n), Some(b' ' | b'\t' | b'{'))
}

/// `/^[ \t]*Slot[ \t]+[A-Za-z_]+/` plus the three `sub()`s that isolate the class name.
fn slot_class(line: &str) -> Option<String> {
    let s = trim_leading_blank(line).strip_prefix("Slot")?;
    let rest = s.trim_start_matches([' ', '\t']);
    if rest.len() == s.len() {
        return None; // `[ \t]+` requires at least one separator
    }
    let name = cut_at_blank_or_brace(rest);
    // `[A-Za-z_]+` must match at least once for awk's `match()` to fire.
    let first = name.as_bytes().first()?;
    if !(first.is_ascii_alphabetic() || *first == b'_') {
        return None;
    }
    Some(name.to_string())
}

/// `/^[ \t]*KEYWORD[ \t]/` — the trailing separator is part of the bash alternation and is what
/// stops `PositionX` matching a line beginning `PositionXY`.
fn starts_with_keyword(line: &str, keyword: &str) -> bool {
    match trim_leading_blank(line).strip_prefix(keyword) {
        Some(rest) => rest.starts_with(' ') || rest.starts_with('\t'),
        None => false,
    }
}

/// `sub(/^[ \t]*[A-Za-z]+[ \t]+/, "", v); sub(/[ \t]*$/, "", v)` — the value half of a
/// `KEYWORD VALUE` line, right-trimmed of blanks only (a trailing `\r` survives, as in awk).
fn keyword_value(line: &str) -> &str {
    let s = trim_leading_blank(line);
    let b = s.as_bytes();
    let mut n = 0;
    while n < b.len() && b[n].is_ascii_alphabetic() {
        n += 1;
    }
    let rest = &s[n..];
    let trimmed = rest.trim_start_matches([' ', '\t']);
    if trimmed.len() == rest.len() {
        return s.trim_end_matches([' ', '\t']); // no `[ \t]+`: the first sub did not fire
    }
    trimmed.trim_end_matches([' ', '\t'])
}

/// `sub(/^[ \t]*/, "", x)`
fn trim_leading_blank(s: &str) -> &str {
    s.trim_start_matches([' ', '\t'])
}

/// `sub(/[ \t].*$/, "", x); sub(/\{.*$/, "", x)`
fn cut_at_blank_or_brace(s: &str) -> &str {
    let end = s
        .find([' ', '\t'])
        .unwrap_or(s.len())
        .min(s.find('{').unwrap_or(s.len()));
    &s[..end]
}

/// `gsub(/"[^"]*"/, "", bl)` — THE fix that stopped GUIDs desyncing the brace counter.
///
/// Non-overlapping, left to right. An unterminated literal is left alone, exactly as the ERE leaves
/// it, so `Text "a { b` still contributes its `{` to the count. See the module docs: without this,
/// the gate reports clean over the very files it was written to reject.
fn strip_quoted(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('"') {
        match rest[open + 1..].find('"') {
            Some(rel) => {
                out.push_str(&rest[..open]);
                rest = &rest[open + 1 + rel + 1..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

/// awk's `v + 0`: the longest leading decimal prefix, else 0.
///
/// Decimal-only on purpose — gawk does not read hex from input data without `--non-decimal-data`.
/// This is why a `SizeMode Fill` line contributes `0` rather than raising an error.
fn awk_to_number(s: &str) -> f64 {
    let t = s.trim_start_matches([' ', '\t', '\n', '\r']);
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    let mut end = i;
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            end = j;
        }
    }
    t[..end].parse::<f64>().unwrap_or(0.0)
}

/// awk's number→string under `%s`: integral values print with `%d`, everything else with `CONVFMT`
/// (`%.6g`). This is why `-480` renders as `-480` and not `-480.000000`.
fn awk_to_string(x: f64) -> String {
    if x.is_finite() && x.fract() == 0.0 && x.abs() < 1e16 {
        // `-(0 + 0)` is IEEE `-0.0`; awk prints it as `0`, and so does the cast.
        return format!("{}", x as i64);
    }
    format_g6(x)
}

/// `%.6g`, which Rust's formatter does not provide.
fn format_g6(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    let exp = x.abs().log10().floor() as i32;
    if !(-4..6).contains(&exp) {
        let mantissa = x / 10f64.powi(exp);
        let m = trim_float(&format!("{mantissa:.5}"));
        return format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    trim_float(&format!("{:.*}", (5 - exp).max(0) as usize, x))
}

fn trim_float(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(body: &str) -> Vec<String> {
        Analyzer::new("T.layout").run(body)
    }

    // ── THE REGRESSION THAT MADE THE FIRST CUT VACUOUS ──────────────────────────────────────

    #[test]
    fn guid_braces_do_not_desync_the_counter() {
        // Every Workbench GUID is a quoted `"{…}"`. Counting braces on the raw line makes this file
        // look balanced at the wrong depth; stripping quotes first makes it balanced at the right
        // one. The file is clean, so the correct answer is: no findings at all.
        let body = "\
OverlayWidgetClass \"{7BD1A70000000750}\" {
 Name \"Root\"
 {
  ImageWidgetClass \"{7BD1A70000000751}\" {
   Name \"Child\"
   Slot OverlayWidgetSlot \"{7BD1A70000000752}\" {
    HorizontalAlign 3
   }
  }
 }
}
";
        assert_eq!(analyse(body), Vec::<String>::new());
    }

    #[test]
    fn guid_desync_would_hide_a_c6() {
        // The pin above only means something if the UNSTRIPPED count is genuinely wrong — otherwise
        // a `strip_quoted` that did nothing would satisfy it. Assert the difference directly.
        let line = " Slot OverlayWidgetSlot \"{7BD1A70000000752}\" {";
        assert_eq!(line.matches('{').count(), 2, "raw count is the bug");
        assert_eq!(line.matches('}').count(), 1);
        assert_eq!(strip_quoted(line).matches('{').count(), 1);
        assert_eq!(strip_quoted(line).matches('}').count(), 0);

        // And end-to-end: a container child whose slot has no HorizontalAlign must be caught. With
        // a desynced counter `owner_line[depth]` misses and C6 never fires — the T-181.51 defect.
        let body = "\
OverlayWidgetClass \"{7BD1A70000000750}\" {
 Name \"Root\"
 {
  ImageWidgetClass \"{7BD1A70000000751}\" {
   Name \"Child\"
   Slot OverlayWidgetSlot \"{7BD1A70000000752}\" {
    VerticalAlign 3
   }
  }
 }
}
";
        let got = analyse(body);
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(
            got[0].starts_with("C6 T.layout:4 OverlayWidgetClass > ImageWidgetClass"),
            "{got:?}"
        );
    }

    // ── one test per arm ────────────────────────────────────────────────────────────────────

    #[test]
    fn c1_unbalanced_at_eof() {
        let got = analyse("FrameWidgetClass {\n Name \"R\"\n");
        assert_eq!(got, vec!["C1 T.layout: unbalanced braces (depth 1 at EOF)"]);
    }

    #[test]
    fn c1_stray_close_also_reports_the_eof_line() {
        // awk `exit` runs END. Two findings from one defect is the script's behaviour, pinned here
        // so a "tidier" port cannot quietly drop the second line the acceptance diff expects.
        let got = analyse("FrameWidgetClass {\n}\n}\n");
        assert_eq!(
            got,
            vec![
                "C1 T.layout:3 closing brace with no opener",
                "C1 T.layout: unbalanced braces (depth -1 at EOF)",
            ]
        );
    }

    #[test]
    fn c2_unattested_slot_class() {
        let body = "FrameWidgetClass {\n Slot BogusSlot {\n  HorizontalAlign 3\n }\n}\n";
        assert_eq!(
            analyse(body),
            vec!["C2 T.layout:2 unattested slot class BogusSlot"]
        );
    }

    #[test]
    fn c3_geometry_mirror() {
        let body = "\
FrameWidgetClass {
 Slot FrameWidgetSlot {
  Anchor 0 0 1 1
  PositionX 5
  OffsetLeft 3
  SizeX 0
  OffsetRight 0
 }
}
";
        assert_eq!(
            analyse(body),
            vec![
                "C3 T.layout:2 PositionX 5 != OffsetLeft 3",
                "C3 T.layout:2 SizeX 0 but -(OffsetLeft 3 + OffsetRight 0) = -3",
            ]
        );
    }

    #[test]
    fn c4_container_child_with_no_slot() {
        let body = "OverlayWidgetClass {\n {\n  FrameWidgetClass {\n  }\n }\n}\n";
        assert_eq!(
            analyse(body),
            vec![
                "C4 T.layout:3 OverlayWidgetClass > FrameWidgetClass has no Slot block — it will \
                 collapse to its desired size"
            ]
        );
    }

    #[test]
    fn multiple_c6_findings_come_out_in_ascending_line_order() {
        // THE ONE DEVIATION FROM THE SCRIPT, pinned so it stays a decision. mawk 1.3.4 prints these
        // two in hash order (29 before 14 on the real TBD_ListRow.layout); a BTreeMap prints them
        // in line order. See the module-level "MEASURED DEVIATION" section for why line order wins.
        let body = "\
OverlayWidgetClass {
 {
  ImageWidgetClass {
   Slot OverlayWidgetSlot {
    VerticalAlign 3
   }
  }
  TextWidgetClass {
   Slot OverlayWidgetSlot {
    VerticalAlign 3
   }
  }
 }
}
";
        let got = analyse(body);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got[0].starts_with("C6 T.layout:3 "), "{got:?}");
        assert!(got[1].starts_with("C6 T.layout:8 "), "{got:?}");
    }

    #[test]
    fn a_non_container_parent_needs_no_slot() {
        // FrameWidgetClass anchors its children, so C4 must not fire under one.
        let body = "FrameWidgetClass {\n {\n  ImageWidgetClass {\n  }\n }\n}\n";
        assert_eq!(analyse(body), Vec::<String>::new());
    }

    // ── the preserved latent bug ────────────────────────────────────────────────────────────

    #[test]
    fn stale_geometry_leaks_across_slots() {
        // `flush_frame` returns before `delete val` when have_frame is false, so an OffsetLeft
        // written on a ButtonWidgetSlot survives into the NEXT FrameWidgetSlot. Reported, not
        // fixed — changing it changes what the gate prints on a broken tree.
        let body = "\
ButtonWidgetClass {
 Slot ButtonWidgetSlot {
  OffsetLeft 99
 }
 {
  ImageWidgetClass {
   Slot FrameWidgetSlot {
    HorizontalAlign 3
    PositionX 0
   }
  }
 }
}
";
        let got = analyse(body);
        assert!(
            got.iter()
                .any(|l| l.contains("PositionX 0 != OffsetLeft 99")),
            "the leak is load-bearing for byte-compatibility: {got:?}"
        );
    }

    // ── awk primitives ──────────────────────────────────────────────────────────────────────

    #[test]
    fn awk_number_conversion_matches_v_plus_zero() {
        assert_eq!(awk_to_number("240"), 240.0);
        assert_eq!(awk_to_number("-480"), -480.0);
        assert_eq!(awk_to_number("0.5"), 0.5);
        assert_eq!(awk_to_number("Fill"), 0.0);
        assert_eq!(awk_to_number("3 0 0 0"), 3.0);
        assert_eq!(awk_to_number(""), 0.0);
    }

    #[test]
    fn awk_string_conversion_uses_percent_d_for_integers() {
        assert_eq!(awk_to_string(240.0), "240");
        assert_eq!(awk_to_string(-0.0), "0");
        assert_eq!(awk_to_string(-3.0), "-3");
        assert_eq!(awk_to_string(0.5), "0.5");
    }

    #[test]
    fn records_keep_a_trailing_carriage_return() {
        // `str::lines()` would eat the `\r` and change what a C3 message prints.
        assert_eq!(awk_records("a\r\nb\n"), vec!["a\r", "b"]);
        assert_eq!(awk_records(""), Vec::<&str>::new());
        assert_eq!(awk_records("a"), vec!["a"]);
    }

    #[test]
    fn widget_decl_predicate_matches_the_bash_regex() {
        assert!(is_widget_decl("ButtonWidgetClass {"));
        assert!(is_widget_decl("  SizeLayoutWidgetClass \"{7BD}\" {"));
        assert!(is_widget_decl("\tOverlayWidgetClass{"));
        assert!(!is_widget_decl("WidgetClass {"), "needs a prefix");
        assert!(!is_widget_decl(" TBD_ListBoxRow \"{7BD}\" {"));
        assert!(!is_widget_decl(" components {"));
    }

    #[test]
    fn geometry_keyword_needs_a_separator() {
        assert!(starts_with_keyword(" PositionX 5", "PositionX"));
        assert!(!starts_with_keyword(" PositionXY 5", "PositionX"));
    }
}
