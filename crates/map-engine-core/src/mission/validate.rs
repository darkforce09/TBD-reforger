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

use serde_json::Value;

use crate::mission::compile::terrain_bounds;

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: &'static str,
    pub severity: Severity,
    pub primitive: Primitive,
    pub message: String,
    pub subject: String,
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
/// early-exit across the objects it walks — returning all findings is the engine's contract.
///
/// `trip_fixture` is the payload that MUST make this rule fire. It is not test scaffolding bolted on
/// the side: it lives on the rule so [`Registry::self_check`] can prove, at the engine level, that
/// the rule is still capable of firing. A rule author cannot add a rule to the registry without also
/// stating the input that trips it, which is exactly the property whose absence let FNF ship 14 dead
/// checks.
pub struct Rule {
    id: &'static str,
    severity: Severity,
    primitive: Primitive,
    /// Mission-shape gate. `applies(payload) == false` ⇒ the rule contributes nothing to this
    /// payload's findings (V1 conditionality). Defaults to always-applies for shape-independent rules.
    applies: fn(&Value) -> bool,
    /// The evaluator. Called only when [`applies`](Rule::applies) held; returns ALL findings.
    eval: fn(&Rule, &Value) -> Vec<Finding>,
    /// A payload that this rule is REQUIRED to fire on — the self-check's oracle (see the struct doc).
    trip_fixture: fn() -> Value,
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

    /// Whether this rule applies to `payload`'s mission shape (V1 conditionality). A rule that does
    /// not apply is *deliberately* inert here — not skipped-and-forgotten: the registry records the
    /// distinction, and the rule's own `trip_fixture` still proves it can fire when it does apply.
    #[must_use]
    pub fn applies(&self, payload: &Value) -> bool {
        (self.applies)(payload)
    }

    /// Evaluate against `payload`, honouring the gate: returns `[]` when the rule does not apply,
    /// otherwise every finding the evaluator produced.
    #[must_use]
    pub fn evaluate(&self, payload: &Value) -> Vec<Finding> {
        if !self.applies(payload) {
            return Vec::new();
        }
        (self.eval)(self, payload)
    }

    /// The payload this rule must fire on. Used by [`Registry::self_check`]; exposed so a caller can
    /// audit the trip corpus.
    #[must_use]
    pub fn trip_fixture(&self) -> Value {
        (self.trip_fixture)()
    }

    /// Convenience for an `eval` body: build a finding carrying this rule's stable identity.
    fn finding(&self, message: String, subject: String) -> Finding {
        Finding {
            rule_id: self.id,
            severity: self.severity,
            primitive: self.primitive,
            message,
            subject,
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

    /// Run every rule and return every finding. Order is: rules in registration order, and within a
    /// rule the evaluator's own order. No rule can suppress another's findings, and no finding is
    /// dropped — the "return all findings, never early-exit" contract.
    #[must_use]
    pub fn evaluate(&self, payload: &Value) -> Vec<Finding> {
        let mut out = Vec::new();
        for rule in &self.rules {
            out.extend(rule.evaluate(payload));
        }
        out
    }

    /// Prove every rule is still capable of firing. For each rule, evaluate it against its own
    /// `trip_fixture` and require that (a) the rule APPLIES to that fixture and (b) it produces at
    /// least one finding CARRYING ITS OWN id. A rule that stays silent — because its subject field
    /// was renamed out from under it, its predicate was inverted, or its gate now excludes its own
    /// trip case — is returned as a [`SelfCheckFailure`]. Returns `Ok(())` only when every rule fires.
    ///
    /// This is the engine-level answer to "a check that does nothing looks like a check that passed":
    /// here, a check that does nothing is a returned error.
    ///
    /// # Errors
    /// Returns the list of rules that failed to fire on their own trip fixture.
    pub fn self_check(&self) -> Result<(), Vec<SelfCheckFailure>> {
        let mut failures = Vec::new();
        for rule in &self.rules {
            let fixture = rule.trip_fixture();
            if !rule.applies(&fixture) {
                failures.push(SelfCheckFailure {
                    rule_id: rule.id,
                    reason:
                        "trip_fixture does not satisfy the rule's own `applies` gate — the rule \
                             can never fire on it"
                            .to_string(),
                });
                continue;
            }
            let findings = rule.evaluate(&fixture);
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
#[must_use]
pub fn default_registry() -> Registry {
    Registry::new(vec![
        rule_v1_player_spawn(),
        rule_v2_faction_max(),
        rule_v3_slot_in_bounds(),
        rule_v4_schema_version(),
    ])
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
/// fire on it (conditionality).
fn declares_players(payload: &Value) -> bool {
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
        eval: |rule, payload| {
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
        applies: |_| true,
        eval: |rule, payload| {
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
    }
}

/* ─────────────────────────── V3 — per-object invariant ─────────────────────────── */

fn rule_v3_slot_in_bounds() -> Rule {
    Rule {
        id: "V3-SLOT-IN-BOUNDS",
        severity: Severity::Error,
        primitive: Primitive::PerObjectInvariant,
        applies: |_| true,
        eval: |rule, payload| {
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
    }
}

/* ─────────────────────────── V4 — field-shape / derivation ─────────────────────────── */

fn rule_v4_schema_version() -> Rule {
    Rule {
        id: "V4-SCHEMA-VERSION",
        severity: Severity::Error,
        primitive: Primitive::FieldShape,
        applies: |_| true,
        eval: |rule, payload| {
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
            assert!(!rule.applies(&empty), "must not apply: {empty}");
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
        let p = json!({
            "schemaVersion": 1,
            "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
            "editor": {
                "factions": [
                    {"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]},
                    {"key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq2"]}
                ],
                "squads": [
                    {"id": "sq1", "callsign": "Alpha", "slotIds": ["s1"]},
                    {"id": "sq2", "callsign": "Grom", "slotIds": ["s2"]}
                ],
                "slots": [
                    {"id": "s1", "role": "SL", "position": {"x": 4839.2, "y": 6620.8, "z": 0.0}},
                    {"id": "s2", "role": "RFL", "position": {"x": 6010.0, "y": 7211.5, "z": 0.0}}
                ]
            }
        });
        assert!(
            validate_editor_payload(&p).is_empty(),
            "{:?}",
            validate_editor_payload(&p)
        );
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
        fn dead_eval(_r: &Rule, _p: &Value) -> Vec<Finding> {
            Vec::new()
        }
        let dead = Rule {
            id: "DEAD-RULE",
            severity: Severity::Error,
            primitive: Primitive::FieldShape,
            applies: |_| true,
            eval: dead_eval,
            trip_fixture: || json!({"anything": true}),
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
            applies: |_| false, // never applies — so it can never fire, on anything
            eval: |rule, _| vec![rule.finding("unreachable".into(), "/x".into())],
            trip_fixture: || json!({}),
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
            applies: |_| true,
            eval: |_, _| Vec::new(),
            trip_fixture: || json!({}),
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
}
