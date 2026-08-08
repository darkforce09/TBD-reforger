//! T-655 — the validation panel: a persistent issue list with a severity rollup, the surface for the
//! validation group (the engine is `map_engine_core::mission::validate`, T-656…T-660; this CONSUMES
//! it).
//!
//! ## Why this exists, and why it floats
//!
//! The rollup ranks #2 of 10 in the tooling analysis. FNF commented THEIRS out inside an HTML
//! comment, so a maker got no summary of failures and — because the rollup was the ONE surface that
//! aggregated the checks — every other check became invisible with it. This panel is that surface,
//! rebuilt on the T-656 engine so a check that does nothing can no longer masquerade as a check that
//! passed: the engine's `self_check` proves every rule can fire, and this panel makes what they find
//! impossible to miss.
//!
//! **It FLOATS.** A collapsible floating card, bottom-left above the status bar — the overlay idiom
//! (the ruler / LoS / transform-widget mount shape). It is NOT docked: docking collides with the
//! dock files program-wide (the ticket's own analysis), and the panel is diagnostics, not furniture.
//!
//! ## Validation is ALWAYS ON, and the panel survives hide-chrome
//!
//! Correctness diagnostics are never gated (T-635's doctrine comment: "telemetry gates, correctness
//! diagnostics never"). So the mount in `mission_editor.rs` is OUTSIDE the `chrome_hidden` gate,
//! beside the ungated dialogs — a Backspace hide-interface leaves the map full-bleed but the
//! validation card stays: a maker who hides the chrome to look at the map is exactly the maker who
//! wants to see "3 errors" without un-hiding first. (This is the deliberate call the ticket asks be
//! documented: the panel is diagnostics, not dock chrome, so it is not chrome-gated.)
//!
//! ## Re-evaluation
//!
//! The rules run over the FULL compiled payload, so they must not run per-frame. Re-eval is driven by
//! the `doc_tick` channel (the T-666 doc-change tick every mutation site bumps) through a **250 ms
//! trailing debounce** ([`Debouncer`], extracted here as pure timer logic so it is unit-tested on the
//! host): a rapid EDIT BURST — a drag commits once at release (T-159.19), but a held key or a bulk
//! paste can fire many commits back-to-back — collapses to a single re-evaluation ~250 ms after the
//! last edit, not one per commit.
//!
//! ## The four things the panel shows (the ticket's anatomy)
//!
//! 1. **Rollup** — a one-line `"3 errors · 5 warnings"` chip, counts by severity, always visible when
//!    non-empty ([`Rollup`]). Clicking it expands/collapses the list.
//! 2. **List** — findings grouped by rule with a per-rule count, CLICK-TO-SELECT (not a clipboard
//!    dump): clicking a finding routes its `subject_id` → the editor selection
//!    (the [`register_select_by_id`] router, installed from `mission_editor.rs` where the doc /
//!    selection handles live), so the offender is pinned on the map and in the trees. A row wears
//!    that click affordance IFF the router resolves its subject — [`finding_is_routable`] asks
//!    ([`register_route_probe`], the same resolution the click runs); a row it says no to renders
//!    inert, because an affordance must not assert what it has not asked.
//! 3. **Legend** — the severity ladder (Error / Warning / Info) with each rung's meaning.
//! 4. **Empty state** — a quiet "No issues", never a celebratory toast (a clean mission is the
//!    baseline, not an achievement).
//!
//! ## No severity fires on correct input
//!
//! The ticket's hard rule. It is the engine's property (every rule is conditional on the shape it
//! applies to — V1 conditionality — and a clean payload produces no findings), and it is re-asserted
//! HERE as a test ([`tests::a_clean_payload_produces_an_empty_panel`]): a clean compiled payload run
//! through the same `Registry` this panel uses yields zero findings, so the panel is empty.
#![allow(dead_code)]
use leptos::prelude::*;

use map_engine_core::mission::validate::{Finding, Primitive, Severity};

/// The trailing debounce window for a doc-change-driven re-evaluation, in milliseconds.
///
/// Chosen small (the maker wants near-live feedback) but not per-edit: the rules walk the whole
/// compiled payload, and an operation like a bulk paste bumps `doc_tick` on every intermediate
/// transaction (a drag bumps it once, at release — T-159.19). 250 ms trailing means the pass runs
/// once, a quarter-second after the LAST edit in a burst — live enough to feel immediate on a single
/// edit, cheap enough that a rapid edit burst runs the engine once, not once per commit.
pub const REEVAL_DEBOUNCE_MS: f64 = 250.0;

/* ═══════════════════════════ pure, host-testable core ═══════════════════════════ */

/// A finding flattened for the panel: an owned, `Clone` value carrying exactly what the card renders
/// and clicks on. The engine's [`Finding`] carries `&'static str` rule ids and is not something the
/// native view should hold across a re-eval; this is the panel's own row type.
///
/// `subject_id` is the T-657 stable entity id — the click-to-select key. `None` when the rule's
/// subject is positional or not a single entity (`V2-FACTION-MAX`, `V4-SCHEMA-VERSION`): those rows
/// still render, inert, because the router resolves nothing for them ([`finding_is_routable`] — the
/// row's clickability is the router's answer, never the presence of this field).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelFinding {
    pub rule_id: String,
    pub severity: Severity,
    pub primitive: Primitive,
    pub message: String,
    pub subject: String,
    pub subject_id: Option<String>,
}

impl PanelFinding {
    /// Flatten an engine [`Finding`] into an owned panel row.
    #[must_use]
    pub fn from_finding(f: &Finding) -> Self {
        Self {
            rule_id: f.rule_id.to_string(),
            severity: f.severity,
            primitive: f.primitive,
            message: f.message.clone(),
            subject: f.subject.clone(),
            subject_id: f.subject_id.clone(),
        }
    }

    /// Whether this finding NAMES an offender at all — a non-empty `subject_id`.
    ///
    /// **This is a fact about the finding, NOT the click affordance, and must never again be used as
    /// one.** Wave 129: the row used to style itself `cursor-pointer` off exactly this, which is a
    /// claim ("clicking selects something") made without asking the thing that would do the
    /// selecting — and the engine's `ASSET-RESOLVES` findings name placed-object ids the router
    /// resolved to nothing, so the claim was false on a surface the maker reaches today. A row is
    /// clickable IFF the ROUTER resolves its subject: see [`finding_is_routable`], which is what the
    /// view asks. `the_row_never_guesses_at_selectability` keeps this method out of the live view
    /// code; it survives only for callers asserting that a rule kept its subject id
    /// (`mission_commands`' compile-findings pin).
    #[must_use]
    pub fn is_selectable(&self) -> bool {
        self.subject_id.as_deref().is_some_and(|s| !s.is_empty())
    }
}

/// The severity rollup — counts by severity, the one-line summary chip.
///
/// The founding-defect fix in miniature: the chip is the aggregate FNF deleted. It is `is_empty()`
/// exactly when there is nothing to report, which is what drives "always visible when NON-empty" —
/// an empty rollup renders the quiet empty state, never a "0 errors" badge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rollup {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl Rollup {
    /// Tally a slice of panel findings by severity.
    #[must_use]
    pub fn of(findings: &[PanelFinding]) -> Self {
        let mut r = Rollup::default();
        for f in findings {
            match f.severity {
                Severity::Error => r.errors += 1,
                Severity::Warning => r.warnings += 1,
                Severity::Info => r.infos += 1,
            }
        }
        r
    }

    /// Total finding count across all severities.
    #[must_use]
    pub fn total(self) -> usize {
        self.errors + self.warnings + self.infos
    }

    /// No findings at all — the empty-state trigger.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.total() == 0
    }

    /// Whether the maker must act before this mission is shippable: any Error present. (Warnings /
    /// Info are advisory — the severity ladder's whole point.)
    #[must_use]
    pub fn has_blocking(self) -> bool {
        self.errors > 0
    }

    /// The one-line chip text: `"3 errors · 5 warnings"`. Only NON-zero severities appear, each
    /// correctly singular/plural; the highest severity leads. Empty rollup → `""` (the caller shows
    /// the empty state instead, so this is never rendered for an empty rollup).
    #[must_use]
    pub fn chip_text(self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.errors > 0 {
            parts.push(count_label(self.errors, "error"));
        }
        if self.warnings > 0 {
            parts.push(count_label(self.warnings, "warning"));
        }
        if self.infos > 0 {
            parts.push(count_label(self.infos, "info"));
        }
        parts.join(" · ")
    }
}

/// `"1 error"` / `"3 errors"` — singular for exactly one, else the `+s` plural. (`info` pluralises to
/// `infos`, which reads fine as a UI count and is what `chip_text` emits.)
#[must_use]
fn count_label(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// One rule's findings, grouped: the rule id, the group's (worst) severity, and its rows. The panel
/// list is "findings grouped by rule with counts" — this is the group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleGroup {
    pub rule_id: String,
    /// The worst severity among the group's findings — how the header row is coloured/ordered. All
    /// findings of one rule share a severity today (a rule has a fixed [`Severity`]), but taking the
    /// max keeps this correct if that ever stops holding.
    pub severity: Severity,
    pub findings: Vec<PanelFinding>,
}

impl RuleGroup {
    /// This group's finding count (the per-rule count the header shows).
    #[must_use]
    pub fn count(&self) -> usize {
        self.findings.len()
    }
}

/// A stable rank for ordering by severity, worst first: Error(0) < Warning(1) < Info(2).
#[must_use]
fn severity_rank(s: Severity) -> u8 {
    match s {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

/// Group findings by `rule_id`, ordered worst-severity-first then by first appearance, with each
/// group's rows in their original (engine) order.
///
/// Grouping is stable and deterministic: two runs over the same payload produce byte-identical
/// groups, so the panel does not reshuffle under the maker mid-read. Within a severity band, groups
/// keep the order their rule first appeared in the findings list (which is registry order — a stable
/// authored order), so the list reads the same every pass.
#[must_use]
pub fn group_by_rule(findings: &[PanelFinding]) -> Vec<RuleGroup> {
    let mut groups: Vec<RuleGroup> = Vec::new();
    for f in findings {
        if let Some(g) = groups.iter_mut().find(|g| g.rule_id == f.rule_id) {
            if severity_rank(f.severity) < severity_rank(g.severity) {
                g.severity = f.severity;
            }
            g.findings.push(f.clone());
        } else {
            groups.push(RuleGroup {
                rule_id: f.rule_id.clone(),
                severity: f.severity,
                findings: vec![f.clone()],
            });
        }
    }
    // Worst severity first; ties keep insertion order (a stable sort over the already-ordered vec).
    groups.sort_by_key(|g| severity_rank(g.severity));
    groups
}

/* ─────────────────────────── the severity ladder / legend ─────────────────────────── */

/// One rung of the severity ladder, for the legend. `label` is the display name, `meaning` is the
/// one-line "what this rung means" the legend spells out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LadderRung {
    pub severity: Severity,
    pub label: &'static str,
    pub meaning: &'static str,
}

/// The severity ladder, worst first — the legend's rows. The meanings are the ladder's contract: an
/// Error blocks, a Warning is advisory, Info is a note. This mirrors the engine's severity doc (a
/// missing player spawn is an Error; a soft ceiling is a Warning), stated for the maker.
pub const SEVERITY_LADDER: [LadderRung; 3] = [
    LadderRung {
        severity: Severity::Error,
        label: "Error",
        meaning: "blocks — the mission will not compile or spawn correctly until fixed",
    },
    LadderRung {
        severity: Severity::Warning,
        label: "Warning",
        meaning:
            "advisory — likely a mistake (fairness, identity, a soft ceiling), but not blocking",
    },
    LadderRung {
        severity: Severity::Info,
        label: "Info",
        meaning: "a note — informational, no action required",
    },
];

/// The stable lowercase severity tag (`"error"`/`"warning"`/`"info"`) — the CSS/severity hook the
/// view keys its per-severity colour on. Re-exports the engine's `Severity::as_str` so the panel and
/// the engine cannot drift on the spelling.
#[must_use]
pub fn severity_tag(s: Severity) -> &'static str {
    s.as_str()
}

/* ─────────────────────────── the pure debounce timer ─────────────────────────── */

/// The trailing-debounce state machine, extracted from the wasm timer so it is unit-testable on the
/// host (the ticket asks for "debounce behaviour (pure timer logic extracted)").
///
/// The contract: a burst of `bump`s (each a `doc_tick` change) must collapse to ONE evaluation, the
/// window after the LAST bump. The machine tracks the timestamp of the most recent bump and a
/// pending flag; `should_fire(now)` answers "has the window elapsed with no newer bump?" without any
/// clock or timer of its own — the caller (a `set_timeout` on wasm, a test on the host) supplies the
/// times. This is deliberately a pure fold over `(bump, now)` events so the trailing semantics are
/// proved by table-driven tests rather than by watching a real timer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Debouncer {
    /// The trailing window, ms.
    window_ms: f64,
    /// Timestamp of the most recent `bump`, or `None` when idle (nothing pending).
    last_bump: Option<f64>,
}

impl Debouncer {
    /// A debouncer with the given trailing window (ms). Starts idle.
    #[must_use]
    pub fn new(window_ms: f64) -> Self {
        Self {
            window_ms,
            last_bump: None,
        }
    }

    /// Record a change at `now`. Any previously-scheduled fire is superseded — the window restarts
    /// from this bump (trailing semantics). Returns `true` (there is now a pending evaluation), which
    /// the wasm caller uses to know it must (re)arm its timer.
    pub fn bump(&mut self, now: f64) -> bool {
        self.last_bump = Some(now);
        true
    }

    /// Whether a fire is due at `now`: something is pending AND the full window has elapsed since the
    /// last bump. A newer bump (a `last_bump` closer to `now` than `window_ms`) returns `false` — the
    /// burst is still going, so the trailing fire waits.
    #[must_use]
    pub fn should_fire(&self, now: f64) -> bool {
        match self.last_bump {
            Some(t) => now - t >= self.window_ms,
            None => false,
        }
    }

    /// Consume the pending state after firing. Idempotent — a second `take_fire` with nothing pending
    /// is a no-op returning `false`. Returns whether there WAS a pending fire consumed.
    pub fn take_fire(&mut self) -> bool {
        let had = self.last_bump.is_some();
        self.last_bump = None;
        had
    }

    /// Whether an evaluation is currently pending (a bump has landed but not yet fired). Drives the
    /// panel's subtle "re-checking…" affordance.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.last_bump.is_some()
    }
}

/* ═══════════ the seam idiom: registered at mount, unregistered at unmount, remount-safe ═══════════
 *
 * **This block is the crate's ONLY definition of the mechanism.** It publishes the FOUR thread_local
 * seams of this file (the payload source, the click-to-select router, the route probe, the publish
 * sink) and, since T-783, the five owned elsewhere as well: `ruler_tool`'s `RULER_CHAIN`, `los_tool`'s
 * `LOS_STATE` / `LOS_SAMPLER` / `VIEWSHED_STATE`, and `world_assets`'s `RENDER_CTX`. Every one of them
 * is a value handed over at mount by a surface that owns `!Send` / reactive state the native-compiled
 * consumer cannot hold.
 *
 * T-778 could not import [`install_seam`] / [`unregister_seam`] (they were module-private) and copied
 * the six-line mechanism into `ruler_tool` instead, leaving ONE identity check and TWO mechanisms that
 * used it. T-783 widened these three items to `pub(crate)` and deleted that copy; `ruler_tool` now
 * re-exports them so `crate::ruler_tool::install_seam` — the path `los_tool` and `world_assets` already
 * import — keeps resolving, to this body. A duplicated vocabulary is its own defect class, and this one
 * guards a defect family found five times in a single wave.
 *
 * The home is deliberate. `validation_panel` is declared UNCONDITIONALLY in `main.rs`, so it is
 * reachable on native and on wasm32 alike; `world_assets` and `select_tool` are `#[cfg(target_arch =
 * "wasm32")]` and could never have hosted it.
 *
 * **Wave-129 F5, the same defect F2 fixed in `eden_dock_right`'s zone hook.** A seam registered at
 * mount and never unregistered stays CALLABLE after the surface that owns it is gone: Backspace
 * hide-chrome unmounts panels while dialogs deliberately survive, and SPA navigation drops the whole
 * editor page. The stale closure then reports SUCCESS — `true`, or a stale payload — while every
 * `set` inside it lands on a DISPOSED signal, which `reactive_graph` 0.2.14 makes a silent no-op.
 * The caller sees a click that "worked" and nothing happened.
 *
 * The naive fix closes only half of it. An UNCONDITIONAL unregister at cleanup introduces the mirror
 * defect: leptos does not guarantee that the dying owner's cleanup runs before the remount's
 * registration, so an old cleanup can delete the LIVE surface's seam and leave it dead again. Hence
 * [`unregister_seam`]'s identity guard — only the LOSING registration is cleared.
 *
 * It is written ONCE, here, and applied nine times across four files. Bespoke copies are how the next
 * seam gets added without one (which is exactly how the route probe arrived in wave 129).
 */

/// A seam's registered value, comparable for IDENTITY against whatever is currently live.
///
/// Identity, not equality: the question [`unregister_seam`] asks is "is the thing in the cell the
/// very registration *I* put there", and two structurally equal hooks from two different mounts must
/// answer `false`. Both impls below are identity comparisons that survive reallocation — an `Rc`'s
/// pointer is kept valid by the clone the installer holds, and a signal's key is a `slotmap` key
/// whose version bumps when the slot is reused. **A bare `usize` address would not do**: the old
/// value drops on re-register, a later registration can be allocated at the freed address, and a
/// stale cleanup would then wrongly clear a live seam (ABA).
pub(crate) trait SeamRegistration: Clone + 'static {
    /// Is `live` — the value currently in the seam's cell — this very registration?
    fn is_same_registration(&self, live: &Self) -> bool;
}

/// The three closure seams: identity is the `Rc` allocation.
impl<T: ?Sized + 'static> SeamRegistration for std::rc::Rc<T> {
    fn is_same_registration(&self, live: &Self) -> bool {
        std::rc::Rc::ptr_eq(self, live)
    }
}

/// The publish sink: identity is the arena key, which `PartialEq` already compares (and `slotmap`
/// versions, so a recycled slot is not mistaken for the signal that used to live in it).
impl<T: 'static, S: 'static> SeamRegistration for RwSignal<T, S> {
    fn is_same_registration(&self, live: &Self) -> bool {
        self == live
    }
}

/// A seam's storage: one thread_local slot holding the current registration, or `None` (host build /
/// pre-mount / everything unmounted) — which every read of it must report as HONEST FAILURE.
pub(crate) type SeamCell<H> = std::thread::LocalKey<std::cell::RefCell<Option<H>>>;

/// Install `hook` into `cell` for the CURRENT reactive owner: register it now, and unregister it
/// when that owner is cleaned up (i.e. at unmount).
///
/// Called with no owner (the host tests, a non-reactive caller) it degrades to a bare register:
/// `on_cleanup` outside an owner is a no-op, which is the pre-existing behaviour.
///
/// The hook is parked in a `StoredValue` with **LOCAL** storage because `on_cleanup` is
/// `Send + Sync`-bound and an `Rc<dyn Fn>` is `!Send`, so the cleanup cannot carry the hook itself.
/// An owner runs its cleanup functions BEFORE it removes its arena nodes, so the read back inside
/// the cleanup is valid; and holding that clone is what keeps the allocation alive, which is what
/// makes the identity check in [`unregister_seam`] meaningful — rather than **a bare `usize` address**
/// a later registration could be re-allocated onto while a stale cleanup wrongly clears it (ABA).
pub(crate) fn install_seam<H: SeamRegistration>(cell: &'static SeamCell<H>, hook: H) {
    let mine = StoredValue::new_local(hook.clone());
    cell.with(|c| *c.borrow_mut() = Some(hook));
    on_cleanup(move || {
        let _ = mine.try_with_value(|mine| unregister_seam(cell, mine));
    });
}

/// Clear `cell` — but ONLY if `mine` is still the LIVE registration.
///
/// Returns whether this call is the one that cleared it; a superseded (losing) cleanup returns
/// `false` and leaves the newer registration alone. The value is taken OUT of the cell and dropped
/// after the borrow ends, so a `Drop` that re-enters this seam cannot hit a double borrow.
pub(crate) fn unregister_seam<H: SeamRegistration>(cell: &'static SeamCell<H>, mine: &H) -> bool {
    let taken = cell.with(|c| {
        let mut slot = c.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|live| mine.is_same_registration(live))
        {
            slot.take()
        } else {
            None
        }
    });
    taken.is_some()
}

/* ═══════════════════════════ the cross-target payload source ═══════════════════════════ */

// The engine is PURE core with no access to the SPA's `!Send` doc / `registry_session` thread_locals.
// The panel view is native-compilable (like `AttributesModal`), so it cannot hold those `Rc`s either.
// The seam — mirroring `ruler_tool::register_ruler_chain` / `mission_editor::register_widget_pivot`
// — is a thread_local getter registered from `mission_editor.rs`'s wasm mount that returns the
// current compiled payload plus the known-asset-id catalogue. The panel calls [`read_payload_source`]
// each re-eval; on the host / pre-mount it is `None` and the panel is simply empty.

/// The inputs one validation pass needs: the compiled editor payload (the `compile_payload` shape the
/// rules read) and the live known-asset-id set for the T-658 `ASSET-RESOLVES` context.
#[derive(Clone, Debug)]
pub struct PayloadSource {
    /// The compiled payload — `map_engine_core::mission::compile::compile_payload(small, slots,
    /// false)` (the Save shape, which carries the `editor.{factions,squads,slots}` block + top-level
    /// `vehicles`/`entities` the rules walk).
    pub payload: serde_json::Value,
    /// The live catalogue ids that resolve (full `resource_name`s + `veh:`/`prop:`/`comp:` aliases),
    /// or `None` when the registry has not loaded — in which case `ASSET-RESOLVES` SKIPS (its gate),
    /// the conservative default, rather than flagging every placed asset as unknown.
    pub known_asset_ids: Option<std::collections::HashSet<String>>,
}

/// The registered payload-source getter's type — a closure returning the current [`PayloadSource`]
/// (or `None` when the doc/registry are not ready). Aliased to keep the thread_local readable.
type PayloadSourceGetter = std::rc::Rc<dyn Fn() -> Option<PayloadSource>>;

thread_local! {
    /// The registered payload-source getter. Set from `mission_editor.rs` (which owns the `!Send`
    /// doc + registry_session `Rc`s); read by the native-compiled panel via [`read_payload_source`].
    /// A thread_local (peer of `ruler_tool::RULER_CHAIN`) so the panel never touches disposed
    /// reactive state and a host / pre-mount build simply sees `None`.
    static PAYLOAD_SOURCE: std::cell::RefCell<Option<PayloadSourceGetter>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the payload-source getter (called once at mount from the wasm block). The closure reads
/// the live doc + registry each time it is called, so the panel always evaluates the CURRENT mission.
///
/// **This is an INSTALL** ([`install_seam`]): the getter is unregistered when the owner that
/// registered it is cleaned up, and a remount's newer getter is not clobbered by the old owner's
/// cleanup. Without that, a getter closing over a dropped editor page's doc would keep answering
/// with a mission that is no longer open — see the F5 note above [`SeamRegistration`].
pub fn register_payload_source(f: PayloadSourceGetter) {
    install_seam(&PAYLOAD_SOURCE, f);
}

/// The current payload source, or `None` (no getter registered — host build / pre-mount / a getter
/// that itself returned `None` because the doc is not ready).
#[must_use]
pub fn read_payload_source() -> Option<PayloadSource> {
    PAYLOAD_SOURCE.with(|c| c.borrow().as_ref().and_then(|f| f()))
}

/// The registered click-to-select router's type — a closure taking a finding's `subject_id` (a slot
/// or vehicle id) and selecting that entity, returning whether one was selected.
type SelectByIdRouter = std::rc::Rc<dyn Fn(&str) -> bool>;

thread_local! {
    /// The registered click-to-select router. Set from `mission_editor.rs` (which owns the `!Send`
    /// doc / selection / engine `Rc`s the routing needs — the panel cannot hold them, being
    /// native-compiled); read by [`route_select_by_subject_id`]. Peer of `PAYLOAD_SOURCE` and
    /// `ruler_tool::RULER_CHAIN`; `None` on the host / pre-mount, so a click there is a no-op.
    static SELECT_BY_ID: std::cell::RefCell<Option<SelectByIdRouter>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the click-to-select router (called once at mount from the wasm block). The closure
/// replaces the selection with the given entity id, centres the camera on it, and refreshes the
/// mirrors — the T-655 `subject_id → select_slot/vehicle` path, living where the `!Send` handles do.
///
/// **This is an INSTALL** ([`install_seam`]): the router is unregistered at that owner's cleanup, so
/// [`route_select_by_subject_id`] reports `false` once the surface holding those handles is gone
/// instead of `true` over a selection that went nowhere — the F5/F2 dead click.
pub fn register_select_by_id(f: SelectByIdRouter) {
    install_seam(&SELECT_BY_ID, f);
}

/// Route a finding's `subject_id` to the editor selection through the registered router. Returns
/// whether an entity was selected (`false` on the host / pre-mount / a stale id that resolved to no
/// entity). This is the panel's click-to-select seam — it holds no doc state itself.
pub fn route_select_by_subject_id(subject_id: &str) -> bool {
    SELECT_BY_ID.with(|c| c.borrow().as_ref().is_some_and(|f| f(subject_id)))
}

/// The registered ROUTE PROBE's type — the router's resolution asked as a QUESTION: "would a click
/// on this `subject_id` select anything?", with no side effect.
type RouteProbe = std::rc::Rc<dyn Fn(&str) -> bool>;

thread_local! {
    /// The registered route probe. Set from `mission_editor.rs` beside [`SELECT_BY_ID`] and backed by
    /// the SAME resolution closure the click runs (`mission_editor::route_target` over the live
    /// document), so the answer this returns is the answer the click will act on.
    ///
    /// `None` on the host / pre-mount — and that resolves to "not clickable", which is the safe
    /// direction: a row renders inert rather than advertising a click into a router that does not
    /// exist yet.
    static ROUTE_PROBE: std::cell::RefCell<Option<RouteProbe>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the route probe (called once at mount, from the block that registers the router).
///
/// **This is an INSTALL** ([`install_seam`]), and here it is load-bearing for correctness rather
/// than hygiene: [`finding_is_routable`] makes a row's clickability the probe's answer, so a probe
/// that outlives its editor would paint rows clickable for a router that can no longer route —
/// re-creating the very dead click the probe was added to prevent.
pub fn register_route_probe(f: RouteProbe) {
    install_seam(&ROUTE_PROBE, f);
}

/// **Would a click on this `subject_id` select anything?** — the router's own resolution, asked
/// before the affordance is drawn.
///
/// Empty id short-circuits to `false` (there is nothing to resolve); everything else goes to the
/// registered probe. No probe registered ⇒ `false`.
#[must_use]
pub fn subject_id_routes(subject_id: &str) -> bool {
    !subject_id.is_empty()
        && ROUTE_PROBE.with(|c| c.borrow().as_ref().is_some_and(|f| f(subject_id)))
}

/// **Is this finding's row clickable?** — the ONE question behind both the row's `cursor-pointer`
/// and its click, and it is [`subject_id_routes`]'s answer, never "the row names an id".
///
/// Wave 129, the peer of `eden_settings::owner_is_routable`: the panel used to reason "the finding
/// carries a `subject_id`, so the row is selectable", which was false for every `ASSET-RESOLVES`
/// finding on a placed object — the router had no `entitiesById` arm and the click silently
/// discarded its own `false`. A view must not paint an affordance it has not asked about.
#[must_use]
pub fn finding_is_routable(f: &PanelFinding) -> bool {
    f.subject_id.as_deref().is_some_and(subject_id_routes)
}

/// Build the T-658 known-asset-id catalogue from the live `registry_session` rows — the set
/// `ASSET-RESOLVES` resolves a placed asset against. This is the T-658 SPA-boundary the ticket lands
/// HERE, in the panel wiring.
///
/// The engine resolves a placed asset by the id AS WRITTEN in the payload (see
/// `validate::placed_asset_refs`): slots carry `assetId` = the full Enfusion `resource_name`, vehicles
/// carry `resourceName`, and placed objects carry a `prop:`/`comp:` **alias** (preferred over their
/// resourceName). So the catalogue must hold both forms:
///
/// * every row's `resource_name` (covers slots + vehicles + any object matched by resourceName), and
/// * for object-kind rows (`crate`/`other`), the derived `prop:`/`comp:` **alias**
///   (`asset_catalog::derive_object_alias`) — the id the Objects palette pins a placed object to.
///
/// A `veh:` alias is NOT added: the doc stores a vehicle's `resourceName` (not its alias), so the
/// vehicle reference resolves against the resource_name already in the set. Character `gear_*` rows
/// contribute their resource_name too (harmless — no placed reference uses them, they resolve if ever
/// referenced). Empty input → an empty set (which still APPLIES the rule: every placed asset is then
/// unresolved — the correct reading of "the catalogue is loaded and holds nothing").
#[must_use]
pub fn known_asset_ids_from_registry(
    items: &[crate::dto::RegistryItem],
) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::with_capacity(items.len() * 2);
    for item in items {
        // Every row's resource_name (slots' assetId + vehicles' resourceName resolve against these).
        set.insert(item.resource_name.clone());
        // Object-kind rows (`crate`/`other`) are placed by their prop:/comp: alias — add that form
        // too so an object placed-by-alias resolves. Mirrors `asset_catalog::is_object_kind`
        // (`matches!(kind, "crate" | "other")`) — a small enum, replicated to avoid depending on a
        // private helper; the derivation itself is the pub `asset_catalog::derive_object_alias`.
        if matches!(item.kind.as_str(), "crate" | "other") {
            set.insert(crate::asset_catalog::derive_object_alias(
                &item.resource_name,
                &item.display_name,
            ));
        }
    }
    set
}

/// Run the validation engine over `source`, returning the panel rows — the ONE place the engine is
/// invoked.
///
/// **Defensive by contract (the ticket's anti-goal: a validation panel that crashes the editor).**
/// T-657 proved rule totality (`orbat_rules_never_panic_on_garbage`), but a panel is not the place to
/// bet the whole editor on that holding for every future rule: the `Registry::evaluate_with_context`
/// call is wrapped in `catch_unwind`, and a panic becomes a logged empty result (the pass "found
/// nothing this tick") rather than an unwind through the render. A rule that somehow panics degrades
/// the panel to blank for that tick, never takes the editor down.
#[must_use]
pub fn evaluate_source(source: &PayloadSource) -> Vec<PanelFinding> {
    use map_engine_core::mission::validate::{default_registry, EvalContext};

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut ctx = EvalContext::default();
        if let Some(ids) = source.known_asset_ids.clone() {
            ctx = ctx.with_known_asset_ids(ids);
        }
        // LoadoutPolicy stays None this wave: no policy UI exists to author thresholds, so the
        // policy-gated T-660 rules (LOADOUT-MAG-COUNT / -HAS-EQUIPMENT / VEHICLE-CARGO) skip — the
        // conservative default (a mission with no declared policy is not "below" one). When a policy
        // surface lands, thread it onto the context here.
        default_registry().evaluate_with_context(&source.payload, &ctx)
    }));

    match result {
        Ok(findings) => findings.iter().map(PanelFinding::from_finding).collect(),
        Err(_) => {
            // The one place a rule panic is swallowed: log it and show nothing this tick rather than
            // crash the editor. (`eprintln!` on the host; the wasm build routes panics to the console
            // via the panic hook, and this branch keeps the render alive regardless.)
            #[cfg(not(target_arch = "wasm32"))]
            eprintln!("validation_panel: a rule panicked; showing no findings this tick");
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(
                &"validation_panel: a rule panicked; showing no findings this tick".into(),
            );
            Vec::new()
        }
    }
}

/* ═══════════════════ T-690 — the compile's findings, published into this panel ═══════════════════ */

thread_local! {
    /// The most recent COMPILE's findings (T-690), already flattened to panel rows. Written by
    /// [`publish_compile_findings`] from the command layer, read by [`evaluate_now`].
    ///
    /// **Why the compile's findings are pushed here rather than evaluated as registry rules.** The
    /// registry runs on every doc change, so a rule in it is an ALWAYS-ON claim about the mission.
    /// The compile's drop findings are not that: `ORBAT-SQUAD-HAS-LEADER` fires when a squad names
    /// no `leaderSlotId` and `COMPILE-DROP-SQUAD-LEADER` fires when it names one, so as registry
    /// rules the pair would be exhaustive over every squad and this panel could never go green —
    /// verbatim the FNF defect (fnf_tooling.md 1.3, "the Analyzer's role accordion can never go
    /// green, which makes it useless"), and unclearable besides, since the emit is parked behind
    /// T-674/T-675. They describe what THIS compile discarded, so they arrive when a compile runs.
    /// The reasoning lives in full on `map_engine_core::mission::flatten`'s `DiagnosticAcc`.
    static COMPILE_FINDINGS: std::cell::RefCell<Vec<PanelFinding>> =
        const { std::cell::RefCell::new(Vec::new()) };

    /// The mounted panel's rendered-findings signal, so a publish repaints IMMEDIATELY rather than
    /// waiting for the next doc-change debounce (an export that produced findings and showed nothing
    /// until the author's next edit would read as a broken button). Registered by [`ValidationPanel`]
    /// at mount; `None` on the host / pre-mount, where a publish simply stores.
    static PANEL_SINK: std::cell::RefCell<Option<RwSignal<Vec<PanelFinding>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the mounted panel's findings signal as the publish sink, for as long as the component
/// that owns that signal is alive.
///
/// **Wave-129 F5.** This seam always had a cleanup, but an UNCONDITIONAL one — `PANEL_SINK = None`
/// at unmount, no matter what was in the cell. That is the second half of the F5 defect and the
/// mirror of the missing-cleanup half: a remounted panel installs its NEW signal before the old
/// panel's cleanup runs, the old cleanup clears it, and the live card then never repaints on a
/// compile. [`install_seam`]'s identity guard makes the losing cleanup a no-op.
pub fn register_panel_sink(sink: RwSignal<Vec<PanelFinding>>) {
    install_seam(&PANEL_SINK, sink);
}

/// Publish the findings a compile produced (`map_engine_core::mission::flatten`'s
/// [`Finding`]s, flattened to panel rows) and repaint. Replaces the previous compile's list whole —
/// a finding describes one compile, so two compiles do not accumulate.
///
/// Called from `mission_commands::export_compiled_now`. This is the T-690 feed: the panel is the
/// render surface and it is not rebuilt here, only fed.
pub fn publish_compile_findings(rows: Vec<PanelFinding>) {
    COMPILE_FINDINGS.with(|c| *c.borrow_mut() = rows);
    let sink = PANEL_SINK.with(|c| *c.borrow());
    if let Some(sig) = sink {
        sig.set(evaluate_now());
    }
}

/// The last published compile findings (empty before any compile has run).
#[must_use]
pub fn compile_findings() -> Vec<PanelFinding> {
    COMPILE_FINDINGS.with(|c| c.borrow().clone())
}

/// Drop the last compile's findings (and repaint the mounted panel, if any).
///
/// **T-761 / wave-116 finding 3.** `COMPILE_FINDINGS` is a thread_local written only by
/// [`publish_compile_findings`] (production caller: `export_compiled_now`). A clean compile already
/// replaces the list whole, but nothing reset the cell on editor mount/hydrate — and
/// `/missions/:id/edit` is a client-side `leptos_router` route, so navigating mission A → mission B
/// reuses the wasm instance. Without this clear, B's panel shows A's build report (with
/// `subject_id`s that resolve to nothing in B). Called from `MissionEditorPage`'s hydrate path.
pub fn clear_compile_findings() {
    publish_compile_findings(Vec::new());
}

/// Evaluate the CURRENT registered payload source, or an empty vec when none is registered (host /
/// pre-mount). The panel's re-eval calls this; the view never touches the engine directly.
///
/// The always-on registry findings come first, then the last compile's ([`publish_compile_findings`]),
/// so `group_by_rule`'s stable ordering puts the mission's own defects above the build report.
#[must_use]
pub fn evaluate_now() -> Vec<PanelFinding> {
    let mut rows = match read_payload_source() {
        Some(source) => evaluate_source(&source),
        None => Vec::new(),
    };
    rows.extend(compile_findings());
    rows
}

/* ═══════════════════════════ the view (native-compilable, like AttributesModal) ═══════════════════════════ */

/// The floating validation card. Mounted ONCE from `mission_editor.rs`, OUTSIDE the `chrome_hidden`
/// gate (diagnostics survive hide-chrome — the doctrine call). Renders bottom-left above the status
/// bar in the overlay idiom.
///
/// `doc_tick` is the re-eval trigger: a wasm-only `Effect` subscribes to it and, through the
/// [`Debouncer`], schedules a trailing 250 ms re-evaluation. The rendered `findings` live in a local
/// signal the effect writes and the view reads — so the panel updates only after the debounce, never
/// per intermediate edit.
///
/// The view itself compiles on the host (the `#[cfg]` split mirrors `AttributesModal`): the layout is
/// native, the doc-reading re-eval is `#[cfg(target_arch = "wasm32")]`.
#[component]
pub fn ValidationPanel(
    /// The doc-change tick (T-666 channel) every mutation site bumps — the re-eval trigger.
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    // The rendered findings + the expand/collapse state. `findings` is written by the debounced
    // re-eval effect (wasm) and read by the view; `expanded` is the card open/closed latch (the
    // rollup chip toggles it). Both are plain session signals owned by this component.
    let findings = RwSignal::new(Vec::<PanelFinding>::new());
    // Default collapsed: the chip is the always-visible summary; the maker expands to read the list.
    let expanded = RwSignal::new(false);
    // A subtle "re-checking…" flag while a debounce is armed (set on bump, cleared on fire).
    let rechecking = RwSignal::new(false);

    // T-690 — register `findings` as the publish sink so a compile's diagnostics repaint the card
    // the moment they are produced, without waiting for the doc-change debounce. Registered on both
    // targets: the signal is the same one the view reads either way. Wave-129 F5 — the registration
    // carries its own unmount (this component's owner), and the identity guard inside it is what
    // keeps a REMOUNT's sink from being cleared by the outgoing panel's cleanup.
    register_panel_sink(findings);

    // ── Re-evaluation: doc_tick → debounce → evaluate (wasm only) ──
    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        // `disposed` is an `Arc<AtomicBool>` (Send + Sync) — the ONLY thing `on_cleanup` may hold,
        // since `on_cleanup` is `Send + Sync`-bound and the timer state (`TimeoutHandle`) is `!Send`
        // (the `arsenal_doll` / `sse.rs` idiom). A leaked trailing timer on route-leave checks this
        // and no-ops rather than firing into a disposed signal; the `Rc<RefCell<…>>` timer/debouncer
        // never cross into `on_cleanup`.
        let disposed = Arc::new(AtomicBool::new(false));
        // One shared debouncer + the handle of the in-flight trailing timer (so a new bump cancels
        // and reschedules — trailing semantics). `Rc<RefCell<…>>` shared by the timer callback and
        // the doc_tick effect (both on the one wasm thread).
        let deb = Rc::new(RefCell::new(Debouncer::new(REEVAL_DEBOUNCE_MS)));
        let timer: Rc<RefCell<Option<leptos::leptos_dom::helpers::TimeoutHandle>>> =
            Rc::new(RefCell::new(None));

        // Run one pass NOW and push it onto the signal. Shared by the initial pass and the timer.
        // Guarded by `disposed` so a queued run after route-leave never touches the dead signal.
        // (`findings`/`rechecking` are `Copy` signals — captured by copy, no rebind needed.)
        let run_eval = {
            let disposed = disposed.clone();
            move || {
                if disposed.load(Ordering::Relaxed) {
                    return;
                }
                findings.set(evaluate_now());
                rechecking.set(false);
            }
        };

        // The trailing-timer arm: schedule a fire REEVAL_DEBOUNCE_MS out, cancelling any pending one.
        let arm = {
            let deb = deb.clone();
            let timer = timer.clone();
            let run_eval = run_eval.clone();
            let disposed = disposed.clone();
            Rc::new(move || {
                if disposed.load(Ordering::Relaxed) {
                    return;
                }
                // Cancel a previously-armed trailing timer (the burst is still going).
                if let Some(h) = timer.borrow_mut().take() {
                    h.clear();
                }
                let deb2 = deb.clone();
                let timer2 = timer.clone();
                let run_eval = run_eval.clone();
                let handle = set_timeout_with_handle(
                    move || {
                        timer2.borrow_mut().take();
                        // Fire only if no newer bump landed inside the window; the Debouncer is the
                        // oracle (its `should_fire` gate is what `t655` proves). now() is monotonic
                        // enough for a UI debounce.
                        let now = now_ms();
                        let fire = {
                            let mut d = deb2.borrow_mut();
                            if d.should_fire(now) {
                                d.take_fire()
                            } else {
                                false
                            }
                        };
                        if fire {
                            run_eval(); // itself guarded by `disposed`
                        }
                    },
                    std::time::Duration::from_millis(REEVAL_DEBOUNCE_MS as u64),
                );
                if let Ok(h) = handle {
                    *timer.borrow_mut() = Some(h);
                }
            })
        };

        // Initial pass: evaluate once at mount so the panel reflects the freshly-hydrated doc without
        // waiting for the first edit. Deferred a frame so the payload-source getter is registered.
        {
            let run_eval = run_eval.clone();
            set_timeout(run_eval, std::time::Duration::from_millis(0));
        }

        // Subscribe to doc_tick: each bump records into the debouncer and (re)arms the trailing timer.
        // (`rechecking` is a `Copy` signal — captured directly.)
        {
            let deb = deb.clone();
            let arm = arm.clone();
            Effect::new(move |_| {
                let _ = doc_tick.get(); // subscribe — re-run on every doc change
                deb.borrow_mut().bump(now_ms());
                rechecking.set(true);
                arm();
            });
        }

        // On unmount, flip the disposed flag — a route-leave mid-debounce then no-ops instead of
        // firing into a dead signal. `Arc<AtomicBool>` is Send + Sync, so `on_cleanup` accepts it
        // (unlike the `!Send` TimeoutHandle, which stays in the Rc cell and is simply abandoned).
        {
            let disposed = disposed.clone();
            on_cleanup(move || disposed.store(true, Ordering::Relaxed));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (doc_tick, rechecking);
    }

    // ── The card view (native-compilable) ──
    move || {
        let rows = findings.get();
        let rollup = Rollup::of(&rows);
        let open = expanded.get();
        let checking = rechecking.get();

        view! {
            <div
                class="pointer-events-auto absolute bottom-14 left-3 z-30 w-80 max-w-[calc(100vw-1.5rem)]"
                data-validation-panel
                data-issue-total=move || Rollup::of(&findings.get()).total()
            >
                {if rollup.is_empty() {
                    empty_state_view(checking)
                } else {
                    populated_view(rollup, open, checking, expanded, findings)
                }}
            </div>
        }
    }
}

/// The quiet empty state — "No issues", never a celebratory toast (the ticket's explicit call). A
/// small, low-contrast pill; a clean mission is the baseline, not an achievement to announce.
fn empty_state_view(checking: bool) -> AnyView {
    view! {
        <div
            class="glass flex items-center gap-2 rounded-lg border border-outline-variant/20 px-3 py-1.5 text-label-md text-on-surface-variant opacity-80"
            data-validation-empty
        >
            <crate::ui::MaterialIcon name="check_circle" />
            <span>"No issues"</span>
            {checking
                .then(|| {
                    view! {
                        <span class="ml-auto text-label-sm text-outline" data-validation-checking>
                            "re-checking…"
                        </span>
                    }
                })}
        </div>
    }
    .into_any()
}

/// The populated card: the rollup chip (always visible), and — when expanded — the grouped list and
/// the legend.
fn populated_view(
    rollup: Rollup,
    open: bool,
    checking: bool,
    expanded: RwSignal<bool>,
    findings: RwSignal<Vec<PanelFinding>>,
) -> AnyView {
    let chip = rollup.chip_text();
    // The chip's accent: red when any error blocks, else the advisory amber.
    let chip_accent = if rollup.has_blocking() {
        "text-error"
    } else {
        "text-tactical-yellow"
    };
    let caret = if open { "expand_more" } else { "chevron_right" };

    view! {
        <div class="glass flex flex-col overflow-hidden rounded-lg border border-outline-variant/30 shadow-xl">
            // ── Rollup chip — always visible when non-empty; click toggles the list ──
            <button
                type="button"
                class="flex items-center gap-2 px-3 py-2 text-left outline-none transition-colors hover:bg-surface-variant/40"
                data-validation-rollup
                aria-expanded=move || expanded.get()
                on:click=move |_| expanded.update(|e| *e = !*e)
            >
                <crate::ui::MaterialIcon name=caret />
                <crate::ui::MaterialIcon name="rule" />
                <span class=format!(
                    "text-label-md font-medium tabular-nums {chip_accent}",
                )>{chip}</span>
                {checking
                    .then(|| {
                        view! {
                            <span
                                class="ml-auto text-label-sm text-outline"
                                data-validation-checking
                            >
                                "re-checking…"
                            </span>
                        }
                    })}
            </button>

            // ── The grouped list + legend, only while expanded ──
            {open
                .then(|| {
                    view! {
                        <div class="flex max-h-[22rem] flex-col overflow-y-auto border-t border-outline-variant/20">
                            {group_list_view(findings.get())}
                        </div>
                        {legend_view()}
                    }
                })}
        </div>
    }
    .into_any()
}

/// The grouped-by-rule list: one header per rule (id + count), each with its findings as
/// click-to-select rows.
fn group_list_view(rows: Vec<PanelFinding>) -> AnyView {
    let groups = group_by_rule(&rows);
    view! {
        <div class="flex flex-col py-1" data-validation-list>
            {groups
                .into_iter()
                .map(rule_group_view)
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

/// One rule group: a small header (severity dot · rule id · count) then its finding rows.
fn rule_group_view(group: RuleGroup) -> AnyView {
    let count = group.count();
    let sev = severity_tag(group.severity);
    let dot = severity_dot_class(group.severity);
    let rule_id = group.rule_id.clone();
    let rule_id_attr = group.rule_id.clone();
    view! {
        <div class="px-1 pb-1" data-validation-group=rule_id_attr data-severity=sev>
            <div class="flex items-center gap-1.5 px-2 pt-1.5 pb-0.5">
                <span class=format!("inline-block size-2 rounded-full {dot}")></span>
                <span class="text-label-sm font-medium tracking-wide text-on-surface-variant">
                    {rule_id}
                </span>
                <span class="ml-auto text-label-sm tabular-nums text-outline">{count}</span>
            </div>
            {group
                .findings
                .into_iter()
                .map(finding_row_view)
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

/// Why an inert finding row is not a click target — peer of [`crate::eden_settings::inert_settings_row_reason`].
/// Positional findings name nobody; named subjects the probe refuses would be dead clicks.
#[must_use]
fn inert_finding_row_reason(f: &PanelFinding) -> String {
    match f.subject_id.as_deref() {
        None | Some("") => {
            "This finding names no selectable subject — there is nothing for click-to-select to              pin."
                .to_string()
        }
        Some(_) => {
            "Found, but not selectable from here: the editor's click-to-select router resolves no              selection for this subject right now, so a click would do nothing."
                .to_string()
        }
    }
}

/// One finding row — CLICK-TO-SELECT. Clicking routes `subject_id` → the editor selection so the
/// offender is pinned on the map + in the trees (NOT a clipboard dump — the ticket's explicit call).
///
/// Wave 129 — the row ASKS. `selectable` is [`finding_is_routable`]'s answer (the router's own
/// resolution of this subject), not "the finding names an id", and it gates the affordance and the
/// click TOGETHER through [`row_cursor_class`] so the two cannot disagree. A row the router resolves
/// nothing for — a positional/cardinality finding, a stale id, a subject kind no selection surface
/// owns — renders INERT rather than wearing a pointer over a dead click. `data-selectable` reports
/// the same boolean, so a gate can read the claim the row is making.
///
/// Wave 132 F3 / T-758 peer — element shape follows the same boolean: routable → focusable
/// `<button>`; inert → non-focusable `<div aria-disabled>` carrying [`inert_finding_row_reason`].
/// `data-selectable=false` alone used to short-circuit the click while leaving a tab-stop.
fn finding_row_view(f: PanelFinding) -> AnyView {
    let selectable = finding_is_routable(&f);
    // The selection key the click routes on (moved into the on:click closure).
    let click_id = f.subject_id.clone().unwrap_or_default();
    // The same id + subject as `data-` attributes (distinct owned copies — the view consumes each).
    let subject_id_attr = f.subject_id.clone().unwrap_or_default();
    let subject_attr = f.subject.clone();
    let message = f.message.clone();
    let subject_body = f.subject.clone();
    let cursor = row_cursor_class(selectable);
    let inert_reason = inert_finding_row_reason(&f);
    let row_class = format!(
        "flex w-full flex-col gap-0.5 rounded px-2 py-1 text-left outline-none transition-colors {cursor}",
    );
    let cells = view! {
        <span class="text-label-md leading-snug text-on-surface">{message}</span>
        <span class="text-label-sm text-outline">{subject_body}</span>
    };
    if selectable {
        view! {
            <button
                type="button"
                class=row_class
                data-validation-finding=subject_attr
                data-subject-id=subject_id_attr
                data-selectable="true"
                on:click=move |_| {
                    // Route only for a row the ROUTER resolved — the same boolean the styling used,
                    // so a click cannot happen where no affordance was drawn (or vice versa). The
                    // wasm-only op is a no-op on the host.
                    select_finding_subject(&click_id);
                }
            >
                {cells}
            </button>
        }
        .into_any()
    } else {
        view! {
            <div
                class=row_class
                data-validation-finding=subject_attr
                data-subject-id=subject_id_attr
                data-selectable="false"
                aria-disabled="true"
                title=inert_reason
            >
                {cells}
            </div>
        }
        .into_any()
    }
}

/// The severity ladder legend — Error / Warning / Info with each rung's meaning (the ticket's
/// "severity ladder legend"). Reads [`SEVERITY_LADDER`] so the panel and the ladder never drift.
fn legend_view() -> AnyView {
    view! {
        <div
            class="flex flex-col gap-1 border-t border-outline-variant/20 px-3 py-2"
            data-validation-legend
        >
            {SEVERITY_LADDER
                .iter()
                .map(|rung| {
                    let dot = severity_dot_class(rung.severity);
                    view! {
                        <div class="flex items-start gap-1.5">
                            <span class=format!(
                                "mt-1 inline-block size-2 shrink-0 rounded-full {dot}",
                            )></span>
                            <span class="text-label-sm text-on-surface-variant">
                                <span class="font-medium text-on-surface">{rung.label}</span>
                                " — "
                                {rung.meaning}
                            </span>
                        </div>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

/// The row's cursor/hover classes — **the affordance itself, as a function of one boolean**, so
/// "clickable" and "looks clickable" cannot be decided in two places and disagree. `clickable` is
/// [`finding_is_routable`]'s answer (the router's), never `subject_id.is_some()`. Peer of
/// `eden_settings::row_cursor_class`, deliberately identical in shape: the two click surfaces over
/// the one router state the rule the same way.
#[must_use]
fn row_cursor_class(clickable: bool) -> &'static str {
    if clickable {
        "cursor-pointer hover:bg-primary/10"
    } else {
        "cursor-default"
    }
}

/// The Tailwind background class for a severity's dot: red / amber / muted-blue — the ladder colours.
#[must_use]
fn severity_dot_class(s: Severity) -> &'static str {
    match s {
        Severity::Error => "bg-error",
        Severity::Warning => "bg-tactical-yellow",
        Severity::Info => "bg-primary",
    }
}

/// Route a finding's `subject_id` → the editor selection (click-to-select), through the registered
/// router ([`register_select_by_id`], installed from `mission_editor.rs`'s wasm mount where the
/// `!Send` doc/selection/engine handles live). A no-op on the host / pre-mount (no router
/// registered), so the native view compiles and a click there does nothing.
fn select_finding_subject(subject_id: &str) {
    route_select_by_subject_id(subject_id);
}

/// `performance.now()` in ms on wasm, a monotonic host clock otherwise — the Debouncer's time source.
#[cfg(target_arch = "wasm32")]
#[must_use]
fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}

/// Host time source (tests supply their own times to the Debouncer; this is only for completeness).
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
fn now_ms() -> f64 {
    0.0
}

/* ═══════════════════════════ tests (host-native) ═══════════════════════════ */

#[cfg(test)]
mod tests {
    use super::*;
    use map_engine_core::mission::validate::{default_registry, EvalContext};
    use serde_json::json;

    /// A payload that fires `ORBAT-CALLSIGN-UNIQUE`: BLUFOR with two squads both called "Alpha" (the
    /// same shape as the rule's own trip fixture, inlined here since the rule constructor is private
    /// to the engine crate). `sq2` is the second row (index 1) and the reported offender.
    fn duplicate_callsign_payload() -> serde_json::Value {
        json!({
            "editor": {
                "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1", "sq2"]}],
                "squads": [
                    {"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": []},
                    {"id": "sq2", "callsign": "Alpha", "name": "Alpha 1-2", "slotIds": []}
                ]
            }
        })
    }

    fn pf(rule_id: &str, severity: Severity, subject_id: Option<&str>) -> PanelFinding {
        PanelFinding {
            rule_id: rule_id.to_string(),
            severity,
            primitive: Primitive::PerObjectInvariant,
            message: format!("{rule_id} says no"),
            subject: format!("/x/{}", subject_id.unwrap_or("none")),
            subject_id: subject_id.map(str::to_string),
        }
    }

    /* ── Rollup counts ── */

    #[test]
    fn rollup_counts_by_severity() {
        let rows = vec![
            pf("A", Severity::Error, Some("s1")),
            pf("B", Severity::Error, Some("s2")),
            pf("C", Severity::Error, Some("s3")),
            pf("D", Severity::Warning, Some("s4")),
            pf("E", Severity::Warning, Some("s5")),
            pf("F", Severity::Info, Some("s6")),
        ];
        let r = Rollup::of(&rows);
        assert_eq!(r.errors, 3);
        assert_eq!(r.warnings, 2);
        assert_eq!(r.infos, 1);
        assert_eq!(r.total(), 6);
        assert!(!r.is_empty());
        assert!(r.has_blocking());
    }

    #[test]
    fn rollup_chip_text_is_the_one_line_summary() {
        // The ticket's example shape: "3 errors · 5 warnings". Worst first; only non-zero appear.
        let mut rows = Vec::new();
        for i in 0..3 {
            rows.push(pf("E", Severity::Error, Some(&format!("e{i}"))));
        }
        for i in 0..5 {
            rows.push(pf("W", Severity::Warning, Some(&format!("w{i}"))));
        }
        let r = Rollup::of(&rows);
        assert_eq!(r.chip_text(), "3 errors · 5 warnings");
    }

    #[test]
    fn rollup_chip_text_singular_and_omits_zero_severities() {
        let rows = vec![
            pf("E", Severity::Error, Some("e1")),
            pf("I", Severity::Info, Some("i1")),
        ];
        let r = Rollup::of(&rows);
        // Exactly one of each present → singular; the absent Warning band is omitted entirely.
        assert_eq!(r.chip_text(), "1 error · 1 info");
    }

    #[test]
    fn empty_rollup_is_empty_and_has_no_chip() {
        let r = Rollup::of(&[]);
        assert!(r.is_empty());
        assert!(!r.has_blocking());
        assert_eq!(r.chip_text(), "");
    }

    /* ── grouping by rule with counts ── */

    #[test]
    fn group_by_rule_groups_and_counts_worst_first() {
        let rows = vec![
            pf("WARN-RULE", Severity::Warning, Some("s1")),
            pf("ERR-RULE", Severity::Error, Some("s2")),
            pf("WARN-RULE", Severity::Warning, Some("s3")),
            pf("ERR-RULE", Severity::Error, Some("s4")),
            pf("WARN-RULE", Severity::Warning, Some("s5")),
        ];
        let groups = group_by_rule(&rows);
        assert_eq!(groups.len(), 2);
        // Errors sort ahead of warnings regardless of first-seen order.
        assert_eq!(groups[0].rule_id, "ERR-RULE");
        assert_eq!(groups[0].count(), 2);
        assert_eq!(groups[0].severity, Severity::Error);
        assert_eq!(groups[1].rule_id, "WARN-RULE");
        assert_eq!(groups[1].count(), 3);
    }

    /* ── click-to-select routing: subject_id → selection call pins ── */

    #[test]
    fn a_finding_with_a_subject_id_names_an_offender() {
        // Click-to-select routes on `subject_id` (T-657). This is the FACT that the rule kept an
        // offender id — NOT the claim that the row is clickable; that one belongs to the router
        // (`finding_is_routable`), see `w129_the_panel_asks_the_router`.
        let f = pf("ORBAT-SLOT-RESOLVES", Severity::Error, Some("slot-7"));
        assert!(f.is_selectable());
        assert_eq!(f.subject_id.as_deref(), Some("slot-7"));
    }

    #[test]
    fn a_positional_finding_names_no_offender() {
        // V2-FACTION-MAX / V4-SCHEMA-VERSION carry no entity id — their row renders, and renders
        // inert, because there is nothing for the router to resolve.
        let f = pf("V2-FACTION-MAX", Severity::Warning, None);
        assert!(!f.is_selectable());
        let blank = pf("X", Severity::Warning, Some(""));
        assert!(!blank.is_selectable(), "an empty subject_id names nobody");
    }

    #[test]
    fn subject_id_survives_the_flatten_from_an_engine_finding() {
        // The click-to-select KEY must survive `PanelFinding::from_finding` — the panel selects on
        // the flattened row, so if the flatten dropped `subject_id`, click-to-select would be dead.
        let payload = duplicate_callsign_payload();
        let engine_findings = default_registry().evaluate(&payload);
        let rows: Vec<PanelFinding> = engine_findings
            .iter()
            .map(PanelFinding::from_finding)
            .collect();
        let callsign = rows
            .iter()
            .find(|r| r.rule_id == "ORBAT-CALLSIGN-UNIQUE")
            .expect("callsign finding present");
        // The T-655 pointer fix: positional subject, stable id in subject_id (the selection key).
        assert_eq!(callsign.subject, "/editor/squads/1");
        assert_eq!(callsign.subject_id.as_deref(), Some("sq2"));
        assert!(callsign.is_selectable());
    }

    /* ── debounce behaviour (pure timer logic) ── */

    #[test]
    fn debounce_fires_once_after_the_trailing_window() {
        let mut d = Debouncer::new(REEVAL_DEBOUNCE_MS);
        d.bump(1000.0);
        // Not yet — the window has not elapsed.
        assert!(!d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS - 1.0));
        // Exactly at the window → due.
        assert!(d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS));
        assert!(d.take_fire());
        // Consumed — a second take with nothing pending is a no-op.
        assert!(!d.take_fire());
        assert!(!d.should_fire(1000.0 + 10_000.0));
    }

    #[test]
    fn debounce_a_burst_collapses_to_one_trailing_fire() {
        // The core contract: many bumps in a burst → ONE evaluation, the window after the LAST bump.
        let mut d = Debouncer::new(REEVAL_DEBOUNCE_MS);
        d.bump(1000.0);
        d.bump(1100.0);
        d.bump(1200.0); // last bump of the burst
                        // A check 250 ms after the FIRST bump must NOT fire — a newer bump reset the window.
        assert!(!d.should_fire(1000.0 + REEVAL_DEBOUNCE_MS));
        // 250 ms after the LAST bump → fires exactly once.
        assert!(d.should_fire(1200.0 + REEVAL_DEBOUNCE_MS));
        assert!(d.take_fire());
        assert!(!d.is_pending());
    }

    #[test]
    fn debounce_is_idle_until_first_bump() {
        let d = Debouncer::new(REEVAL_DEBOUNCE_MS);
        assert!(!d.is_pending());
        assert!(!d.should_fire(1_000_000.0));
    }

    /* ── NO SEVERITY ON CORRECT INPUT: a clean payload → an empty panel ── */

    #[test]
    fn a_clean_payload_produces_an_empty_panel() {
        // The ticket's hard rule, asserted at the panel level: a clean, well-formed mission run
        // through the SAME registry the panel uses yields zero findings, so `Rollup::is_empty()` and
        // the panel shows the quiet empty state — never a severity on correct input.
        //
        // "Clean" per the T-657 tightening: every squad has an identity (callsign) AND a leader; every
        // slot resolves a role AND a squad; ≤4 factions; a valid schemaVersion; slots in bounds.
        let clean = json!({
            "schemaVersion": 1,
            "map": {"terrain": "everon"},
            "editor": {
                "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
                "squads": [{
                    "id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1",
                    "slotIds": ["s1"], "leaderSlotId": "s1"
                }],
                "slots": [{
                    "id": "s1", "role": "SL",
                    "position": {"x": 6400.0, "y": 6400.0, "z": 0.0}
                }]
            }
        });
        let findings = default_registry().evaluate(&clean);
        let rows: Vec<PanelFinding> = findings.iter().map(PanelFinding::from_finding).collect();
        let rollup = Rollup::of(&rows);
        assert!(
            rollup.is_empty(),
            "a clean payload must produce NO findings (no severity on correct input); got: {rows:?}"
        );
    }

    #[test]
    fn a_clean_payload_stays_clean_with_a_supplied_catalogue() {
        // Same clean mission but now with a slot that carries an assetId that DOES resolve in the
        // supplied catalogue — the T-658 context path must also produce no findings on correct input.
        let asset = "{ABC}Prefabs/Characters/Rifleman.et";
        let clean = json!({
            "schemaVersion": 1,
            "map": {"terrain": "everon"},
            "editor": {
                "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
                "squads": [{
                    "id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1",
                    "slotIds": ["s1"], "leaderSlotId": "s1"
                }],
                "slots": [{
                    "id": "s1", "role": "SL", "assetId": asset,
                    "position": {"x": 6400.0, "y": 6400.0, "z": 0.0}
                }]
            }
        });
        let ids: std::collections::HashSet<String> = [asset.to_string()].into_iter().collect();
        let ctx = EvalContext::default().with_known_asset_ids(ids);
        let findings = default_registry().evaluate_with_context(&clean, &ctx);
        assert!(
            findings.is_empty(),
            "a clean payload with a resolvable asset must produce NO findings; got: {findings:?}"
        );
    }

    /* ── fire the rollup rule once: perturb → fail → restore (the ticket's fired proof) ── */

    #[test]
    fn perturbing_a_clean_mission_fires_the_rollup_then_restoring_clears_it() {
        // The ticket asks the rollup be fired once. Start clean (empty rollup), PERTURB the mission
        // into a defect (a second squad sharing the callsign on the same side → ORBAT-CALLSIGN-UNIQUE
        // fires), assert the rollup now counts it, then RESTORE and assert the rollup is empty again.
        let base_squads = |dup: bool| {
            let second = if dup { "Alpha" } else { "Bravo" };
            json!({
                "schemaVersion": 1,
                "map": {"terrain": "everon"},
                "editor": {
                    "factions": [{"key": "BLUFOR", "name": "US", "squadIds": ["sq1", "sq2"]}],
                    "squads": [
                        {"id": "sq1", "callsign": "Alpha", "name": "A", "slotIds": ["s1"], "leaderSlotId": "s1"},
                        {"id": "sq2", "callsign": second, "name": "B", "slotIds": ["s2"], "leaderSlotId": "s2"}
                    ],
                    "slots": [
                        {"id": "s1", "role": "SL", "position": {"x": 6400.0, "y": 6400.0, "z": 0.0}},
                        {"id": "s2", "role": "SL", "position": {"x": 6410.0, "y": 6410.0, "z": 0.0}}
                    ]
                }
            })
        };

        // Clean baseline: distinct callsigns → empty rollup.
        let clean_rows: Vec<PanelFinding> = default_registry()
            .evaluate(&base_squads(false))
            .iter()
            .map(PanelFinding::from_finding)
            .collect();
        assert!(
            Rollup::of(&clean_rows).is_empty(),
            "baseline must be clean; got {clean_rows:?}"
        );

        // Perturb: duplicate callsign on one side → the rule fires; the rollup counts a warning.
        let dirty_rows: Vec<PanelFinding> = default_registry()
            .evaluate(&base_squads(true))
            .iter()
            .map(PanelFinding::from_finding)
            .collect();
        let dirty_rollup = Rollup::of(&dirty_rows);
        assert!(
            !dirty_rollup.is_empty(),
            "perturbed mission must fire a finding"
        );
        assert!(
            dirty_rows
                .iter()
                .any(|r| r.rule_id == "ORBAT-CALLSIGN-UNIQUE"),
            "the perturbation must fire ORBAT-CALLSIGN-UNIQUE; got {dirty_rows:?}"
        );
        assert_eq!(dirty_rollup.warnings, 1);
        // The rollup chip renders the fired count.
        assert_eq!(dirty_rollup.chip_text(), "1 warning");

        // Restore: back to distinct callsigns → the rollup is empty again.
        let restored_rows: Vec<PanelFinding> = default_registry()
            .evaluate(&base_squads(false))
            .iter()
            .map(PanelFinding::from_finding)
            .collect();
        assert!(
            Rollup::of(&restored_rows).is_empty(),
            "restoring must clear the rollup; got {restored_rows:?}"
        );
    }

    /* ── evaluate_source: the panel's engine call is defensive + threads the catalogue ── */

    #[test]
    fn evaluate_source_runs_the_engine_and_flattens() {
        let source = PayloadSource {
            payload: duplicate_callsign_payload(),
            known_asset_ids: None,
        };
        let rows = evaluate_source(&source);
        assert!(rows.iter().any(|r| r.rule_id == "ORBAT-CALLSIGN-UNIQUE"));
    }

    #[test]
    fn evaluate_now_is_empty_without_a_registered_source() {
        // On the host (and pre-mount) no payload source is registered, so the panel evaluates to
        // empty rather than panicking — the "native build simply sees None" contract.
        assert!(evaluate_now().is_empty());
    }

    #[test]
    fn click_to_select_is_a_no_op_without_a_registered_router() {
        // The click-to-select seam: with no router registered (host / pre-mount) a finding click is
        // a safe no-op returning false — it never panics and never touches a disposed doc. On wasm
        // the router (installed from `mission_editor.rs`) does the real subject_id → selection route.
        assert!(!route_select_by_subject_id("slot-7"));
        // A registered router IS consulted, and its verdict is returned verbatim (id-shape agnostic).
        register_select_by_id(std::rc::Rc::new(|id: &str| id == "slot-7"));
        assert!(route_select_by_subject_id("slot-7"));
        assert!(!route_select_by_subject_id("slot-other"));
    }

    /* ── the severity ladder legend is complete ── */

    #[test]
    fn the_severity_ladder_covers_every_severity_with_a_meaning() {
        assert_eq!(SEVERITY_LADDER.len(), 3);
        assert_eq!(SEVERITY_LADDER[0].severity, Severity::Error);
        assert_eq!(SEVERITY_LADDER[1].severity, Severity::Warning);
        assert_eq!(SEVERITY_LADDER[2].severity, Severity::Info);
        for rung in SEVERITY_LADDER {
            assert!(!rung.label.is_empty());
            assert!(!rung.meaning.is_empty(), "{rung:?} needs a meaning");
        }
        // The tag hook agrees with the engine's spelling.
        assert_eq!(severity_tag(Severity::Error), "error");
        assert_eq!(severity_tag(Severity::Warning), "warning");
        assert_eq!(severity_tag(Severity::Info), "info");
    }
}

/* ═══════════ wave 129 — a finding row is clickable IFF the router resolves its subject ═══════════
 *
 * The defect T-754 killed on the settings surface, found alive here by the wave-129 adversarial
 * verifier and reachable today: `placed_asset_refs` emits `ASSET-RESOLVES` findings whose subject is
 * a placed-OBJECT id, `route_target` had no `entitiesById` arm, and the row styled itself
 * `cursor-pointer` off `subject_id.is_some()` — a GUESS about selectability rather than the router's
 * answer. Both halves are fixed; this module is the correspondence between them.
 *
 * The pin is the CORRESPONDENCE, not the arm: it compares what the row WEARS against what the router
 * RESOLVES, for every subject kind, in both directions, and refuses to be vacuous (it must have seen
 * both clickable and inert rows). Perturbation RED, either half: revert `finding_is_routable` to
 * `subject_id.is_some()`, or delete the Entity arm from `route_target`.
 */
#[cfg(test)]
mod w129_the_panel_asks_the_router {
    use super::{finding_is_routable, register_route_probe, row_cursor_class, PanelFinding};
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};
    use crate::mission_editor::route_target;
    use map_engine_core::mission::validate::{Primitive, Severity};
    use serde_json::json;

    /// A document root in `small_maps_json` shape carrying one row of every kind the router can meet
    /// — plus the two shapes it must refuse: an object row with no position, and (by omission) an id
    /// that is no longer in the document.
    fn doc() -> serde_json::Value {
        json!({
            "vehiclesById": { "v1": { "position": { "x": 7.0, "y": 9.0 } } },
            "entitiesById": {
                "e1": {
                    "id": "e1",
                    "alias": "prop:ammo_crate",
                    "position": { "x": 100.0, "y": 200.0, "z": 0.0, "rotation": 90.0 }
                },
                "e-nopos": { "id": "e-nopos", "alias": "prop:x" }
            },
            "zonesById": { "z1": { "shape": { "circle": { "x": 1.0, "z": 2.0, "r": 50.0 } } } }
        })
    }

    /// The one fact the small-maps root cannot answer, supplied here as the editor supplies it.
    fn is_slot(id: &str) -> bool {
        id == "slot-7"
    }

    fn pf(rule_id: &str, subject_id: Option<&str>) -> PanelFinding {
        PanelFinding {
            rule_id: rule_id.to_string(),
            severity: Severity::Error,
            primitive: Primitive::PerObjectInvariant,
            message: format!("{rule_id} says no"),
            subject: format!("/x/{}", subject_id.unwrap_or("none")),
            subject_id: subject_id.map(str::to_string),
        }
    }

    /// Install the probe exactly as `mission_editor`'s mount installs it: the router's own
    /// resolution over the document root, asked as a question.
    fn install_probe(root: serde_json::Value) {
        register_route_probe(std::rc::Rc::new(move |id: &str| {
            route_target(&root, id, &is_slot).is_some()
        }));
    }

    /// **THE pin: the affordance is true row by row.** What the row wears versus what the router
    /// resolves, for every subject kind, in both directions.
    #[test]
    fn a_finding_row_is_clickable_iff_the_router_resolves_its_subject() {
        let d = doc();
        install_probe(d.clone());
        let rows = [
            pf("ORBAT-SLOT-RESOLVES", Some("slot-7")), // slot
            pf("V-VEHICLE-ASSET", Some("v1")),         // vehicle
            pf("ASSET-RESOLVES", Some("e1")),          // placed object — the wave-129 arm
            pf("ZONE-SHAPE", Some("z1")),              // zone
            pf("ASSET-RESOLVES", Some("e-nopos")),     // object without a position
            pf("ASSET-RESOLVES", Some("e-deleted")),   // stale id (deleted since the last re-eval)
            pf("V2-FACTION-MAX", None),                // positional subject — no offender at all
            pf("X", Some("")),                         // an empty id names nobody
        ];
        let pointer = format!("cursor{}", "-pointer");
        let (mut clickable_seen, mut inert_seen) = (0usize, 0usize);
        for f in &rows {
            let wears_pointer = row_cursor_class(finding_is_routable(f)).contains(&pointer);
            let resolves = f
                .subject_id
                .as_deref()
                .is_some_and(|id| !id.is_empty() && route_target(&d, id, &is_slot).is_some());
            assert_eq!(
                wears_pointer, resolves,
                "wave 129: `{}` wears the click affordance = {wears_pointer}, but the router \
                 resolves it = {resolves}. A row must look clickable IFF clicking it selects \
                 something — a dead click dressed as an affordance is the whole defect.",
                f.subject
            );
            if resolves {
                clickable_seen += 1;
            } else {
                inert_seen += 1;
            }
        }
        // Not vacuous: the fixture exercised BOTH sides of the iff.
        assert!(
            clickable_seen >= 4 && inert_seen >= 4,
            "wave 129: this pin is only worth anything if it saw both clickable and inert rows \
             (saw {clickable_seen} / {inert_seen})"
        );
        // The reachable case, named: a placed-object finding — the row the verifier caught wearing a
        // pointer over nothing — is clickable now, and clickable BECAUSE the router resolves it.
        assert!(
            finding_is_routable(&pf("ASSET-RESOLVES", Some("e1"))),
            "wave 129: an ASSET-RESOLVES finding on a placed object must route — the engine emits \
             these today, which is what made this the reachable half of the defect"
        );
        // And the affordance is the styling, not a name: the "yes" class carries pointer AND hover,
        // the "no" class carries neither.
        let yes = row_cursor_class(true);
        let no = row_cursor_class(false);
        assert!(
            yes.contains(&pointer) && yes.contains("hover:"),
            "wave 129: a clickable row must actually LOOK clickable"
        );
        assert!(
            !no.contains(&pointer) && !no.contains("hover:"),
            "wave 129: an unroutable row must wear neither cursor-pointer nor a hover state"
        );
    }

    /// With no router installed — the host build, and the editor before its mount — every row is
    /// INERT. The safe direction: a panel that cannot ask does not claim.
    #[test]
    fn with_no_router_registered_every_row_renders_inert() {
        // A probe that resolves nothing stands in for "no probe": both are the `false` answer, and
        // this thread's thread_local is the panel's whole state.
        register_route_probe(std::rc::Rc::new(|_: &str| false));
        for f in [
            pf("ASSET-RESOLVES", Some("e1")),
            pf("ORBAT-SLOT-RESOLVES", Some("slot-7")),
            pf("V2-FACTION-MAX", None),
        ] {
            assert!(
                !finding_is_routable(&f),
                "wave 129: with nothing to ask, a row must render inert rather than hopeful"
            );
        }
    }

    /// One decision, one place. The row takes its classes from [`row_cursor_class`] and its
    /// `clickable` from [`finding_is_routable`] — which asks the SHIPPED router — and no live code in
    /// this panel decides clickability from the mere presence of an id.
    #[test]
    fn the_row_never_guesses_at_selectability() {
        let src = live_code(include_str!("validation_panel.rs"));
        let row = only_body(&src, &format!("fn finding{}", "_row_view"));
        assert!(
            row.contains(&format!("finding{}", "_is_routable(")),
            "wave 129: the row must decide clickability by asking the router"
        );
        assert!(
            row.contains(&format!("row{}", "_cursor_class(")),
            "wave 129: the row must take its cursor/hover classes from the one affordance function"
        );
        let routable = only_body(&src, &format!("fn finding{}", "_is_routable"));
        assert!(
            routable.contains(&format!("subject_id{}", "_routes")),
            "wave 129: clickability must be the ROUTER's resolution, not a second opinion about \
             which findings look selectable"
        );
        // NEGATIVE — the widest haystack in which the claim is even statable: the whole of this
        // file's LIVE code (the test module, which legitimately asserts the FACT `is_selectable`
        // states, is cut first). Any view or affordance code that calls it again goes red here.
        assert_eq!(
            src.matches(&format!(".is{}()", "_selectable")).count(),
            0,
            "wave 129: no live code in this panel may take `names an id` for `is clickable` — that \
             substitution IS the defect"
        );
        // Literals KEPT (the class text is code that ships), over the whole file's live half: the
        // affordance is spelled in exactly ONE place, so a second pointer class cannot appear beside
        // a hand-rolled guard.
        let lit = live_source(include_str!("validation_panel.rs"));
        assert_eq!(
            lit.matches(&format!("cursor{}", "-pointer")).count(),
            1,
            "wave 129: `cursor-pointer` belongs to `row_cursor_class` and nowhere else"
        );
        let row_lit = only_body(&lit, &format!("fn finding{}", "_row_view"));
        assert!(
            !row_lit.contains(&format!("cursor{}", "-pointer")),
            "wave 129: the row must not hand-roll the affordance beside the function that owns it"
        );
    }
}

// Wave 132 F3 — inert validation finding rows must not be focusable dead buttons. Peer of T-758
// (`eden_settings::t758_inert_row_a11y`): clickable → `<button>`; inert → non-focusable
// `<div aria-disabled>` with [`inert_finding_row_reason`]. Clickability remains
// [`finding_is_routable`] → `subject_id_routes`. Needles are fragment-assembled; `live_code` /
// `live_source` blank literals and cut test modules so a hollow comment cannot green these pins.
#[cfg(test)]
mod w132_inert_finding_row_a11y {
    use super::{
        finding_is_routable, inert_finding_row_reason, register_route_probe, row_cursor_class,
        PanelFinding,
    };
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};
    use map_engine_core::mission::validate::{Primitive, Severity};

    fn pf(rule_id: &str, subject_id: Option<&str>) -> PanelFinding {
        PanelFinding {
            rule_id: rule_id.to_string(),
            severity: Severity::Error,
            primitive: Primitive::PerObjectInvariant,
            message: format!("{rule_id} says no"),
            subject: format!("/x/{}", subject_id.unwrap_or("none")),
            subject_id: subject_id.map(str::to_string),
        }
    }

    /// Positional / empty-id findings name nobody: inert with an explicit reason. Affordance stays
    /// glued to [`finding_is_routable`].
    #[test]
    fn a_positional_finding_row_is_inert_with_a_reason() {
        register_route_probe(std::rc::Rc::new(|_: &str| true));
        let f = pf("V2-FACTION-MAX", None);
        assert!(
            !finding_is_routable(&f),
            "wave 132: a positional finding must stay inert even under an always-true probe"
        );
        assert!(
            !row_cursor_class(finding_is_routable(&f)).contains("cursor-pointer"),
            "wave 132: an inert finding must wear no pointer affordance"
        );
        let reason = inert_finding_row_reason(&f);
        assert!(
            reason.to_lowercase().contains("no selectable")
                || reason.to_lowercase().contains("nothing"),
            "wave 132: inert reason must tell the author why the row is not a click target, got              {reason:?}"
        );
    }

    /// Named subject the probe refuses — same inert shape; reason names the refusal.
    #[test]
    fn an_unroutable_finding_row_is_inert_with_a_reason() {
        register_route_probe(std::rc::Rc::new(|_: &str| false));
        let f = pf("ASSET-RESOLVES", Some("e1"));
        assert!(
            !finding_is_routable(&f),
            "wave 132: probe refusal ⇒ not clickable"
        );
        let reason = inert_finding_row_reason(&f);
        assert!(
            reason.to_lowercase().contains("not selectable")
                || reason.to_lowercase().contains("resolves no"),
            "wave 132: entity inert reason must name the router refusal, got {reason:?}"
        );
    }

    /// **THE shape pin.** `finding_row_view` must branch: selectable → `<button>`; inert →
    /// non-focusable element with `aria-disabled` + `inert_finding_row_reason`. Restoring the
    /// always-`<button>` shape makes this red (wave-115 MINOR class / T-758 peer).
    #[test]
    fn an_inert_finding_row_is_not_a_focusable_button() {
        let lit = live_source(include_str!("validation_panel.rs"));
        let row = only_body(&lit, &format!("fn finding{}", "_row_view"));
        assert!(
            row.contains("if selectable"),
            "wave 132: the row must BRANCH on the same boolean that owns clickability"
        );
        assert!(
            row.contains("<button") && row.contains("</button>"),
            "wave 132: a selectable finding must still be a real button"
        );
        assert!(
            row.contains("<div")
                && row.contains("aria-disabled")
                && row.contains(&format!("inert{}", "_finding_row_reason")),
            "wave 132: an inert finding must be a non-focusable element carrying aria-disabled and              the reason — not a tab-stop button that does nothing"
        );
        assert_eq!(
            row.matches("<button").count(),
            1,
            "wave 132: exactly one <button> in finding_row_view (the selectable arm)"
        );
    }

    /// Clickability remains the registered probe — shape follows that boolean, does not replace it.
    #[test]
    fn inert_finding_shape_still_asks_subject_id_routes() {
        let src = live_code(include_str!("validation_panel.rs"));
        let routable = only_body(&src, &format!("fn finding{}", "_is_routable"));
        assert!(
            routable.contains(&format!("subject_id{}", "_routes")),
            "wave 132: clickability must remain subject_id_routes"
        );
        let row = only_body(&src, &format!("fn finding{}", "_row_view"));
        assert!(
            row.contains(&format!("finding{}", "_is_routable(")),
            "wave 132: finding_row_view must still decide clickable via finding_is_routable"
        );
        assert!(
            !row.contains("matches!") && !row.contains("DocKind::"),
            "wave 132: finding_row_view must not hardcode kind lists for the element shape"
        );
    }
}

/* ══ wave-129 F5 — EVERY seam here is unregistered at unmount, and no remount is clobbered ═════════
 *
 * The lifecycle half of the dead click, pinned across all four of this file's thread_local seams at
 * once. Wave 129 fixed this shape three times before this test existed — `eden_dock_right`'s zone
 * hook (F2), and then the two seams F2 found here — while a FOURTH (the route probe) was being added
 * without it in the same wave. So the pin is table-driven: a fifth seam that forgets [`install_seam`]
 * joins this table and goes red, rather than shipping and being found by the next reader.
 *
 * These drive real `Owner`s and call `Owner::cleanup` — the code path leptos runs at unmount — in
 * the three shapes that matter:
 *   1. never installed                    -> the seam reports FAILURE (the baseline, so a green
 *                                            elsewhere cannot be "it was already false");
 *   2. install -> cleanup                 -> FAILURE, not `true`/`Some` over a DISPOSED no-op;
 *   3. install(A) -> install(B) -> A's cleanup -> B SURVIVES and still answers (the identity
 *      guard's entire reason for existing: leptos does not guarantee that a dying owner's cleanup
 *      runs before the remount registers).
 *
 * Perturbation RED, and they redden DIFFERENTLY, which is the point: drop the `on_cleanup` from
 * `install_seam` and shape 2 goes red; keep the cleanup but make it unconditional (delete the
 * `is_same_registration` guard from `unregister_seam`) and shape 3 goes red ALONE — that is the
 * failure a naive fix ships.
 */
#[cfg(test)]
mod f5_seam_lifecycle {
    use super::{
        publish_compile_findings, read_payload_source, register_panel_sink,
        register_payload_source, register_route_probe, register_select_by_id,
        route_select_by_subject_id, subject_id_routes, PanelFinding, PayloadSource,
    };
    use leptos::prelude::*;
    use map_engine_core::mission::validate::{Primitive, Severity};
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        /// Every tag that ANSWERED a seam's question, in call order. "Did anything actually happen"
        /// is answered by WHICH registration ran, not only by the seam's boolean.
        static ANSWERED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };

        /// The publish sink's candidate signals, one per tag. They are created under a LONG-LIVED
        /// owner and only INSTALLED from the short-lived one, so that after a cleanup the test can
        /// still tell "the seam is empty" apart from "the signal is disposed". A disposed signal
        /// swallows its `set` silently — that silence is the lie under test, so the test must not
        /// rely on it.
        static SINKS: RefCell<Vec<(&'static str, RwSignal<Vec<PanelFinding>>)>> =
            const { RefCell::new(Vec::new()) };
    }

    fn note(tag: &'static str) {
        ANSWERED.with(|l| l.borrow_mut().push(tag.to_string()));
    }
    fn answered() -> Vec<String> {
        ANSWERED.with(|l| l.borrow().clone())
    }
    fn forget_answers() {
        ANSWERED.with(|l| l.borrow_mut().clear());
    }

    /// A row to publish: the sink is asked "did a compile's findings reach you", so it needs one.
    fn row() -> PanelFinding {
        PanelFinding {
            rule_id: "F5-PROBE".into(),
            severity: Severity::Warning,
            primitive: Primitive::PerObjectInvariant,
            message: "f5".into(),
            subject: "f5".into(),
            subject_id: None,
        }
    }

    /// The long-lived signal registered for `tag` (created by [`prepare_sinks`]).
    fn sink_for(tag: &'static str) -> RwSignal<Vec<PanelFinding>> {
        SINKS
            .with(|s| {
                s.borrow()
                    .iter()
                    .find(|(t, _)| *t == tag)
                    .map(|(_, sig)| *sig)
            })
            .expect("prepare_sinks creates one signal per tag before anything is installed")
    }

    /// Create the sink signals under `owner`, which outlives every owner the seams are installed in.
    fn prepare_sinks(owner: &Owner) {
        let sinks = owner.with(|| {
            ["A", "B"]
                .map(|tag| (tag, RwSignal::new(Vec::<PanelFinding>::new())))
                .to_vec()
        });
        SINKS.with(|s| *s.borrow_mut() = sinks);
    }

    /// Publish, then report which sink (if any) the publish reached — the sink's own version of
    /// "did the seam answer, and who answered".
    fn ask_sink() -> bool {
        SINKS.with(|s| {
            for (_, sig) in s.borrow().iter() {
                sig.set(Vec::new());
            }
        });
        publish_compile_findings(vec![row()]);
        let mut reached = false;
        SINKS.with(|s| {
            for (tag, sig) in s.borrow().iter() {
                if !sig.get_untracked().is_empty() {
                    note(tag);
                    reached = true;
                }
            }
        });
        reached
    }

    /// One seam, reduced to the two operations its lifecycle turns on.
    struct Seam {
        /// The thread_local's name, so a failure names the seam rather than a row index.
        name: &'static str,
        /// Register a `tag`-marked value under the CURRENT reactive owner.
        install: fn(&'static str),
        /// Ask the seam its OWN question. `true` = a live registration reported success.
        ask: fn() -> bool,
    }

    /// All four seams this file publishes. A new one belongs here.
    fn seams() -> [Seam; 4] {
        [
            Seam {
                name: "PAYLOAD_SOURCE",
                install: |tag| {
                    register_payload_source(Rc::new(move || {
                        note(tag);
                        Some(PayloadSource {
                            payload: serde_json::json!({}),
                            known_asset_ids: None,
                        })
                    }));
                },
                ask: || read_payload_source().is_some(),
            },
            Seam {
                name: "SELECT_BY_ID",
                install: |tag| {
                    register_select_by_id(Rc::new(move |_id: &str| {
                        note(tag);
                        true
                    }));
                },
                ask: || route_select_by_subject_id("f5-subject"),
            },
            Seam {
                name: "ROUTE_PROBE",
                install: |tag| {
                    register_route_probe(Rc::new(move |_id: &str| {
                        note(tag);
                        true
                    }));
                },
                ask: || subject_id_routes("f5-subject"),
            },
            Seam {
                name: "PANEL_SINK",
                install: |tag| register_panel_sink(sink_for(tag)),
                ask: ask_sink,
            },
        ]
    }

    /// Shape 1 — the baseline. Nothing has ever been installed on this thread, so every seam must
    /// report failure. Without this, a green in the other two could just be "it was never true".
    #[test]
    fn a_seam_with_nothing_installed_reports_failure() {
        let root = Owner::new();
        prepare_sinks(&root);
        for seam in seams() {
            assert!(
                !(seam.ask)(),
                "F5 {}: nothing has ever been installed, so the seam must report failure",
                seam.name
            );
        }
        assert!(
            answered().is_empty(),
            "F5: no registration exists, so none can have answered — got {:?}",
            answered()
        );
    }

    /// Shape 2 — unmount unregisters. After the installing owner is cleaned up the seam must report
    /// FAILURE and the stale registration must not run. Reporting success here is the whole defect:
    /// the caller acts on that boolean while every `set` inside the dead closure lands on a DISPOSED
    /// signal, which `reactive_graph` 0.2.14 makes a silent no-op.
    #[test]
    fn unmount_unregisters_every_seam_so_none_reports_success() {
        let root = Owner::new();
        prepare_sinks(&root);
        for seam in seams() {
            let mounted = root.child();
            mounted.with(|| (seam.install)("A"));

            forget_answers();
            assert!(
                (seam.ask)(),
                "F5 {} precondition: while mounted the seam really does answer",
                seam.name
            );
            assert_eq!(
                answered(),
                vec!["A".to_string()],
                "F5 {} precondition: the LIVE registration is the one that answered",
                seam.name
            );

            mounted.cleanup();

            forget_answers();
            assert!(
                !(seam.ask)(),
                "F5 {}: the installing owner is gone, so the seam must report FAILURE rather than \
                 success over a disposed no-op",
                seam.name
            );
            assert!(
                answered().is_empty(),
                "F5 {}: the stale registration must not be called at all after unmount — got {:?}",
                seam.name,
                answered()
            );
        }
    }

    /// Shape 3 — the identity guard. A remount installs its NEWER value before the old owner's
    /// cleanup runs (leptos guarantees no other interleaving). The losing cleanup must recognise it
    /// is no longer the live registration and leave the new one alone — otherwise the fix for a
    /// stale seam becomes a fresh way to kill a live one, and the click is dead again.
    #[test]
    fn an_older_owners_cleanup_does_not_clobber_a_newer_registration() {
        let root = Owner::new();
        prepare_sinks(&root);
        for seam in seams() {
            // Siblings, not parent/child: two successive mounts under the page owner. A child would
            // be cleaned up BY the parent and would prove nothing about the guard.
            let old = root.child();
            let new = root.child();
            old.with(|| (seam.install)("A"));
            new.with(|| (seam.install)("B"));

            old.cleanup();

            forget_answers();
            assert!(
                (seam.ask)(),
                "F5 {}: the NEW mount is live — the superseded owner's cleanup must not unregister \
                 it",
                seam.name
            );
            assert_eq!(
                answered(),
                vec!["B".to_string()],
                "F5 {}: the surviving registration must be the NEWER one, not a leftover that \
                 merely happens to answer",
                seam.name
            );

            new.cleanup();

            forget_answers();
            assert!(
                !(seam.ask)(),
                "F5 {}: the live mount's OWN cleanup does clear it — the guard skips losers, not \
                 everyone",
                seam.name
            );
        }
    }

    /// T-783 — the mechanism is defined ONCE in the crate, and the definition is real code.
    ///
    /// Wave 129 wrote it here; T-778 could not import it (module-private) and copied the six lines
    /// into `ruler_tool`, leaving one identity trait and TWO mechanisms consulting it. That is the
    /// duplicated-vocabulary defect class, and this mechanism guards a defect family found five times
    /// in one wave and reintroduced once by a fix. So the count is pinned rather than trusted.
    ///
    /// **Unscoped by construction.** The input is the crate's whole `src` tree walked from
    /// `CARGO_MANIFEST_DIR`, not a hand-listed set of files — a third copy in `los_tool`, in
    /// `world_assets`, in a file that does not exist yet, reddens this. Two independent counts, and
    /// both must be 1:
    ///
    /// * over `live_code` — test modules cut, comments AND string literals blanked. This is the half
    ///   that proves the surviving definition is code that SHIPS, not prose describing one;
    /// * over the RAW bytes — which is deliberately the looser input here, because `live_code` cuts
    ///   from the first `#[cfg(test)]` to end-of-file, so a copy parked below a test module would be
    ///   invisible to it. As an upper bound (`<= 1`, expressed as `== 1` alongside the live count)
    ///   including the test half is exactly right: it is the direction where seeing MORE is safer.
    ///
    /// Superstring names are counted too: a definition suffixed `…_seam_later`, the decoy shape that
    /// greened wave 142's `RENDER_CTX` pin, still contains the needle and would REDDEN this one.
    /// Over-counting is the safe direction for a "there is exactly one" question.
    #[test]
    fn the_seam_mechanism_is_defined_exactly_once_in_the_crate() {
        // Fragment-assembled so this test's own body never carries the needle verbatim.
        let needles = [
            ["fn ", "install", "_seam"].concat(),
            ["fn ", "unregister", "_seam"].concat(),
        ];

        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            let entries = std::fs::read_dir(dir)
                .unwrap_or_else(|e| panic!("T-783: cannot read {}: {e}", dir.display()));
            for ent in entries {
                let path = ent.expect("read_dir entry").path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
        }
        let src_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        walk(&src_root, &mut files);
        files.sort();
        assert!(
            files.len() > 40,
            "T-783: the crate walk found only {} .rs files — the pin's input is wrong, so its \
             green would mean nothing",
            files.len()
        );

        // Scrub each file ONCE, not once per needle: `live_code` is O(file) with char-vector
        // copies and the crate carries a few 5k-line modules.
        let sources: Vec<(String, String, String)> = files
            .iter()
            .map(|path| {
                let raw = std::fs::read_to_string(path)
                    .unwrap_or_else(|e| panic!("T-783: cannot read {}: {e}", path.display()));
                let name = path
                    .strip_prefix(&src_root)
                    .unwrap_or(path)
                    .display()
                    .to_string();
                let live = crate::arsenal::class_r_scrub::live_code(&raw);
                (name, raw, live)
            })
            .collect();

        for needle in &needles {
            let mut live_hits: Vec<String> = Vec::new();
            let mut raw_hits: Vec<String> = Vec::new();
            for (name, raw, live) in &sources {
                let n_raw = raw.matches(needle.as_str()).count();
                if n_raw > 0 {
                    raw_hits.push(format!("{name} x{n_raw}"));
                }
                let n_live = live.matches(needle.as_str()).count();
                if n_live > 0 {
                    live_hits.push(format!("{name} x{n_live}"));
                }
            }
            assert_eq!(
                live_hits,
                vec!["validation_panel.rs x1".to_string()],
                "T-783: `{needle}` must be defined exactly ONCE in live crate code, beside the \
                 SeamRegistration trait it depends on. Found: {live_hits:?}. Import it \
                 (`crate::validation_panel::install_seam`, or the `ruler_tool` re-export the \
                 wasm-only seams already use) instead of writing a second copy — one identity check \
                 with two mechanisms is how the remount guard drifts out of one of them."
            );
            assert_eq!(
                raw_hits,
                vec!["validation_panel.rs x1".to_string()],
                "T-783: `{needle}` appears outside live code as well. Found: {raw_hits:?}. The raw \
                 count catches a copy the scrubber cannot see — it cuts from the first test-module \
                 attribute to end of file, so a definition parked below one would hide."
            );
        }
    }
}

/* ═══════════════════ T-761 — compile findings must not survive a mission switch ═══════════════════
 *
 * Wave-116 finding 3: `COMPILE_FINDINGS` is a thread_local written only by
 * `publish_compile_findings` (`export_compiled_now`). A clean compile already replaces the list
 * whole (pinned in mission_commands), but nothing reset the cell on editor hydrate. Client-side
 * `/missions/:id/edit` remounts reuse the wasm instance, so mission B inherited A's build report —
 * including `subject_id`s that resolve to nothing in B. This pin fails if that inheritance returns.
 */
#[cfg(test)]
mod t761_compile_findings_do_not_survive_mission_switch {
    use super::{
        clear_compile_findings, compile_findings, evaluate_now, publish_compile_findings,
        PanelFinding,
    };
    use map_engine_core::mission::validate::{Primitive, Severity};

    fn mission_a_compile_row() -> PanelFinding {
        PanelFinding {
            rule_id: "COMPILE-DROP-SQUAD-LEADER".into(),
            severity: Severity::Warning,
            primitive: Primitive::PerObjectInvariant,
            message: "mission A squad dropped".into(),
            subject: "A/Alpha".into(),
            // An id that exists only in mission A — clicking it on B selects nothing.
            subject_id: Some("mission-a-squad-1".into()),
        }
    }

    /// Behaviour: hydrate clear drops the previous mission's compile findings so evaluate_now
    /// cannot surface their subject_ids on the next mission.
    #[test]
    fn a_second_mission_does_not_inherit_the_previous_missions_compile_findings() {
        publish_compile_findings(vec![mission_a_compile_row()]);
        assert_eq!(
            compile_findings().len(),
            1,
            "precondition: mission A published a compile finding"
        );
        assert_eq!(
            compile_findings()[0].subject_id.as_deref(),
            Some("mission-a-squad-1")
        );

        // Mission B's editor hydrate — the production call site in MissionEditorPage.
        clear_compile_findings();

        assert!(
            compile_findings().is_empty(),
            "T-761: after hydrate clear, mission B must not inherit mission A's compile findings"
        );
        assert!(
            evaluate_now()
                .iter()
                .all(|r| r.subject_id.as_deref() != Some("mission-a-squad-1")),
            "T-761: evaluate_now must not surface mission A's subject_id on mission B; got {:?}",
            evaluate_now()
        );
    }

    /// Class-R — the clear is the named hydrate seam, not an accidental empty publish buried
    /// only in tests.
    #[test]
    fn clear_compile_findings_is_the_hydrate_reset_seam() {
        // wave-136 F3 — scope to the production body only. Whole-file `src.contains(…)` self-feeds
        // off this assert's own string literal, and a string decoy in production greened without
        // `live_code`.
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let src = live_code(include_str!("validation_panel.rs"));
        let body = only_body(&src, "pub fn clear_compile_findings(");
        assert!(
            body.contains("Vec::new()"),
            "T-761: clear_compile_findings must empty via Vec::new() in the production body; got:\n{body}"
        );
        assert!(
            body.contains("publish_compile_findings"),
            "T-761: clear must route through publish_compile_findings; got:\n{body}"
        );
    }
}
