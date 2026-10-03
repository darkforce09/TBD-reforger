//! The tbd-framework mod's JSON wire matches the published contracts: every class, body builder
//! and constant group carrying `//! @contract` agrees with the definition it cites, the roster
//! wire version agrees across mod, schema and backend, the mod accepts every mission
//! `schemaVersion` the compiler emits, and every JSON shape the mod reads, writes or builds by
//! hand cites a contract or is listed below with the reason it has none.
//!
//! The scripts are read at test runtime from `apps/mod/tbd-framework/Scripts`; the tag grammar
//! is documented in `enfscript_source_support/contract_tag.rs`.

use std::collections::{BTreeMap, BTreeSet};

use axum::http::StatusCode;
use serde_json::Value;

mod common;
mod enfscript_source_support;
mod event_eligibility_support;
mod fleet_support;

use enfscript_source_support::contract_tag::{
    ContractTag, contract_tags, resolve, schema_document,
};
use enfscript_source_support::{
    ScriptClass, ScriptField, ScriptFile, ScriptMethod, hand_built_json_keys, json_context_calls,
    read_scripts, type_class_names,
};
use event_eligibility_support::{EventShape, Fixture};
use mission_compiler::{MissionMeta, flatten_to_mod_document};

const SUITE: &str = "contract_parity_mod_wire";

/// JSON shapes of the mod that cite no contract, as (script path, class or `Class.Method`,
/// reason). A row names a shape the discovery below finds untagged; a stale row fails.
const UNCONTRACTED_JSON: &[(&str, &str, &str)] = &[
    (
        "Game/TBD/API/Http/TBD_BackendConfig.c",
        "TBD_BackendConfigFile",
        "the operator's `$profile:TBD_BackendConfig.json` settings file; it never crosses the API",
    ),
    (
        "Game/TBD/API/Http/TBD_GameRuntimeAnswer.c",
        "TBD_GameRuntimeErrorBody",
        "the `{error, details}` error envelope has no definition in contracts/definitions",
    ),
    (
        "Game/TBD/API/Http/TBD_GameRuntimeAnswer.c",
        "TBD_GameRuntimeRefusalDetails",
        "reads RuntimeFenceRefusal and the fleet report refusal `{code, state}`, which no \
         definition declares",
    ),
    (
        "Game/TBD/API/FleetCommands/TBD_FleetLoadMissionAction.c",
        "TBD_FleetLoadMissionAction.SettleVerification",
        FREE_FORM_OUTCOME,
    ),
    (
        "Game/TBD/API/FleetCommands/TBD_FleetPlayerActions.c",
        "TBD_FleetPlayerActions.Broadcast",
        FREE_FORM_OUTCOME,
    ),
    (
        "Game/TBD/API/FleetCommands/TBD_FleetPlayerActions.c",
        "TBD_FleetPlayerActions.KickNow",
        FREE_FORM_OUTCOME,
    ),
    (
        "Game/TBD/API/MatchTelemetry/Queue/TBD_TelemetryQueueState.c",
        "TBD_TelemetryMatchCounters",
        TELEMETRY_QUEUE_FILE,
    ),
    (
        "Game/TBD/API/MatchTelemetry/Queue/TBD_TelemetryQueueState.c",
        "TBD_TelemetryQueueState",
        TELEMETRY_QUEUE_FILE,
    ),
    (
        "Game/TBD/API/MatchTelemetry/Queue/TBD_TelemetryQueueState.c",
        "TBD_TelemetryQueueState.ToJson",
        TELEMETRY_QUEUE_FILE,
    ),
    (
        "Game/TBD/API/MatchTelemetry/Queue/TBD_TelemetryQueueStorage.c",
        "TBD_TelemetryEntryHeader",
        TELEMETRY_QUEUE_FILE,
    ),
    (
        "Game/TBD/API/MatchTelemetry/Queue/TBD_TelemetryQueueStorage.c",
        "TBD_TelemetryQueueStorage.WriteEntry",
        TELEMETRY_QUEUE_FILE,
    ),
    (
        "Game/TBD/Core/Hashing/TBD_Sha256SelfTest.c",
        "TBD_Sha256SelfTest.CheckBytesAbove127",
        SELF_TEST_TEXT,
    ),
    (
        "Game/TBD/Core/Hashing/TBD_Sha256SelfTest.c",
        "TBD_Sha256SelfTestText",
        SELF_TEST_TEXT,
    ),
    (
        "Game/TBD/Systems/Mission/Data/TBD_MissionParams.c",
        "TBD_MissionParamLaunchFile",
        "the server's `$profile:TBD_MissionParams.json` launch selections; it never crosses the API",
    ),
    (
        "Game/TBD/Systems/Mission/Data/TBD_MissionParams.c",
        "TBD_MissionParamSelectionWire",
        "one row of `$profile:TBD_MissionParams.json`; it never crosses the API",
    ),
    (
        "Game/TBD/Systems/Mission/Loaders/Mission/TBD_MissionVariantSources.c",
        "TBD_VariantConfigWire",
        "the server's `$profile:TBD_VariantConfig.json` variant selection; it never crosses the API",
    ),
];

/// Reason of the fleet command effects that write a result `outcome`.
const FREE_FORM_OUTCOME: &str = "an ExecutionResult `outcome`, which fleet-command.schema.json \
                                 declares as a free-form object with no per-action definition";

/// Reason of the telemetry queue's own files.
const TELEMETRY_QUEUE_FILE: &str =
    "the telemetry queue's own `$profile` files; the platform receives only the tagged bodies";

/// Reason of the SHA-256 self-test.
const SELF_TEST_TEXT: &str = "a UTF-8 self-test decoding a literal in process; it never leaves it";

/// Fields of tagged classes whose JSON key is not their name, as (class, field, JSON key, how
/// the key reaches the field). `JsonLoadContext` binds a key to the field of the same name, so a
/// field bound to another key is read only by the code named here.
const RENAMED_FIELDS: &[(&str, &str, &str, &str)] = &[
    (
        "TBD_MusicCueStruct",
        "cueEvent",
        "event",
        "TBD_AudioEmitter renames the key before the read; `event` is an Enforce keyword",
    ),
    (
        "TBD_MissionParamStruct",
        "authoredDefault",
        "default",
        "TBD_MissionParams reads the key in a second pass; `default` is an Enforce keyword",
    ),
];

/// The JSON key of a tagged class's field: its name, or the key of a listed rename. A field bound
/// to a key other than its name and not listed in [`RENAMED_FIELDS`] is recorded in `problems`.
fn json_key<'a>(
    class: &ScriptClass,
    field: &'a ScriptField,
    renames_used: &mut BTreeSet<(&'static str, &'static str)>,
    problems: &mut Vec<String>,
) -> &'a str {
    let Some(bound) = field
        .binding
        .as_deref()
        .filter(|bound| *bound != field.name)
    else {
        return &field.name;
    };
    let listed = RENAMED_FIELDS
        .iter()
        .find(|(c, f, key, _)| *c == class.name && *f == field.name && *key == bound);
    match listed {
        Some((c, f, _, _)) => {
            renames_used.insert((*c, *f));
        }
        None => problems.push(format!(
            "{}.{} (line {}): documented as JSON key `{bound}`, but JsonLoadContext reads the \
             field name; rename the field or list the rename in RENAMED_FIELDS",
            class.name, field.name, field.line
        )),
    }
    bound
}

/// The script classes, by name.
fn class_index(scripts: &[ScriptFile]) -> BTreeMap<&str, (&ScriptFile, &ScriptClass)> {
    let mut index = BTreeMap::new();
    for file in scripts {
        for class in file.classes.iter().filter(|class| !class.modded) {
            index.insert(class.name.as_str(), (file, class));
        }
    }
    index
}

/// The tags of a banner; a malformed tag is recorded in `problems`.
fn tags_of(banner: &[String], at: &str, problems: &mut Vec<String>) -> Vec<ContractTag> {
    contract_tags(banner).unwrap_or_else(|error| {
        problems.push(format!("{at}: {error}"));
        Vec::new()
    })
}

fn report(what: &str, problems: &[String]) {
    assert!(
        problems.is_empty(),
        "{what}: {} problem(s):\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}

#[test]
fn contract_parity_mod_wire_structs_match_their_schemas() {
    let scripts = read_scripts();
    let mut problems = Vec::new();
    let (mut classes, mut citations, mut partial, mut open) = (0, 0, 0, Vec::new());
    let mut renames_used = BTreeSet::new();
    for file in &scripts {
        for class in &file.classes {
            let at = format!("{}:{} {}", file.path, class.line, class.name);
            let tags = tags_of(&class.banner, &at, &mut problems);
            if class.name.ends_with("Struct") && !class.modded && tags.is_empty() {
                problems.push(format!("{at}: named *Struct but cites no contract"));
            }
            if tags.is_empty() {
                continue;
            }
            classes += 1;
            if class.fields.is_empty() {
                problems.push(format!("{at}: cites a contract but declares no field"));
            }
            let keys: Vec<(&ScriptField, &str)> = class
                .fields
                .iter()
                .map(|field| {
                    (
                        field,
                        json_key(class, field, &mut renames_used, &mut problems),
                    )
                })
                .collect();
            let key_set: BTreeSet<&str> = keys.iter().map(|(_, key)| *key).collect();
            for tag in &tags {
                citations += 1;
                let view = resolve(tag);
                for (field, key) in &keys {
                    if !view.admits(key) {
                        problems.push(format!(
                            "{}:{} {}.{}: JSON key `{key}` is not a property of {}",
                            file.path, field.line, class.name, field.name, tag.text
                        ));
                    }
                }
                let missing: Vec<&str> = view
                    .required
                    .iter()
                    .map(String::as_str)
                    .filter(|name| !key_set.contains(name))
                    .collect();
                if tag.partial {
                    partial += 1;
                    if missing.is_empty() {
                        problems.push(format!(
                            "{at}: declares `partial` but carries every property {} requires",
                            tag.text
                        ));
                    }
                } else if !missing.is_empty() {
                    problems.push(format!(
                        "{at}: misses {missing:?}, required by {}; carry them or declare the \
                         projection `partial`",
                        tag.text
                    ));
                }
                if view.open {
                    open.push(format!("{} ({})", class.name, tag.text));
                }
            }
        }
    }
    println!(
        "mod wire classes: {classes} tagged, {citations} citations, {partial} partial, open \
         objects {open:?}"
    );
    for (class, field, key, _) in RENAMED_FIELDS {
        if !renames_used.contains(&(*class, *field)) {
            problems.push(format!(
                "{class}.{field} -> `{key}`: stale RENAMED_FIELDS row"
            ));
        }
    }
    assert!(classes > 0, "no tagged class found under the mod scripts");
    report("tagged mod classes against their contracts", &problems);
}

#[test]
fn contract_parity_mod_wire_body_builders_emit_their_schema_keys() {
    let scripts = read_scripts();
    let mut problems = Vec::new();
    let mut emitted: BTreeMap<String, (BTreeSet<String>, bool, BTreeSet<String>)> = BTreeMap::new();
    let mut builders = 0;
    for file in &scripts {
        for class in &file.classes {
            for method in &class.methods {
                let at = format!(
                    "{}:{} {}.{}",
                    file.path, method.line, class.name, method.name
                );
                let tags = tags_of(&method.banner, &at, &mut problems);
                if tags.is_empty() {
                    continue;
                }
                builders += 1;
                let keys = hand_built_json_keys(method);
                if keys.is_empty() {
                    problems.push(format!("{at}: cites a contract but writes no JSON key"));
                }
                for tag in &tags {
                    let view = resolve(tag);
                    for key in &keys {
                        if !view.admits(key) && !view.reachable.contains(key) {
                            problems.push(format!(
                                "{at}: writes `{key}`, which {} does not declare",
                                tag.text
                            ));
                        }
                    }
                    let citation = format!("{}{}", tag.schema_file, tag.pointer);
                    let entry = emitted.entry(citation).or_insert((
                        view.required.clone(),
                        false,
                        BTreeSet::new(),
                    ));
                    entry.1 |= tag.partial;
                    entry.2.extend(keys.iter().cloned());
                }
            }
        }
    }
    for (citation, (required, partial, keys)) in &emitted {
        let missing: Vec<&String> = required.difference(keys).collect();
        if !partial && !missing.is_empty() {
            problems.push(format!(
                "the builders citing {citation} never write {missing:?}, which it requires"
            ));
        }
    }
    println!(
        "mod wire body builders: {builders} tagged, {} contracts",
        emitted.len()
    );
    assert!(
        builders > 0,
        "no tagged body builder found under the mod scripts"
    );
    report(
        "tagged mod body builders against their contracts",
        &problems,
    );
}

#[test]
fn contract_parity_mod_wire_constants_are_values_of_their_schema_enums() {
    let scripts = read_scripts();
    let mut problems = Vec::new();
    let mut groups = 0;
    for file in &scripts {
        for class in &file.classes {
            for group in &class.constant_groups {
                let at = format!("{}:{} {}", file.path, group.line, class.name);
                for tag in tags_of(&group.banner, &at, &mut problems) {
                    groups += 1;
                    let view = resolve(&tag);
                    if view.values.is_empty() {
                        problems.push(format!("{at}: {} declares no enum or const", tag.text));
                    }
                    for constant in &group.constants {
                        match &constant.string_value {
                            Some(value) if view.values.contains(value) => {}
                            value => problems.push(format!(
                                "{}:{} {}: {value:?} is not a value of {}",
                                file.path, constant.line, constant.name, tag.text
                            )),
                        }
                    }
                }
            }
        }
    }
    println!("mod wire constant groups: {groups} tagged");
    report("tagged mod constants against their contracts", &problems);
}

/// Every class `JsonLoadContext`/`JsonSaveContext` reads or writes, and the classes its fields
/// nest.
fn json_classes<'a>(
    scripts: &'a [ScriptFile],
    index: &BTreeMap<&'a str, (&'a ScriptFile, &'a ScriptClass)>,
    problems: &mut Vec<String>,
) -> BTreeSet<&'a str> {
    const VALUE_TYPES: &[&str] = &["string", "int", "float", "bool", "vector"];
    let mut pending: Vec<&str> = Vec::new();
    for file in scripts {
        for call in json_context_calls(file) {
            match index.get_key_value(call.value_type.as_str()) {
                Some((name, _)) => pending.push(name),
                None if VALUE_TYPES.contains(&call.value_type.as_str()) => {}
                None => problems.push(format!(
                    "{}:{}: reads or writes JSON into `{}`, which is no mod class",
                    call.path, call.line, call.value_type
                )),
            }
        }
    }
    let mut found = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !found.insert(name) {
            continue;
        }
        for field in &index[name].1.fields {
            for nested in type_class_names(&field.type_text) {
                if let Some((nested, _)) = index.get_key_value(nested.as_str()) {
                    pending.push(nested);
                }
            }
        }
    }
    found
}

fn method_carries_tag(method: &ScriptMethod) -> bool {
    method
        .banner
        .iter()
        .any(|line| line.starts_with("@contract"))
}

#[test]
fn contract_parity_every_mod_api_dto_cites_a_contract() {
    let scripts = read_scripts();
    let index = class_index(&scripts);
    let mut problems = Vec::new();
    let mut untagged: BTreeSet<(String, String)> = BTreeSet::new();
    let classes = json_classes(&scripts, &index, &mut problems);
    for name in &classes {
        let (file, class) = index[name];
        if !class
            .banner
            .iter()
            .any(|line| line.starts_with("@contract"))
        {
            untagged.insert((file.path.clone(), class.name.clone()));
        }
    }
    let mut builders = 0;
    for file in &scripts {
        let mut bound = 0;
        for class in &file.classes {
            let count = |banner: &[String]| {
                banner
                    .iter()
                    .filter(|line| line.starts_with("@contract"))
                    .count()
            };
            bound += count(&class.banner);
            bound += class
                .constant_groups
                .iter()
                .map(|g| count(&g.banner))
                .sum::<usize>();
            for method in &class.methods {
                bound += count(&method.banner);
                if hand_built_json_keys(method).is_empty() {
                    continue;
                }
                builders += 1;
                if !method_carries_tag(method) {
                    let symbol = format!("{}.{}", class.name, method.name);
                    untagged.insert((file.path.clone(), symbol));
                }
            }
        }
        if bound != file.contract_tag_lines.len() {
            problems.push(format!(
                "{}: {} @contract line(s) at {:?}, {bound} bound to a class, method or constant",
                file.path,
                file.contract_tag_lines.len(),
                file.contract_tag_lines
            ));
        }
    }
    let listed: BTreeSet<(String, String)> = UNCONTRACTED_JSON
        .iter()
        .map(|(path, symbol, reason)| {
            assert!(!reason.trim().is_empty(), "{path} {symbol}: empty reason");
            ((*path).to_owned(), (*symbol).to_owned())
        })
        .collect();
    for (path, symbol) in untagged.difference(&listed) {
        problems.push(format!(
            "{path} {symbol}: JSON crosses a boundary without a //! @contract and is not listed"
        ));
    }
    for (path, symbol) in listed.difference(&untagged) {
        problems.push(format!("{path} {symbol}: stale UNCONTRACTED_JSON row"));
    }
    println!(
        "mod JSON shapes: {} classes read or written, {builders} hand-built bodies, {} listed \
         without a contract",
        classes.len(),
        listed.len()
    );
    assert!(!classes.is_empty() && builders > 0, "no JSON shape found");
    report("mod JSON shapes without a contract", &problems);
}

/// The value of the integer constant `name` declared in `file`.
fn integer_constant(file: &ScriptFile, name: &str) -> i64 {
    let values: Vec<i64> = file
        .code
        .lines()
        .filter(|line| line.contains("const ") && line.contains(&format!(" {name} ")))
        .filter_map(|line| line.split_once('=').map(|(_, value)| value))
        .filter_map(|value| value.trim().trim_end_matches(';').trim().parse().ok())
        .collect();
    assert_eq!(
        values.len(),
        1,
        "{}: one integer constant {name}",
        file.path
    );
    values[0]
}

fn script<'a>(scripts: &'a [ScriptFile], suffix: &str) -> &'a ScriptFile {
    let found: Vec<&ScriptFile> = scripts
        .iter()
        .filter(|f| f.path.ends_with(suffix))
        .collect();
    assert_eq!(found.len(), 1, "one script ends with {suffix}");
    found[0]
}

#[tokio::test]
async fn contract_parity_mod_roster_wire_version_matches_backend_and_schema() {
    let scripts = read_scripts();
    let mod_version = integer_constant(script(&scripts, "/TBD_RosterLoader.c"), "WIRE_VERSION");
    let schema = schema_document("game-runtime-roster.schema.json");
    let schema_version = schema["properties"]["version"]["const"]
        .as_i64()
        .expect("the roster schema pins `version` with a const");

    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = fleet_support::register_server(&f, "Roster wire host").await;
    fleet_support::bind_event(&f, server).await;
    let secret = fleet_support::credential(&f, server, "mod_runtime").await;
    let (status, roster) = f
        .call(
            &fleet_support::machine(&secret),
            "GET",
            &format!("/api/v1/game-runtime/events/{}/roster", f.event),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{roster}");
    let backend_version = roster["version"]
        .as_i64()
        .unwrap_or_else(|| panic!("the roster carries an integer version: {roster}"));

    assert_eq!(
        (mod_version, backend_version),
        (schema_version, schema_version),
        "TBD_RosterLoader.WIRE_VERSION, the backend's roster `version` and the schema const agree"
    );
}

/// The string constants declared in `file`, by name.
fn string_constants(file: &ScriptFile) -> BTreeMap<String, String> {
    let mut constants = BTreeMap::new();
    for line in file
        .text
        .lines()
        .filter(|line| line.contains("const string "))
    {
        let Some((left, right)) = line.split_once('=') else {
            continue;
        };
        let name = left
            .split_whitespace()
            .last()
            .unwrap_or_default()
            .to_owned();
        let literal = right
            .trim_start()
            .strip_prefix('"')
            .and_then(|rest| rest.split_once('"'));
        if let Some((value, _)) = literal {
            constants.insert(name, value.to_owned());
        }
    }
    constants
}

/// The `schemaVersion` values the mod's `CheckSchemaVersion` accepts: the constants it compares
/// the document's version against.
fn mod_schema_version_window(scripts: &[ScriptFile]) -> BTreeSet<String> {
    let file = script(scripts, "/TBD_MissionStructureChecks.c");
    let constants = string_constants(file);
    let method = file
        .classes
        .iter()
        .flat_map(|class| &class.methods)
        .find(|method| method.name == "CheckSchemaVersion")
        .expect("TBD_MissionStructureChecks declares CheckSchemaVersion");
    let window: BTreeSet<String> = method
        .body
        .split("==")
        .skip(1)
        .filter_map(|rest| {
            let name: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            constants.get(&name).cloned()
        })
        .collect();
    assert!(
        !window.is_empty(),
        "CheckSchemaVersion compares no version constant"
    );
    window
}

/// Every `schemaVersion` literal the mission compiler chooses between.
fn compiler_schema_versions() -> BTreeSet<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/mission/mission_compiler/src/game_document/compile_graph.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let start = source
        .find("let schema_version =")
        .expect("the compiler binds `schema_version`");
    let expression = &source[start..start + source[start..].find(';').expect("statement end")];
    let versions: BTreeSet<String> = expression
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    assert!(
        !versions.is_empty(),
        "the compiler names no schemaVersion literal"
    );
    versions
}

#[test]
fn contract_parity_mod_mission_schema_version_window_includes_the_compiler_version() {
    let scripts = read_scripts();
    let window = mod_schema_version_window(&scripts);
    let schema = schema_document("mission.schema.json");
    let defined: BTreeSet<String> = schema["properties"]["schemaVersion"]["enum"]
        .as_array()
        .expect("mission.schema.json enumerates schemaVersion")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let emittable = compiler_schema_versions();
    let meta = MissionMeta {
        id: "contract-parity-mod-wire".into(),
        title: "Contract parity".to_owned(),
        terrain: "everon".to_owned(),
        ..MissionMeta::default()
    };
    let compiled = flatten_to_mod_document(&meta, common::COMPILABLE_EDITOR_PAYLOAD.as_bytes())
        .unwrap_or_else(|error| panic!("the compilable payload compiles: {error:?}"));

    println!(
        "mission schemaVersion: mod window {window:?}, schema {defined:?}, compiler {emittable:?}, \
         compiled {}",
        compiled.schema_version
    );
    assert!(
        emittable.contains(&compiled.schema_version),
        "the compiled version {} is one the compiler source names ({emittable:?})",
        compiled.schema_version
    );
    let unread: Vec<&String> = emittable.difference(&window).collect();
    assert!(
        unread.is_empty(),
        "the mod refuses compiler versions {unread:?}"
    );
    let undefined: Vec<&String> = emittable
        .union(&window)
        .filter(|v| !defined.contains(*v))
        .collect();
    assert!(
        undefined.is_empty(),
        "versions outside mission.schema.json: {undefined:?}"
    );
}
