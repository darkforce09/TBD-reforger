//! The Workbench selection tables generated from the gameplay policy.
//!
//! **Role:** writes the export addon's generated policy tables, or with `--check` compares them and
//! reports a difference or an extra file.
//! **Position:** under [`crate::equipment_gameplay`]; `mod generate-equipment-gameplay-policy`
//! calls it.
//! **Signals & state:** none beyond the files it writes.
//! **Invariants:** `--check` writes nothing; the generated text depends only on the policy.

use super::policy::Policy;
use crate::error::{Result, ensure};
use repository_layout::enfusion_mod_folders::EXPORT_ADDON_DIR;
use std::{collections::BTreeMap, fs, path::Path};

pub(super) fn generate(root: &Path, check: bool) -> Result<()> {
    let policy = Policy::load(root)?;
    let directory = root
        .join(EXPORT_ADDON_DIR)
        .join("Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated");
    let mut sections = BTreeMap::<String, Vec<String>>::new();
    for class in policy.classes.values() {
        let lines = sections.entry(class.section.clone()).or_default();
        lines.push(format!(
            "\t\tpolicy.AddClass({}, {});",
            literal(&class.class_name),
            literal(&class.section)
        ));
        for rule in &class.rules {
            let fields = rule
                .fields
                .iter()
                .map(|f| format!("{}\t{}", f.property, f.native_type))
                .collect::<Vec<_>>()
                .join("|");
            ensure!(
                !rule.fields.iter().any(|f| f.property.contains(['|', '\t'])),
                "unsupported policy separator"
            );
            lines.push(format!(
                "\t\tpolicy.AddRule({}, {}, {}, {}, {}, {});",
                literal(&class.class_name),
                literal(&fields),
                literal(&rule.disposition),
                literal(&rule.section),
                rule.follow_reference,
                literal(&rule.reason)
            ));
        }
    }
    let mut output = BTreeMap::new();
    let mut main = format!(
        "// Generated from contracts/rules/equipment-gameplay.\nclass TBD_GameplayPolicyGenerated\n{{\n\tstatic const int VERSION = 1;\n\tstatic const string DIGEST = {};\n\tstatic void Apply(TBD_GameplaySelectionPolicy policy)\n\t{{\n",
        literal(&policy.digest)
    );
    for (section, lines) in sections {
        for (index, chunk) in lines.chunks(180).enumerate() {
            let class_name = format!("TBD_GameplayPolicy_{section}_{index}");
            output.insert(format!("{section}/{class_name}.c"), format!("// Generated from the authoritative gameplay field-selection policy.\nclass {class_name}\n{{\n\tstatic void Apply(TBD_GameplaySelectionPolicy policy)\n\t{{\n{}\n\t}}\n}}\n", chunk.join("\n")));
            main.push_str(&format!("\t\t{class_name}.Apply(policy);\n"));
        }
    }
    main.push_str("\t}\n}\n");
    output.insert("TBD_GameplayPolicyGenerated.c".into(), main);
    for (relative, content) in &output {
        let path = directory.join(relative);
        if check {
            ensure!(
                fs::read_to_string(&path).ok().as_deref() == Some(content),
                "generated gameplay policy drift: {}",
                path.display()
            );
        } else {
            fs::create_dir_all(path.parent().expect("policy parent"))?;
            fs::write(path, content)?;
        }
    }
    if directory.is_dir() {
        for entry in walkdir::WalkDir::new(&directory)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let relative = entry.path().strip_prefix(&directory)?.to_string_lossy();
            ensure!(
                output.contains_key(relative.as_ref()),
                "unexpected generated policy file: {relative}"
            );
        }
    }
    eprintln!(
        "Gameplay policy: {} field decisions; {} generated files",
        policy.fields.len(),
        output.len()
    );
    Ok(())
}

fn literal(text: &str) -> String {
    serde_json::to_string(text).expect("policy string")
}
