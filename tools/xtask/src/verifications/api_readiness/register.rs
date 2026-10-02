//! The acceptance register binds requirements to independently executable checks.

use crate::core::repository_layout::documentation::API_READINESS_REGISTER;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Register {
    pub version: u32,
    pub requirements: Vec<Requirement>,
    pub checks: Vec<Check>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Requirement {
    pub id: String,
    pub behavior: String,
    pub implementation: Vec<String>,
    pub checks: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum EvidenceClass {
    Implementation,
    Property,
    Operational,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Check {
    pub id: String,
    pub class: EvidenceClass,
    /// None means an external staging runner must supply a receipt.
    pub command: Option<Vec<String>>,
    pub timeout_seconds: u64,
    pub minimum_cases: u64,
    pub success_marker: String,
    /// Regex matching successful executable acceptance cases in the output.
    pub case_pattern: String,
    #[serde(default)]
    pub properties: Vec<PropertyRequirement>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PropertyRequirement {
    pub id: String,
    pub minimum_cases: u32,
}

pub(super) fn read(root: &Path) -> Result<Register> {
    let bytes = std::fs::read(root.join(API_READINESS_REGISTER))
        .with_context(|| format!("read API requirement register {API_READINESS_REGISTER}"))?;
    let register: Register =
        serde_json::from_slice(&bytes).context("parse API requirement register")?;
    validate(root, &register)?;
    Ok(register)
}

pub(super) fn relative_path(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

pub(super) fn validate(root: &Path, register: &Register) -> Result<()> {
    ensure!(register.version == 1, "unsupported API register version");
    ensure!(
        !register.requirements.is_empty() && !register.checks.is_empty(),
        "empty API register"
    );
    let mut check_ids = BTreeSet::new();
    for check in &register.checks {
        ensure!(
            identifier(&check.id) && check_ids.insert(&check.id),
            "invalid or duplicate check {}",
            check.id
        );
        ensure!(
            check.timeout_seconds > 0
                && check.minimum_cases > 0
                && !check.success_marker.trim().is_empty(),
            "vacuous check {}",
            check.id
        );
        regex::Regex::new(&check.case_pattern).context("invalid acceptance case pattern")?;
        ensure!(
            !check.case_pattern.is_empty(),
            "empty case pattern for {}",
            check.id
        );
        ensure!(
            (check.class == EvidenceClass::Property) != check.properties.is_empty(),
            "property checks must enumerate required generated-case evidence: {}",
            check.id
        );
        let mut property_ids = BTreeSet::new();
        for property in &check.properties {
            ensure!(
                identifier(&property.id)
                    && property.minimum_cases > 0
                    && property_ids.insert(&property.id),
                "invalid or duplicate required property on {}",
                check.id
            );
        }
        if let Some(command) = &check.command {
            ensure!(
                command.len() > 1 && command.iter().all(|s| !s.is_empty()),
                "empty command for {}",
                check.id
            );
        }
    }
    let mut ids = BTreeSet::new();
    let mut referenced = BTreeSet::new();
    for requirement in &register.requirements {
        ensure!(
            identifier(&requirement.id) && ids.insert(&requirement.id),
            "invalid or duplicate requirement {}",
            requirement.id
        );
        ensure!(
            !requirement.behavior.trim().is_empty()
                && !requirement.implementation.is_empty()
                && !requirement.checks.is_empty(),
            "incomplete requirement {}",
            requirement.id
        );
        for path in &requirement.implementation {
            ensure!(
                relative_path(path) && root.join(path).exists(),
                "missing implementation for {}: {}",
                requirement.id,
                path
            );
        }
        for check in &requirement.checks {
            ensure!(
                check_ids.contains(check),
                "unknown check {check} on {}",
                requirement.id
            );
            referenced.insert(check);
        }
        ensure!(
            requirement.assumptions.iter().all(|a| !a.trim().is_empty()),
            "blank assumption on {}",
            requirement.id
        );
    }
    ensure!(
        referenced == check_ids,
        "unreferenced checks in API register"
    );
    Ok(())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
