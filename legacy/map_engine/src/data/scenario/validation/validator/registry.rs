//! Role: registry.
//! Position: `mission/validation/validator` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{EvalContext, Finding, Primitive, Severity, Value};

/// Validation rule with an applicability predicate, evaluator, and a fixture that proves it can fail.
pub struct Rule {
    /// Id.
    pub(super) id: &'static str,
    /// Severity.
    pub(super) severity: Severity,
    /// Primitive.
    pub(super) primitive: Primitive,

    /// Applies.
    pub(super) applies: fn(&Value, &EvalContext) -> bool,

    /// Eval.
    pub(super) eval: fn(&Rule, &Value, &EvalContext) -> Vec<Finding>,

    /// A payload that this rule is REQUIRED to fire on — the self-check's oracle (see the struct doc).
    pub(super) trip_fixture: fn() -> Value,

    /// Trip context.
    pub(super) trip_context: fn() -> Option<EvalContext>,
}

impl Rule {
    /// Id using the supplied domain data.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Severity using the supplied domain data.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Primitive using the supplied domain data.
    #[must_use]
    pub const fn primitive(&self) -> Primitive {
        self.primitive
    }

    /// Applies using the supplied domain data.
    #[must_use]
    pub fn applies(&self, payload: &Value, ctx: &EvalContext) -> bool {
        (self.applies)(payload, ctx)
    }

    /// Evaluate using the supplied domain data.
    #[must_use]
    pub fn evaluate(&self, payload: &Value) -> Vec<Finding> {
        self.evaluate_with_context(payload, &EvalContext::default())
    }

    /// Evaluate against `payload` and `ctx`, honouring the gate: returns `[]` when the rule does not apply, otherwise every finding the evaluator produced.
    #[must_use]
    pub fn evaluate_with_context(&self, payload: &Value, ctx: &EvalContext) -> Vec<Finding> {
        if !self.applies(payload, ctx) {
            return Vec::new();
        }
        (self.eval)(self, payload, ctx)
    }

    /// The payload this rule must fire on. Used by [`Registry::self_check`]; exposed so a caller can audit the trip corpus.
    #[must_use]
    pub fn trip_fixture(&self) -> Value {
        (self.trip_fixture)()
    }

    /// Trip context using the supplied domain data.
    #[must_use]
    pub fn trip_context(&self) -> Option<EvalContext> {
        (self.trip_context)()
    }

    /// Finding using the supplied domain data.
    pub(super) fn finding(&self, message: String, subject: String) -> Finding {
        Finding {
            rule_id: self.id,
            severity: self.severity,
            primitive: self.primitive,
            message,
            subject,
            subject_id: None,
        }
    }

    /// Finding id using the supplied domain data.
    pub(super) fn finding_id(
        &self,
        message: String,
        subject: String,
        subject_id: String,
    ) -> Finding {
        Finding {
            rule_id: self.id,
            severity: self.severity,
            primitive: self.primitive,
            message,
            subject,
            subject_id: Some(subject_id),
        }
    }

    /// Finding id opt using the supplied domain data.
    pub(super) fn finding_id_opt(
        &self,
        message: String,
        subject: String,
        subject_id: Option<String>,
    ) -> Finding {
        Finding {
            rule_id: self.id,
            severity: self.severity,
            primitive: self.primitive,
            message,
            subject,
            subject_id: subject_id.filter(|s| !s.is_empty()),
        }
    }
}

/// Ordered validation rules with a self-check that rejects rules whose declared failure fixture stays silent.
pub struct Registry {
    /// Rules.
    pub(super) rules: Vec<Rule>,
}

/// One rule failed [`Registry::self_check`]: it stayed silent on a payload it declared it would fire on. This is the "loud failure" the founding defect demands — surfaced as a value the caller can assert on (`self_check` returns `Result`) and as the panic message behind [`Registry::assert_self_check`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfCheckFailure {
    /// Rule id.
    pub rule_id: &'static str,
    /// Reason.
    pub reason: String,
}

impl std::fmt::Display for SelfCheckFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rule {}: {}", self.rule_id, self.reason)
    }
}

impl Registry {
    /// Build a registry from an explicit rule list. Panics if two rules share an id — a duplicate id would make findings ambiguous to a consumer routing on it, and it is a load-time authoring error, not a runtime input.
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

    /// Evaluate using the supplied domain data.
    #[must_use]
    pub fn evaluate(&self, payload: &Value) -> Vec<Finding> {
        self.evaluate_with_context(payload, &EvalContext::default())
    }

    /// Evaluate with context using the supplied domain data.
    #[must_use]
    pub fn evaluate_with_context(&self, payload: &Value, ctx: &EvalContext) -> Vec<Finding> {
        let mut out = Vec::new();
        for rule in &self.rules {
            out.extend(rule.evaluate_with_context(payload, ctx));
        }
        out
    }

    /// Self check using the supplied domain data.
    pub fn self_check(&self) -> Result<(), Vec<SelfCheckFailure>> {
        let mut failures = Vec::new();
        for rule in &self.rules {
            let fixture = rule.trip_fixture();

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

    /// [`self_check`](Registry::self_check), but panic on failure — the form a service calls once at startup so a dead rule takes the process down loudly instead of shipping.
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
