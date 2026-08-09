//! Small UI helpers ported from lib/utils.ts (`cn`) + components/MaterialIcon.tsx + AuthGate.tsx.
use crate::auth::AuthStore;
// T-633 — the ONE state vocabulary (T-668), consumed rather than re-invented. The dependency runs
// `ui` → `eden_layout` and it is deliberate: `eden_layout` owns the four named state recipes and is
// their single source of truth, so a shared primitive that re-typed `hover:bg-white/10` here would
// be a second definition of a rule the chrome files are pinned against. Nothing runtime crosses the
// boundary — these are `&'static str` class recipes resolved at compile time.
use crate::eden_layout::{DISABLED_GLYPH, HOVER_FILL};
use crate::nav::{has_min_role_authed, Role};
use leptos::prelude::*;

/// Neutral inline avatar (data URI) shown when a user has no Discord avatar — byte-identical to
/// lib/avatar.ts `DEFAULT_AVATAR` (`encodeURIComponent`-encoded SVG).
pub const DEFAULT_AVATAR: &str = "data:image/svg+xml;utf8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2064%2064%22%3E%3Crect%20width%3D%2264%22%20height%3D%2264%22%20rx%3D%228%22%20fill%3D%22%23394150%22%2F%3E%3Ccircle%20cx%3D%2232%22%20cy%3D%2225%22%20r%3D%2212%22%20fill%3D%22%237a8699%22%2F%3E%3Cpath%20d%3D%22M12%2058c0-11%209-19%2020-19s20%208%2020%2019z%22%20fill%3D%22%237a8699%22%2F%3E%3C%2Fsvg%3E";

/// Minimal class-string join (clsx-like): drop empties, space-join. NOTE: unlike the React `cn`
/// (clsx + tailwind-merge), this does NOT resolve Tailwind conflicts — the V gate proves the
/// shell's class combos have none; a twMerge-equivalent lands only if a conflicting combo appears.
pub fn cn(classes: &[&str]) -> String {
    classes
        .iter()
        .filter(|c| !c.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Material Symbols icon — a font-glyph span whose text is the ligature name. Ported from
/// MaterialIcon.tsx (`<span class="material-symbols-outlined …" style?>{name}</span>`). `filled`
/// renders the FILL-1 variant; React sets it via CSSOM (`el.style.fontVariationSettings`), which the
/// browser reflects onto the style attribute as `font-variation-settings: 'FILL' 1;` — matched here.
#[component]
pub fn MaterialIcon(
    name: &'static str,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] filled: bool,
) -> impl IntoView {
    let style = filled.then_some("font-variation-settings: \"FILL\" 1;");
    view! { <span class=cn(&["material-symbols-outlined", class]) style=style>{name}</span> }
}

/// Page title + optional subtitle header. Ported from components/PageHeader.tsx.
#[component]
pub fn PageHeader(title: &'static str, #[prop(optional)] subtitle: &'static str) -> impl IntoView {
    view! {
        <header class="mb-8">
            <h1 class="mb-2 text-3xl font-bold text-on-surface">{title}</h1>
            {(!subtitle.is_empty())
                .then(|| view! { <p class="max-w-3xl text-on-surface-variant">{subtitle}</p> })}
        </header>
    }
}

/// AuthGate — API-backed pages show a sign-in CTA for guests (and a "Loading session…" state while
/// bootstrapping), otherwise the children. Ported from components/AuthGate.tsx. Reactive on the
/// AuthStore so it flips to the content once a session lands.
#[component]
pub fn AuthGate(children: ChildrenFn) -> impl IntoView {
    let auth = expect_context::<AuthStore>();
    move || {
        if auth.bootstrapping.get() {
            view! {
                <div class="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
                    "Loading session…"
                </div>
            }
            .into_any()
        } else if !auth.is_authenticated() {
            view! {
                <div class="flex min-h-[40vh] flex-col items-center justify-center gap-4 text-center">
                    <p class="text-on-surface-variant">
                        "Sign in to load live data from the platform."
                    </p>
                    <a
                        href="/login"
                        class="rounded-lg bg-primary px-6 py-2.5 text-sm font-medium text-on-primary"
                    >
                        "Sign in with Discord"
                    </a>
                </div>
            }
            .into_any()
        } else {
            children().into_any()
        }
    }
}

/// Badge variant classes — components/ui/badge.tsx `badgeVariants` (cva) with the base merged in.
/// (React's twMerge collision quirks don't apply here: no caller passes a conflicting override.)
#[allow(dead_code)]
pub fn badge_class(variant: &str) -> String {
    let v = match variant {
        "primary" => "border-primary/30 bg-primary/10 text-primary",
        "tertiary" => "border-tertiary/30 bg-tertiary/10 text-tertiary",
        "warning" => "border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow",
        "success" => "border-success/30 bg-success/15 text-success",
        "error" => "border-error-alert/30 bg-error-alert/10 text-error-alert",
        _ => "border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant",
    };
    format!("inline-flex items-center gap-1 rounded border px-2 py-0.5 text-label-sm uppercase whitespace-nowrap {v}")
}

/* ═══════════════ T-633 — the suite's range + select primitives ═══════════════
 *
 * Two browser-chrome controls were rendering inside an otherwise custom UI: the editor's time
 * scrubber was a raw `<input type="range">` painting its track and thumb in the UA accent (browser
 * blue) against Aegis `#adc6ff`, and the weather picker was a raw `<select>` with the platform's
 * native arrow. Neither could be fixed where it stood, because THIS FILE — the suite's shared
 * primitives home (`MaterialIcon` / `PageHeader` / `AuthGate` / `Dialog` / `Sheet` / `AdminGate`) —
 * had no slider and no select to reach for. So the fix is to CREATE them, here, where the rest of
 * the platform can use them too; the top strip is then just the first caller.
 *
 * THE PERFORMANCE CONSTRAINT, because it shapes the API. The scrubber is dragged, and each drag
 * emits values at roughly 30/second into `eden_top_strip`'s `RowMirror` (dedupe → debounce →
 * single-flight). A "controlled" primitive — one that owns an internal `RwSignal`, writes it on
 * every event and re-renders itself from it — would put a Leptos render on every one of those
 * values and defeat the sequencing that exists to keep the wire quiet. So both controls stay
 * **native and uncontrolled**: the browser owns the drag, `prop:value` is a one-line DOM property
 * write from the caller's signal (exactly what the raw markup did), and the only handler is the
 * settle event. There is no `on:input` in either component and no signal of their own.
 *
 * STYLING IS `appearance-none` PLUS EXPLICIT PARTS, not a colour override. `accent-color` (what the
 * old scrubber used, `accent-[--color-primary]`) only tints the UA widget; the track geometry, the
 * thumb shape and the select's arrow are still the browser's. Painting the parts ourselves —
 * `::-webkit-slider-runnable-track` / `::-webkit-slider-thumb` and their `::-moz-range-*` twins, and
 * a Material `expand_more` glyph over an `appearance-none` select — is what actually takes the
 * controls off browser chrome and onto the Aegis palette.
 */

/// The scrubber's own geometry — the element box. Deliberately `bg-transparent`: the visible track
/// is the `::-*-track` pseudo-element below, so the element box is free to carry the T-668 hover
/// fill without double-painting the rail.
const SLIDER_BOX: &str = "h-5 cursor-pointer appearance-none rounded bg-transparent px-1 outline-none focus-visible:outline-1 focus-visible:outline-primary/60";
/// WebKit/Blink rail + handle. `-mt-1` is not decoration: `::-webkit-slider-thumb` is laid out
/// against the TOP of the runnable track, so a 12px thumb on a 4px rail needs `(4-12)/2 = -4px` to
/// sit on the rail's centre line. Firefox centres its thumb for us, hence no twin below.
const SLIDER_WEBKIT: &str = "[&::-webkit-slider-runnable-track]:h-1 [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-surface-container-highest [&::-webkit-slider-thumb]:-mt-1 [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary";
/// Gecko rail + handle. `border-0` is load-bearing — Firefox's default range thumb carries a UA
/// border that reads as a light halo on a dark surface if it is not cleared.
const SLIDER_MOZ: &str = "[&::-moz-range-track]:h-1 [&::-moz-range-track]:rounded-full [&::-moz-range-track]:bg-surface-container-highest [&::-moz-range-thumb]:size-3 [&::-moz-range-thumb]:appearance-none [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-primary";

/// Aegis range slider — the suite's `<input type="range">` primitive (T-633).
///
/// Uncontrolled by construction (see the block comment above): `value` is written to the DOM
/// `value` **property**, so a caller whose signal updates 30×/second costs one property write per
/// update and no re-render, and `on_change` fires on the native `change` event — the settle — not
/// on `input`. A caller that needs live-drag feedback should keep a local mid-drag preview signal
/// (the top strip's `HH:MM` label tracks settled `env` time via `doc_tick`, not thumb drag) rather
/// than asking for a per-pixel callback the debounce downstream would only have to throw away.
///
/// `value`/`on_change` are `i32` because a `step`-quantised range emits integers; a value the
/// control cannot have produced is dropped rather than guessed at.
///
/// State: [`HOVER_FILL`] and [`DISABLED_GLYPH`] — the T-668 vocabulary, not a local invention.
#[component]
#[allow(dead_code)]
pub fn Slider(
    /// Accessible name. Also the tooltip, and it is NOT gated on `!disabled` — T-668 rule (3): a
    /// control that cannot act must still explain itself.
    label: &'static str,
    min: i32,
    max: i32,
    #[prop(default = 1)] step: i32,
    /// Where the handle sits. Read reactively; written straight to the DOM property.
    #[prop(into)]
    value: Signal<i32>,
    /// The settled value (native `change`). Never `input` — see the module note.
    #[prop(into)]
    on_change: Callback<i32>,
    /// Extra classes — the caller owns the WIDTH (`w-28`, `w-full`, …), this owns the paint.
    #[prop(optional)]
    class: &'static str,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    view! {
        <input
            type="range"
            min=min
            max=max
            step=step
            aria-label=label
            title=label
            disabled=disabled
            class=cn(&[SLIDER_BOX, SLIDER_WEBKIT, SLIDER_MOZ, HOVER_FILL, DISABLED_GLYPH, class])
            prop:value=move || value.get().to_string()
            on:change=move |ev| {
                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                    on_change.run(v);
                }
            }
        />
    }
}

/// Aegis select — the suite's `<select>` primitive (T-633).
///
/// `appearance-none` kills the native arrow and the UA's own padding/background; the chevron is a
/// Material `expand_more` laid over the control and `pointer-events-none`, so clicking it still
/// opens the list. The element stays a real `<select>` — the popup is the platform's, which is the
/// correct trade: it is keyboard- and screen-reader-native, and it is the one part of the control
/// no page CSS can reach anyway. `<option>`s carry the dark surface explicitly, because a UA that
/// paints its list from the page (Chrome on Linux) otherwise renders white-on-white.
///
/// `options` is a `&'static` `(value, label)` table — the `MENUS`-style const idiom, so a caller's
/// option set is data with one definition rather than markup repeated per call site.
///
/// State: [`HOVER_FILL`] and [`DISABLED_GLYPH`], same as [`Slider`].
#[component]
#[allow(dead_code)]
pub fn Select(
    /// Accessible name, and the tooltip — retained while `disabled` (T-668 rule 3).
    label: &'static str,
    /// `(wire value, human label)`, rendered in order.
    options: &'static [(&'static str, &'static str)],
    /// The selected wire value. Written to the DOM property, so a value not in `options` shows as
    /// no selection rather than silently rewriting the document.
    #[prop(into)]
    value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    view! {
        // `peer` + sibling `peer-disabled:` — DISABLED_GLYPH dims the <select> only; without the
        // peer variant the Material chevron stays full-lit beside a 30%-opacity control (T-751).
        <span class="relative inline-flex items-center">
            <select
                aria-label=label
                title=label
                disabled=disabled
                class=cn(
                    &[
                        "peer appearance-none rounded border border-outline-variant/40 bg-surface-container py-0.5 pr-6 pl-1.5 text-xs text-on-surface outline-none focus-visible:outline-1 focus-visible:outline-primary/60",
                        HOVER_FILL,
                        DISABLED_GLYPH,
                        class,
                    ],
                )
                prop:value=move || value.get()
                on:change=move |ev| on_change.run(event_target_value(&ev))
            >
                {options
                    .iter()
                    .map(|(v, l)| {
                        view! {
                            <option class="bg-surface-container text-on-surface" value=*v>
                                {*l}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
            <MaterialIcon
                name="expand_more"
                class="pointer-events-none absolute right-0.5 text-base leading-none text-on-surface-variant peer-disabled:opacity-30"
            />
        </span>
    }
}

/* ═══════════════ T-700 — one search box ═══════════════
 *
 * There are at least five hand-rolled search inputs in the editor alone: three in the asset dock
 * (`eden_dock_right`, T-084's operator grammar), the Locations filter (`eden_dock_left`, T-696) and
 * the Layers filter (`eden_dock_left`, T-637) — plus more across the suite (`missions`, `audit`,
 * `personnel`, `leaderboards`, `arsenal`). Every one is its own `<input type="search">` with its own
 * copy of the border/background/placeholder recipe, its own `aria-label` habit, and — uniformly —
 * no way to clear it but selecting the text and pressing Backspace. Five definitions of one control
 * is five places a change has to land and four places it can be forgotten.
 *
 * So this is the sixth-and-last: the primitive, here beside [`Slider`] and [`Select`], where the
 * whole suite can reach it. It follows T-633's conventions on purpose — `label` doubles as the
 * accessible name AND the tooltip and is NOT gated on `!disabled` (T-668 rule 3), state comes from
 * the [`HOVER_FILL`]/[`DISABLED_GLYPH`] vocabulary rather than a local re-typing of
 * `hover:bg-white/10`, and the caller owns the width.
 *
 * TWO DELIBERATE DEPARTURES from Slider/Select, both forced by what a search box is:
 *
 *   1. **`on_input`, not `on_change`.** T-633 bound the settle event because its caller was a
 *      dragged scrubber feeding a debounced wire. A filter box is the opposite: the list must narrow
 *      as the operator types, and `change` on a text input does not fire until blur. The name says
 *      which event it is so no caller has to read the body to find out.
 *   2. **`class` lands on the WRAPPER, not the input.** The control is three elements (glyph ·
 *      input · clear) in a `relative` box; the input is `w-full` inside it, so a caller's `w-64`
 *      has to size the box or it sizes nothing. Slider/Select are single elements and had no such
 *      split.
 *
 * The clear button is the one thing the primitive ADDS over the five it replaces, and it is why the
 * UA's own affordance is switched off: WebKit's `::-webkit-search-cancel-button` is browser chrome
 * in the exact sense T-633's block comment objects to — it does not exist in Firefox at all, so
 * "can I clear this filter without selecting the text" currently has a different answer per engine.
 */

/// The search field's own box. `pl-7`/`pr-7` are not padding taste: they are the gutters the
/// leading glyph and the trailing clear button sit in, so text can never slide under either.
const SEARCH_BOX: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 py-1.5 pl-7 pr-7 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60";
/// `type="search"` is kept — it is what tells assistive tech and the browser's own autofill that
/// this is a search field — but its two WebKit decorations are switched off, because the clear
/// affordance below is ours and must look and behave the same on every engine.
const SEARCH_UA_PARTS: &str = "[&::-webkit-search-cancel-button]:appearance-none [&::-webkit-search-decoration]:appearance-none";

/// Aegis search box — the suite's one `<input type="search">` (T-700).
///
/// Fires [`on_input`](SearchBoxProps::on_input) on every keystroke (see the block comment above:
/// `change` would not fire until blur, and a filter that narrows on blur is not a filter). The
/// value is written to the DOM `value` **property** from the caller's signal, exactly as
/// [`Slider`] and [`Select`] do, so the caller keeps ownership of the query string and this
/// component holds no state of its own.
///
/// The clear button is rendered only while there is something to clear, and it routes through the
/// SAME `on_input` callback — clearing is not a separate event a caller could forget to handle.
///
/// State: [`HOVER_FILL`] and [`DISABLED_GLYPH`] — the T-668 vocabulary, same as its two siblings.
// `dead_code` is allowed the same way [`Slider`] and [`Select`] allow it. Note what it does NOT
// reach: `#[component]` generates a `SearchBoxProps` struct, and because the allow makes this fn a
// non-root, rustc reports that struct's fields as never read until the FIRST caller lands. That
// warning is a true statement — every box this replaces lives in a file T-700 does not own (see
// the block comment for the five) — and it clears itself on the first adoption rather than being
// papered over here.
#[component]
#[allow(dead_code)]
pub fn SearchBox(
    /// Accessible name, and the tooltip — retained while `disabled` (T-668 rule 3). Also the stem
    /// of the clear button's own label, so a page with two filters does not present two buttons
    /// both called "Clear".
    label: &'static str,
    /// Visible hint. Defaults to empty rather than to a generic "Search…" — a filter that says
    /// what it filters ("Filter layers…") is the whole point of the prop.
    #[prop(optional)]
    placeholder: &'static str,
    /// The live query. Owned by the caller; read reactively, written to the DOM property.
    #[prop(into)]
    value: Signal<String>,
    /// Every keystroke, and the clear button's empty string.
    #[prop(into)]
    on_input: Callback<String>,
    /// Extra classes for the WRAPPER — the caller owns the width (`w-full`, `w-64`, `mt-1`, …).
    #[prop(optional)]
    class: &'static str,
    #[prop(optional)] disabled: bool,
    /// `data-testid` for the gate harness. Omitted entirely when empty, so a `[data-testid]`
    /// selector cannot match an unlabelled box.
    #[prop(optional)]
    test_id: &'static str,
) -> impl IntoView {
    view! {
        <span class=cn(&["relative flex items-center", class])>
            <MaterialIcon
                name="search"
                class="pointer-events-none absolute left-1.5 text-base leading-none text-outline"
            />
            <input
                type="search"
                aria-label=label
                title=label
                placeholder=placeholder
                disabled=disabled
                data-testid=(!test_id.is_empty()).then_some(test_id)
                class=cn(&[SEARCH_BOX, SEARCH_UA_PARTS, HOVER_FILL, DISABLED_GLYPH])
                prop:value=move || value.get()
                on:input=move |ev| on_input.run(event_target_value(&ev))
            />
            <Show when=move || !disabled && !value.get().is_empty()>
                <button
                    type="button"
                    aria-label=format!("Clear {label}")
                    title="Clear"
                    class=cn(&["absolute right-1 rounded-sm p-0.5 text-outline", HOVER_FILL])
                    on:click=move |_| on_input.run(String::new())
                >
                    <MaterialIcon name="close" class="text-base leading-none" />
                </button>
            </Show>
        </span>
    }
}

/// ═══════════════ T-333 — one Escape, one dialog ═══════════════
///
/// [`Dialog`] and [`Sheet`] each install a **window-level** `keydown` listener, one per instance.
/// Every listener sees every Escape, and each one's only guard was "am I open?", so pressing Esc
/// with a confirm stacked on an edit form closed **both** — the operator lost unsaved edits to
/// dismiss a confirmation. Found by T-226, which could not fix it (`ui.rs` was not in its owns) and
/// worked around it by driving the confirm through clicks instead.
///
/// There is no modal-stack primitive underneath to lean on: these are hand-rolled overlays, not
/// `@base-ui/react` — that dependency died with the React app at T-159.29.3, and the Leptos port
/// reimplemented the dismissal by hand (see the comment this replaces). So the stack is ours to
/// hold, and this is it.
///
/// **Topmost means topmost on screen, not most-recently-opened.** Every overlay in this file is
/// `z-50`; with equal z-index the browser paints in DOM order, which is why `event_manager.rs`
/// declares its detach confirm *last in the tree on purpose* and says so. Leptos mounts in tree
/// order, so registration order **is** paint order, and the dialog the user sees on top is the
/// last-registered one that is currently open. A most-recently-opened stack would be the wrong
/// answer for a pair declared in one order and opened in the other: it would route Escape to a
/// dialog painted underneath.
///
/// **Out-of-order close is not a special case here, by construction.** Nothing is pushed on open or
/// popped on close: openness is read *live* at keydown time, so a dialog that closes from the
/// middle of the stack simply stops being a candidate on the next keystroke, and one that reopens
/// becomes a candidate again — in its original paint position, which is where it reopens on screen.
/// Unmount removes **by id**, not by popping, so components torn down in any order leave a
/// consistent registry. Both the listener and the registration are released in the same
/// [`on_cleanup`], so neither can outlive the component.
pub(crate) mod modal_stack {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// One registered overlay. `is_open` is read *live* (see below); `open_seq` is the monotonic
    /// stamp of the last time this overlay was observed transitioning closed→open, or `0` while it
    /// reports closed. Registration (mount) order and open order are **different** questions — Esc
    /// routing wants the former (T-333, unchanged), z-index wants the latter (T-786).
    struct Entry {
        id: u64,
        is_open: Rc<dyn Fn() -> bool>,
        /// `Cell` so [`reconcile_open_order`] can stamp it without a `&mut` to the whole registry:
        /// the reconcile has to call `is_open` (arbitrary user code) with the registry *not*
        /// borrowed, then write the stamps back.
        open_seq: Cell<u64>,
    }

    thread_local! {
        /// Overlays in mount order. Not a `Vec<bool>`: the flag has to be read at keydown time, not
        /// cached at registration time, or a dialog closed by a button would still be holding
        /// Escape. `open_seq` rides alongside for the z-index question (T-786).
        static REGISTRY: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
        static NEXT_ID: Cell<u64> = const { Cell::new(1) };
        /// Monotonic open-order clock. Bumped every time [`reconcile_open_order`] catches an overlay
        /// on its closed→open edge; the overlay with the highest live stamp is the last-opened, and
        /// so the one that paints on top (T-786 O-3: "a real modal stack — last-opened wins").
        static OPEN_CLOCK: Cell<u64> = const { Cell::new(0) };
    }

    /// Register an overlay and return the id that identifies it for the rest of its life.
    pub(crate) fn register(is_open: impl Fn() -> bool + 'static) -> u64 {
        let id = NEXT_ID.with(|n| {
            let id = n.get();
            n.set(id + 1);
            id
        });
        REGISTRY.with_borrow_mut(|r| {
            r.push(Entry {
                id,
                is_open: Rc::new(is_open),
                open_seq: Cell::new(0),
            });
        });
        id
    }

    /// Drop an overlay's registration. Removes by id, so unmount order does not matter, and is a
    /// no-op on an id that is already gone (double cleanup must not panic or evict a stranger).
    pub(crate) fn unregister(id: u64) {
        REGISTRY.with_borrow_mut(|r| r.retain(|e| e.id != id));
    }

    /// Whether `id` is the last-registered overlay that is currently open — i.e. the one painted
    /// on top. False when nothing is open, and false for `id`s that are not registered.
    ///
    /// The predicates are cloned out from under the borrow before any of them runs: they read
    /// Leptos signals, and a signal read is arbitrary user code. Evaluating them while the
    /// `RefCell` is borrowed would make a re-entrant `register` from inside one a panic instead of
    /// a merely surprising ordering.
    pub(crate) fn is_topmost_open(id: u64) -> bool {
        let entries: Vec<(u64, Rc<dyn Fn() -> bool>)> =
            REGISTRY.with_borrow(|r| r.iter().map(|e| (e.id, Rc::clone(&e.is_open))).collect());
        entries
            .iter()
            .rev()
            .find(|(_, is_open)| is_open())
            .is_some_and(|(top, _)| *top == id)
    }

    /// Whether any registered overlay currently reports open.
    ///
    /// T-726 — non-overlay Escape consumers (the editor's shared ruler/LoS/viewshed arm) consult
    /// this so an open dialog/menu/picker owns the keystroke. Overlay handlers themselves use
    /// [`is_topmost_open`]; this is the "is anyone claiming Esc?" half for listeners that are not
    /// stack entries.
    ///
    /// Same clone-out-before-call discipline as [`is_topmost_open`]: predicates may re-enter.
    #[allow(dead_code)]
    pub(crate) fn any_open() -> bool {
        let preds: Vec<Rc<dyn Fn() -> bool>> =
            REGISTRY.with_borrow(|r| r.iter().map(|e| Rc::clone(&e.is_open)).collect());
        preds.iter().any(|is_open| is_open())
    }

    /// How many overlays are registered — the leak check for the tests.
    ///
    /// Deliberately **not** `#[cfg(test)]`. `class_r_scrub::cut_test_module` treats the *first*
    /// `#[cfg(test)]` in a file as the start of the test half and drops everything after it, so a
    /// test-gated helper up here would hide `Dialog` and `Sheet` from every pin in this file. It
    /// fails closed — the wiring pin below goes red with "found 0" rather than quietly passing —
    /// but the cure is to keep the attribute out of the production half, not to weaken the pin.
    #[allow(dead_code)]
    pub(crate) fn depth() -> usize {
        REGISTRY.with_borrow(Vec::len)
    }

    /// Catch every overlay on its open/close edges and keep each `open_seq` current, then hand back
    /// `(id, open_seq)` for every overlay that is open right now.
    ///
    /// Newly-open overlays get the next tick of [`OPEN_CLOCK`]; overlays that have gone closed drop
    /// back to `0` so a reopen counts as a *fresh* open (last-opened, not last-ever-opened). Because
    /// z is read on essentially every render, an open edge is observed within a frame of happening,
    /// so this poll-at-query scheme tracks real open order without a signal-subscription plumbed
    /// through every call site — the registry is the only thing that has to change.
    ///
    /// The one thing it cannot see is a close **and** reopen collapsed into a single frame with no
    /// render (hence no query) between them: the poll only ever sees the final state, so such an
    /// overlay keeps the stamp it already had. That is not a real UI event — every `open` write in
    /// the editor is a signal set that schedules a render, and the z-consuming views re-run on it —
    /// so open order is always sampled between two distinct opens in practice.
    ///
    /// Same re-entrancy discipline as [`is_topmost_open`]: the predicates (arbitrary Leptos signal
    /// reads) are cloned out and called with the `RefCell` **not** borrowed; the stamps are written
    /// back through the per-entry `Cell` afterward, so a re-entrant `register` from inside a
    /// predicate cannot deadlock.
    fn reconcile_open_order() -> Vec<(u64, u64)> {
        let snapshot: Vec<(u64, Rc<dyn Fn() -> bool>, u64)> = REGISTRY.with_borrow(|r| {
            r.iter()
                .map(|e| (e.id, Rc::clone(&e.is_open), e.open_seq.get()))
                .collect()
        });
        // Decide the new stamp for each id with the registry unborrowed.
        let mut updates: Vec<(u64, u64)> = Vec::with_capacity(snapshot.len());
        let mut open_now: Vec<(u64, u64)> = Vec::new();
        for (id, is_open, prev) in snapshot {
            let new_seq = if is_open() {
                if prev == 0 {
                    OPEN_CLOCK.with(|c| {
                        let next = c.get() + 1;
                        c.set(next);
                        next
                    })
                } else {
                    prev // already open — keep the stamp it opened at
                }
            } else {
                0 // closed — clear so the next open is a fresh edge
            };
            updates.push((id, new_seq));
            if new_seq != 0 {
                open_now.push((id, new_seq));
            }
        }
        // Write the stamps back. Removals between snapshot and here are fine: we match by id.
        REGISTRY.with_borrow(|r| {
            for (id, seq) in &updates {
                if let Some(e) = r.iter().find(|e| e.id == *id) {
                    e.open_seq.set(*seq);
                }
            }
        });
        open_now
    }

    /// Whether `id` is the overlay that was opened **most recently** and is still open — the one
    /// that must paint on top (T-786). This is the *open-order* question and is deliberately
    /// separate from [`is_topmost_open`], which answers the *mount-order* question for Esc routing
    /// (T-333) and must not change: a pair mounted in one order but opened in the other needs
    /// opposite answers from the two, which is the whole point of O-3.
    pub(crate) fn is_top_by_open_order(id: u64) -> bool {
        reconcile_open_order()
            .into_iter()
            .max_by_key(|(_, seq)| *seq)
            .is_some_and(|(top, _)| top == id)
    }

    /// The z-index class an overlay should carry so the last-opened surface wins the paint order
    /// (T-786 O-3). The top-of-open-order surface sits at the modal tier (`z-50`, the value every
    /// overlay hard-coded before this ticket); any overlay open *underneath* a later one drops one
    /// tier to `z-40`, which is above page content but below the surface on top.
    ///
    /// Why a two-tier class and not `z-{50+rank}`: the sibling Arsenal/Attributes surface keeps its
    /// hard-coded `z-50` this wave (T-785 owns that file), so the surfaces that *do* consume this
    /// have to be able to go **below** 50 to let the Arsenal — opened last, over ORBAT — win. A
    /// consumer at open-order-top ties the Arsenal at 50 only when the Arsenal is closed, so the tie
    /// never decides a visible collision. `elementFromPoint(arsenal centre)` then lands in the
    /// Arsenal, which is the acceptance.
    pub(crate) fn z_class(id: u64) -> &'static str {
        if is_top_by_open_order(id) {
            "z-50"
        } else {
            "z-40"
        }
    }
}

/// Frosted, centered macOS modal — the components/ui/dialog.tsx port (T-159.25). Renders **no DOM
/// while closed** (transient overlay: V captures of default states are unaffected; base-ui's
/// enter/exit transition attributes are not replicated). Esc and the backdrop close it.
///
/// T-333: Escape is handled only when this is the topmost open overlay — see [`modal_stack`].
#[component]
#[allow(dead_code)]
pub fn Dialog(
    open: RwSignal<bool>,
    #[prop(optional)] title: &'static str,
    #[prop(optional)] description: &'static str,
    /// Extra classes on the popup (React `className`, e.g. `max-w-lg`).
    #[prop(optional)]
    class: &'static str,
    children: ChildrenFn,
) -> impl IntoView {
    // T-333 — `try_get_untracked` rather than `get_untracked`: the registration is dropped in
    // `on_cleanup`, which Leptos runs before the arena disposes the signal, but a disposed signal
    // read must answer "not open" rather than panic if that order ever changes.
    let modal_id = modal_stack::register(move || open.try_get_untracked().unwrap_or(false));
    // Esc closes (base-ui behavior). Window-level like React's focus-trap dismissal.
    let esc = leptos::prelude::window_event_listener(leptos::ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" && modal_stack::is_topmost_open(modal_id) {
            open.set(false);
        }
    });
    on_cleanup(move || {
        esc.remove();
        modal_stack::unregister(modal_id);
    });
    move || {
        open.get().then(|| {
            view! {
                <div
                    class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                    on:click=move |_| open.set(false)
                ></div>
                <div class=cn(
                    &[
                        "glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200",
                        class,
                    ],
                )>
                    {(!title.is_empty())
                        .then(|| {
                            view! {
                                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                                    <div class="min-w-0">
                                        <h2 class="text-headline-sm text-on-surface">{title}</h2>
                                        {(!description.is_empty())
                                            .then(|| {
                                                view! {
                                                    <p class="mt-1 text-label-md text-on-surface-variant">
                                                        {description}
                                                    </p>
                                                }
                                            })}
                                    </div>
                                    <button
                                        type="button"
                                        aria-label="Close"
                                        on:click=move |_| open.set(false)
                                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                                    >
                                        <MaterialIcon name="close" />
                                    </button>
                                </div>
                            }
                        })}
                    <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">{children()}</div>
                </div>
            }
        })
    }
}

/// macOS slide-over panel — the components/ui/sheet.tsx port (right side; `bleed` = children own
/// the full layout). Same no-DOM-while-closed / no-transition-attrs notes as `Dialog`.
///
/// **T-333 covers this too, and had to.** The ticket names `Dialog`, but `Sheet` is the same
/// window-level-listener-per-instance shape at the same `z-50`, and the two stack on each other in
/// production: `missions.rs:789` opens the mission dossier in a `Sheet` and the armory/delete
/// confirms are `Dialog`s over it. Fixing only `Dialog` would have left Escape closing the dossier
/// out from under an open confirm — the same defect, one component over. One registry, both
/// components.
#[component]
#[allow(dead_code)]
pub fn Sheet(
    open: RwSignal<bool>,
    #[prop(optional)] title: &'static str,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] bleed: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let modal_id = modal_stack::register(move || open.try_get_untracked().unwrap_or(false));
    let esc = leptos::prelude::window_event_listener(leptos::ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" && modal_stack::is_topmost_open(modal_id) {
            open.set(false);
        }
    });
    on_cleanup(move || {
        esc.remove();
        modal_stack::unregister(modal_id);
    });
    move || {
        open.get().then(|| {
            view! {
                // T-173 P5 — no backdrop-filter on either overlay or the sliding panel: two
                // stacked blurs recomputed per translate frame were the sheet-enter hitch. The
                // scrim carries the dimming; the panel gets an opaque surface (same border stack).
                <div
                    class="animate-overlay-fade fixed inset-0 z-50 bg-black/60 transition-opacity duration-300"
                    on:click=move |_| open.set(false)
                ></div>
                <div class=cn(
                    &[
                        "animate-sheet-in fixed z-50 flex flex-col border border-outline-variant/30 bg-surface-container shadow-2xl outline-none transition-transform duration-300 ease-out inset-y-0 right-0 h-full w-[92vw] max-w-md border-l",
                        class,
                    ],
                )>
                    {if bleed {
                        children().into_any()
                    } else {
                        view! {
                            {(!title.is_empty())
                                .then(|| {
                                    view! {
                                        <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                                            <div class="min-w-0">
                                                <h2 class="text-headline-sm text-on-surface">{title}</h2>
                                                {(!description.is_empty())
                                                    .then(|| {
                                                        view! {
                                                            <p class="mt-1 text-label-md text-on-surface-variant">
                                                                {description}
                                                            </p>
                                                        }
                                                    })}
                                            </div>
                                            <button
                                                type="button"
                                                aria-label="Close"
                                                on:click=move |_| open.set(false)
                                                class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                                            >
                                                <MaterialIcon name="close" />
                                            </button>
                                        </div>
                                    }
                                })}
                            <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                                {children()}
                            </div>
                        }
                            .into_any()
                    }}
                </div>
            }
        })
    }
}

/// AdminGate — AuthGate + an admin-role check. Ported from components/AdminGate.tsx: authed
/// non-admins see "Admin access required." instead of the children.
///
/// T-458: uses [`has_min_role_authed`] (`None=>false`), not browse-mode [`crate::nav::has_min_role`]
/// / `AuthStore::has_min_role` (`None=>true`). The reactive `move ||` + `auth.user.get()` re-reads
/// after bootstrap so a pre-session guest cannot flash admin children.
#[component]
pub fn AdminGate(children: ChildrenFn) -> impl IntoView {
    view! {
        <AuthGate>
            {
                let children = children.clone();
                move || {
                    let auth = expect_context::<AuthStore>();
                    if has_min_role_authed(auth.user.get().map(|u| u.role), Role::Admin) {
                        children().into_any()
                    } else {
                        view! {
                            <div class="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
                                "Admin access required."
                            </div>
                        }
                        .into_any()
                    }
                }
            }
        </AuthGate>
    }
}

#[cfg(test)]
mod tests {
    /// Strip `//` / `/* */` so bans cannot false-red on doc comments (T-457 / T-458).
    fn strip_rust_comments(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let mut chars = src.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '/' {
                match chars.peek() {
                    Some('/') => {
                        chars.next();
                        while let Some(n) = chars.next() {
                            if n == '\n' {
                                out.push('\n');
                                break;
                            }
                        }
                        continue;
                    }
                    Some('*') => {
                        chars.next();
                        while let Some(n) = chars.next() {
                            if n == '*' && matches!(chars.peek(), Some('/')) {
                                chars.next();
                                break;
                            }
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            out.push(c);
        }
        out
    }

    fn collapse_ws(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// T-458 / T-460 Class-R — AdminGate must not use browse-mode `has_min_role(None)=>true`.
    /// Binds to the live `if` condition (same spirit as T-457 wiki Memo bind): a dead
    /// `has_min_role_authed(...)` pin beside `if true` must FAIL. Bans browse-mode one-shot.
    #[test]
    fn admin_gate_uses_authed_reactive_role() {
        const SRC: &str = include_str!("ui.rs");
        let production = SRC
            .split("mod tests {")
            .next()
            .expect("tests module marker");
        let code = collapse_ws(&strip_rust_comments(production));
        // T-460: require the live `if` — presence of the helper call alone is false-green.
        assert!(
            code.contains("if has_min_role_authed(auth.user.get().map(|u| u.role), Role::Admin)"),
            "AdminGate must gate via `if has_min_role_authed(auth.user.get()…, Role::Admin)` \
             (dead pin + if true is a fail; browse-mode None=>true is a fail)"
        );
        // Mask the authed helper so a free `has_min_role(` / one-shot store call stands out.
        let masked = code.replace("has_min_role_authed", "HAS_MIN_ROLE_AUTHED");
        assert!(
            !masked.contains("has_min_role("),
            "AdminGate production must not call browse-mode has_min_role( — use has_min_role_authed only"
        );
        // Split the needle so this assert's own source text cannot false-red the include_str scan.
        let one_shot = format!("auth.has_min_role({}::Admin)", "Role");
        assert!(
            !code.contains(&one_shot),
            "auth.has_min_role(Admin) is browse-mode None=>true (T-286/T-454 contract)"
        );
    }

    /* ═══════════════ T-700 — the shared search box ═══════════════ */

    /// The primitive's contract, pinned where `cargo test` cannot instantiate a `view!` tree.
    ///
    /// Scrubbed through `class_r_scrub` rather than this module's own `strip_rust_comments`,
    /// because `scrub` cuts the TEST MODULE as its first pass — a bare `include_str!` scan would
    /// otherwise be satisfied by the needles written in this very function (T-759).
    #[test]
    fn the_search_box_fires_on_every_keystroke_and_clears_through_the_same_callback() {
        use crate::arsenal::class_r_scrub::{live_code, live_source, only_item};
        let code = only_item(&live_code(include_str!("ui.rs")), "pub fn SearchBox(").to_string();
        // `live_source` keeps string literals — every assertion below that is ABOUT a class recipe,
        // an element type or user-visible copy has to read this one, not `code`.
        let src = only_item(&live_source(include_str!("ui.rs")), "pub fn SearchBox(").to_string();
        // (1) It narrows as you type. `change` on a text input does not fire until blur, so a
        // search box bound to the settle event is a search box that does nothing while you use it.
        assert!(
            code.contains("on:input=move |ev| on_input.run(event_target_value(&ev))"),
            "SearchBox must emit on every keystroke; body was:\n{code}"
        );
        assert!(
            !code.contains("on:change="),
            "SearchBox must not bind the settle event — that is Slider/Select's constraint, not \
             a filter's"
        );
        // (2) Clearing is not a second, separate event. A caller wires ONE callback and gets both.
        assert!(
            code.contains("on:click=move |_| on_input.run(String::new())"),
            "the clear button must route through the same on_input callback"
        );
        // (3) No state of its own — the caller owns the query, exactly like Slider and Select.
        assert!(
            !code.contains("RwSignal::new(") && !code.contains("signal("),
            "SearchBox must stay uncontrolled: a private copy of the query is a second source of \
             truth for it"
        );
        // (4) T-633's precedent: the T-668 state vocabulary is CONSUMED, not re-typed here.
        assert!(
            code.contains("DISABLED_GLYPH") && code.contains("HOVER_FILL"),
            "SearchBox must compose the eden_layout state recipes"
        );
        assert!(
            !src.contains("disabled:opacity") && !src.contains("hover:bg-white/10"),
            "a second definition of a T-668 recipe is the drift T-633 imported them to prevent"
        );
        // (5) The element itself, and the two switched-off WebKit parts. The recipe is a const
        // beside the component (T-633's `SLIDER_*` idiom), so the composition is asserted on the
        // fn and the contents on the const — a `cn(&[…])` that dropped the const would pass a
        // whole-file scan.
        let search_type = format!("type=\"{}\"", "search");
        assert!(
            src.contains(&search_type),
            "the element stays a real search input — that is what assistive tech reads"
        );
        assert!(
            src.contains("SEARCH_UA_PARTS"),
            "the input must compose the UA-parts recipe"
        );
        let recipe = live_source(include_str!("ui.rs"));
        let recipe = recipe
            .split("SEARCH_UA_PARTS: &str =")
            .nth(1)
            .and_then(|t| t.split(';').next())
            .expect("SEARCH_UA_PARTS must be declared");
        for part in [
            "::-webkit-search-cancel-button",
            "::-webkit-search-decoration",
        ] {
            assert!(
                recipe.contains(part),
                "{part} must be switched off — the clear affordance is ours, and WebKit's does \
                 not exist in Firefox at all; recipe was:\n{recipe}"
            );
        }
        // (6) Tooltip survives `disabled` (T-668 rule 3), same as Slider/Select.
        assert!(
            src.contains("title=label"),
            "a control that cannot act must still explain itself"
        );
    }

    /* ═══════════════ T-333 — Esc must reach exactly one dialog ═══════════════ */

    use super::modal_stack;
    use std::cell::Cell;
    use std::rc::Rc;

    /// A stand-in for a `RwSignal<bool>` the component reads at keydown time.
    fn overlay(open: bool) -> (Rc<Cell<bool>>, u64) {
        let flag = Rc::new(Cell::new(open));
        let read = Rc::clone(&flag);
        let id = modal_stack::register(move || read.get());
        (flag, id)
    }

    /// The bug, stated as a test. Before T-333 every open overlay answered Escape, so *both* of
    /// these would have closed; the guard is `is_topmost_open`, and only the confirm may say true.
    ///
    /// RED on the unfixed component is the second half of this: the source pin below proves the
    /// guard is actually in the keydown closure, because a passing stack with an unwired component
    /// is exactly the "reports success over an input it never examined" failure this repo hunts.
    #[test]
    fn only_the_topmost_open_overlay_answers_escape() {
        let start = modal_stack::depth();
        // Mount order = paint order: the form is declared first, the confirm last (the arrangement
        // `event_manager.rs` documents at its detach dialog).
        let (form, form_id) = overlay(true);
        let (confirm, confirm_id) = overlay(true);

        assert!(
            modal_stack::is_topmost_open(confirm_id),
            "the last-registered open overlay is the one painted on top"
        );
        assert!(
            !modal_stack::is_topmost_open(form_id),
            "the form behind an open confirm must not answer Escape — this is T-333"
        );

        // Esc dismisses the confirm only; the form is still open and now becomes the target.
        confirm.set(false);
        assert!(modal_stack::is_topmost_open(form_id));
        assert!(!modal_stack::is_topmost_open(confirm_id));

        // Nothing open → Escape has no owner at all.
        form.set(false);
        assert!(!modal_stack::is_topmost_open(form_id));
        assert!(!modal_stack::is_topmost_open(confirm_id));

        modal_stack::unregister(form_id);
        modal_stack::unregister(confirm_id);
        assert_eq!(modal_stack::depth(), start, "registrations must not leak");
    }

    /// Closed out of order: the overlay *underneath* goes away first (its own Cancel button, or a
    /// save that dismisses the form while the confirm it launched is still up). The survivor must
    /// still own Escape, and the departed one must not come back as topmost when it reopens under
    /// something else.
    #[test]
    fn a_dialog_closed_out_of_order_leaves_the_stack_consistent() {
        let start = modal_stack::depth();
        let (form, form_id) = overlay(true);
        let (confirm, confirm_id) = overlay(true);

        form.set(false); // the middle of the stack leaves while the top is still up
        assert!(modal_stack::is_topmost_open(confirm_id));
        assert!(!modal_stack::is_topmost_open(form_id));

        // …and reopens underneath. Paint order is unchanged, so it is still not the top.
        form.set(true);
        assert!(modal_stack::is_topmost_open(confirm_id));
        assert!(!modal_stack::is_topmost_open(form_id));

        // Unmount out of order too: the top component is torn down first, by id, not by popping.
        modal_stack::unregister(confirm_id);
        assert!(
            modal_stack::is_topmost_open(form_id),
            "removing a registration from the top must promote the one below, not orphan it"
        );
        assert!(
            !modal_stack::is_topmost_open(confirm_id),
            "an unregistered id must never be reported as topmost"
        );
        confirm.set(true); // a stale handle to a torn-down overlay changes nothing
        assert!(modal_stack::is_topmost_open(form_id));

        modal_stack::unregister(form_id);
        modal_stack::unregister(form_id); // double cleanup is a no-op, not a panic
        assert_eq!(modal_stack::depth(), start, "registrations must not leak");
        assert!(!modal_stack::is_topmost_open(form_id));
    }

    /// **The wiring.** The stack above is only a fix if the components consult it. Both `Dialog`
    /// and `Sheet` install a window-level listener, so both must carry the guard; a keydown closure
    /// that still reads `if open.get_untracked() && ev.key() == "Escape"` and nothing else is the
    /// unfixed component.
    ///
    /// Scrubbed source (T-601/T-622 `class_r_scrub`), and `live_code` at that, so the needle cannot
    /// be satisfied by this doc comment, by a string literal, or by an item the build drops.
    #[test]
    fn both_overlay_components_gate_escape_on_the_modal_stack() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let prod = live_code(include_str!("ui.rs"));
        for component in ["pub fn Dialog(", "pub fn Sheet("] {
            let body = only_body(&prod, component);
            assert!(
                body.contains("modal_stack::register("),
                "{component} must register with the modal stack. Body was: {body}"
            );
            assert!(
                body.contains("modal_stack::is_topmost_open(modal_id)"),
                "{component} must gate its Escape handler on being topmost (T-333). \
                 Body was: {body}"
            );
            assert!(
                body.contains("modal_stack::unregister(modal_id)"),
                "{component} must release its registration on cleanup or the registry leaks \
                 one dead entry per mount. Body was: {body}"
            );
        }
    }
    /// T-726 — stacked dialogs: prefs over settings. One Escape must belong to the topmost only.
    /// Mirrors EditorPreferencesDialog mounting after MissionSettingsDialog (eden_settings).
    #[test]
    fn stacked_dialogs_only_topmost_answers_escape() {
        let start = modal_stack::depth();
        // Mount order = paint order: settings first, prefs on top (sibling mount in eden_settings).
        let (settings, settings_id) = overlay(true);
        let (prefs, prefs_id) = overlay(true);
        assert!(
            modal_stack::is_topmost_open(prefs_id),
            "prefs stacked over settings must own Escape"
        );
        assert!(
            !modal_stack::is_topmost_open(settings_id),
            "settings under open prefs must not answer Escape — this is T-726"
        );
        assert!(
            modal_stack::any_open(),
            "any_open must be true while either dialog is open"
        );

        // First Esc would close prefs only.
        prefs.set(false);
        assert!(modal_stack::is_topmost_open(settings_id));
        assert!(!modal_stack::is_topmost_open(prefs_id));
        assert!(modal_stack::any_open());

        // Second Esc would close settings; then nobody owns Escape.
        settings.set(false);
        assert!(!modal_stack::is_topmost_open(settings_id));
        assert!(!modal_stack::any_open());

        modal_stack::unregister(settings_id);
        modal_stack::unregister(prefs_id);
        assert_eq!(modal_stack::depth(), start, "registrations must not leak");
    }

    /* ═══════════════ T-786 O-3 — z-index follows open order, Esc follows mount order ══════════ */

    /// The O-3 defect, stated as a test. `AttributesModal` mounts *before* `OrbatManagerDialog`
    /// (`mission_editor.rs`), so with equal `z-50` the browser paints ORBAT (later in the DOM) on
    /// top — hit-testing the Arsenal's centre returns ORBAT, and the author's click hits nothing.
    /// The fix drives z from OPEN order: the Arsenal, opened last, must be the top tier and ORBAT
    /// must drop below it. Meanwhile Esc still routes by MOUNT order (T-333, unchanged), and the two
    /// orders genuinely disagree here — that disagreement is the whole ticket.
    #[test]
    fn arsenal_opened_over_orbat_wins_z_while_esc_stays_mount_order() {
        let start = modal_stack::depth();
        // Mount order (registration): Attributes first, ORBAT second — as `mission_editor` mounts
        // them. Both start closed; the editor mounts both and toggles `open`.
        let (attrs, attrs_id) = overlay(false);
        let (orbat, orbat_id) = overlay(false);

        // Author opens ORBAT, then OPEN ARSENAL from a slot → Attributes opens on top, LAST.
        orbat.set(true);
        assert_eq!(
            modal_stack::z_class(orbat_id),
            "z-50",
            "ORBAT alone is the last-opened surface, so it holds the top modal tier"
        );
        attrs.set(true);

        // z-index: the Arsenal (opened last) is the top of open order; ORBAT drops one tier so the
        // Arsenal's z-50 wins the hit-test. This is the RED assertion before the fix — the old code
        // had every overlay pinned at z-50 with no way to drop.
        assert_eq!(
            modal_stack::z_class(attrs_id),
            "z-50",
            "the Arsenal opened last and must paint on top (T-786 O-3)"
        );
        assert_eq!(
            modal_stack::z_class(orbat_id),
            "z-40",
            "ORBAT opened first and must drop below the Arsenal that opened over it"
        );
        assert!(
            modal_stack::is_top_by_open_order(attrs_id),
            "open order: Arsenal is on top"
        );
        assert!(!modal_stack::is_top_by_open_order(orbat_id));

        // Esc still follows MOUNT order (T-333): ORBAT registered last, so it is `is_topmost_open`.
        // The two questions answer differently for this pair, which is exactly O-3.
        assert!(
            modal_stack::is_topmost_open(orbat_id),
            "Esc routing is unchanged: the last-MOUNTED open overlay owns Escape"
        );
        assert!(!modal_stack::is_topmost_open(attrs_id));

        // Close the Arsenal → ORBAT is the last-open again and climbs back to the top tier.
        attrs.set(false);
        assert_eq!(
            modal_stack::z_class(orbat_id),
            "z-50",
            "with the Arsenal gone, ORBAT is last-opened again and returns to the top tier"
        );

        modal_stack::unregister(attrs_id);
        modal_stack::unregister(orbat_id);
        assert_eq!(modal_stack::depth(), start, "registrations must not leak");
    }

    /// A reopen is a FRESH open: a surface that closes and reopens jumps to the top of open order,
    /// even though its mount position never moved. This is the open-order counterpart to the
    /// mount-order `a_dialog_closed_out_of_order_leaves_the_stack_consistent` above — and the reason
    /// z could not simply reuse `is_topmost_open`, which (correctly, for Esc) keeps the reopened
    /// surface underneath.
    #[test]
    fn a_reopen_takes_the_top_of_open_order() {
        let start = modal_stack::depth();
        let (lower, lower_id) = overlay(false);
        let (upper, upper_id) = overlay(false);

        lower.set(true);
        upper.set(true);
        assert!(
            modal_stack::is_top_by_open_order(upper_id),
            "upper opened last"
        );

        // Lower closes, then (in a later frame) reopens — now it is the most-recently-opened. The
        // close and the reopen are separate renders in the live editor, and z is recomputed each
        // render, so a reconcile lands between them; the query here stands in for that render and is
        // what lets the reopen register as a fresh open edge (see `reconcile_open_order`'s cadence
        // note — a close+reopen collapsed into one frame with no render between is not a real UI
        // event and is not tracked).
        lower.set(false);
        assert!(
            !modal_stack::is_top_by_open_order(lower_id),
            "with lower closed, upper is on top"
        );
        lower.set(true);
        assert!(
            modal_stack::is_top_by_open_order(lower_id),
            "a reopen is a fresh open and takes the top of open order (z), unlike Esc's mount order"
        );
        // …while Esc's mount order still puts `upper` (registered last) on top.
        assert!(
            modal_stack::is_topmost_open(upper_id),
            "Esc mount order is unchanged by a reopen — matches T-333's reopen-underneath test"
        );

        modal_stack::unregister(lower_id);
        modal_stack::unregister(upper_id);
        assert_eq!(modal_stack::depth(), start, "registrations must not leak");
    }

    /// **The wiring for O-3.** The stack utility only fixes the paint order if the ORBAT surface
    /// actually consumes it. `OrbatManagerDialog`'s scrim and panel must derive their z from
    /// `modal_stack::z_class` rather than a literal `z-50`; a body that still hard-codes `z-50` on
    /// the ORBAT overlay is the unfixed component (the Arsenal keeps its literal `z-50` this wave by
    /// design — it is the surface ORBAT yields *to*).
    #[test]
    fn orbat_manager_overlay_derives_z_from_the_modal_stack() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let scrubbed = live_code(include_str!("orbat_manager.rs"));
        let body = only_body(&scrubbed, "pub fn OrbatManagerDialog(");
        assert!(
            body.contains("modal_stack::z_class(modal_id)"),
            "OrbatManagerDialog must take its overlay z from modal_stack::z_class (T-786 O-3). \
             Body was: {body}"
        );
    }
}

/// T-633 — the range and select primitives, pinned.
///
/// These are Leptos views a native `cargo test` cannot render, so the pins read scrubbed source
/// (T-601/T-622 `class_r_scrub`): `live_code` blanks comments AND string literals, so a needle it
/// finds is a real reference and not a mention; `live_source` keeps the literals, which is where a
/// Tailwind class recipe is real code. Needles that assert an ABSENCE are assembled from fragments
/// so this module's own source can never satisfy them.
#[cfg(test)]
mod t633_range_and_select {
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

    fn code() -> String {
        live_code(include_str!("ui.rs"))
    }

    fn source() -> String {
        live_source(include_str!("ui.rs"))
    }

    /// The primitives EXIST here. That is half the ticket: `ui.rs` is where the suite's shared
    /// components live (MaterialIcon / PageHeader / AuthGate / Dialog / Sheet / AdminGate) and it
    /// had no slider and no select, which is why two raw browser controls were sitting in the
    /// editor's top strip with nothing to replace them with.
    #[test]
    fn the_suite_has_a_slider_and_a_select_primitive() {
        let code = code();
        for component in ["pub fn Slider(", "pub fn Select("] {
            assert!(
                code.contains(component),
                "T-633: `{component}` must be defined in ui.rs — the shared primitives home"
            );
        }
    }

    /// **The performance contract, stated as a test.** The time scrubber is dragged, and a drag
    /// emits ~30 values/second into `eden_top_strip`'s RowMirror (dedupe → debounce →
    /// single-flight). A primitive that listened on `input`, or that owned a signal it wrote per
    /// event and re-rendered from, would put a Leptos render on every one of those values and
    /// defeat the sequencing. So: `change` only, `prop:value` straight to the DOM property, and no
    /// signal of the component's own.
    #[test]
    fn neither_control_re_renders_per_event() {
        let code = code();
        // Assembled so this file's prose cannot satisfy the absence checks.
        let per_pixel = ["on:", "input"].concat();
        let owned_signal = ["RwSignal::", "new"].concat();
        for component in ["pub fn Slider(", "pub fn Select("] {
            let body = only_body(&code, component);
            assert!(
                body.contains("on:change"),
                "{component} must commit on the native `change` (the settle), not on drag"
            );
            assert!(
                !body.contains(&per_pixel),
                "T-633: {component} must NOT listen on `input` — that is an event per pixel, and it \
                 defeats the RowMirror debounce the scrubber is sequenced by"
            );
            assert!(
                !body.contains(&owned_signal),
                "T-633: {component} must stay uncontrolled — an internal signal written per event \
                 is a re-render per event"
            );
            assert!(
                body.contains("prop:value"),
                "{component} must write the DOM value PROPERTY (uncontrolled), not re-render to it"
            );
        }
    }

    /// Off browser chrome and onto the Aegis palette. `accent-color` alone was the old scrubber's
    /// approach and it only tints the UA widget — the track geometry, the thumb and the select's
    /// arrow stayed the browser's. Both controls must therefore be `appearance-none` with the parts
    /// painted explicitly: the `::-*-track` / `::-*-thumb` pseudo-elements for the slider (both
    /// engine prefixes — a WebKit-only recipe leaves Firefox on browser chrome), and a Material
    /// chevron for the select.
    #[test]
    fn both_controls_paint_their_own_parts() {
        let src = source();
        for part in [
            "::-webkit-slider-runnable-track",
            "::-webkit-slider-thumb",
            "::-moz-range-track",
            "::-moz-range-thumb",
        ] {
            assert!(
                src.contains(part),
                "T-633: the slider must paint `{part}` itself — accent-color does not reshape it"
            );
        }
        assert!(
            src.matches("appearance-none").count() >= 2,
            "T-633: both the slider and the select must be `appearance-none` — otherwise the UA \
             still draws the widget underneath"
        );
        let select_body = only_body(&src, "pub fn Select(");
        assert!(
            select_body.contains("expand_more"),
            "T-633: the select's native arrow is replaced by a Material chevron, not merely hidden"
        );
        assert!(
            select_body.contains("pointer-events-none"),
            "T-633: the chevron overlays the control, so it must not eat the click that opens it"
        );
    }

    /// T-751 — a disabled Select dims via DISABLED_GLYPH on the <select>, but the chevron is a
    /// *sibling* span, so `disabled:` variants never reach it. The select must be a Tailwind
    /// `peer` and the chevron must carry `peer-disabled:opacity-30` (matching DISABLED_GLYPH's
    /// opacity) or a disabled control is half-lit. Needles are fragment-assembled so this module
    /// is not its own haystack.
    #[test]
    fn disabled_select_chevron_dims_with_peer_disabled() {
        let src = source();
        let select_body = only_body(&src, "pub fn Select(");
        let peer = format!("{}{}", "peer ", "appearance-none");
        let dim = format!("{}{}", "peer-disabled:", "opacity-30");
        assert!(
            select_body.contains(&peer),
            "T-751: the <select> must be a Tailwind `peer` so a sibling can react to :disabled"
        );
        assert!(
            select_body.contains(&dim),
            "T-751: the Material chevron must carry peer-disabled:opacity-30 — DISABLED_GLYPH cannot reach a sibling"
        );
    }

    /// Built on the SHIPPED state vocabulary (T-668), not on new state classes. Both controls take
    /// their hover and disabled treatment from `eden_layout`'s named recipes, so the chrome keeps
    /// ONE state language; a hand-rolled `hover:bg-…` here would be a second definition of a rule
    /// the chrome files are already pinned against.
    #[test]
    fn both_controls_consume_the_t668_recipes() {
        let code = code();
        for component in ["pub fn Slider(", "pub fn Select("] {
            let body = only_body(&code, component);
            for recipe in ["HOVER_FILL", "DISABLED_GLYPH"] {
                assert!(
                    body.contains(recipe),
                    "T-633/T-668: {component} must consume {recipe} rather than invent a state class"
                );
            }
        }
        // …and no local hover fill anywhere in the file's own class strings (assembled needle).
        let hand_rolled = ["hover:bg-", "white/"].concat();
        assert!(
            !source().contains(&hand_rolled),
            "T-633/T-668: the neutral hover fill has one definition (eden_layout::HOVER_FILL); \
             re-typing it here is the duplication the vocabulary exists to remove"
        );
    }

    /// Rule (3)'s tooltip half: the `title=` is emitted unconditionally, NOT gated on `!disabled`.
    /// A control that cannot act must still explain itself.
    #[test]
    fn a_disabled_control_keeps_its_tooltip() {
        let code = code();
        for component in ["pub fn Slider(", "pub fn Select("] {
            let body = only_body(&code, component);
            assert!(
                body.contains("title=label") && body.contains("disabled=disabled"),
                "{component} must carry `title=label` beside `disabled=disabled` — the tooltip is \
                 not gated on the control being enabled (T-668 rule 3)"
            );
        }
    }
}
