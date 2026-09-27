use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Field {
    pub property: String,
    pub native_type: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Rule {
    pub disposition: String,
    pub section: String,
    pub follow_reference: bool,
    pub reason: String,
    pub fields: Vec<Field>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Class {
    pub class_name: String,
    pub ancestor_types: Vec<String>,
    pub section: String,
    pub rules: Vec<Rule>,
}

pub(super) struct Policy {
    pub classes: BTreeMap<String, Class>,
    pub fields: BTreeMap<(String, String, String), Rule>,
    pub digest: String,
}

impl Policy {
    pub fn load(root: &Path) -> Result<Self> {
        let directory = root.join("contracts_v2/rules/equipment-gameplay");
        let bytes = fs::read(directory.join("policy.json"))?;
        let manifest: Value = serde_json::from_slice(&bytes)?;
        ensure!(
            manifest["policy_version"] == 1,
            "unsupported gameplay policy"
        );
        let mut hash = Sha256::new();
        hash.update(bytes);
        let mut classes = BTreeMap::new();
        let mut fields = BTreeMap::new();
        for file in manifest["class_files"]
            .as_array()
            .context("policy class files")?
        {
            let relative = file.as_str().context("policy file name")?;
            ensure!(
                !relative.contains("..") && !relative.starts_with('/'),
                "unsafe policy file"
            );
            let bytes = fs::read(directory.join(relative))?;
            hash.update(&bytes);
            for class in serde_json::from_slice::<Vec<Class>>(&bytes)? {
                for rule in &class.rules {
                    ensure!(
                        [
                            "retain_value",
                            "retain_relationship",
                            "traverse_required_container",
                            "exclude"
                        ]
                        .contains(&rule.disposition.as_str()),
                        "invalid disposition"
                    );
                    ensure!(!rule.reason.is_empty(), "field decision lacks its purpose");
                    for field in &rule.fields {
                        ensure!(
                            fields
                                .insert(
                                    (
                                        class.class_name.clone(),
                                        field.property.clone(),
                                        field.native_type.clone()
                                    ),
                                    rule.clone()
                                )
                                .is_none(),
                            "duplicate policy field"
                        );
                    }
                }
                ensure!(
                    classes.insert(class.class_name.clone(), class).is_none(),
                    "duplicate policy class"
                );
            }
        }
        ensure!(
            Some(fields.len() as u64) == manifest["baseline_field_combinations"].as_u64(),
            "policy coverage differs from baseline census"
        );
        Ok(Self {
            classes,
            fields,
            digest: format!("{:x}", hash.finalize()),
        })
    }

    pub fn rule(&self, class: &str, property: &str, native_type: &str) -> Result<&Rule> {
        self.fields
            .get(&(class.into(), property.into(), native_type.into()))
            .with_context(|| format!("unreviewed field: {class}.{property} ({native_type})"))
    }

    pub fn selected(&self, class: &str) -> Result<bool> {
        Ok(self
            .classes
            .get(class)
            .with_context(|| format!("unreviewed class: {class}"))?
            .section
            != "excluded")
    }
}
