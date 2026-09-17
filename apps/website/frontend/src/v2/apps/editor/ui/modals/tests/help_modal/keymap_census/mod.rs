//! Editor key binding census used by source-based test gates.

use crate::v2::core::test_support::class_r_scrub::live_source;
use std::collections::{BTreeMap, BTreeSet};

/// A modifier PREDICATE, as read out of a live arm guard. `Some(true)` = the modifier is
/// required, `Some(false)` = forbidden, `None` = the binding does not constrain it (and so
/// claims the key with the modifier held **and** released).
///
/// `modk` is the editor's `ctrl || meta`, the one abstraction the arms already use.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Mods {
    pub(crate) modk: Option<bool>,
    pub(crate) alt: Option<bool>,
    pub(crate) shift: Option<bool>,
}

impl Mods {
    /// No constraint at all — what a bare `ev.key() == "Escape"` listener claims.
    pub(crate) const ANY: Self = Self {
        modk: None,
        alt: None,
        shift: None,
    };

    /// Does a real keypress with these modifier states reach this binding?
    pub(crate) fn accepts(self, modk: bool, alt: bool, shift: bool) -> bool {
        self.modk.is_none_or(|w| w == modk)
            && self.alt.is_none_or(|w| w == alt)
            && self.shift.is_none_or(|w| w == shift)
    }

    /// Every `(modk, alt, shift)` triple this predicate accepts. Three booleans, so the space is
    /// eight events wide and enumerating it is exact — no reasoning about guard algebra.
    fn matrix(self) -> Vec<(bool, bool, bool)> {
        let mut out = Vec::new();
        for modk in [false, true] {
            for alt in [false, true] {
                for shift in [false, true] {
                    if self.accepts(modk, alt, shift) {
                        out.push((modk, alt, shift));
                    }
                }
            }
        }
        out
    }

    /// Is there a keypress BOTH bindings answer? This is the collision relation.
    pub(crate) fn overlaps(self, other: Self) -> bool {
        self.matrix()
            .into_iter()
            .any(|(m, a, s)| other.accepts(m, a, s))
    }

    /// Is every keypress `self` answers already answered by `other`? Within one ordered `match`
    /// that means `self` is dead code.
    pub(crate) fn covered_by(self, other: Self) -> bool {
        self.matrix()
            .into_iter()
            .all(|(m, a, s)| other.accepts(m, a, s))
    }

    /// The arm guard AND the listener-level precondition above it.
    fn and(self, pre: Self) -> Self {
        fn one(arm: Option<bool>, pre: Option<bool>, what: &str) -> Option<bool> {
            match (arm, pre) {
                (Some(a), Some(p)) => {
                    assert_eq!(
                        a, p,
                        "T-703: an arm guard and its listener's precondition contradict on \
                         {what} — the arm can never run"
                    );
                    Some(a)
                }
                (Some(a), None) => Some(a),
                (None, p) => p,
            }
        }
        Self {
            modk: one(self.modk, pre.modk, "ctrl/meta"),
            alt: one(self.alt, pre.alt, "alt"),
            shift: one(self.shift, pre.shift, "shift"),
        }
    }

    /// Human form for a failure message.
    fn describe(self) -> String {
        let part = |v: Option<bool>, name: &str| match v {
            Some(true) => format!("+{name}"),
            Some(false) => format!("-{name}"),
            None => format!("?{name}"),
        };
        format!(
            "[{} {} {}]",
            part(self.modk, "mod"),
            part(self.alt, "alt"),
            part(self.shift, "shift")
        )
    }
}

/// Read a `match` arm guard (everything between `if` and `=>`) as a [`Mods`].
///
/// **Fail-closed.** A term this does not recognise PANICS rather than being ignored: a guard
/// silently read as "no constraint" would widen the binding and could turn a real collision
/// into a phantom one, and a guard silently dropped could hide one. Teach it the term.
fn parse_guard(guard: &str) -> Mods {
    let mut m = Mods::ANY;
    for raw in guard.split("&&") {
        let term: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
        if term.is_empty() {
            continue;
        }
        let (slot, want) = match term.as_str() {
            "modk" | "ev.ctrl_key()||ev.meta_key()" => (0, true),
            "!modk" | "!(ev.ctrl_key()||ev.meta_key())" => (0, false),
            "ev.alt_key()" => (1, true),
            "!ev.alt_key()" => (1, false),
            "ev.shift_key()" => (2, true),
            "!ev.shift_key()" => (2, false),
            other => panic!(
                "T-703: unreadable modifier guard term `{other}`. The collision census refuses \
                 to guess: a term it cannot read would silently widen or narrow the binding, \
                 and either way the answer it gives about collisions would be about code that \
                 is not there. Teach `parse_guard` the term."
            ),
        };
        let slotted = match slot {
            0 => &mut m.modk,
            1 => &mut m.alt,
            _ => &mut m.shift,
        };
        *slotted = Some(want);
    }
    m
}

/// One binding: a key code, the modifier predicate that reaches it, and where it lives.
#[derive(Clone, Debug)]
pub(crate) struct Binding {
    pub(crate) code: String,
    pub(crate) mods: Mods,
    pub(crate) file: &'static str,
    /// Index of the window-level listener within its file, in source order.
    pub(crate) listener: usize,
    /// Position within that listener — `match` arms are ordered and the order is load-bearing.
    pub(crate) order: usize,
    /// `ev.code()` or `ev.key()` — the accessor the listener reads.
    pub(crate) via: &'static str,
}

impl Binding {
    fn site(&self) -> String {
        format!("{}#{} ({})", self.file, self.listener, self.via)
    }
}

/// One window-level `keydown` closure: its scrubbed source and the bindings inside it.
pub(crate) struct Listener {
    pub(crate) file: &'static str,
    pub(crate) index: usize,
    pub(crate) src: String,
    pub(crate) bindings: Vec<Binding>,
}

/// The EDITOR SURFACE: every module that installs a window-level `keydown` while the Mission
/// Creator is up, with the number of such listeners each one is expected to install.
///
/// The count is declared, not merely observed, so that ADDING a listener is red until someone
/// has looked at whether it collides — which is the whole ticket. Modules whose Escape belongs
/// to the suite rather than the editor (`ui`'s `Dialog`/`Modal`, `layout`'s nav) are out of
/// scope here and stay out; they are gated on `modal_stack::is_topmost_open`, which is a
/// different (and stricter) discipline from this one.
///
/// **The membership test is "does `mission_editor` mount it", not "is it named like an editor
/// module".** T-774 found the list short by two on exactly that confusion: `faction_manager` and
/// `orbat_manager` each install a raw window-level keydown, both are mounted by
/// `MissionEditorPage`, and neither is gated on the modal stack — so both belong here, and
/// neither the scope note above nor the tripwire below had any way to say so. `orbat_manager`
/// hid the longest because `mission_editor` reaches it through a bare `pub use` re-export in
/// `eden_chrome`, so a symbol search for `OrbatManagerDialog` lands on the re-export and never
/// on the file that owns the listener. Grep for the LISTENER HEADS, not for the component.
fn editor_surface() -> Vec<(&'static str, &'static str, usize)> {
    vec![
        // The input layer's keyboard half carries BOTH window-level keydowns: the editor's
        // own chord closure and the undo/redo one that calls into
        // `bridge/document_host/history.rs`. Two
        // listeners in one file, adjudicated against each other like any other pair.
        (
            "window_keydown.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/input/window_keydown.rs"
            )),
            2,
        ),
        // T-946.86 — the gesture owner cancels its private Z/vertex arm on Escape.
        (
            "pointer_gestures.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/input/pointer_gestures.rs"
            )),
            1,
        ),
        // T-939.4 — and the page is BACK on the surface with one listener: the six Arrange
        // chords. It is not in `window_keydown.rs` because that file is another slice's `owns`,
        // and it is not in `top_strip.rs` (where the Arrange list lives) because the strip unmounts
        // behind the `chrome_hidden` gate and would take the chords with it. Being censused is
        // what matters — these six are adjudicated against every other binding in the editor by
        // `no_two_listeners_claim_the_same_chord` below, wherever the closure sits.
        (
            "mission_editor.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/mission_editor.rs"
            )),
            1,
        ),
        // T-934.11 — the asset picker / comment editor / connections panel (each installing
        // one Escape listener) moved out of `mission_editor.rs` into the canvas overlays file.
        (
            "overlays.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/bridge/overlays.rs"
            )),
            3,
        ),
        (
            "attributes.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
            )),
            1,
        ),
        (
            "top_strip_view.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/docks/top_strip/view.rs"
            )),
            1,
        ),
        (
            "context_menu.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/docks/context_menu.rs"
            )),
            1,
        ),
        (
            "mission_dialog.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/modals/settings_modal/mission_dialog.rs"
            )),
            1,
        ),
        (
            "preferences_dialog.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/modals/settings_modal/preferences_dialog.rs"
            )),
            1,
        ),
        (
            "all_settings_dialog.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/modals/settings_modal/all_settings_dialog.rs"
            )),
            1,
        ),
        (
            "faction_manager.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/modals/faction_manager.rs"
            )),
            1,
        ),
        (
            "dialog_lifecycle.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/v2/apps/editor/ui/modals/orbat_manager/dialog_lifecycle.rs"
            )),
            1,
        ),
    ]
}

/// How a window-level keydown closure is registered. Both idioms the frontend uses; a third
/// would be invisible to the census, so `every_editor_surface_listener_is_censused` counts.
const LISTENER_HEADS: [&str; 2] = [
    "window_event_listener(leptos::ev::keydown,",
    "Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(",
];

/// Listener-level modifier PRECONDITIONS: an early `return` above the arm list narrows every arm
/// in that closure, and reading only the arm heads would therefore over-claim.
/// The undo/redo listener bails on anything that is not Ctrl/Cmd-without-Alt before it looks at the
/// code at all, which is why its bare `"KeyZ"` arm is not the modifier-free claim it appears to
/// be.
///
/// Keyed by a needle that must appear in the closure's own source, so the entry can only apply
/// to code that is really there — and if the source is reworded, the precondition simply stops
/// applying and the arms widen, which produces MORE collisions, not fewer. Fail-closed.
const PRECONDITIONS: [(&str, Mods); 1] = [(
    "if !(ev.ctrl_key() || ev.meta_key()) || ev.alt_key() {",
    Mods {
        modk: Some(true),
        alt: Some(false),
        shift: None,
    },
)];

/// Escape is claimed by every dismissable surface in the editor, deliberately. See the module
/// docs; the two pins below stop this from becoming a place to bury a real collision.
pub(crate) const SHARED_CHANNELS: [&str; 1] = ["Escape"];

/// Small-integer spelling, for the prose counts the census derives. One copy, consumed by
/// `mission_editor`'s help-blurb pin too — the same discipline this whole module is about.
/// Deliberately narrow: a count outside the range panics with instructions rather than
/// silently spelling nothing.
pub(crate) fn spell(n: usize) -> String {
    const ONES: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 4] = ["twenty", "thirty", "forty", "fifty"];
    assert!(
        n < 60,
        "extend `spell` past {n} before the editor gets there"
    );
    if n < 20 {
        return ONES[n].to_string();
    }
    let tens = TENS[n / 10 - 2];
    if n.is_multiple_of(10) {
        tens.to_string()
    } else {
        format!("{tens}-{}", ONES[n % 10])
    }
}

/// Byte index of the `}` closing the `{` at `open`.
fn balanced(src: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in src.char_indices() {
        if i < open {
            continue;
        }
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Every window-level keydown closure body in `raw`, comment-scrubbed, in source order.
///
/// The closure body is sliced from the RAW source and scrubbed after, not before: `live_source`
/// cuts at the first `#[cfg(test)]` it sees and `mission_editor` has one on an inner
/// `clear_for_test` helper at line 88, so scrubbing the whole file first would hand back an
/// empty editor. Scrubbing from the listener head onward is safe — every one of these closures
/// closes long before its file's test module.
fn listener_bodies(raw: &str) -> Vec<String> {
    let mut heads: Vec<usize> = Vec::new();
    for head in LISTENER_HEADS {
        let mut from = 0usize;
        while let Some(i) = raw[from..].find(head) {
            heads.push(from + i);
            from += i + head.len();
        }
    }
    heads.sort_unstable();
    heads
        .into_iter()
        .filter_map(|start| {
            let live = live_source(&raw[start..]);
            let open = live.find('{')?;
            let end = balanced(&live, open)?;
            let body = live[open..=end].to_string();
            // T-776 — a keydown registration that reads neither `ev.key()` nor `ev.code()`
            // (or whose event parameter is not named `ev`) used to be silently DROPPED from
            // discovery. That is the hollow shape the census exists to eliminate: the listener
            // never inflated `found`, so every per-file count and the empty-bindings check
            // stayed green over an incomplete input. Fail closed — teach the census the new
            // idiom, or rename the parameter to `ev` and use those accessors.
            assert!(
                body.contains("ev.key()") || body.contains("ev.code()"),
                "T-776: a window-level keydown closure reads neither `ev.key()` nor                      `ev.code()` — it would have been silently dropped from the census. Body                      starts:\n{}",
                &body[..body.len().min(160)]
            );
            Some(body)
        })
        .collect()
}

/// Arm heads of the `match` opened by `head` inside `body`: `(literal, guard)`, in source order.
///
/// Only literals in ARM-HEAD position (the next non-space text is `=>`, `if ` or `|`) count — a
/// string constant inside an arm body is not a binding and must never be read as one.
fn match_arms(body: &str, head: &str) -> Vec<(String, Mods)> {
    let Some(at) = body.find(head) else {
        return Vec::new();
    };
    let rest = &body[at..];
    let end = rest.find("_ =>").map_or(rest.len(), |i| i + 4);
    let a: Vec<char> = rest[..end].chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < a.len() {
        if a[i] != '"' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        // A `KeyboardEvent` code literal carries no escapes, so a plain scan to the next quote
        // is exact for the arm heads; anything weirder is not an arm head anyway.
        while j < a.len() && a[j] != '"' {
            j += 1;
        }
        let lit: String = a[start..j].iter().collect();
        let mut k = j + 1;
        while k < a.len() && a[k].is_whitespace() {
            k += 1;
        }
        let peek: String = a[k..a.len().min(k + 3)].iter().collect();
        if peek.starts_with("=>") || peek.starts_with("if ") || peek.starts_with('|') {
            // The guard is whatever sits between this literal and the arm's `=>`. Taking the
            // LAST `if ` means an alternation (`"A" | "B" if g =>`) gives both literals the
            // guard that really applies to them.
            let mut p = k;
            while p + 1 < a.len() && !(a[p] == '=' && a[p + 1] == '>') {
                p += 1;
            }
            let head_tail: String = a[k..p].iter().collect();
            let mods = match head_tail.rfind("if ") {
                Some(g) => parse_guard(&head_tail[g + 3..]),
                None => Mods::ANY,
            };
            out.push((lit, mods));
        }
        i = j + 1;
    }
    out
}

/// Every `ev.key() == "X"` comparison in `body`. These carry no modifier guard whatsoever —
/// they answer `Ctrl+Esc` and `Shift+Esc` as readily as `Esc` — which is why they are recorded
/// as [`Mods::ANY`] rather than being quietly assumed bare.
fn key_equals(body: &str) -> Vec<String> {
    let needle = "ev.key() == \"";
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(i) = body[from..].find(needle) {
        let s = from + i + needle.len();
        let e = body[s..].find('"').map_or(body.len(), |d| s + d);
        out.push(body[s..e].to_string());
        from = e;
    }
    out
}

fn precondition(body: &str) -> Mods {
    let mut m = Mods::ANY;
    for (needle, pre) in PRECONDITIONS {
        if body.contains(needle) {
            m = m.and(pre);
        }
    }
    m
}

/// THE census: every window-level keydown listener in the editor surface, with its bindings.
pub(crate) fn listeners() -> Vec<Listener> {
    let mut out = Vec::new();
    for (file, raw, _expected) in editor_surface() {
        for (index, body) in listener_bodies(raw).into_iter().enumerate() {
            let pre = precondition(&body);
            let mut bindings = Vec::new();
            let mut push = |code: String, mods: Mods, via: &'static str, order: &mut usize| {
                bindings.push(Binding {
                    code,
                    mods,
                    file,
                    listener: index,
                    order: *order,
                    via,
                });
                *order += 1;
            };
            let mut order = 0usize;
            for (code, mods) in match_arms(&body, "match ev.code().as_str() {") {
                push(code, mods.and(pre), "ev.code()", &mut order);
            }
            for (code, mods) in match_arms(&body, "match ev.key().as_str() {") {
                push(code, mods.and(pre), "ev.key()", &mut order);
            }
            for code in key_equals(&body) {
                push(code, pre, "ev.key()", &mut order);
            }
            out.push(Listener {
                file,
                index,
                src: body,
                bindings,
            });
        }
    }
    out
}

/// Every binding in the editor surface, flattened.
pub(crate) fn all_bindings() -> Vec<Binding> {
    listeners().into_iter().flat_map(|l| l.bindings).collect()
}

/// Every distinct key code the editor binds. This is what the T-692 coverage pins compare
/// [`super::SHORTCUTS`] against.
pub(crate) fn all_bound_codes() -> BTreeSet<String> {
    all_bindings().into_iter().map(|b| b.code).collect()
}

/// The window-level editor keydown's ARM LIST as TEXT, comment-scrubbed with the `"KeyX"` arm
/// literals kept.
///
/// This is the shape the older census pins in `mission_editor` grep against (they assert on
/// exact guard spellings such as `"KeyA" if modk && !ev.alt_key() && !ev.shift_key() =>`), and
/// it is the function that existed in four copies before T-703. It stays here, beside the
/// structured census, so there is exactly one of it.
///
/// The assertion is new: the first `match ev.code().as_str()` in a file must sit in the
/// PRODUCTION half. A census that slices a fixture out of a test module and reports on that is
/// the hollow-pin failure this programme keeps finding, and it costs one line to refuse.
pub(crate) fn keydown_arms(src: &str) -> String {
    let head = "match ev.code().as_str() {";
    let at = src.find(head).expect("an editor keydown match is present");
    assert!(
        !src[..at].contains("\n#[cfg(test)]"),
        "T-703: the first `match ev.code().as_str()` in this source sits after a top-level \
         `#[cfg(test)]` — the census would be reading a test fixture instead of the shipped \
         listener, which is a pin that proves nothing"
    );
    let rest = &src[at..];
    let end = rest.find("_ =>").map_or(rest.len(), |i| i + 4);
    live_source(&rest[..end])
}

#[cfg(test)]
mod tests;
