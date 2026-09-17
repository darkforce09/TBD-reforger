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
//! **It IS THE TOP-BAR ERROR CHIP.** T-798 retired the floating card. The visible surface is now a
//! count chip in `eden_top_strip`'s row-2 toolbar — `"N errors · M warnings"` — that drops the
//! findings list (and the severity legend) on click, the operator-decision-3 shape. This module keeps
//! the eval engine, the [`Rollup`], and the list/legend rendering; the strip renders the chip and
//! calls [`findings_dropdown`] for the drop. [`ValidationPanel`] is now HEADLESS — it is the mounted
//! eval loop (doc_tick → debounce → evaluate), publishing into the [`chip_findings`] sink the strip
//! subscribes to. It renders no DOM of its own.
//!
//! ## Why the chip lives in the strip: Backspace hides it BY CONSTRUCTION
//!
//! The floating card was mounted OUTSIDE the `chrome_hidden` gate (the old "diagnostics survive
//! hide-chrome" call), so a Backspace hide-interface — the feature that exists to produce a clean
//! map screenshot — left the expanded legend in the corner (review F-35: buttons 62→19 but the
//! legend persisted). Moving the chip INTO the strip fixes that at its cause: `mission_editor` gates
//! the whole strip subtree on `chrome_hidden` (it leaves the DOM on Backspace), so the chip and its
//! dropdown hide with the rest of the chrome and there is no second gate to drift out of step — the
//! same by-construction gating the Controls Hint and the debug HUD get from living inside gated
//! chrome. Validation is still ALWAYS ON while the chrome is shown (T-635's doctrine); it is only
//! that a screenshot of the bare map is now actually bare.
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

use website_map_engine::data::scenario::validate::Finding;
use website_map_engine::data::scenario::validate::Primitive;
use website_map_engine::data::scenario::validate::Severity;

/// The trailing debounce window for a doc-change-driven re-evaluation, in milliseconds.
///
/// Chosen small (the maker wants near-live feedback) but not per-edit: the rules walk the whole
/// compiled payload, and an operation like a bulk paste bumps `doc_tick` on every intermediate
/// transaction (a drag bumps it once, at release — T-159.19). 250 ms trailing means the pass runs
/// once, a quarter-second after the LAST edit in a burst — live enough to feel immediate on a single
/// edit, cheap enough that a rapid edit burst runs the engine once, not once per commit.
pub const REEVAL_DEBOUNCE_MS: f64 = 250.0;

/// T-798 (a) — the mount-seed initial-eval poll. The doc hydrates on an async path, so the load-time
/// pass re-runs until [`read_payload_source`] resolves, then evaluates the true state once. Bounded
/// so a host / pre-mount build (which has no source and never will) stops after a fixed budget rather
/// than spinning: `INITIAL_EVAL_MAX_TICKS` × `INITIAL_EVAL_TICK_MS` ≈ 1.6 s, comfortably longer than
/// a local hydrate and far shorter than any user-noticeable stall. The poll re-arms nothing on the
/// doc_tick path, so the first genuine edit still evaluates exactly once.
pub const INITIAL_EVAL_TICK_MS: u64 = 50;
/// Max re-runs of the load-time pass before it gives up (host build has no source). See
/// [`INITIAL_EVAL_TICK_MS`].
pub const INITIAL_EVAL_MAX_TICKS: u32 = 32;

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
 * re-exports them so `crate::v2::apps::editor::input::tools::ruler_tool::install_seam` — the path `los_tool` and `world_assets` already
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
    items: &[crate::v2::core::api::dto::RegistryItem],
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
            set.insert(
                crate::v2::apps::editor::arsenal::asset_catalog::derive_object_alias(
                    &item.resource_name,
                    &item.display_name,
                ),
            );
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
    use website_map_engine::data::scenario::validate::default_registry;
    use website_map_engine::data::scenario::validate::EvalContext;

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

/// The mounted eval loop's findings signal — the ONE the strip's error chip subscribes to (T-798).
///
/// `PANEL_SINK` already holds exactly this signal (the headless [`ValidationPanel`] registers it at
/// mount, and both the doc_tick re-eval and a compile publish write it), so the chip reads its
/// findings from the same place a compile repaints — no second signal to keep in step. Returns `None`
/// before the eval loop has mounted (the strip renders first — `mission_editor.rs` mounts the strip
/// at ~:5923 and `ValidationPanel` at ~:6120); the chip's `doc_tick`-driven closure re-reads this on
/// the mount-seed tick, by which point the sink is registered, and thereafter subscribes to the
/// signal directly so a compile publish (which bumps no `doc_tick`) still repaints the chip.
#[must_use]
pub fn chip_findings() -> Option<RwSignal<Vec<PanelFinding>>> {
    PANEL_SINK.with(|c| *c.borrow())
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

/// The HEADLESS validation eval loop (T-798). Mounted ONCE from `mission_editor.rs` (ungated, so the
/// engine runs whether or not the chrome is shown), it renders NO DOM: the visible surface is the
/// top-strip error chip (`eden_top_strip`), which subscribes to the [`chip_findings`] sink this
/// component publishes. It kept its own mount rather than folding into the strip because the strip
/// unmounts on Backspace (`chrome_hidden`) and the debounce/timer state must survive that, and
/// because the engine must keep evaluating so the count is correct the instant the chrome returns.
///
/// `doc_tick` is the re-eval trigger: a wasm-only `Effect` subscribes to it and, through the
/// [`Debouncer`], schedules a trailing 250 ms re-evaluation. The findings live in a session signal
/// this component owns and registers as [`chip_findings`]; the strip reads it — so the chip updates
/// only after the debounce, never per intermediate edit. A load-time poll (T-798 a) seeds the true
/// state at t0 even though the doc hydrates async.
///
/// The `#[cfg]` split mirrors `AttributesModal`: the doc-reading re-eval is
/// `#[cfg(target_arch = "wasm32")]`; the native build compiles the shell so the strip's tests (which
/// import [`Rollup`] / [`findings_dropdown`]) run on the host.
#[component]
pub fn ValidationPanel(
    /// The doc-change tick (T-666 channel) every mutation site bumps — the re-eval trigger.
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    // The findings signal — written by the debounced re-eval effect (wasm) + a compile publish, read
    // by the top-strip chip through [`chip_findings`]. A plain session signal owned by this (once-
    // mounted, ungated) component, so it is stable for the whole editor session.
    let findings = RwSignal::new(Vec::<PanelFinding>::new());
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
        // (the arsenal `doll` / `sse.rs` idiom). A leaked trailing timer on route-leave checks this
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

        // ── T-798 (a) — LOAD-TIME evaluation, seeded from the PayloadSource, robust to async hydrate.
        //
        // Before this the initial pass was a SINGLE `set_timeout(0)`. That fires one frame after
        // mount, but the doc is hydrated on an ASYNC path (`mission_editor` restore/`hydrate_from_
        // server`), so at frame 0 `read_payload_source()` can still return `None` (doc mid-swap) and
        // the pass evaluates to EMPTY — the chip says "No issues" until the FIRST `doc_tick`. That is
        // review F-11: a mission that declares a faction but has no slots is unspawnable at t0
        // (V1-PLAYER-SPAWN), yet reported clean until an edit — and placing a marker (which mints
        // `faction-{SIDE}`) was the edit that "summoned an error about something you didn't touch".
        //
        // The fix: re-run the initial pass until the source is READY, then evaluate the true state.
        // A short bounded poll (not an unbounded spin — a host / pre-mount build has no source and
        // must not loop forever): each tick, if the payload source resolves, do ONE real eval and
        // stop; otherwise reschedule, up to `INITIAL_EVAL_MAX_TICKS`. This touches only the mount
        // seed — it does NOT re-arm the debounce, so the first genuine `doc_tick` still evaluates
        // exactly once (no double-eval on the first edit). If the source never arrives within the
        // budget the last run leaves an empty list, which is the same conservative empty the single
        // shot produced — never worse.
        {
            let run_eval = run_eval.clone();
            let disposed = disposed.clone();
            let attempts = Rc::new(std::cell::Cell::new(0u32));
            // `Rc<RefCell<Option<Box<dyn Fn()>>>>` so the closure can reschedule ITSELF by name.
            let seed: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
            let seed_run: Rc<dyn Fn()> = {
                let seed = seed.clone();
                Rc::new(move || {
                    if disposed.load(Ordering::Relaxed) {
                        return;
                    }
                    // Evaluate every tick: once the source is ready this paints the true state; while
                    // it is `None` it is a cheap empty re-set. `run_eval` is itself disposed-guarded.
                    run_eval();
                    // Ready ⇒ we have the true state, stop polling. Not ready and budget left ⇒
                    // reschedule the next frame. `read_payload_source` is the same readiness the eval
                    // uses, so "ready" here means the eval above was real.
                    let ready = read_payload_source().is_some();
                    let n = attempts.get() + 1;
                    attempts.set(n);
                    if !ready && n < INITIAL_EVAL_MAX_TICKS {
                        if let Some(next) = seed.borrow().as_ref().cloned() {
                            set_timeout(
                                move || next(),
                                std::time::Duration::from_millis(INITIAL_EVAL_TICK_MS),
                            );
                        }
                    } else {
                        // Terminated (ready, or budget spent): drop the self-reference so the
                        // `seed → seed_run → seed` Rc cycle is broken and the closure is freed rather
                        // than leaked for the session.
                        seed.borrow_mut().take();
                    }
                })
            };
            *seed.borrow_mut() = Some(seed_run.clone());
            set_timeout(move || seed_run(), std::time::Duration::from_millis(0));
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

    // ── HEADLESS (T-798) ──
    // The visible surface is the top-strip error chip (`eden_top_strip`), which subscribes to the
    // [`chip_findings`] sink this component publishes; the chip owns its own open/closed latch. This
    // component renders NO DOM: it is the mounted eval loop only, kept as its own mount (ungated,
    // mounted once from `mission_editor.rs`) so the engine runs and repaints the sink regardless of
    // whether the chrome is currently shown, while the chip that DISPLAYS the result rides the strip's
    // `chrome_hidden` gate (review F-35 — Backspace now hides the legend too).
}

/// The chip's dropdown BODY (T-798): the grouped findings list (or the quiet "No issues" empty state
/// when clean) followed by the severity legend. The top-strip error chip renders this when it is
/// dropped open; the CHIP itself (count text + accent) is strip furniture, so it lives there, but the
/// LIST and the LEGEND — and the pinned V1 copy inside them — stay HERE so there is one home for the
/// findings vocabulary. Callers wrap it in the strip's `MENU_PANEL` surface; this is the inner scroll
/// column + legend only.
///
/// The empty state stays a quiet "No issues", never a celebratory toast (the ticket's explicit call):
/// a clean mission is the baseline, not an achievement to announce. The legend content is retained
/// verbatim — the review pinned it into the dropdown.
#[must_use]
pub fn findings_dropdown(rows: Vec<PanelFinding>) -> AnyView {
    let empty = rows.is_empty();
    view! {
        <div class="flex flex-col" data-validation-dropdown>
            {if empty {
                view! {
                    <div
                        class="flex items-center gap-2 px-3 py-2.5 text-label-md text-on-surface-variant opacity-80"
                        data-validation-empty
                    >
                        <crate::v2::core::ui::MaterialIcon name="check_circle" />
                        <span>"No issues"</span>
                    </div>
                }
                    .into_any()
            } else {
                view! {
                    <div class="flex max-h-[22rem] flex-col overflow-y-auto">
                        {group_list_view(rows)}
                    </div>
                }
                    .into_any()
            }} {legend_view()}
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

/// Why an inert finding row is not a click target — peer of [`crate::v2::apps::editor::ui::modals::settings_modal::inert_settings_row_reason`].
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

#[cfg(test)]
#[path = "tests/validation_panel/finding_rollup_and_debounce.rs"]
mod finding_rollup_and_debounce_tests;
#[cfg(test)]
#[path = "tests/validation_panel/finding_route_probe.rs"]
mod finding_route_probe_tests;
#[cfg(test)]
#[path = "tests/validation_panel/inert_finding_accessibility.rs"]
mod inert_finding_accessibility_tests;
#[cfg(test)]
#[path = "tests/validation_panel/mission_switch_compile_findings.rs"]
mod mission_switch_compile_findings_tests;
#[cfg(test)]
#[path = "tests/validation_panel/registration_lifecycle.rs"]
mod registration_lifecycle_tests;
#[cfg(test)]
#[path = "tests/validation_panel/top_bar_findings_chip.rs"]
mod top_bar_findings_chip_tests;
