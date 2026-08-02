//! T-661 — the Bottom Toolbelt, split from `eden_chrome.rs`.
//!
//! T-636 splits the single floating pill into TWO mounts, mirroring Eden: the mode buttons
//! (Select / Ruler / LoS) live on a toolbar ([`ModeToolbar`]), and the numeric readouts (CUR / OBJ
//! / SEL / SZ) live in a full-width status bar docked at the bottom of the viewport ([`StatusBar`]).
//! Tools and telemetry are different jobs with different interaction models — one is a set of mode
//! toggles, the other a passive read-out — so conflating them in one ~580 px centred pill was the
//! defect (`editor_chrome_direction.md`). The operator's direction is explicit: the bottom bar
//! STAYS, its content and feel unchanged, stretched to span the viewport instead of floating centred.
//!
//! The status bar also carries the two natural homes the full-width geometry creates:
//!   * a left/centre slot for map furniture — the scale bar and grid references (T-667, wave 106;
//!     built here as an obvious empty slot, NOT filled), and
//!   * a right-end slot for a primary action on its own surface — Eden's `PLAY SCENARIO`; ours is
//!     `OPEN` per `editor_chrome_direction.md` §Open (the slot is built; what the button *does* is
//!     the undecided part of §Open).
//!   * the debug telemetry HUD (T-719) gets a legitimate visible slot in the right section, before
//!     OPEN, still behind its Ctrl+Alt+D toggle and the `chrome_hidden` gate — it was previously
//!     invisible, painted over by DockRight's z-20 column.
//!
//! Not cfg-gated: the native view shell renders both too (the doc-reading `sel_xyz` branch is
//! `#[cfg(target_arch = "wasm32")]` inside the memo).
#![allow(dead_code)]
use leptos::prelude::*;

use crate::ui::MaterialIcon;

// ── Toolbelt class recipes (React `overlay.ts`) ────────────────────────────────────────────────────

/// The floating mode-toolbar pill — `cn(overlayPanel, 'flex items-center gap-1 px-1.5 py-1.5')`.
/// This is the tools half of the old TOOLBELT recipe; the readouts half moved to the status bar.
const MODEBAR: &str = "pointer-events-auto rounded-xl border border-white/10 bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex items-center gap-1 px-1.5 py-1.5";

/// The full-width status bar surface — the `overlayDocked` glass (same tokens as the docks/strip),
/// stretched edge-to-edge across the bottom. `border-t` gives it the docked seam Eden's status bar
/// has; the height is `TOOLBELT_BAND_PX`-worth of chrome (see `eden_layout`).
const STATUSBAR: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-9 w-full items-center gap-3 border-t border-white/10 px-3";

/// A toolbelt tool button — active (Select) vs disabled stub (Ruler / LoS).
const TOOL_ACTIVE: &str =
    "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors bg-primary/20 text-primary";
const TOOL_DISABLED: &str = "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors text-on-surface-variant opacity-30 hover:bg-transparent";

/// Format a cursor axis for the mono readout. React `BottomToolbelt.fmtCoord`:
/// `n.toFixed(3).padStart(9, ' ')`, and the off-map cell is 7 spaces + an em dash. HTML collapses
/// the leading runs in both engines — `tabular-nums` does the real aligning — so this mirrors the
/// oracle rather than "fixing" it.
fn fmt_coord(v: Option<f64>) -> String {
    match v {
        Some(n) => format!("{n:>9.3}"),
        None => "       —".to_string(),
    }
}

/// The mode toolbar — Select (active) + Ruler / LoS disabled stubs. Tools only; different job from
/// the readouts, so a separate mount (T-636). It floats above the full-width [`StatusBar`], keeping
/// the operator's "content and feel unchanged" — these are the same three buttons in the same pill,
/// just no longer sharing the strip with telemetry.
#[component]
pub fn ModeToolbar() -> impl IntoView {
    view! {
        <div class=MODEBAR>
            <button type="button" class=TOOL_ACTIVE aria-pressed="true" title="Select">
                <MaterialIcon name="arrow_selector_tool" class="block text-base" />
                <span class="hidden sm:inline">"Select"</span>
            </button>
            <button type="button" class=TOOL_DISABLED disabled=true title="Ruler (soon)">
                <MaterialIcon name="straighten" class="block text-base" />
                <span class="hidden sm:inline">"Ruler"</span>
            </button>
            <button type="button" class=TOOL_DISABLED disabled=true title="Line of sight (soon)">
                <MaterialIcon name="visibility" class="block text-base" />
                <span class="hidden sm:inline">"LoS"</span>
            </button>
        </div>
    }
}

/// Full-width status bar — the mono CUR X/Y/Z + SEL/OBJ/SZ readout, plus the map-furniture slot
/// (T-667), the debug HUD slot (T-719), and the OPEN primary-action slot (§Open).
///
/// T-172 B2/B9: Z is DEM-fed (em-dash until the grid publishes / off-coverage), and with exactly
/// one slot selected the readout swaps CUR→SEL and shows that slot's x/y/z (React parity). The
/// per-axis `title="Cursor …"` handles stay constant — they are the frozen cur-smoke's DOM hooks.
#[component]
pub fn StatusBar(
    /// Cursor world position + DEM z, `None` when the pointer is off the map (em-dash cells).
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    /// Live selection mirror — drives the CUR↔SEL swap.
    selected_ids: RwSignal<Vec<String>>,
    /// T-172 B9 — debounced compiled-payload estimate (None → `—`).
    #[prop(optional)]
    sz_bytes: Option<RwSignal<Option<usize>>>,
    /// T-719 — the wgpu telemetry HUD string (`z … · c… · glyph … · … FPS · rf …ms`); empty until
    /// the rAF sampler has a value. Its own visibility is gated by `hud_shown` (Ctrl+Alt+D) so it
    /// only paints when the operator has asked for it AND has content.
    #[prop(optional)]
    debug_hud: Option<RwSignal<String>>,
    /// T-719 — the Ctrl+Alt+D toggle for the HUD (default hidden). Together with `chrome_hidden`
    /// (which unmounts the whole bar) this keeps the HUD behind exactly the gates T-635 pinned.
    #[prop(optional)]
    hud_shown: Option<RwSignal<bool>>,
) -> impl IntoView {
    // Exactly-one-selected → that slot's x/y/z from the doc. Recomputes on selection change AND
    // on the post-mutation selected_ids re-set (drag commit), so it never shows a stale position.
    // (`editor_ops` is wasm-only; the native view shell always renders CUR.)
    let sel_xyz = Memo::new(move |_| -> Option<(f64, f64, f64)> {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            #[cfg(target_arch = "wasm32")]
            {
                return crate::editor_ops::read_attrs(&ids[0]).map(|a| (a.x, a.y, a.z));
            }
        }
        let _ = ids;
        None
    });
    let axis_val = move |i: usize| match sel_xyz.get() {
        Some((x, y, z)) => fmt_coord(Some([x, y, z][i])),
        None => fmt_coord(cursor.get().and_then(|c| match i {
            0 => Some(c.0),
            1 => Some(c.1),
            _ => c.2,
        })),
    };
    view! {
        <div class=STATUSBAR>
            // ── Readouts (left) — the old pill's telemetry, verbatim ──────────────────────────────
            <div class="flex items-center gap-2 font-mono text-code-md text-on-surface-variant">
                <span class="text-outline" title="Cursor">
                    {move || if sel_xyz.get().is_some() { "SEL" } else { "CUR" }}
                </span>
                // T-159.22 — `title` (not `aria-label`): these are roleless `<span>`s, where an
                // `aria-label` is ignored by AT and would be a fake a11y name. `title` is a real
                // tooltip AND the CUR gate's DOM handle, matching the `title="Cursor"` idiom above.
                <span title="Cursor X">
                    "X"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(0)}</span>
                </span>
                <span title="Cursor Y">
                    "Y"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(1)}</span>
                </span>
                <span title="Cursor Z">
                    "Z"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(2)}</span>
                </span>
            </div>
            <span class="h-5 w-px bg-white/10"></span>
            <div
                class="flex items-center gap-2 font-mono text-code-md tabular-nums text-on-surface-variant"
                title="Placed slots on map / current selection"
            >
                <span>
                    "OBJ"
                    <span class="ml-1 text-on-surface">{move || obj_count.get()}</span>
                </span>
                <span>
                    "SEL"
                    <span class="ml-1 text-on-surface">{move || sel_count.get()}</span>
                </span>
                <span title="Estimated save payload">
                    "SZ"
                    <span class="ml-1 text-on-surface">
                        {move || {
                            sz_bytes
                                .and_then(|s| s.get())
                                .map_or_else(
                                    || "—".to_string(),
                                    crate::mission_size::format_bytes,
                                )
                        }}
                    </span>
                </span>
            </div>
            // ── Map-furniture slot (T-667, wave 106) — the scale bar + edge grid references land
            // here. Left EMPTY on purpose: the full-width bar is the natural home for them, so this
            // slice reserves the obvious slot (with the `flex-1` spacer that pushes the HUD + OPEN
            // to the right edge) rather than building furniture that is not this ticket. The
            // `data-*` hook lets T-667 target it without touching this component's structure again.
            <span class="h-5 w-px bg-white/10"></span>
            <div
                data-status-furniture
                class="flex min-w-0 flex-1 items-center gap-2 font-mono text-code-md text-outline"
                title="Scale bar and grid references (T-667)"
            >
            </div>
            // ── Debug HUD slot (T-719) — a legitimate VISIBLE home in the right section, before
            // OPEN. Before T-636 the HUD lived at `right-3 bottom-3` on the overlay with no z-index,
            // painted over by DockRight's z-20 column, so it was invisible. Inside the status bar it
            // is on the same surface as the readouts and can never be occluded. Still gated: it only
            // renders when the operator toggled it on (Ctrl+Alt+D → `hud_shown`) AND the sampler has
            // a non-empty string — and the whole bar is already behind `chrome_hidden`, so the T-635
            // gate stack (chrome_hidden AND hud_shown AND non-empty) is preserved.
            {move || {
                let text = debug_hud.map(|h| h.get()).unwrap_or_default();
                let on = hud_shown.map(|s| s.get()).unwrap_or(false);
                (on && !text.is_empty()).then(|| {
                    view! {
                        <div
                            data-status-hud
                            class="pointer-events-none flex items-center font-mono text-[11px] text-success/90"
                        >
                            {text}
                        </div>
                    }
                })
            }}
            // ── Primary-action slot (§Open) — Eden's bottom-right `PLAY SCENARIO` position, on its
            // own surface. Ours is OPEN. The SLOT is what this ticket builds; what the action does is
            // the undecided part of §Open, so the button is inert here (no handler exists in the
            // owned files) but occupies the real Eden slot with the real Eden weight.
            <button
                type="button"
                data-status-open
                class="flex items-center gap-1.5 rounded-md bg-primary/90 px-3 py-1 text-label-md font-medium text-on-primary transition-colors hover:bg-primary"
                title="Open"
            >
                <MaterialIcon name="folder_open" class="block text-base" />
                <span>"OPEN"</span>
            </button>
        </div>
    }
}

/// Back-compat shim for the pre-T-636 single-pill mount. `eden_chrome` re-exports this name (the
/// stable `crate::eden_chrome::*` import surface the T-661 split promised not to break), so it stays
/// a real public component. It is NOT the mount `mission_editor` uses — the split put the tools
/// ([`ModeToolbar`]) and the readouts ([`StatusBar`]) at two independent mount points, each behind
/// its own `chrome_hidden` gate — but keeping the symbol lets the re-export shim compile without
/// churning a file outside this ticket's scope. It composes the two halves so the name still means
/// "the whole bottom belt" for any caller that reaches for it.
#[component]
pub fn BottomToolbelt(
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    selected_ids: RwSignal<Vec<String>>,
    /// Forwarded to `StatusBar`. A required (non-optional) param here so the compat shim can hand it
    /// straight through — the live mount in `mission_editor` passes `sz_bytes` too, so this loses no
    /// generality; a caller with no size estimate can build its own `RwSignal::new(None)`.
    sz_bytes: RwSignal<Option<usize>>,
) -> impl IntoView {
    view! {
        <ModeToolbar />
        <StatusBar cursor sel_count obj_count selected_ids sz_bytes />
    }
}

/// T-636 — the split is a Leptos view whose innards are structural, so (following `eden_dock_right`
/// / `orbat_manager` precedent) it is pinned by SOURCE INSPECTION rather than a mount: a native test
/// cannot render it, but it can fail loudly if the two-mount structure, the reserved T-667 slot, the
/// T-719 HUD slot, or the §Open slot is unpicked.
///
/// **Every needle is assembled at run time.** This test searches the file it lives in, so a needle
/// spelled out contiguously would put itself in the haystack — an absence check could then never
/// pass. Needles are split/reassembled so the file's own prose never satisfies them (this program's
/// signature defect: a check reporting success over an input it never truly examined).
#[cfg(test)]
mod t636_status_bar {
    use crate::arsenal::class_r_scrub::{live_code, live_source};

    /// This module's file, with comments blanked but string literals KEPT — so the Tailwind class
    /// strings and the readout labels survive as structural landmarks for ordering proofs.
    fn src_kept() -> String {
        live_source(include_str!("eden_toolbelt.rs"))
    }

    /// (structure) The single conflated pill is split into TWO components — a tools mount and a
    /// readouts mount — which is what makes them two independent mount points in `mission_editor`.
    #[test]
    fn tools_and_readouts_are_two_separate_components() {
        let src = live_code(include_str!("eden_toolbelt.rs"));
        let mode_fn = format!("pub fn {}", "ModeToolbar(");
        let status_fn = format!("pub fn {}", "StatusBar(");
        assert!(
            src.contains(&mode_fn) && src.contains(&status_fn),
            "T-636: the belt must split into a ModeToolbar AND a StatusBar component"
        );
    }

    /// (no conflation) The tools mount carries ONLY tools — none of the CUR/OBJ/SEL/SZ readout
    /// labels leak into `ModeToolbar`; they all live in `StatusBar`. Proven by slicing each
    /// component body out of the string-kept source and checking where the labels land.
    #[test]
    fn mode_toolbar_holds_no_readouts_and_status_bar_holds_them() {
        let src = src_kept();
        let mode_at = src
            .find(&format!("fn {}", "ModeToolbar("))
            .expect("ModeToolbar present");
        let status_at = src
            .find(&format!("fn {}", "StatusBar("))
            .expect("StatusBar present");
        assert!(
            mode_at < status_at,
            "ModeToolbar must be defined before StatusBar"
        );
        let mode_body = &src[mode_at..status_at];
        let status_at2 = status_at;
        let compat_at = src
            .find(&format!("fn {}", "BottomToolbelt("))
            .expect("compat shim present");
        let status_body = &src[status_at2..compat_at];

        // The three tool controls live in the toolbar (Select active + Ruler/LoS stubs).
        for tool in ["Select", "Ruler", "LoS"] {
            assert!(
                mode_body.contains(tool),
                "ModeToolbar must carry the {tool} tool"
            );
        }
        // The readout labels must NOT be in the toolbar…
        for label in ["\"CUR\"", "\"OBJ\"", "\"SEL\"", "\"SZ\""] {
            // (labels appear as `"OBJ"` etc. in the view; `Cursor` titles are separate.)
            let bare = label.trim_matches('"');
            let quoted = format!("\"{bare}\"");
            assert!(
                !mode_body.contains(&quoted),
                "T-636: readout {bare} must not live in the tools mount (that conflation is the bug)"
            );
            // …and every readout label must be in the status bar.
            assert!(
                status_body.contains(&quoted),
                "T-636: readout {bare} must live in the full-width StatusBar"
            );
        }
        // The status bar spans the viewport (full width), not a centred fixed pill: its surface
        // recipe carries `w-full`, and the component wears that recipe.
        let recipe_at = src
            .find(&format!("const {}", "STATUSBAR"))
            .expect("STATUSBAR recipe present");
        let recipe = &src[recipe_at
            ..src[recipe_at..]
                .find(';')
                .map(|i| recipe_at + i)
                .unwrap_or(src.len())];
        assert!(
            recipe.contains("w-full"),
            "T-636: the STATUSBAR recipe must be full-width (w-full), stretched across the viewport"
        );
        assert!(
            status_body.contains("class=STATUSBAR"),
            "T-636: StatusBar must wear the full-width STATUSBAR recipe"
        );
    }

    /// (T-667) The map-furniture slot is RESERVED but EMPTY — an obvious home for the wave-106 scale
    /// bar + grid references, with the `flex-1` spacer that pushes the HUD + OPEN to the right edge.
    /// The `data-*` hook lets T-667 target it without re-touching this component.
    #[test]
    fn reserves_an_empty_t667_furniture_slot() {
        let src = src_kept();
        let hook = format!("data-status-{}", "furniture");
        let at = src
            .find(&hook)
            .expect("T-667: a reserved map-furniture slot must exist");
        // It carries the flex spacer that eats the middle of the bar.
        let window = &src[at..src[at..]
            .find("</div>")
            .map(|i| at + i)
            .unwrap_or(src.len())];
        assert!(
            window.contains("flex-1"),
            "T-667: the furniture slot must carry the flex-1 spacer (it owns the bar's middle)"
        );
        // And it is genuinely EMPTY — no scale-bar / grid content built this ticket. The opening
        // `<div … data-status-furniture …>` is immediately followed (ignoring whitespace) by its
        // `</div>`; there is no child element between them.
        let open_end = at + src[at..].find('>').expect("furniture div opens");
        let body = src[open_end + 1..]
            .split_once("</div>")
            .map(|(b, _)| b)
            .unwrap_or("");
        assert!(
            !body.contains('<'),
            "T-667: the furniture slot must be left EMPTY this ticket (reserve, do not build)"
        );
    }

    /// (§Open) The primary-action slot exists on its own surface at the right end — Eden's
    /// `PLAY SCENARIO` position; ours is OPEN. The slot is built; the button's behaviour is the
    /// undecided part of §Open, so it is inert here.
    #[test]
    fn builds_the_open_primary_action_slot() {
        let src = src_kept();
        let hook = format!("data-status-{}", "open");
        assert!(
            src.contains(&hook),
            "§Open: the primary-action slot (OPEN) must be built at the bar's right end"
        );
        let label = ["OP", "EN"].concat();
        let at = src.find(&hook).expect("open slot present");
        let window = &src[at..src[at..]
            .find("</button>")
            .map(|i| at + i)
            .unwrap_or(src.len())];
        assert!(
            window.contains(&label) && window.contains("folder_open"),
            "§Open: the slot must present an OPEN button (label + folder_open glyph)"
        );
    }

    /// (T-719) The debug HUD gets a legitimate VISIBLE home inside the status bar's right section,
    /// BEFORE the OPEN slot, gated on `hud_shown` (Ctrl+Alt+D) AND a non-empty sampler string. The
    /// `chrome_hidden` half of the gate is the StatusBar mount wrapper (pinned in `mission_editor`).
    #[test]
    fn hud_slot_is_gated_and_sits_before_open() {
        // Gate expression on scrubbed code (strings blanked) so it is the real gate, not a comment.
        let code = live_code(include_str!("eden_toolbelt.rs"));
        assert!(
            code.contains("on && !text.is_empty()"),
            "T-719: the HUD slot must render only when (hud_shown AND non-empty sampler string)"
        );
        // hud_shown / debug_hud are real optional props threaded into StatusBar.
        assert!(
            code.contains("hud_shown") && code.contains("debug_hud"),
            "T-719: StatusBar must accept the HUD toggle + text signals"
        );
        // Ordering: the HUD slot precedes the OPEN slot in the right section.
        let src = src_kept();
        let hud = src
            .find(&format!("data-status-{}", "hud"))
            .expect("HUD slot present");
        let open = src
            .find(&format!("data-status-{}", "open"))
            .expect("OPEN slot present");
        assert!(
            hud < open,
            "T-719: the HUD slot must sit BEFORE the OPEN slot"
        );
    }
}
