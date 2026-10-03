//! The names a caller of the validator imports with `use mission_validation::prelude::*;`.

pub use crate::{
    AssetId, Error, EvalContext, Finding, LoadoutPolicy, Primitive, Registry, Result, Rule, RuleId,
    SelfCheckFailure, Severity, SubjectId, default_registry, validate_editor_payload,
};
