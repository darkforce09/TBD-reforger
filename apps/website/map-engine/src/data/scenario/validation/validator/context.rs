//! Role: context.
//! Position: `mission/validation/validator` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::HashSet;

/// Facts supplied by the caller for context-dependent validation; absent facts leave those rules inactive.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct EvalContext {
    /// Known asset ids.
    pub known_asset_ids: Option<HashSet<String>>,

    /// Cargo phys.
    pub cargo_phys: Option<crate::data::scenario::wire_safety::CargoPhysCatalog>,

    /// Loadout policy.
    pub loadout_policy: Option<LoadoutPolicy>,
}

impl EvalContext {
    /// With known asset ids using the supplied domain data.
    #[must_use]
    pub fn with_known_asset_ids(mut self, ids: HashSet<String>) -> Self {
        self.known_asset_ids = Some(ids);
        self
    }

    /// With cargo phys using the supplied domain data.
    #[must_use]
    pub fn with_cargo_phys(
        mut self,
        catalog: crate::data::scenario::wire_safety::CargoPhysCatalog,
    ) -> Self {
        self.cargo_phys = Some(catalog);
        self
    }

    /// A context carrying the mission loadout/cargo policy thresholds (see [`LoadoutPolicy`]). Chainable.
    #[must_use]
    pub fn with_loadout_policy(mut self, policy: LoadoutPolicy) -> Self {
        self.loadout_policy = Some(policy);
        self
    }
}

/// `#[non_exhaustive]`: policy dimensions grow; build it via [`LoadoutPolicy::default`] + the setters, which chain, for the same cross-crate E0639 reason [`EvalContext`] documents.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct LoadoutPolicy {
    /// The minimum magazine count a slot that carries a primary weapon must load — WOG `fn_check_weapon`'s "below-standard magazine count" as a typed threshold (FNF compares a serialised sentinel string; a typed floor deletes that brittleness). `None` ⇒ no floor configured ⇒ [`rule_loadout_mag_count`] skips.
    pub min_magazines: Option<u64>,

    /// Required equipment.
    pub required_equipment: Option<HashSet<String>>,

    /// The maximum number of cargo ITEMS (summed `qty`) a placed vehicle's inventory may carry — MissionAnalyzer's R9 (a PvP fairness check: one side pre-stuffing a truck with ammo). `None` ⇒ no ceiling ⇒ [`rule_vehicle_cargo_policy`] skips.
    pub max_vehicle_cargo_items: Option<u64>,
}

impl LoadoutPolicy {
    /// Set the minimum-magazine floor (WOG below-standard-magazine check). Chainable.
    #[must_use]
    pub fn with_min_magazines(mut self, n: u64) -> Self {
        self.min_magazines = Some(n);
        self
    }

    /// Set the required-equipment kind set (WOG missing map/compass/radio). Chainable.
    #[must_use]
    pub fn with_required_equipment(mut self, kinds: HashSet<String>) -> Self {
        self.required_equipment = Some(kinds);
        self
    }

    /// Set the per-vehicle cargo-item ceiling (MissionAnalyzer R9). Chainable.
    #[must_use]
    pub fn with_max_vehicle_cargo_items(mut self, n: u64) -> Self {
        self.max_vehicle_cargo_items = Some(n);
        self
    }
}

/// Domain representation of severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    /// Domain representation of error.
    Error,
    /// Domain representation of warning.
    Warning,
    /// Domain representation of info.
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

/// Which of the four primitives a rule is an instance of. Carried on every [`Finding`] so a downstream consumer (the panel, an analytics pass) can group findings by the shape of the check that produced them without re-deriving it from the id.
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

/// One thing a rule found wrong. The stable half (`rule_id`, `severity`, `primitive`) lets a consumer filter/group without parsing prose; `message` is the author-facing sentence (same register as the wire-safety scanners — where, what, why, in one line); `subject` is the JSON-pointer-ish path into the payload the author can act on (`/editor/slots/3/position`), so the panel can focus the offender.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Rule id.
    pub rule_id: &'static str,
    /// Severity.
    pub severity: Severity,
    /// Primitive.
    pub primitive: Primitive,
    /// Message.
    pub message: String,
    /// Subject.
    pub subject: String,

    /// Subject id.
    pub subject_id: Option<String>,
}
