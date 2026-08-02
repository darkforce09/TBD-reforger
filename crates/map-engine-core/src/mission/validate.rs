//! Mission validation rule engine — the four primitives the validation group is built on (T-656).
//!
//! FNF's `MissionAnalyzer` declares 27 checks; 21 are live-evaluable, and when you stop reading them
//! as 21 features and start reading them as shapes, they are FOUR:
//!
//! * **V1 — required-entity presence, conditional on mission shape.** "A player-spawnable slot
//!   exists" is only a defect *when the mission declares players*; a blank draft that declares no
//!   sides is not missing anything. Conditionality is the whole point — it is why this tool never
//!   needs FNF's "the following missing items can be ignored" disclaimer. FNF runs every presence
//!   check unconditionally and then hands the operator a list of which failures to ignore; a rule
//!   that knows the shape it applies to does not fire on the shape it does not.
//! * **V2 — cardinality.** At most / at least N of a kind (FNF's "too many playable factions",
//!   count ceilings).
//! * **V3 — per-object invariant.** A predicate that must hold for every object of a kind (every
//!   slot inside terrain bounds).
//! * **V4 — field-shape / derivation.** A single field must parse / derive into a well-formed value
//!   (a version integer, a semver string).
//!
//! Four primitives is an ENGINE, not a feature, so this file is an engine: a [`Rule`] with a stable
//! [`Rule::id`], a [`Severity`], a [`Primitive`] kind, and an [`Rule::eval`] over the editor payload;
//! a [`Registry`] that runs every rule and returns EVERY finding (it never early-exits); and a
//! per-rule *mission-shape condition* ([`Rule::applies`]) that is how V1's conditionality is
//! expressed. The domain rule sets — ORBAT/slot (T-657), catalogue resolution (T-658), cargo/loadout
//! (T-660) — and the validation panel (T-655) build on these types. This ships the engine plus a
//! MINIMAL seed that exercises each primitive once on a payload shape that exists *today* (see
//! [`default_registry`]); it does not ship the domain rules, and it does not seed a rule whose
//! subject data the editor cannot yet produce.
//!
//! ## Same shape as the wire-safety scanners, on purpose
//!
//! `wire_safety::scan_editor_payload` and `scan_cargo_capacity` are already "editor payload in,
//! `Vec<String>` findings out, never early-exit" — the exact contract a validation pass wants. This
//! engine keeps that contract ([`Registry::evaluate`] → `Vec<Finding>`; `Finding::message` is the
//! same author-facing sentence those scanners emit) and adds the structure the domain waves need on
//! top: a stable id per finding, a severity, and the primitive it came from. Those scanners STAY
//! where they live; T-660 reconciles cargo into a V-kind rule, not this ticket.
//!
//! ## The founding defect this engine is built to be incapable of
//!
//! This program's founding defect is a tool reporting success over an input it never examined: FNF's
//! own validator runs 14 of its 27 checks and nobody noticed for years, because a check that silently
//! does nothing looks exactly like a check that passed. The engine is structured so that cannot
//! recur. A rule is a value in a registry, and EVERY rule in [`default_registry`] ships with a
//! fail-on-demand test — a fixture built to TRIP it, asserting the finding's `rule_id` and its
//! message, not merely that a clean payload stays green. A rule that cannot be made to fire has no
//! such test and is a hole in the suite by construction. On top of that, [`Registry::self_check`]
//! is an engine-level guard the caller can assert at startup: it runs every rule against its own
//! declared trip fixture and PANICS if any rule stays silent — so a rule that has quietly stopped
//! firing (its subject field renamed, its predicate inverted) is a loud failure at boot, not a check
//! that "passes" by doing nothing. The `engine_self_check_*` tests exercise both the healthy case and
//! a deliberately-misdeclared rule to prove the guard is not itself a silent pass.

use std::collections::HashSet;

use serde_json::Value;

use crate::mission::compile::terrain_bounds;

/// Ambient facts a rule may need that are NOT in the editor payload — the engine's evaluation
/// context (T-658). The engine is PURE core code with no access to the SPA's thread_locals, so a
/// rule that must resolve a placed asset against the *live* catalogue cannot reach it directly:
/// the caller (the SPA panel, T-655's W111 wiring) threads the live ids in through here instead.
///
/// Each field is an `Option` whose `None` means "this fact is not available in this call" — the
/// conservative default. A context-dependent rule reads its field and, when it is `None`, SKIPS via
/// its `applies` gate rather than guessing (see [`rule_asset_resolves`]): a cold-registry /
/// server-side call must not flag every asset as unknown just because the catalogue was not handed
/// in. `evaluate()` uses [`EvalContext::default`] (all `None`), so nothing context-dependent fires
/// unless a caller opts in with [`Registry::evaluate_with_context`].
///
/// `#[non_exhaustive]` on purpose: T-660 lands cargo/loadout rules next wave and may add fields
/// (e.g. a known-loadout set, capacity tables) — a new field is a non-breaking addition here, and
/// callers build the context with `..Default::default()` so they never have to name every field.
/// Add new facts as new `Option` fields; keep the "None ⇒ rule skips via its gate" discipline.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct EvalContext {
    /// The set of asset ids that resolve in the live registry catalogue — full Enfusion
    /// `resource_name`s AND any `veh:`/`prop:`/`comp:` aliases the catalogue exposes. `Some(set)`
    /// ⇒ the resolution rule runs against it; `None` ⇒ the catalogue was not supplied (cold /
    /// server-side) and the rule skips. See [`rule_asset_resolves`] for how a placed asset id is
    /// matched against it (exact id, plus alias forms the payload carries).
    pub known_asset_ids: Option<HashSet<String>>,
}

impl EvalContext {
    /// A context carrying a known-asset-id set — the shape the SPA panel builds from its
    /// `registry_session` cache (T-655 W111). Convenience over `EvalContext { known_asset_ids:
    /// Some(ids), ..Default::default() }` for the common case.
    #[must_use]
    pub fn with_known_asset_ids(ids: HashSet<String>) -> Self {
        Self {
            known_asset_ids: Some(ids),
        }
    }
}

/// How much a finding matters. The domain waves (T-657/T-658/T-660) and the panel (T-655) route on
/// this: an `Error` blocks, a `Warning` is advisory, `Info` is a note. The seed keeps the mapping
/// honest — a missing player spawn is an `Error`, an out-of-bounds slot is an `Error`, a soft ceiling
/// is a `Warning`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    /// Stable lowercase tag for a finding payload / log line (`"error"`, `"warning"`, `"info"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

/// Which of the four primitives a rule is an instance of. Carried on every [`Finding`] so a
/// downstream consumer (the panel, an analytics pass) can group findings by the shape of the check
/// that produced them without re-deriving it from the id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Primitive {
    /// V1 — required-entity presence, conditional on mission shape.
    RequiredEntity,
    /// V2 — cardinality (at most / at least N of a kind).
    Cardinality,
    /// V3 — per-object invariant (a predicate over every object of a kind).
    PerObjectInvariant,
    /// V4 — field-shape / derivation (a field parses into a well-formed value).
    FieldShape,
}

impl Primitive {
    /// The `V1`..`V4` tag, for a finding payload or a grouped panel header.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Primitive::RequiredEntity => "V1",
            Primitive::Cardinality => "V2",
            Primitive::PerObjectInvariant => "V3",
            Primitive::FieldShape => "V4",
        }
    }
}

/// One thing a rule found wrong. The stable half (`rule_id`, `severity`, `primitive`) lets a consumer
/// filter/group without parsing prose; `message` is the author-facing sentence (same register as the
/// wire-safety scanners — where, what, why, in one line); `subject` is the JSON-pointer-ish path into
/// the payload the author can act on (`/editor/slots/3/position`), so the panel can focus the offender.
///
/// `subject_id` is the **stable entity id** the finding is about (a slot id, a squad id), when the
/// rule knows one. It is a T-655 forward constraint (from the wave-104 verifier): the panel's
/// click-to-select needs the entity id, not just the positional JSON pointer — a pointer like
/// `/editor/slots/3` shifts when slot 2 is deleted, but `s1` does not, so the pointer is for
/// display/focus and the id is for selection. The seed rules leave it `None` (their subjects are
/// positional and, for `V2-FACTION-MAX` / `V4-SCHEMA-VERSION`, not a single entity at all); the
/// T-657 ORBAT/slot rules populate it with the offending slot or squad id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub primitive: Primitive,
    pub message: String,
    pub subject: String,
    /// Stable id of the entity this finding is about (slot/squad id), or `None` when the rule's
    /// subject is positional or not a single entity. See the struct doc (T-655 forward constraint).
    pub subject_id: Option<String>,
}

/// A validation rule: a stable identity + primitive kind, a mission-shape gate, and an evaluator.
///
/// The gate ([`applies`](Rule::applies)) is how V1's conditionality is a first-class part of the
/// engine rather than an `if` buried in one rule's body: the registry asks every rule whether it
/// applies to *this* payload before it evaluates, so a rule that does not apply produces no findings
/// AND is not silently skipped — the distinction the founding defect erased. A rule whose gate is
/// "always" (most V2/V3/V4 rules) just returns `true`.
///
/// `eval` returns every finding the rule sees on the payload (a V3 invariant can return one per
/// offending object). It is only ever called when `applies` returned `true`, and it must not itself
/// early-exit across the objects it walks — returning all findings is the engine's contract. It also
/// receives the [`EvalContext`] (T-658) so a rule can consult ambient facts (the live catalogue) the
/// payload does not carry; a payload-only rule simply ignores it.
///
/// `applies` receives the same `(payload, ctx)` pair: a context-dependent rule expresses "I have no
/// facts to check against, so I do not apply" as a gate condition (e.g. `ctx.known_asset_ids
/// .is_some()`), which is how a `None` context makes the rule *deliberately inert* rather than
/// silently wrong — the same first-class conditionality V1 uses for mission shape.
///
/// `trip_fixture` is the payload that MUST make this rule fire. It is not test scaffolding bolted on
/// the side: it lives on the rule so [`Registry::self_check`] can prove, at the engine level, that
/// the rule is still capable of firing. A rule author cannot add a rule to the registry without also
/// stating the input that trips it, which is exactly the property whose absence let FNF ship 14 dead
/// checks.
///
/// `trip_context` is the companion for a **context-dependent** rule: the [`EvalContext`] its
/// `trip_fixture` must be evaluated *against* to fire. `None` (the default, via
/// [`Rule::no_trip_context`]) means "a default context suffices" — every payload-only rule. A rule
/// whose `applies`/`eval` reads a context field MUST supply a `trip_context`, or its self-check runs
/// against the default (all-`None`) context, its gate excludes its own trip, and it is reported as a
/// loud failure — exactly the "a context rule that cannot fire is a loud failure" discipline T-658
/// requires, so the seam cannot hide a dead context rule any more than the base engine hides a dead
/// payload rule.
pub struct Rule {
    id: &'static str,
    severity: Severity,
    primitive: Primitive,
    /// Mission-shape / context gate. `applies(payload, ctx) == false` ⇒ the rule contributes nothing
    /// to this payload's findings (V1 conditionality; T-658 context conditionality). Defaults to
    /// always-applies for shape- and context-independent rules.
    applies: fn(&Value, &EvalContext) -> bool,
    /// The evaluator. Called only when [`applies`](Rule::applies) held; returns ALL findings. Reads
    /// the [`EvalContext`] for ambient facts (T-658); payload-only rules ignore it.
    eval: fn(&Rule, &Value, &EvalContext) -> Vec<Finding>,
    /// A payload that this rule is REQUIRED to fire on — the self-check's oracle (see the struct doc).
    trip_fixture: fn() -> Value,
    /// The context [`trip_fixture`](Rule::trip_fixture) must be evaluated against to fire (T-658).
    /// `None` ⇒ a default context suffices (payload-only rules). See the struct doc.
    trip_context: fn() -> Option<EvalContext>,
}

impl Rule {
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    #[must_use]
    pub const fn primitive(&self) -> Primitive {
        self.primitive
    }

    /// Whether this rule applies to `payload`'s mission shape (V1 conditionality) and the ambient
    /// [`EvalContext`] (T-658 context conditionality). A rule that does not apply is *deliberately*
    /// inert here — not skipped-and-forgotten: the registry records the distinction, and the rule's
    /// own `trip_fixture` (+ `trip_context`) still proves it can fire when it does apply.
    #[must_use]
    pub fn applies(&self, payload: &Value, ctx: &EvalContext) -> bool {
        (self.applies)(payload, ctx)
    }

    /// Evaluate against `payload` with a **default** (empty) context — the back-compat entry. A rule
    /// gated on a context field (T-658) does not fire here; use [`evaluate_with_context`]
    /// (Rule::evaluate_with_context) to supply the live catalogue.
    #[must_use]
    pub fn evaluate(&self, payload: &Value) -> Vec<Finding> {
        self.evaluate_with_context(payload, &EvalContext::default())
    }

    /// Evaluate against `payload` and `ctx`, honouring the gate: returns `[]` when the rule does not
    /// apply, otherwise every finding the evaluator produced.
    #[must_use]
    pub fn evaluate_with_context(&self, payload: &Value, ctx: &EvalContext) -> Vec<Finding> {
        if !self.applies(payload, ctx) {
            return Vec::new();
        }
        (self.eval)(self, payload, ctx)
    }

    /// The payload this rule must fire on. Used by [`Registry::self_check`]; exposed so a caller can
    /// audit the trip corpus.
    #[must_use]
    pub fn trip_fixture(&self) -> Value {
        (self.trip_fixture)()
    }

    /// The context this rule's [`trip_fixture`](Rule::trip_fixture) must be evaluated against to fire
    /// (T-658), or `None` when a default context suffices. Used by [`Registry::self_check`].
    #[must_use]
    pub fn trip_context(&self) -> Option<EvalContext> {
        (self.trip_context)()
    }

    /// Convenience for an `eval` body: build a finding carrying this rule's stable identity, with no
    /// entity `subject_id` (positional subject). The seed rules use this; the T-657 rules that know
    /// the offending entity id use [`finding_id`](Rule::finding_id).
    fn finding(&self, message: String, subject: String) -> Finding {
        Finding {
            rule_id: self.id,
            severity: self.severity,
            primitive: self.primitive,
            message,
            subject,
            subject_id: None,
        }
    }

    /// Like [`finding`](Rule::finding) but carrying the stable id of the entity the finding is about
    /// (a slot or squad id) — the T-655 forward constraint. Use this in any rule whose subject IS a
    /// single identifiable entity so the panel can select it directly.
    fn finding_id(&self, message: String, subject: String, subject_id: String) -> Finding {
        Finding {
            rule_id: self.id,
            severity: self.severity,
            primitive: self.primitive,
            message,
            subject,
            subject_id: Some(subject_id),
        }
    }
}

/// A set of rules, run as one pass. `evaluate` returns the union of every applicable rule's findings
/// and NEVER early-exits — the design contract the domain waves rely on (all findings, always, so a
/// second defect is not hidden behind the first).
pub struct Registry {
    rules: Vec<Rule>,
}

/// One rule failed [`Registry::self_check`]: it stayed silent on a payload it declared it would fire
/// on. This is the "loud failure" the founding defect demands — surfaced as a value the caller can
/// assert on (`self_check` returns `Result`) and as the panic message behind
/// [`Registry::assert_self_check`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfCheckFailure {
    pub rule_id: &'static str,
    pub reason: String,
}

impl std::fmt::Display for SelfCheckFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rule {}: {}", self.rule_id, self.reason)
    }
}

impl Registry {
    /// Build a registry from an explicit rule list. Panics if two rules share an id — a duplicate id
    /// would make findings ambiguous to a consumer routing on it, and it is a load-time authoring
    /// error, not a runtime input.
    #[must_use]
    pub fn new(rules: Vec<Rule>) -> Self {
        for (i, r) in rules.iter().enumerate() {
            if rules[..i].iter().any(|o| o.id == r.id) {
                panic!("duplicate rule id in registry: {}", r.id);
            }
        }
        Self { rules }
    }

    /// The rules in this registry, in registration order.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Run every rule with a **default** (empty) context and return every finding — the back-compat
    /// entry ([`validate_editor_payload`] uses it). Context-dependent rules (T-658) stay inert here;
    /// pass a populated context via [`evaluate_with_context`](Registry::evaluate_with_context).
    #[must_use]
    pub fn evaluate(&self, payload: &Value) -> Vec<Finding> {
        self.evaluate_with_context(payload, &EvalContext::default())
    }

    /// Run every rule against `payload` and `ctx`, returning every finding. Order is: rules in
    /// registration order, and within a rule the evaluator's own order. No rule can suppress
    /// another's findings, and no finding is dropped — the "return all findings, never early-exit"
    /// contract. `ctx` carries ambient facts (the live catalogue) that payload-only rules ignore and
    /// context-dependent rules gate on (T-658).
    #[must_use]
    pub fn evaluate_with_context(&self, payload: &Value, ctx: &EvalContext) -> Vec<Finding> {
        let mut out = Vec::new();
        for rule in &self.rules {
            out.extend(rule.evaluate_with_context(payload, ctx));
        }
        out
    }

    /// Prove every rule is still capable of firing. For each rule, evaluate it against its own
    /// `trip_fixture` — **and its own `trip_context`** (T-658; a default context when the rule
    /// declares none) — and require that (a) the rule APPLIES to that fixture+context and (b) it
    /// produces at least one finding CARRYING ITS OWN id. A rule that stays silent — because its
    /// subject field was renamed out from under it, its predicate was inverted, its gate now excludes
    /// its own trip case, or a context-dependent rule shipped without the `trip_context` that lets it
    /// fire — is returned as a [`SelfCheckFailure`]. Returns `Ok(())` only when every rule fires.
    ///
    /// This is the engine-level answer to "a check that does nothing looks like a check that passed":
    /// here, a check that does nothing is a returned error. Extending the oracle to carry a context
    /// keeps that guarantee across the T-658 seam — a context rule that cannot fire (no supplied
    /// catalogue) is caught here as a loud failure, not passed by doing nothing.
    ///
    /// # Errors
    /// Returns the list of rules that failed to fire on their own trip fixture (+ context).
    pub fn self_check(&self) -> Result<(), Vec<SelfCheckFailure>> {
        let mut failures = Vec::new();
        for rule in &self.rules {
            let fixture = rule.trip_fixture();
            // A context-dependent rule states the context it needs; a payload-only rule declares
            // none and self-checks against the default (empty) context — same as `evaluate()`.
            let ctx = rule.trip_context().unwrap_or_default();
            if !rule.applies(&fixture, &ctx) {
                failures.push(SelfCheckFailure {
                    rule_id: rule.id,
                    reason:
                        "trip_fixture does not satisfy the rule's own `applies` gate (with its \
                             trip_context) — the rule can never fire on it"
                            .to_string(),
                });
                continue;
            }
            let findings = rule.evaluate_with_context(&fixture, &ctx);
            if !findings.iter().any(|f| f.rule_id == rule.id) {
                failures.push(SelfCheckFailure {
                    rule_id: rule.id,
                    reason:
                        "produced no finding carrying its own id on its trip_fixture — the rule \
                             has gone silent"
                            .to_string(),
                });
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures)
        }
    }

    /// [`self_check`](Registry::self_check), but panic on failure — the form a service calls once at
    /// startup so a dead rule takes the process down loudly instead of shipping.
    ///
    /// # Panics
    /// Panics if any rule fails its self-check.
    pub fn assert_self_check(&self) {
        if let Err(failures) = self.self_check() {
            let joined = failures
                .iter()
                .map(SelfCheckFailure::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            panic!("validation registry self-check failed: {joined}");
        }
    }
}

/// The seed registry: one rule per primitive, each on a payload shape the editor produces today.
///
/// * **V1 [`RequiredEntity`](Primitive::RequiredEntity)** — `V1-PLAYER-SPAWN`: a mission that
///   declares at least one faction (i.e. declares players) must have at least one slot to spawn them
///   into. GATED on the mission shape: a payload with no `editor.factions[]` declares no players and
///   the rule does not apply, so an empty draft is never flagged — the V1 conditionality that spares
///   this tool FNF's "you may ignore the following" list.
/// * **V2 [`Cardinality`](Primitive::Cardinality)** — `V2-FACTION-MAX`: at most four factions, the
///   four canonical sides (`BLUFOR/OPFOR/INDFOR/CIV` — `mission-editor-payload.schema.json`
///   `editorFaction` records the bound, "the editor mints at most one row per side"). A `Warning`,
///   because the editor does not itself prevent two rows slugging onto one side.
/// * **V3 [`PerObjectInvariant`](Primitive::PerObjectInvariant)** — `V3-SLOT-IN-BOUNDS`: every
///   `editor.slots[].position` with `x`/`y` outside `terrain_bounds(map.terrain)` is a finding. The
///   compiler already computes those bounds ([`terrain_bounds`]); a slot outside them spawns off the
///   playable terrain.
/// * **V4 [`FieldShape`](Primitive::FieldShape)** — `V4-SCHEMA-VERSION`: when `schemaVersion` is
///   present it must derive as a positive integer (the editor-payload format version — the schema
///   pins integer, this pins the value is usable). Absent is fine (fresh docs omit it).
///
/// Each rule carries the `trip_fixture` that [`Registry::self_check`] fires it against. That is the
/// acceptance bar of this ticket expressed in the data: no rule is in this list without an input that
/// proves it can fail.
///
/// ## The T-657 ORBAT/slot rules (this wave)
///
/// The seed above exercises each primitive on a payload shape the editor produced *at T-656*. T-657
/// adds the first **domain** rule set — five rules that query the ORBAT graph
/// ([`editor_squads`] / [`editor_slots`] / [`editor_factions`], the exact `compile_payload` shape):
///
/// * **[`ORBAT-SLOT-RESOLVES`](rule_orbat_slot_resolves)** — `Error`, V3. Every slot must resolve a
///   **role** (non-empty `role`) and a **squad** (its id appears in some squad's `slotIds`). This is
///   FNF's R3 (the one rule it rated `error`, mirrored here) and the anchor of the R3+R4+R5 collapse.
/// * **[`ORBAT-IDENTITY-FILLED`](rule_orbat_identity_filled)** — `Warning`, V3. No squad carries a
///   default/empty identity: a blank or whitespace-only `callsign` **or** `name`.
/// * **[`ORBAT-SQUAD-HAS-LEADER`](rule_orbat_squad_has_leader)** — `Warning`, V3. A non-empty squad
///   must name a `leaderSlotId` that is one of its own `slotIds`.
/// * **[`ORBAT-CALLSIGN-UNIQUE`](rule_orbat_callsign_unique)** — `Warning`, V3. No two squads on the
///   **same side** share a callsign (a duplicate makes the ORBAT tree ambiguous). Two sides may reuse
///   a callsign — that does NOT fire.
/// * **[`ORBAT-TEMPLATE-COVERAGE`](rule_orbat_template_coverage)** — `Warning`, V3. A squad
///   instantiated from a template (`template.requiredRoles`) must fill every required role. This is
///   the D3-D8 revival: six commented-out FNF per-squad coverage rules become ONE rule parameterised
///   by the squad's own template descriptor. A squad with no template is not checked (skips).
///
/// ### Why one query replaces FNF's R3/R4/R5 + D3-D8
///
/// FNF's `MissionAnalyzer` needed R3, R4 and R5 as *three* overlapping rules with hardcoded name
/// lists ONLY because Eden packs role and callsign into one `.sqm` string, so "does this slot have a
/// role" and "is this callsign a real one" could only be asked by pattern-matching that string
/// against a maintained list of known role/callsign spellings. TBD stores `role`, `callsign` and the
/// squad↔slot edges as typed fields, so the same three questions are a field-presence check, a
/// blank-string check and a set-membership check — no name list, and they cannot drift out of date.
/// D3-D8 were six near-identical per-squad-type coverage rules; a typed `requiredRoles` list makes
/// them one parameterised rule.
#[must_use]
pub fn default_registry() -> Registry {
    Registry::new(vec![
        rule_v1_player_spawn(),
        rule_v2_faction_max(),
        rule_v3_slot_in_bounds(),
        rule_v4_schema_version(),
        // ── T-657 ORBAT/slot rules ──
        rule_orbat_slot_resolves(),
        rule_orbat_identity_filled(),
        rule_orbat_squad_has_leader(),
        rule_orbat_callsign_unique(),
        rule_orbat_template_coverage(),
        // ── T-658 catalogue-resolution rule ──
        rule_asset_resolves(),
    ])
}

/// The `trip_context` a payload-only rule declares: none — its `trip_fixture` fires against the
/// default (empty) context, exactly as `evaluate()` runs it. Only a context-dependent rule (T-658,
/// e.g. [`rule_asset_resolves`]) overrides this with the context its trip fixture needs.
fn no_trip_context() -> Option<EvalContext> {
    None
}

/// Convenience: `default_registry().evaluate(payload)`. The one-call entry the API/SPA use.
#[must_use]
pub fn validate_editor_payload(payload: &Value) -> Vec<Finding> {
    default_registry().evaluate(payload)
}

/* ─────────────────────────── shared payload accessors ─────────────────────────── */

/// `editor.factions[]` as a slice, or empty. The editor graph is under `editor`; a payload with no
/// `editor` block (the `{}` a fresh mission stores) has no factions.
fn editor_factions(payload: &Value) -> &[Value] {
    payload
        .get("editor")
        .and_then(|e| e.get("factions"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// `editor.slots[]` as a slice, or empty.
fn editor_slots(payload: &Value) -> &[Value] {
    payload
        .get("editor")
        .and_then(|e| e.get("slots"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// `editor.squads[]` as a slice, or empty. Squad rows carry `id`, `callsign`, `name`, `slotIds`,
/// `leaderSlotId` verbatim from the document core (`compile_payload` clones `squadsById` whole).
fn editor_squads(payload: &Value) -> &[Value] {
    payload
        .get("editor")
        .and_then(|e| e.get("squads"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A TOP-LEVEL payload array (`vehicles[]`, `entities[]`) as a slice, or empty. Unlike the ORBAT
/// graph these live at the payload root, not under `editor` (`compile::compile_payload` copies
/// `vehiclesById`/`entitiesById` → top-level `vehicles`/`entities`). Total over any payload shape.
fn top_level_array<'a>(payload: &'a Value, key: &str) -> &'a [Value] {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A string field on an object as `&str`, or `""` — a missing key, a null, or a non-string are all
/// "absent" here. Total: never panics, so an `eval` calling it is a total function over any payload.
fn str_field<'a>(obj: &'a Value, key: &str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or("")
}

/// A string-array field as an iterator of `&str`, skipping any non-string element. Total over any
/// payload shape (a missing / non-array field yields an empty iterator; a `[1, "s1"]` yields `s1`).
fn str_array<'a>(obj: &'a Value, key: &str) -> impl Iterator<Item = &'a str> {
    obj.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_str)
}

/// A slot's stable id (`slots[].id`), or `""` when absent — the `subject_id` a slot-scoped finding
/// carries. Ids are minted by the document core (`slot-...`) and are non-empty in practice; a blank
/// id is itself a malformed row the rule still reports (with an empty `subject_id`), never a panic.
fn slot_id(slot: &Value) -> &str {
    str_field(slot, "id")
}

/// A squad's stable id (`squads[].id`), or `""` when absent.
fn squad_id(squad: &Value) -> &str {
    str_field(squad, "id")
}

/// The authored terrain key (`map.terrain`), defaulting to `everon` exactly as the compiler does
/// (`compile.rs`: `meta.terrain ?? 'everon'`). Feeds [`terrain_bounds`].
fn terrain_key(payload: &Value) -> &str {
    payload
        .get("map")
        .and_then(|m| m.get("terrain"))
        .and_then(Value::as_str)
        .unwrap_or("everon")
}

/* ─────────────────────────── V1 — required-entity presence ─────────────────────────── */

/// A mission "declares players" iff it has at least one faction row. Faction rows are how the editor
/// expresses sides/players; a payload with none is a draft that has declared nothing, and V1 must not
/// fire on it (conditionality). Shape-only gate — the `ctx` is unused (T-658 signature).
fn declares_players(payload: &Value, _ctx: &EvalContext) -> bool {
    !editor_factions(payload).is_empty()
}

fn rule_v1_player_spawn() -> Rule {
    Rule {
        id: "V1-PLAYER-SPAWN",
        severity: Severity::Error,
        primitive: Primitive::RequiredEntity,
        // GATE: only a mission that declares a faction has players to spawn. This is the line that
        // makes V1 conditional — an empty `{}` or a factionless draft is not "missing" a spawn.
        applies: declares_players,
        eval: |rule, payload, _ctx| {
            if editor_slots(payload).is_empty() {
                vec![rule.finding(
                    "This mission declares a faction but has no slots — there is nowhere for a \
                     player to spawn. Add at least one slot to a squad."
                        .to_string(),
                    "/editor/slots".to_string(),
                )]
            } else {
                Vec::new()
            }
        },
        // Trips because: factions declared (gate holds) AND slots empty.
        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
                    "squads": [{"id": "sq1", "callsign": "Alpha", "slotIds": []}],
                    "slots": []
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/* ─────────────────────────── V2 — cardinality ─────────────────────────── */

/// The canonical side count. `mission-editor-payload.schema.json` `editorFaction`:
/// `FACTION_SIDES = [BLUFOR, OPFOR, INDFOR, CIV]` — four, and the editor mints at most one row per
/// side.
const MAX_FACTIONS: usize = 4;

fn rule_v2_faction_max() -> Rule {
    Rule {
        id: "V2-FACTION-MAX",
        severity: Severity::Warning,
        primitive: Primitive::Cardinality,
        applies: |_, _| true,
        eval: |rule, payload, _ctx| {
            let n = editor_factions(payload).len();
            if n > MAX_FACTIONS {
                vec![rule.finding(
                    format!(
                        "{n} factions declared, but a mission has at most {MAX_FACTIONS} sides \
                         (BLUFOR / OPFOR / INDFOR / CIV) — extra faction rows will not map to a \
                         side."
                    ),
                    "/editor/factions".to_string(),
                )]
            } else {
                Vec::new()
            }
        },
        // Trips because: five faction rows exceed the ceiling of four.
        trip_fixture: || {
            let factions: Vec<Value> = (0..MAX_FACTIONS + 1)
                .map(
                    |i| serde_json::json!({"key": format!("SIDE{i}"), "name": format!("Side {i}")}),
                )
                .collect();
            serde_json::json!({ "editor": { "factions": factions } })
        },
        trip_context: no_trip_context,
    }
}

/* ─────────────────────────── V3 — per-object invariant ─────────────────────────── */

fn rule_v3_slot_in_bounds() -> Rule {
    Rule {
        id: "V3-SLOT-IN-BOUNDS",
        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,
        applies: |_, _| true,
        eval: |rule, payload, _ctx| {
            let [min_x, min_y, max_x, max_y] = terrain_bounds(terrain_key(payload));
            let mut out = Vec::new();
            // Walk EVERY slot — the invariant is per-object and the pass returns one finding per
            // offender (never early-exits on the first, so a second bad slot is not hidden).
            for (i, slot) in editor_slots(payload).iter().enumerate() {
                let Some(pos) = slot.get("position") else {
                    continue; // no position authored → nothing to test (T-357 accept-set posture)
                };
                let x = pos.get("x").and_then(Value::as_f64);
                let y = pos.get("y").and_then(Value::as_f64);
                let (Some(x), Some(y)) = (x, y) else {
                    continue; // a non-numeric coord is a shape fault, not this rule's (schema's job)
                };
                if x < min_x || x > max_x || y < min_y || y > max_y {
                    out.push(rule.finding(
                        format!(
                            "slot position ({x:.1}, {y:.1}) is outside the {} terrain bounds \
                             [{min_x:.0}, {min_y:.0}]–[{max_x:.0}, {max_y:.0}] — it would spawn off \
                             the playable map.",
                            terrain_key(payload),
                        ),
                        format!("/editor/slots/{i}/position"),
                    ));
                }
            }
            out
        },
        // Trips because: on the default (everon, 12800²) terrain, a slot at (20000, 20000) is out.
        trip_fixture: || {
            serde_json::json!({
                "map": {"terrain": "everon"},
                "editor": {
                    "slots": [
                        {"id": "s1", "role": "RFL", "position": {"x": 20000.0, "y": 20000.0, "z": 0.0}}
                    ]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/* ─────────────────────────── V4 — field-shape / derivation ─────────────────────────── */

fn rule_v4_schema_version() -> Rule {
    Rule {
        id: "V4-SCHEMA-VERSION",
        severity: Severity::Error,
        primitive: Primitive::FieldShape,
        applies: |_, _| true,
        eval: |rule, payload, _ctx| {
            let Some(raw) = payload.get("schemaVersion") else {
                return Vec::new(); // absent is fine — fresh docs omit it, compiler defaults to 1
            };
            // The field must DERIVE into a usable positive integer format-version. A string
            // "1" (the canonical-vs-editor namespace confusion the schema warns about), a float,
            // a zero, or a negative all fail the derivation.
            let ok = raw.as_u64().is_some_and(|v| v >= 1);
            if ok {
                Vec::new()
            } else {
                vec![rule.finding(
                    format!(
                        "schemaVersion must be a positive integer (the editor-payload format \
                         version); got {raw}. A string or fractional value here is the \
                         canonical-vs-editor namespace confusion."
                    ),
                    "/schemaVersion".to_string(),
                )]
            }
        },
        // Trips because: a STRING "1" is present, which does not derive to a u64 ≥ 1.
        trip_fixture: || serde_json::json!({ "schemaVersion": "1" }),
        trip_context: no_trip_context,
    }
}

/* ═══════════════════════════ T-657 — ORBAT / slot rules ═══════════════════════════ */
//
// These five rules query the ORBAT graph the way `orbat::derive_orbat_from_editor` does — the
// authored `editor` block, NOT the compiled `mission.schema.json` `orbat` map. The vocabulary is
// fixed by the document core's writers (`doc/store.rs`: `add_faction` → `key`/`squadIds`; `add_squad`
// → `id`/`callsign`/`name`/`slotIds`; `set_leader` → `leaderSlotId`; `add_slot` → `id`/`role`/…) and
// carried verbatim into the payload by `compile::compile_payload` (`editor.{factions,squads,slots}`
// are `Object.values(*ById)`). All five gate on `applies` only where a shape condition genuinely
// makes the rule inert (V1 conditionality); the rest apply always and simply produce no findings on
// a clean graph. Every `eval` is a TOTAL function over arbitrary JSON: it reads through `str_field` /
// `str_array` (which treat missing/null/wrong-typed as absent) and never indexes, unwraps or expects
// on payload data — a malformed payload yields findings or is skipped, never a panic (the wave-104
// forward constraint; proved by `orbat_rules_never_panic_on_garbage`).

/* ─────────────── ORBAT-SLOT-RESOLVES — every slot resolves a role AND a squad ─────────────── */

/// A mission "has an ORBAT" iff it declares at least one squad. With no squads there are no slot↔squad
/// edges to check and "unattached slot" is meaningless (a factionless/squadless draft is not broken),
/// so this rule — like V1 — is conditional on that shape. Shape-only gate — the `ctx` is unused
/// (T-658 signature).
fn declares_orbat(payload: &Value, _ctx: &EvalContext) -> bool {
    !editor_squads(payload).is_empty()
}

/// The set of slot ids referenced by *some* squad's `slotIds`. A slot whose id is absent from this
/// set is not filed under any squad — it resolves no squad.
fn attached_slot_ids(payload: &Value) -> std::collections::HashSet<&str> {
    let mut set = std::collections::HashSet::new();
    for sq in editor_squads(payload) {
        for id in str_array(sq, "slotIds") {
            set.insert(id);
        }
    }
    set
}

fn rule_orbat_slot_resolves() -> Rule {
    Rule {
        id: "ORBAT-SLOT-RESOLVES",
        // Error — mirrors FNF's R3, the single check it rated `error` rather than `warning`: a slot
        // that resolves no role or no squad does not compile to a usable seat.
        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,
        // GATE: only when the mission declares squads. A factionless/squadless draft has no ORBAT to
        // be inconsistent with — the same V1 conditionality that spares the tool an ignore-list.
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            let attached = attached_slot_ids(payload);
            let mut out = Vec::new();
            // Walk EVERY slot; report each offender (never early-exit — a second unresolved slot must
            // not hide behind the first).
            for (i, slot) in editor_slots(payload).iter().enumerate() {
                let id = slot_id(slot);
                let role = str_field(slot, "role").trim();
                let has_role = !role.is_empty();
                // A slot resolves a squad when its id is in some squad's slotIds. A blank id can be in
                // no squad's list (ids are minted non-empty), so it correctly reads as unattached.
                let has_squad = !id.is_empty() && attached.contains(id);
                if has_role && has_squad {
                    continue;
                }
                let missing = match (has_role, has_squad) {
                    (false, false) => "resolves neither a role nor a squad",
                    (true, false) => "is not filed under any squad",
                    (false, true) => "has no role",
                    (true, true) => unreachable!(),
                };
                out.push(rule.finding_id(
                    format!(
                        "slot {} {missing} — every slot must name a role and belong to a squad, \
                         or it compiles to no usable seat.",
                        if id.is_empty() { "(no id)" } else { id },
                    ),
                    format!("/editor/slots/{i}"),
                    id.to_string(),
                ));
            }
            out
        },
        // Trips because: the squad declares an ORBAT (gate holds) but slot `s1` is listed in no
        // squad's `slotIds` AND has an empty role — it resolves neither.
        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "squads": [{"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": []}],
                    "slots": [{"id": "s1", "role": ""}]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/* ─────────────── ORBAT-IDENTITY-FILLED — no default/empty identity fields ─────────────── */

fn rule_orbat_identity_filled() -> Rule {
    Rule {
        id: "ORBAT-IDENTITY-FILLED",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            let mut out = Vec::new();
            for (i, sq) in editor_squads(payload).iter().enumerate() {
                let id = squad_id(sq);
                let callsign_blank = str_field(sq, "callsign").trim().is_empty();
                let name_blank = str_field(sq, "name").trim().is_empty();
                // Both blank / whitespace-only is a default identity — the squad shows as a nameless
                // row and, callsign-side, decodes to `orbat_slots.callsign = ""` downstream. Reported
                // per squad so the panel can jump to the offender.
                if callsign_blank && name_blank {
                    out.push(rule.finding_id(
                        format!(
                            "squad {} has no callsign and no name — give it an identity so it is \
                             addressable in the ORBAT and the roster.",
                            if id.is_empty() { "(no id)" } else { id },
                        ),
                        format!("/editor/squads/{i}"),
                        id.to_string(),
                    ));
                }
            }
            out
        },
        // Trips because: squad `sq1` carries neither a callsign nor a name (both empty).
        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "squads": [{"id": "sq1", "callsign": "", "name": "", "slotIds": []}]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/* ─────────────── ORBAT-SQUAD-HAS-LEADER — no leaderless squads ─────────────── */

fn rule_orbat_squad_has_leader() -> Rule {
    Rule {
        id: "ORBAT-SQUAD-HAS-LEADER",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            let mut out = Vec::new();
            for (i, sq) in editor_squads(payload).iter().enumerate() {
                let id = squad_id(sq);
                let members: Vec<&str> = str_array(sq, "slotIds").collect();
                if members.is_empty() {
                    continue; // an empty squad has no body to lead — not this rule's concern
                }
                let leader = str_field(sq, "leaderSlotId");
                // A leader must be one of the squad's own bodies. Absent, blank, or pointing at a slot
                // outside this squad all read as leaderless (the document core's `set_leader` only
                // writes an id that is in `slotIds`, so a violation here is a genuinely broken row).
                let has_leader = !leader.is_empty() && members.contains(&leader);
                if !has_leader {
                    out.push(rule.finding_id(
                        format!(
                            "squad {} has {} slot(s) but no leader — one of its slots must be the \
                             leader (leaderSlotId).",
                            if id.is_empty() { "(no id)" } else { id },
                            members.len(),
                        ),
                        format!("/editor/squads/{i}"),
                        id.to_string(),
                    ));
                }
            }
            out
        },
        // Trips because: squad `sq1` holds a slot but names no leaderSlotId.
        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "squads": [{"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": ["s1"]}],
                    "slots": [{"id": "s1", "role": "SL"}]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/* ─────────────── ORBAT-CALLSIGN-UNIQUE — no duplicate callsigns within a side ─────────────── */

fn rule_orbat_callsign_unique() -> Rule {
    Rule {
        id: "ORBAT-CALLSIGN-UNIQUE",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            // Index squads by id so a faction's `squadIds` resolve to callsigns. The uniqueness scope
            // is ONE SIDE (one faction): two sides may reuse a callsign ("Alpha" on both BLUFOR and
            // OPFOR is legal and common) — that must NOT fire, so we group per faction, not globally.
            use std::collections::HashMap;
            let squads_by_id: HashMap<&str, &Value> = editor_squads(payload)
                .iter()
                .map(|s| (squad_id(s), s))
                .collect();
            let mut out = Vec::new();
            for faction in editor_factions(payload) {
                // callsign (trimmed, lowercased for a case-insensitive clash) → first squad id seen.
                let mut seen: HashMap<String, &str> = HashMap::new();
                for member_id in str_array(faction, "squadIds") {
                    let Some(sq) = squads_by_id.get(member_id) else {
                        continue; // dangling squad ref — not this rule's fault (SLOT-RESOLVES-adjacent)
                    };
                    let callsign = str_field(sq, "callsign").trim();
                    if callsign.is_empty() {
                        continue; // a blank callsign is IDENTITY-FILLED's concern, not a duplicate
                    }
                    let key = callsign.to_lowercase();
                    if let Some(&first) = seen.get(&key) {
                        let id = squad_id(sq);
                        out.push(rule.finding_id(
                            format!(
                                "callsign {callsign:?} is used by more than one squad on side {:?} \
                                 (also squad {first}) — callsigns must be unique within a side.",
                                str_field(faction, "key"),
                            ),
                            format!("/editor/squads/{member_id}/callsign"),
                            id.to_string(),
                        ));
                    } else {
                        seen.insert(key, member_id);
                    }
                }
            }
            out
        },
        // Trips because: BLUFOR lists two squads both called "Alpha".
        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1", "sq2"]}],
                    "squads": [
                        {"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": []},
                        {"id": "sq2", "callsign": "Alpha", "name": "Alpha 1-2", "slotIds": []}
                    ]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/* ─────────────── ORBAT-TEMPLATE-COVERAGE — D3-D8 revival (one parameterised rule) ─────────────── */

/// A squad's declared required roles, read from `squad.template.requiredRoles` (an array of role
/// strings). This is the T-657 D3-D8 shape: rather than six hardcoded per-squad-type coverage rules,
/// a squad instantiated from a template carries the template's required-role list on itself, and ONE
/// rule checks coverage against whatever it declares. A squad with no `template` block (or an empty /
/// wrong-typed `requiredRoles`) declares nothing required and is not checked — the "a squad with no
/// template skips the coverage rule" boundary.
fn required_roles(squad: &Value) -> Vec<&str> {
    squad
        .get("template")
        .and_then(|t| t.get("requiredRoles"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .collect()
}

fn rule_orbat_template_coverage() -> Rule {
    Rule {
        id: "ORBAT-TEMPLATE-COVERAGE",
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        applies: declares_orbat,
        eval: |rule, payload, _ctx| {
            use std::collections::HashSet;
            // Slot id → role, so a squad's `slotIds` resolve to the roles it actually fills.
            let role_of: std::collections::HashMap<&str, &str> = editor_slots(payload)
                .iter()
                .map(|s| (slot_id(s), str_field(s, "role").trim()))
                .collect();
            let mut out = Vec::new();
            for (i, sq) in editor_squads(payload).iter().enumerate() {
                let required = required_roles(sq);
                if required.is_empty() {
                    continue; // no template / no required roles → this rule does not apply to it
                }
                // The roles this squad's bodies actually fill (case-insensitive; blanks dropped).
                let filled: HashSet<String> = str_array(sq, "slotIds")
                    .filter_map(|id| role_of.get(id))
                    .map(|r| r.to_lowercase())
                    .filter(|r| !r.is_empty())
                    .collect();
                let mut missing: Vec<&str> = required
                    .iter()
                    .copied()
                    .filter(|r| !filled.contains(&r.to_lowercase()))
                    .collect();
                if missing.is_empty() {
                    continue;
                }
                missing.dedup(); // adjacent dupes in the template list collapse for the message
                let id = squad_id(sq);
                out.push(rule.finding_id(
                    format!(
                        "squad {} is missing required role(s) [{}] for its template — a squad \
                         instantiated from a template must fill every role the template requires.",
                        if id.is_empty() { "(no id)" } else { id },
                        missing.join(", "),
                    ),
                    format!("/editor/squads/{i}"),
                    id.to_string(),
                ));
            }
            out
        },
        // Trips because: squad `sq1`'s template requires [SL, MED] but its only slot fills SL — MED
        // is uncovered.
        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "squads": [{
                        "id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1",
                        "slotIds": ["s1"],
                        "leaderSlotId": "s1",
                        "template": {"requiredRoles": ["SL", "MED"]}
                    }],
                    "slots": [{"id": "s1", "role": "SL"}]
                }
            })
        },
        trip_context: no_trip_context,
    }
}

/* ═══════════════════════════ T-658 — catalogue-resolution rule ═══════════════════════════ */

// ASSET-RESOLVES revives MissionAnalyzer's dead rule D13: every PLACED asset must resolve in the
// live registry catalogue, catching modset drift the MOMENT an asset is placed rather than at
// compile. The check is context-dependent (T-658): the engine is PURE core with no access to the
// SPA's `mission_editor.rs` thread_local `registry_session` cache, so the caller threads the live
// ids in through `EvalContext.known_asset_ids` and the rule reads them from there. When that set is
// `None` — a cold registry or a server-side call that has no catalogue to check against — the rule
// SKIPS via its applies-gate: the conservative default (do not flag every asset as unknown just
// because nobody handed us the catalogue). The SPA wiring that fills the context from
// `registry_session` is T-655's panel work (W111), NOT this ticket; here the seam + rule are proven
// by tests that build the context directly.

/// Asset-id PREFIXES a placed object may carry as an ALIAS, in addition to the exact
/// `resource_name`. Mirrored (not imported — the engine must not depend on frontend code) from the
/// alias derivation in `apps/website/frontend/src/asset_catalog.rs::derive_object_alias`
/// (`fn derive_object_alias` @ ~L352): a placed object's alias is `comp:<slug>` when its
/// `resource_name` names a Composition, else `prop:<slug>`; vehicles carry a `veh:` alias in the mod
/// spawn registry (`apps/mod/tbd-framework/Data/registry.json` entries — `veh:`/`prop:`/`comp:`).
/// The rule treats an id that starts with one of these as an alias form and resolves it against the
/// same `known_asset_ids` set — a catalogue that lists a placed object by its alias resolves it.
const ASSET_ALIAS_PREFIXES: &[&str] = &["veh:", "prop:", "comp:"];

/// Whether `id` looks like an alias form (`veh:` / `prop:` / `comp:` …) rather than a bare
/// `resource_name`. Purely a shape test on the string; resolution is still membership in the
/// supplied catalogue set.
fn is_alias_form(id: &str) -> bool {
    ASSET_ALIAS_PREFIXES.iter().any(|p| id.starts_with(p))
}

/// Every placed-asset reference in the payload the resolution rule must check, as
/// `(subject_pointer, subject_id, asset_id)` triples. The asset-id vocabulary is fixed by the
/// document core's writers and carried verbatim by `compile::compile_payload`:
///
/// * **Slots** — `editor.slots[].assetId` is the FULL Enfusion `resource_name` the palette dropped
///   (`doc/store.rs::add_slot` writes `assetId`; `asset_catalog.rs::PlacePayload.asset_id =
///   resource_name`). `subject_id` = the slot id (the T-657 convention).
/// * **Vehicles** — `vehicles[].resourceName` (`doc/store.rs::add_vehicle`; `compile.rs` copies
///   `vehiclesById` → top-level `vehicles`). Vehicles may also carry a `veh:` alias.
/// * **Entities** (placed world objects) — `entities[].alias` (a `prop:`/`comp:` alias) AND
///   `entities[].resourceName` (`doc/store.rs::add_entity`; `compile.rs` copies `entitiesById` →
///   top-level `entities`). The rule resolves the ALIAS when one is present (that is the id the
///   Objects palette is pinned to in the mod spawn registry, T-439), else the resource_name.
///
/// A row with no usable id (all fields blank/absent/wrong-typed) contributes no reference — a
/// malformed row is the schema's concern, and this stays a total function over arbitrary JSON
/// (reads through `str_field`, never indexes/unwraps on payload data). Kept small and allocation-
/// light: one `String` id per placed reference.
fn placed_asset_refs(payload: &Value) -> Vec<(String, String, String)> {
    let mut refs = Vec::new();

    // Slots: assetId = full resource_name. Only slots that actually carry an assetId are checked —
    // an ORBAT slot placed before an asset was assigned has none, and "resolve a role/squad" is
    // ORBAT-SLOT-RESOLVES's job, not this rule's.
    for (i, slot) in editor_slots(payload).iter().enumerate() {
        let asset = str_field(slot, "assetId");
        if asset.is_empty() {
            continue;
        }
        let id = slot_id(slot);
        refs.push((
            format!("/editor/slots/{i}/assetId"),
            id.to_string(),
            asset.to_string(),
        ));
    }

    // Vehicles: resourceName (may also be a veh: alias). Top-level `vehicles[]`.
    for (i, veh) in top_level_array(payload, "vehicles").iter().enumerate() {
        let asset = str_field(veh, "resourceName");
        if asset.is_empty() {
            continue;
        }
        let id = str_field(veh, "id");
        refs.push((
            format!("/vehicles/{i}/resourceName"),
            id.to_string(),
            asset.to_string(),
        ));
    }

    // Entities (placed world objects): resolve the alias when present (the id the mod spawn registry
    // is keyed on, T-439), else the resourceName. Top-level `entities[]`.
    for (i, ent) in top_level_array(payload, "entities").iter().enumerate() {
        let alias = str_field(ent, "alias");
        let (field, asset) = if alias.is_empty() {
            ("resourceName", str_field(ent, "resourceName"))
        } else {
            ("alias", alias)
        };
        if asset.is_empty() {
            continue;
        }
        let id = str_field(ent, "id");
        refs.push((
            format!("/entities/{i}/{field}"),
            id.to_string(),
            asset.to_string(),
        ));
    }

    refs
}

/// A placed asset id RESOLVES against `known` when the id (a `resource_name` OR an alias form) is a
/// member of the catalogue set. The catalogue is expected to carry ids in whatever forms the payload
/// uses — full `resource_name`s for slots/vehicles and `veh:`/`prop:`/`comp:` aliases for objects
/// (T-655 populates it from `registry_session`, which holds both) — so resolution is a single exact
/// membership test on the id as written. `is_alias_form` is not used to *transform* the id (the
/// engine has no display-name to re-derive a slug from, and must not import the frontend's
/// derivation); it only documents, and lets tests assert, that alias-form ids are first-class here:
/// a catalogue that lists `prop:ammo_crate` resolves a placed object whose `alias` is
/// `prop:ammo_crate`.
fn asset_resolves(asset_id: &str, known: &HashSet<String>) -> bool {
    known.contains(asset_id)
}

fn rule_asset_resolves() -> Rule {
    Rule {
        id: "ASSET-RESOLVES",
        // Error — a placed asset that does not resolve in the live catalogue is modset drift: the
        // prefab is gone (or the mod that provided it is unloaded), so the mission will not spawn it.
        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,
        // GATE (T-658 context conditionality): only when a live catalogue was supplied. `None` ⇒
        // cold registry / server-side call ⇒ the rule is deliberately inert (the conservative
        // default), NOT silently skipped — the registry records that it did not apply, and its
        // trip_context still proves it fires when a catalogue IS present. A supplied set that is
        // empty still applies: with no known ids, every placed asset is unresolved, which is the
        // correct (if drastic) reading of "the catalogue is loaded and contains nothing".
        applies: |_payload, ctx| ctx.known_asset_ids.is_some(),
        eval: |rule, payload, ctx| {
            // Safe by the gate: `applies` guaranteed `Some`. `as_ref` (not unwrap) keeps eval total
            // even if called directly — a `None` here yields no findings rather than a panic.
            let Some(known) = ctx.known_asset_ids.as_ref() else {
                return Vec::new();
            };
            let mut out = Vec::new();
            // Walk EVERY placed reference; report each unresolved one (never early-exit — a second
            // missing asset must not hide behind the first, the engine's contract).
            for (subject, subject_id, asset_id) in placed_asset_refs(payload) {
                if asset_resolves(&asset_id, known) {
                    continue;
                }
                // Name the id's kind in the message so the author knows what to look for — an alias
                // form (`prop:`/`comp:`/`veh:`) points at a mod spawn-registry entry, a bare id at a
                // raw prefab `resource_name`. Uses the mirrored prefix const (T-658).
                let kind = if is_alias_form(&asset_id) {
                    "alias"
                } else {
                    "prefab"
                };
                out.push(rule.finding_id(
                    format!(
                        "placed asset {kind} {asset_id:?} does not resolve in the live catalogue — \
                         the entry is missing (modset drift), so this placement will not spawn. \
                         Re-pick it from the palette or restore the mod that provides it."
                    ),
                    subject,
                    subject_id,
                ));
            }
            out
        },
        // Trips because: the fixture places a slot whose assetId is NOT in the trip_context's known
        // set — the moment-of-placement modset-drift case. (An in-set asset would resolve; see the
        // trip_context below, which lists a DIFFERENT id.)
        trip_fixture: || {
            serde_json::json!({
                "editor": {
                    "slots": [
                        {"id": "s1", "role": "RFL", "assetId": "{ABC}Prefabs/Characters/Ghost.et"}
                    ]
                }
            })
        },
        // The catalogue the trip_fixture is checked against — deliberately does NOT contain the
        // placed asset id, so the rule fires. Supplying a context here is what makes ASSET-RESOLVES
        // self-checkable: without it, self_check would run against the default (`None`) context, the
        // applies-gate would exclude the trip, and the rule would be reported as a loud failure —
        // the "a context rule that cannot fire is a loud failure" discipline (T-658).
        trip_context: || {
            Some(EvalContext::with_known_asset_ids(
                ["{XYZ}Prefabs/Characters/SomethingElse.et".to_string()]
                    .into_iter()
                    .collect(),
            ))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Find the finding a given rule id produced (or fail the test). The whole suite asserts on the
    /// STABLE id + message, per the acceptance bar — never merely "some finding appeared".
    fn finding_for<'a>(findings: &'a [Finding], rule_id: &str) -> &'a Finding {
        findings
            .iter()
            .find(|f| f.rule_id == rule_id)
            .unwrap_or_else(|| panic!("expected a finding from {rule_id}, got {findings:?}"))
    }

    /* ── V1-PLAYER-SPAWN: fail-on-demand + conditionality ── */

    #[test]
    fn v1_player_spawn_fires_when_a_faction_has_no_slots() {
        let findings = validate_editor_payload(&rule_v1_player_spawn().trip_fixture());
        let f = finding_for(&findings, "V1-PLAYER-SPAWN");
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.primitive, Primitive::RequiredEntity);
        assert_eq!(f.subject, "/editor/slots");
        assert!(f.message.contains("nowhere for a player to spawn"), "{f:?}");
    }

    #[test]
    fn v1_is_conditional_a_factionless_draft_does_not_fire() {
        // The V1 conditionality claim, tested directly: no factions ⇒ rule does not apply ⇒ no
        // finding, even though there are zero slots. This is what spares the tool FNF's ignore-list.
        for empty in [json!({}), json!({"editor": {"slots": []}})] {
            let rule = rule_v1_player_spawn();
            assert!(
                !rule.applies(&empty, &EvalContext::default()),
                "must not apply: {empty}"
            );
            assert!(
                validate_editor_payload(&empty)
                    .iter()
                    .all(|f| f.rule_id != "V1-PLAYER-SPAWN"),
                "V1 must stay silent on a factionless draft: {empty}"
            );
        }
    }

    #[test]
    fn v1_does_not_fire_when_the_declared_faction_has_a_slot() {
        let p = json!({"editor": {
            "factions": [{"key": "BLUFOR", "name": "US", "squadIds": ["sq1"]}],
            "squads": [{"id": "sq1", "callsign": "Alpha", "slotIds": ["s1"]}],
            "slots": [{"id": "s1", "role": "RFL", "position": {"x": 100.0, "y": 100.0}}]
        }});
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "V1-PLAYER-SPAWN"),
            "{p}"
        );
    }

    /* ── V2-FACTION-MAX: fail-on-demand + boundary ── */

    #[test]
    fn v2_faction_max_fires_on_five_factions() {
        let findings = validate_editor_payload(&rule_v2_faction_max().trip_fixture());
        let f = finding_for(&findings, "V2-FACTION-MAX");
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.primitive, Primitive::Cardinality);
        assert_eq!(f.subject, "/editor/factions");
        assert!(f.message.contains("5 factions declared"), "{f:?}");
        assert!(f.message.contains("at most 4"), "{f:?}");
    }

    #[test]
    fn v2_does_not_fire_at_exactly_four_factions() {
        let factions: Vec<Value> = ["BLUFOR", "OPFOR", "INDFOR", "CIV"]
            .iter()
            .map(|k| json!({"key": k, "name": k}))
            .collect();
        let p = json!({ "editor": { "factions": factions } });
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "V2-FACTION-MAX"),
            "four is the ceiling, not over it: {p}"
        );
    }

    /* ── V3-SLOT-IN-BOUNDS: fail-on-demand + one-per-offender + terrain-aware ── */

    #[test]
    fn v3_slot_in_bounds_fires_on_an_out_of_bounds_slot() {
        let findings = validate_editor_payload(&rule_v3_slot_in_bounds().trip_fixture());
        let f = finding_for(&findings, "V3-SLOT-IN-BOUNDS");
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.primitive, Primitive::PerObjectInvariant);
        assert_eq!(f.subject, "/editor/slots/0/position");
        assert!(
            f.message.contains("outside the everon terrain bounds"),
            "{f:?}"
        );
        assert!(f.message.contains("20000.0"), "{f:?}");
    }

    #[test]
    fn v3_returns_one_finding_per_offending_slot_never_early_exits() {
        // Two bad slots, one good one between them: the pass must report BOTH bad ones (the "never
        // early-exit, a second defect is not hidden behind the first" contract), keyed on each index.
        let p = json!({"map": {"terrain": "everon"}, "editor": {"slots": [
            {"id": "a", "position": {"x": -5.0, "y": 100.0}},
            {"id": "b", "position": {"x": 100.0, "y": 100.0}},
            {"id": "c", "position": {"x": 100.0, "y": 99999.0}}
        ]}});
        let v3: Vec<Finding> = validate_editor_payload(&p)
            .into_iter()
            .filter(|f| f.rule_id == "V3-SLOT-IN-BOUNDS")
            .collect();
        assert_eq!(v3.len(), 2, "{v3:?}");
        assert_eq!(v3[0].subject, "/editor/slots/0/position");
        assert_eq!(v3[1].subject, "/editor/slots/2/position");
    }

    #[test]
    fn v3_uses_the_authored_terrain_bounds_not_a_fixed_size() {
        // (5000, 5000) is INSIDE everon (12800²) but OUTSIDE arland (4096²). The same slot must
        // pass on one terrain and fail on the other — proof the rule reads `map.terrain`.
        let slot = json!({"id": "s", "position": {"x": 5000.0, "y": 5000.0}});
        let everon = json!({"map": {"terrain": "everon"}, "editor": {"slots": [slot.clone()]}});
        let arland = json!({"map": {"terrain": "arland"}, "editor": {"slots": [slot]}});
        assert!(
            validate_editor_payload(&everon)
                .iter()
                .all(|f| f.rule_id != "V3-SLOT-IN-BOUNDS"),
            "in-bounds on everon"
        );
        let arland_findings = validate_editor_payload(&arland);
        let f = finding_for(&arland_findings, "V3-SLOT-IN-BOUNDS");
        assert!(f.message.contains("arland"), "{f:?}");
    }

    #[test]
    fn v3_ignores_slots_without_a_numeric_position() {
        // No position, or a non-numeric coord, is the schema's fault to catch — not this rule's, and
        // it must not crash or false-fire on it.
        let p = json!({"map": {"terrain": "everon"}, "editor": {"slots": [
            {"id": "a"},
            {"id": "b", "position": {"x": "nope", "y": 100.0}},
            {"id": "c", "position": {}}
        ]}});
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "V3-SLOT-IN-BOUNDS"),
            "{p}"
        );
    }

    /* ── V4-SCHEMA-VERSION: fail-on-demand + accepts absent/valid ── */

    #[test]
    fn v4_schema_version_fires_on_a_string_version() {
        let findings = validate_editor_payload(&rule_v4_schema_version().trip_fixture());
        let f = finding_for(&findings, "V4-SCHEMA-VERSION");
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.primitive, Primitive::FieldShape);
        assert_eq!(f.subject, "/schemaVersion");
        assert!(f.message.contains("positive integer"), "{f:?}");
    }

    #[test]
    fn v4_fires_on_zero_and_negative_and_fractional() {
        for bad in [json!(0), json!(-1), json!(1.5)] {
            let p = json!({ "schemaVersion": bad });
            assert!(
                validate_editor_payload(&p)
                    .iter()
                    .any(|f| f.rule_id == "V4-SCHEMA-VERSION"),
                "must fire on schemaVersion={bad}"
            );
        }
    }

    #[test]
    fn v4_accepts_absent_and_a_valid_positive_integer() {
        for ok in [
            json!({}),
            json!({"schemaVersion": 1}),
            json!({"schemaVersion": 2}),
        ] {
            assert!(
                validate_editor_payload(&ok)
                    .iter()
                    .all(|f| f.rule_id != "V4-SCHEMA-VERSION"),
                "must accept {ok}"
            );
        }
    }

    /* ── Engine-level guarantees ── */

    #[test]
    fn a_clean_realistic_payload_produces_no_findings() {
        // A complete, valid editor payload (the FIXTURE shape from flatten.rs, trimmed): two sides,
        // slots inside everon bounds, integer schemaVersion. The engine must be GREEN on it — the
        // green-path counterpart to every fail-on-demand test above.
        //
        // T-657 tightened what "clean" means: each squad now carries an identity (callsign) AND names
        // a `leaderSlotId` that is one of its slots, and every slot resolves a role + a squad. This
        // fixture is filled out to meet that bar so it stays the honest all-rules-green counterpart.
        let p = clean_orbat_payload();
        assert!(
            validate_editor_payload(&p).is_empty(),
            "{:?}",
            validate_editor_payload(&p)
        );
    }

    /// A complete, all-rules-green ORBAT payload, reused by the T-657 perturb-and-restore tests: two
    /// sides, each with one led squad whose single slot resolves a role and a squad, slots in bounds,
    /// integer schemaVersion. Perturbing exactly one field trips exactly one rule; restoring it
    /// returns to green — the fired-proof pattern the ticket asks for beyond the structural
    /// self-check.
    fn clean_orbat_payload() -> Value {
        json!({
            "schemaVersion": 1,
            "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
            "editor": {
                "factions": [
                    {"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]},
                    {"key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq2"]}
                ],
                "squads": [
                    {"id": "sq1", "callsign": "Alpha", "name": "Alpha 1-1",
                     "slotIds": ["s1"], "leaderSlotId": "s1"},
                    {"id": "sq2", "callsign": "Grom", "name": "Grom 1-1",
                     "slotIds": ["s2"], "leaderSlotId": "s2"}
                ],
                "slots": [
                    {"id": "s1", "role": "SL", "position": {"x": 4839.2, "y": 6620.8, "z": 0.0}},
                    {"id": "s2", "role": "RFL", "position": {"x": 6010.0, "y": 7211.5, "z": 0.0}}
                ]
            }
        })
    }

    #[test]
    fn every_seed_rule_has_a_distinct_id_and_a_known_primitive() {
        let reg = default_registry();
        let mut ids: Vec<&str> = reg.rules().iter().map(Rule::id).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "rule ids must be unique");
        // One rule per primitive in the seed (the "exercise each primitive once" contract).
        let prims: Vec<Primitive> = reg.rules().iter().map(Rule::primitive).collect();
        for want in [
            Primitive::RequiredEntity,
            Primitive::Cardinality,
            Primitive::PerObjectInvariant,
            Primitive::FieldShape,
        ] {
            assert!(prims.contains(&want), "seed missing a {} rule", want.tag());
        }
    }

    #[test]
    fn engine_self_check_passes_for_the_seed_registry() {
        // The engine-level guarantee: every seed rule fires on its own declared trip fixture. If a
        // rule ever goes silent, THIS is the loud failure — not a green run that examined nothing.
        default_registry()
            .self_check()
            .expect("every seed rule must fire on its trip fixture");
        default_registry().assert_self_check(); // the panic form a service would call at boot
    }

    #[test]
    fn engine_self_check_catches_a_rule_that_cannot_fire() {
        // A deliberately-misdeclared rule: it claims a trip fixture but its eval NEVER produces a
        // finding (the FNF failure mode — a check that does nothing). self_check must catch it and
        // name it, proving the guard is not itself a silent pass.
        fn dead_eval(_r: &Rule, _p: &Value, _c: &EvalContext) -> Vec<Finding> {
            Vec::new()
        }
        let dead = Rule {
            id: "DEAD-RULE",
            severity: Severity::Error,
            primitive: Primitive::FieldShape,
            applies: |_, _| true,
            eval: dead_eval,
            trip_fixture: || json!({"anything": true}),
            trip_context: no_trip_context,
        };
        let reg = Registry::new(vec![dead]);
        let err = reg
            .self_check()
            .expect_err("a rule that never fires must fail self-check");
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].rule_id, "DEAD-RULE");
        assert!(err[0].reason.contains("gone silent"), "{:?}", err[0]);
    }

    #[test]
    fn engine_self_check_catches_a_rule_whose_gate_excludes_its_own_trip() {
        // The other silent-skip shape: a rule whose `applies` gate can never be true for its trip
        // fixture — it would evaluate to nothing not because it passed but because it was skipped.
        let misgated = Rule {
            id: "MISGATED-RULE",
            severity: Severity::Warning,
            primitive: Primitive::RequiredEntity,
            applies: |_, _| false, // never applies — so it can never fire, on anything
            eval: |rule, _, _| vec![rule.finding("unreachable".into(), "/x".into())],
            trip_fixture: || json!({}),
            trip_context: no_trip_context,
        };
        let err = Registry::new(vec![misgated])
            .self_check()
            .expect_err("a rule gated off from its own trip must fail self-check");
        assert_eq!(err[0].rule_id, "MISGATED-RULE");
        assert!(err[0].reason.contains("`applies` gate"), "{:?}", err[0]);
    }

    #[test]
    #[should_panic(expected = "duplicate rule id")]
    fn registry_rejects_duplicate_ids() {
        // Two rules with the same id would make a finding ambiguous to a consumer routing on id.
        let _ = Registry::new(vec![rule_v4_schema_version(), rule_v4_schema_version()]);
    }

    #[test]
    #[should_panic(expected = "self-check failed")]
    fn assert_self_check_panics_loudly_on_a_dead_rule() {
        let dead = Rule {
            id: "DEAD",
            severity: Severity::Error,
            primitive: Primitive::FieldShape,
            applies: |_, _| true,
            eval: |_, _, _| Vec::new(),
            trip_fixture: || json!({}),
            trip_context: no_trip_context,
        };
        Registry::new(vec![dead]).assert_self_check();
    }

    #[test]
    fn evaluate_returns_all_findings_across_rules_never_early_exits() {
        // One payload that trips THREE rules at once (out-of-bounds slot + bad schemaVersion +
        // five factions). The engine must return all three — proof it does not stop at the first.
        let factions: Vec<Value> = (0..5)
            .map(|i| json!({"key": format!("S{i}"), "name": format!("S{i}")}))
            .collect();
        let p = json!({
            "schemaVersion": "bad",
            "map": {"terrain": "everon"},
            "editor": {
                "factions": factions,
                "slots": [{"id": "s", "position": {"x": 99999.0, "y": 1.0}}]
            }
        });
        let findings = validate_editor_payload(&p);
        for want in ["V2-FACTION-MAX", "V3-SLOT-IN-BOUNDS", "V4-SCHEMA-VERSION"] {
            assert!(
                findings.iter().any(|f| f.rule_id == want),
                "missing {want} in {findings:?}"
            );
        }
    }

    /* ═══════════════════════════ T-657 — ORBAT / slot rules ═══════════════════════════ */

    /* ── ORBAT-SLOT-RESOLVES: fail-on-demand (asserts id/severity/primitive/subject/subject_id) ── */

    #[test]
    fn orbat_slot_resolves_fires_when_a_slot_resolves_neither() {
        let findings = validate_editor_payload(&rule_orbat_slot_resolves().trip_fixture());
        let f = finding_for(&findings, "ORBAT-SLOT-RESOLVES");
        assert_eq!(f.severity, Severity::Error); // R3 — the one FNF rated error
        assert_eq!(f.primitive, Primitive::PerObjectInvariant);
        assert_eq!(f.subject, "/editor/slots/0");
        assert_eq!(f.subject_id.as_deref(), Some("s1")); // T-655 forward constraint: the entity id
        assert!(f.message.contains("resolves neither"), "{f:?}");
    }

    #[test]
    fn orbat_slot_resolves_distinguishes_missing_role_from_missing_squad() {
        // A slot with a role but filed under no squad → "not filed under any squad"; a slot filed
        // under a squad but with a blank role → "has no role". Both fire, one per offender.
        let p = json!({"editor": {
            "squads": [{"id": "sq1", "callsign": "A", "name": "A", "slotIds": ["s2"], "leaderSlotId": "s2"}],
            "slots": [
                {"id": "s1", "role": "RFL"},          // has role, unattached
                {"id": "s2", "role": ""}              // attached, no role
            ]
        }});
        let findings = validate_editor_payload(&p);
        let fs: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.rule_id == "ORBAT-SLOT-RESOLVES")
            .collect();
        // s1 — has a role but is filed under no squad; s2 — attached but blank role.
        let s1 = fs
            .iter()
            .find(|f| f.subject_id.as_deref() == Some("s1"))
            .expect("a finding for s1");
        let s2 = fs
            .iter()
            .find(|f| f.subject_id.as_deref() == Some("s2"))
            .expect("a finding for s2");
        assert!(s1.message.contains("not filed under any squad"), "{s1:?}");
        assert!(s2.message.contains("has no role"), "{s2:?}");
    }

    #[test]
    fn orbat_slot_resolves_is_conditional_on_a_declared_orbat() {
        // No squads ⇒ no ORBAT ⇒ the rule does not apply, even to a slot with no role. This is the
        // V1-style conditionality: a squadless draft is not "broken".
        for p in [
            json!({}),
            json!({"editor": {"slots": [{"id": "s1", "role": ""}]}}),
        ] {
            assert!(
                !rule_orbat_slot_resolves().applies(&p, &EvalContext::default()),
                "must not apply: {p}"
            );
            assert!(
                validate_editor_payload(&p)
                    .iter()
                    .all(|f| f.rule_id != "ORBAT-SLOT-RESOLVES"),
                "must stay silent without an ORBAT: {p}"
            );
        }
    }

    /* ── ORBAT-IDENTITY-FILLED: fail-on-demand + accepts a partial identity ── */

    #[test]
    fn orbat_identity_filled_fires_on_a_squad_with_no_callsign_and_no_name() {
        let findings = validate_editor_payload(&rule_orbat_identity_filled().trip_fixture());
        let f = finding_for(&findings, "ORBAT-IDENTITY-FILLED");
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.primitive, Primitive::PerObjectInvariant);
        assert_eq!(f.subject, "/editor/squads/0");
        assert_eq!(f.subject_id.as_deref(), Some("sq1"));
        assert!(f.message.contains("no callsign and no name"), "{f:?}");
    }

    #[test]
    fn orbat_identity_filled_accepts_either_a_callsign_or_a_name() {
        // A squad with a callsign but no name, or a name but no callsign, is addressable — not a
        // default identity. Only both-blank fires.
        for sq in [
            json!({"id": "sq1", "callsign": "Alpha", "name": "", "slotIds": []}),
            json!({"id": "sq1", "callsign": "  ", "name": "Alpha 1-1", "slotIds": []}),
        ] {
            let p = json!({"editor": {"squads": [sq.clone()]}});
            assert!(
                validate_editor_payload(&p)
                    .iter()
                    .all(|f| f.rule_id != "ORBAT-IDENTITY-FILLED"),
                "a partial identity must pass: {sq}"
            );
        }
    }

    /* ── ORBAT-SQUAD-HAS-LEADER: fail-on-demand + empty squad is not leaderless ── */

    #[test]
    fn orbat_squad_has_leader_fires_on_a_manned_squad_with_no_leader() {
        let findings = validate_editor_payload(&rule_orbat_squad_has_leader().trip_fixture());
        let f = finding_for(&findings, "ORBAT-SQUAD-HAS-LEADER");
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.primitive, Primitive::PerObjectInvariant);
        assert_eq!(f.subject, "/editor/squads/0");
        assert_eq!(f.subject_id.as_deref(), Some("sq1"));
        assert!(f.message.contains("no leader"), "{f:?}");
    }

    #[test]
    fn orbat_squad_has_leader_ignores_an_empty_squad_and_accepts_a_valid_leader() {
        // Empty squad → no body to lead → no finding. A squad whose leaderSlotId is one of its slots
        // → no finding. A squad whose leaderSlotId points OUTSIDE its slots → fires (leaderless).
        let ok = json!({"editor": {
            "squads": [
                {"id": "sq-empty", "callsign": "E", "name": "E", "slotIds": []},
                {"id": "sq-led", "callsign": "L", "name": "L", "slotIds": ["s1"], "leaderSlotId": "s1"}
            ],
            "slots": [{"id": "s1", "role": "SL"}]
        }});
        assert!(
            validate_editor_payload(&ok)
                .iter()
                .all(|f| f.rule_id != "ORBAT-SQUAD-HAS-LEADER"),
            "empty + validly-led squads must pass: {:?}",
            validate_editor_payload(&ok)
        );
        let foreign = json!({"editor": {
            "squads": [{"id": "sq1", "callsign": "A", "name": "A", "slotIds": ["s1"], "leaderSlotId": "s2"}],
            "slots": [{"id": "s1", "role": "SL"}]
        }});
        assert!(
            validate_editor_payload(&foreign)
                .iter()
                .any(|f| f.rule_id == "ORBAT-SQUAD-HAS-LEADER"),
            "a leaderSlotId outside the squad is leaderless"
        );
    }

    /* ── ORBAT-CALLSIGN-UNIQUE: fail-on-demand + cross-side reuse does NOT fire ── */

    #[test]
    fn orbat_callsign_unique_fires_on_two_squads_sharing_a_callsign_on_one_side() {
        let findings = validate_editor_payload(&rule_orbat_callsign_unique().trip_fixture());
        let f = finding_for(&findings, "ORBAT-CALLSIGN-UNIQUE");
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.primitive, Primitive::PerObjectInvariant);
        assert_eq!(f.subject, "/editor/squads/sq2/callsign"); // the SECOND squad seen is the offender
        assert_eq!(f.subject_id.as_deref(), Some("sq2"));
        assert!(f.message.contains("unique within a side"), "{f:?}");
    }

    #[test]
    fn orbat_callsign_unique_does_not_fire_across_different_sides() {
        // "Alpha" on BLUFOR and "Alpha" on OPFOR is legal — uniqueness is per side, not global.
        let p = json!({"editor": {
            "factions": [
                {"key": "BLUFOR", "name": "US", "squadIds": ["sq1"]},
                {"key": "OPFOR", "name": "SOV", "squadIds": ["sq2"]}
            ],
            "squads": [
                {"id": "sq1", "callsign": "Alpha", "name": "A", "slotIds": []},
                {"id": "sq2", "callsign": "Alpha", "name": "B", "slotIds": []}
            ]
        }});
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "ORBAT-CALLSIGN-UNIQUE"),
            "same callsign on two DIFFERENT sides must not fire: {:?}",
            validate_editor_payload(&p)
        );
    }

    /* ── ORBAT-TEMPLATE-COVERAGE (D3-D8 revival): fail-on-demand + no-template skip ── */

    #[test]
    fn orbat_template_coverage_fires_when_a_required_role_is_unfilled() {
        let findings = validate_editor_payload(&rule_orbat_template_coverage().trip_fixture());
        let f = finding_for(&findings, "ORBAT-TEMPLATE-COVERAGE");
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.primitive, Primitive::PerObjectInvariant);
        assert_eq!(f.subject, "/editor/squads/0");
        assert_eq!(f.subject_id.as_deref(), Some("sq1"));
        assert!(f.message.contains("MED"), "{f:?}"); // the uncovered required role
        assert!(f.message.contains("template"), "{f:?}");
    }

    #[test]
    fn orbat_template_coverage_skips_a_squad_with_no_template() {
        // The boundary the ticket names explicitly: a squad with no `template` block declares no
        // required roles, so the coverage rule does not apply to it — even if it is a bare one-slot
        // squad that fills nothing in particular.
        let p = json!({"editor": {
            "squads": [{"id": "sq1", "callsign": "A", "name": "A", "slotIds": ["s1"], "leaderSlotId": "s1"}],
            "slots": [{"id": "s1", "role": "RFL"}]
        }});
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "ORBAT-TEMPLATE-COVERAGE"),
            "a squad with no template must skip coverage: {:?}",
            validate_editor_payload(&p)
        );
    }

    #[test]
    fn orbat_template_coverage_passes_when_every_required_role_is_filled() {
        // Same template as the trip fixture (requires SL + MED) but now both roles are filled → green.
        let p = json!({"editor": {
            "squads": [{
                "id": "sq1", "callsign": "A", "name": "A",
                "slotIds": ["s1", "s2"], "leaderSlotId": "s1",
                "template": {"requiredRoles": ["SL", "MED"]}
            }],
            "slots": [{"id": "s1", "role": "SL"}, {"id": "s2", "role": "MED"}]
        }});
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "ORBAT-TEMPLATE-COVERAGE"),
            "full coverage must pass: {:?}",
            validate_editor_payload(&p)
        );
    }

    /* ── Boundary: empty ORBAT is silent, not a panic ── */

    #[test]
    fn an_empty_orbat_produces_no_orbat_findings_and_does_not_panic() {
        // A completely empty payload and a payload with an empty editor block: none of the five
        // ORBAT rules apply (no squads), and evaluation returns cleanly.
        for p in [
            json!({}),
            json!({"editor": {}}),
            json!({"editor": {"factions": [], "squads": [], "slots": []}}),
        ] {
            let findings = validate_editor_payload(&p);
            for id in [
                "ORBAT-SLOT-RESOLVES",
                "ORBAT-IDENTITY-FILLED",
                "ORBAT-SQUAD-HAS-LEADER",
                "ORBAT-CALLSIGN-UNIQUE",
                "ORBAT-TEMPLATE-COVERAGE",
            ] {
                assert!(
                    findings.iter().all(|f| f.rule_id != id),
                    "{id} must be silent on {p}"
                );
            }
        }
    }

    /* ── Forward constraint 2: no eval panics on garbage payloads ── */

    #[test]
    fn orbat_rules_never_panic_on_garbage() {
        // The wave-104 forward constraint: eval panics propagate and would wasm-trap under always-on
        // eval, so every eval must be TOTAL. Feed deliberately malformed payloads — wrong types where
        // objects/arrays/strings are expected, nulls, deep nesting, non-string ids in id arrays —
        // through the WHOLE registry and assert only that evaluation returns without panicking.
        let garbage = [
            json!(null),
            json!(42),
            json!("a string, not an object"),
            json!([]),
            json!({"editor": 7}), // editor not an object
            json!({"editor": {"squads": "nope", "slots": 3, "factions": {}}}),
            json!({"editor": {"squads": [null, 5, "x", {}]}}), // non-object squad rows
            json!({"editor": {"slots": [null, 9, {"id": 5, "role": []}]}}), // id/role wrong types
            json!({"editor": {
                "factions": [{"key": null, "squadIds": [1, 2, {}, "sq1"]}],
                "squads": [{"id": null, "callsign": 5, "name": [], "slotIds": "x", "leaderSlotId": {}}],
                "slots": [{"id": 1, "role": 2}]
            }}),
            json!({"editor": {"squads": [{
                "id": "sq1", "slotIds": [null, 3, "s1"],
                "template": {"requiredRoles": [null, 7, "SL", ""]}
            }], "slots": [{"id": "s1", "role": null}]}}),
            json!({"schemaVersion": {"nested": [1, 2, 3]}, "map": {"terrain": []}}),
        ];
        let reg = default_registry();
        for p in garbage {
            // Must not panic. We do not assert on the findings — only that eval is total.
            let _ = reg.evaluate(&p);
            let _ = validate_editor_payload(&p);
        }
    }

    /* ── Structural: all T-657 rules are registered, and self_check still passes ── */

    #[test]
    fn t657_rules_are_registered_and_self_check_passes() {
        let reg = default_registry();
        let ids: Vec<&str> = reg.rules().iter().map(Rule::id).collect();
        for want in [
            "ORBAT-SLOT-RESOLVES",
            "ORBAT-IDENTITY-FILLED",
            "ORBAT-SQUAD-HAS-LEADER",
            "ORBAT-CALLSIGN-UNIQUE",
            "ORBAT-TEMPLATE-COVERAGE",
        ] {
            assert!(ids.contains(&want), "registry missing {want}");
        }
        // The engine-level guard must still pass with the new rules present — each fires on its own
        // trip fixture (a rule that cannot fire is a loud failure, per T-656).
        reg.self_check()
            .expect("every rule (seed + T-657) must fire on its trip fixture");
    }

    /* ── Fired proof beyond self_check: perturb one field, one rule fires, restore → green ── */

    #[test]
    fn perturb_and_restore_fires_exactly_the_leader_rule() {
        // Start from the all-green ORBAT payload. Remove sq1's leaderSlotId → ORBAT-SQUAD-HAS-LEADER
        // fires (and ONLY it among the ORBAT rules); restore it → green again. This is the
        // "fire one rule once (perturb/fail/restore)" proof the ticket asks for on top of the
        // structural self_check.
        let clean = clean_orbat_payload();
        assert!(
            validate_editor_payload(&clean).is_empty(),
            "baseline must be green: {:?}",
            validate_editor_payload(&clean)
        );

        let mut broken = clean.clone();
        broken["editor"]["squads"][0]
            .as_object_mut()
            .unwrap()
            .remove("leaderSlotId");
        let findings = validate_editor_payload(&broken);
        let leader: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.rule_id == "ORBAT-SQUAD-HAS-LEADER")
            .collect();
        assert_eq!(
            leader.len(),
            1,
            "exactly one leaderless finding: {findings:?}"
        );
        assert_eq!(leader[0].subject_id.as_deref(), Some("sq1"));
        // No OTHER ORBAT rule should have fired on this single-field perturbation.
        for other in [
            "ORBAT-SLOT-RESOLVES",
            "ORBAT-IDENTITY-FILLED",
            "ORBAT-CALLSIGN-UNIQUE",
            "ORBAT-TEMPLATE-COVERAGE",
        ] {
            assert!(
                findings.iter().all(|f| f.rule_id != other),
                "only the leader rule should fire; {other} also fired: {findings:?}"
            );
        }

        // Restore → back to fully green.
        assert!(
            validate_editor_payload(&clean).is_empty(),
            "restoring must return to green"
        );
    }

    /* ═══════════════════════════ T-658 — catalogue-resolution rule ═══════════════════════════ */

    use std::collections::HashSet;

    /// Build an `EvalContext` carrying a known-asset-id set from string literals — the shape the SPA
    /// panel builds from `registry_session` (T-655 W111), constructed directly here.
    fn ctx_with(ids: &[&str]) -> EvalContext {
        let set: HashSet<String> = ids.iter().map(|s| (*s).to_string()).collect();
        EvalContext::with_known_asset_ids(set)
    }

    /* ── ASSET-RESOLVES: fail-on-demand (id/severity/primitive/subject/subject_id/message) ── */

    #[test]
    fn asset_resolves_fires_on_an_unknown_placed_asset() {
        // The rule's own trip fixture + trip context: a placed slot whose assetId is absent from the
        // supplied catalogue. Asserts the STABLE half AND the message, per the acceptance bar.
        let rule = rule_asset_resolves();
        let ctx = rule
            .trip_context()
            .expect("ASSET-RESOLVES declares a trip_context");
        let findings = default_registry().evaluate_with_context(&rule.trip_fixture(), &ctx);
        let f = finding_for(&findings, "ASSET-RESOLVES");
        assert_eq!(f.severity, Severity::Error); // D13 — modset drift blocks the spawn
        assert_eq!(f.primitive, Primitive::PerObjectInvariant); // V3
        assert_eq!(f.subject, "/editor/slots/0/assetId");
        assert_eq!(f.subject_id.as_deref(), Some("s1")); // T-657 convention: the placed entity id
        assert!(
            f.message.contains("does not resolve in the live catalogue"),
            "{f:?}"
        );
        assert!(
            f.message.contains("{ABC}Prefabs/Characters/Ghost.et"),
            "{f:?}"
        );
    }

    #[test]
    fn asset_resolves_passes_when_every_placed_asset_is_in_the_catalogue() {
        // The same placed slot, but now its assetId IS in the supplied catalogue → green. The
        // green-path counterpart to the fail-on-demand case above.
        let p = json!({"editor": {"slots": [
            {"id": "s1", "role": "RFL", "assetId": "{ABC}Prefabs/Characters/Ghost.et"}
        ]}});
        let ctx = ctx_with(&["{ABC}Prefabs/Characters/Ghost.et"]);
        assert!(
            default_registry()
                .evaluate_with_context(&p, &ctx)
                .iter()
                .all(|f| f.rule_id != "ASSET-RESOLVES"),
            "a resolvable asset must not fire: {:?}",
            default_registry().evaluate_with_context(&p, &ctx)
        );
    }

    /* ── The skips-when-None gate (the conservative default) ── */

    #[test]
    fn asset_resolves_skips_when_no_catalogue_is_supplied() {
        // No known_asset_ids (cold registry / server-side) ⇒ the rule DOES NOT APPLY, even though the
        // placed asset would be unresolved against an empty world. This is the T-658 conservative
        // default: do not flag every asset just because nobody handed us the catalogue.
        let p = json!({"editor": {"slots": [
            {"id": "s1", "role": "RFL", "assetId": "{ABC}Prefabs/Characters/Ghost.et"}
        ]}});
        let rule = rule_asset_resolves();
        // Default context has known_asset_ids == None.
        assert!(
            !rule.applies(&p, &EvalContext::default()),
            "must not apply without a catalogue"
        );
        // Both the back-compat evaluate() (default ctx) and an explicit None-ctx stay silent.
        assert!(
            validate_editor_payload(&p)
                .iter()
                .all(|f| f.rule_id != "ASSET-RESOLVES"),
            "evaluate() (default ctx) must not fire ASSET-RESOLVES"
        );
        assert!(
            default_registry()
                .evaluate_with_context(&p, &EvalContext::default())
                .iter()
                .all(|f| f.rule_id != "ASSET-RESOLVES"),
            "explicit empty context must not fire ASSET-RESOLVES"
        );
    }

    #[test]
    fn asset_resolves_applies_but_flags_all_when_the_catalogue_is_empty_but_present() {
        // `Some(empty set)` is DISTINCT from `None`: the catalogue is loaded and contains nothing, so
        // every placed asset is unresolved. The rule applies (a supplied set, even empty) and fires —
        // proof the gate keys on Some/None, not on emptiness.
        let p = json!({"editor": {"slots": [
            {"id": "s1", "role": "RFL", "assetId": "{ABC}X.et"}
        ]}});
        let ctx = EvalContext::with_known_asset_ids(HashSet::new());
        assert!(
            rule_asset_resolves().applies(&p, &ctx),
            "Some(empty) applies"
        );
        let findings = default_registry().evaluate_with_context(&p, &ctx);
        assert!(
            findings.iter().any(|f| f.rule_id == "ASSET-RESOLVES"),
            "an empty-but-present catalogue resolves nothing: {findings:?}"
        );
    }

    /* ── Alias-form resolution: vehicles (veh:/resourceName) + entities (prop:/comp: alias) ── */

    #[test]
    fn asset_resolves_resolves_alias_forms_for_vehicles_and_entities() {
        // A vehicle carried by resourceName, and a placed object carried by a `prop:` alias — both
        // present in the catalogue in the form the payload uses → green, proving the rule resolves
        // BOTH the exact resource_name form (slot/vehicle) AND the alias form (object).
        let p = json!({
            "editor": {"slots": [
                {"id": "s1", "role": "RFL", "assetId": "{ABC}Prefabs/Characters/US_Rifleman.et"}
            ]},
            "vehicles": [
                {"id": "v1", "resourceName": "{ABC}Prefabs/Vehicles/Humvee.et"}
            ],
            "entities": [
                {"id": "e1", "alias": "prop:ammo_crate", "resourceName": "{ABC}Prefabs/Props/AmmoBox.et"}
            ]
        });
        let ctx = ctx_with(&[
            "{ABC}Prefabs/Characters/US_Rifleman.et",
            "{ABC}Prefabs/Vehicles/Humvee.et",
            "prop:ammo_crate", // the entity is resolved by its ALIAS, not its resourceName
        ]);
        let findings = default_registry().evaluate_with_context(&p, &ctx);
        assert!(
            findings.iter().all(|f| f.rule_id != "ASSET-RESOLVES"),
            "all three forms must resolve: {findings:?}"
        );
    }

    #[test]
    fn asset_resolves_fires_per_unresolved_reference_across_kinds_never_early_exits() {
        // One good slot, one bad vehicle, one bad object (by alias): the pass must report BOTH bad
        // ones (never early-exit), each keyed on its own subject pointer + entity id, and name the
        // id's kind (prefab vs alias) in the message.
        let p = json!({
            "editor": {"slots": [
                {"id": "s1", "role": "RFL", "assetId": "{ABC}Known.et"}
            ]},
            "vehicles": [
                {"id": "v1", "resourceName": "{ABC}GoneVehicle.et"}
            ],
            "entities": [
                {"id": "e1", "alias": "comp:gone_comp", "resourceName": "{ABC}GoneObj.et"}
            ]
        });
        let ctx = ctx_with(&["{ABC}Known.et"]);
        let all = default_registry().evaluate_with_context(&p, &ctx);
        let asset_findings: Vec<&Finding> = all
            .iter()
            .filter(|f| f.rule_id == "ASSET-RESOLVES")
            .collect();
        assert_eq!(asset_findings.len(), 2, "{asset_findings:?}");
        let v = asset_findings
            .iter()
            .find(|f| f.subject_id.as_deref() == Some("v1"))
            .expect("a finding for the vehicle v1");
        let e = asset_findings
            .iter()
            .find(|f| f.subject_id.as_deref() == Some("e1"))
            .expect("a finding for the entity e1");
        assert_eq!(v.subject, "/vehicles/0/resourceName");
        assert!(
            v.message.contains("prefab"),
            "vehicle id is a raw prefab: {v:?}"
        );
        assert_eq!(e.subject, "/entities/0/alias");
        assert!(e.message.contains("alias"), "object id is an alias: {e:?}");
    }

    #[test]
    fn asset_resolves_ignores_placements_with_no_asset_id() {
        // An ORBAT slot placed before an asset was assigned carries no assetId; a vehicle/entity row
        // with a blank/absent id likewise. None of these is this rule's concern (they carry no asset
        // reference) — the rule must stay silent on them even with a catalogue supplied.
        let p = json!({
            "editor": {"slots": [
                {"id": "s1", "role": "RFL"},                 // no assetId — ORBAT-only slot
                {"id": "s2", "role": "MED", "assetId": ""}   // blank assetId
            ]},
            "vehicles": [{"id": "v1"}],                       // no resourceName
            "entities": [{"id": "e1"}]                        // no alias, no resourceName
        });
        let ctx = ctx_with(&["something-unrelated"]);
        assert!(
            default_registry()
                .evaluate_with_context(&p, &ctx)
                .iter()
                .all(|f| f.rule_id != "ASSET-RESOLVES"),
            "rows with no asset id carry no reference to resolve: {:?}",
            default_registry().evaluate_with_context(&p, &ctx)
        );
    }

    /* ── Total-function discipline: empty payload + garbage never panic, even WITH a context ── */

    #[test]
    fn asset_resolves_never_panics_on_empty_or_garbage_with_a_context() {
        // The T-657 total-function discipline, extended across the T-658 seam: feed empty and
        // deliberately malformed payloads through the WHOLE registry WITH a populated context and
        // assert only that evaluation returns without panicking (the rule reads through str_field /
        // top_level_array, never indexes/unwraps on payload data).
        let ctx = ctx_with(&["{ABC}Known.et", "prop:ok"]);
        let garbage = [
            json!({}),
            json!(null),
            json!(42),
            json!("a string, not an object"),
            json!([]),
            json!({"editor": 7, "vehicles": 9, "entities": "no"}),
            json!({"vehicles": [null, 5, {"resourceName": 3}, {"id": 1}]}),
            json!({"entities": [null, "x", {"alias": [], "resourceName": {}}]}),
            json!({"editor": {"slots": [null, 9, {"id": 5, "assetId": []}]}}),
            json!({"editor": {"slots": [{"id": "s1", "assetId": "{ABC}Missing.et"}]},
                   "vehicles": [{"id": "v1", "resourceName": "{ABC}Missing.et"}],
                   "entities": [{"id": "e1", "alias": "prop:missing"}]}),
        ];
        let reg = default_registry();
        for p in garbage {
            let _ = reg.evaluate_with_context(&p, &ctx); // must not panic
            let _ = reg.evaluate(&p); // default-ctx path too
        }
        // Empty payload with a catalogue: the rule applies (Some ctx) but finds no placed refs → no
        // ASSET-RESOLVES findings, cleanly.
        assert!(
            reg.evaluate_with_context(&json!({}), &ctx)
                .iter()
                .all(|f| f.rule_id != "ASSET-RESOLVES"),
            "empty payload has no placed assets to flag"
        );
    }

    /* ── The self_check context extension: ASSET-RESOLVES self-checks green via its trip_context ── */

    #[test]
    fn asset_resolves_is_registered_and_self_check_passes_with_the_context_extension() {
        let reg = default_registry();
        assert!(
            reg.rules().iter().any(|r| r.id() == "ASSET-RESOLVES"),
            "registry missing ASSET-RESOLVES"
        );
        // The engine-level guard must pass WITH the context-dependent rule present: self_check now
        // evaluates each rule against its own trip_context (a default when none), so ASSET-RESOLVES —
        // which cannot fire without a catalogue — fires against the one its trip_context supplies.
        reg.self_check()
            .expect("every rule (incl. ASSET-RESOLVES) must fire on its trip fixture + context");
        reg.assert_self_check(); // the panic form a service calls at boot
    }

    #[test]
    fn self_check_catches_a_context_rule_that_ships_without_its_trip_context() {
        // The "a context rule that cannot fire is a loud failure" discipline (T-658), tested
        // directly: a rule that GATES on a context field but declares NO trip_context self-checks
        // against the default (None) context, its gate excludes its own trip, and self_check reports
        // it — proving the seam cannot hide a dead context rule.
        let ctx_rule_no_trip = Rule {
            id: "CTX-NO-TRIP",
            severity: Severity::Error,
            primitive: Primitive::PerObjectInvariant,
            applies: |_p, ctx| ctx.known_asset_ids.is_some(), // needs a catalogue
            eval: |rule, _p, _c| vec![rule.finding("x".into(), "/x".into())],
            trip_fixture: || json!({}),
            trip_context: no_trip_context, // BUG: a context rule with no trip context
        };
        let err = Registry::new(vec![ctx_rule_no_trip])
            .self_check()
            .expect_err("a context rule with no trip_context must fail self-check");
        assert_eq!(err[0].rule_id, "CTX-NO-TRIP");
        assert!(err[0].reason.contains("`applies` gate"), "{:?}", err[0]);
    }

    /* ── Back-compat proof: the pre-T-658 evaluate()/self_check surface is unchanged ── */

    #[test]
    fn evaluate_default_context_matches_the_pre_t658_behaviour() {
        // The seam is additive: evaluate(payload) still returns exactly what it did before T-658 —
        // the seed + T-657 findings, with ASSET-RESOLVES inert (its gate needs a catalogue). This is
        // the back-compat proof: existing callers (validate_editor_payload, the backend /compiled
        // path) see no behaviour change.
        let p = json!({
            "schemaVersion": "bad",
            "map": {"terrain": "everon"},
            "editor": {
                "factions": [{"key": "BLUFOR", "name": "US", "squadIds": ["sq1"]}],
                "squads": [{"id": "sq1", "callsign": "Alpha", "name": "A", "slotIds": []}],
                "slots": [{"id": "s1", "position": {"x": 99999.0, "y": 1.0},
                           "assetId": "{ABC}WouldBeUnknown.et"}]
            }
        });
        let via_free_fn = validate_editor_payload(&p);
        let via_default_ctx = default_registry().evaluate_with_context(&p, &EvalContext::default());
        assert_eq!(
            via_free_fn, via_default_ctx,
            "evaluate() and evaluate_with_context(default) must agree"
        );
        // ASSET-RESOLVES is inert on the default path even though s1's assetId would be unknown.
        assert!(
            via_free_fn.iter().all(|f| f.rule_id != "ASSET-RESOLVES"),
            "ASSET-RESOLVES must stay inert without a catalogue: {via_free_fn:?}"
        );
        // The pre-T-658 rules still fire on the same payload (proof the default path is live).
        for want in ["V3-SLOT-IN-BOUNDS", "V4-SCHEMA-VERSION"] {
            assert!(
                via_free_fn.iter().any(|f| f.rule_id == want),
                "{want} must still fire on the default path: {via_free_fn:?}"
            );
        }
    }
}
