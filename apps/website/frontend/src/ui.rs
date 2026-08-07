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
/// on `input`. A caller that needs live-drag feedback should render its own readout from the same
/// signal (which is what the top strip's `HH:MM` label does) rather than asking for a per-pixel
/// callback the debounce downstream would only have to throw away.
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
        <span class="relative inline-flex items-center">
            <select
                aria-label=label
                title=label
                disabled=disabled
                class=cn(
                    &[
                        "appearance-none rounded border border-outline-variant/40 bg-surface-container py-0.5 pr-6 pl-1.5 text-xs text-on-surface outline-none focus-visible:outline-1 focus-visible:outline-primary/60",
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
                class="pointer-events-none absolute right-0.5 text-base leading-none text-on-surface-variant"
            />
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

    thread_local! {
        /// `(id, is_open)` in mount order. Not a `Vec<bool>`: the flag has to be read at keydown
        /// time, not cached at registration time, or a dialog closed by a button would still be
        /// holding Escape.
        static REGISTRY: RefCell<Vec<(u64, Rc<dyn Fn() -> bool>)>> =
            const { RefCell::new(Vec::new()) };
        static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    }

    /// Register an overlay and return the id that identifies it for the rest of its life.
    pub(crate) fn register(is_open: impl Fn() -> bool + 'static) -> u64 {
        let id = NEXT_ID.with(|n| {
            let id = n.get();
            n.set(id + 1);
            id
        });
        REGISTRY.with_borrow_mut(|r| r.push((id, Rc::new(is_open))));
        id
    }

    /// Drop an overlay's registration. Removes by id, so unmount order does not matter, and is a
    /// no-op on an id that is already gone (double cleanup must not panic or evict a stranger).
    pub(crate) fn unregister(id: u64) {
        REGISTRY.with_borrow_mut(|r| r.retain(|(other, _)| *other != id));
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
            REGISTRY.with_borrow(|r| r.iter().map(|(i, f)| (*i, Rc::clone(f))).collect());
        entries
            .iter()
            .rev()
            .find(|(_, is_open)| is_open())
            .is_some_and(|(top, _)| *top == id)
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
