use developer_tools::repository_layout::mission_fixtures_valid_dir;

use super::{read_json, repo_root, strip_enfusion_comments_and_strings};
use std::fs;

fn body(source: &str, signature: &str) -> String {
    let start = source.find(signature).expect(signature);
    let start = start + source[start..].find('{').expect("method body");
    let mut depth = 0;
    for (at, ch) in source[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return source[start..=start + at].to_owned();
        }
    }
    panic!("unclosed method: {signature}");
}

#[test]
fn invalid_side_is_neutral_but_absent_and_valid_sides_keep_their_roles() {
    let root = repo_root().expect("repo root");
    let lane = root.join("apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives");
    let objective = strip_enfusion_comments_and_strings(
        &fs::read_to_string(lane.join("TBD_Objective.c")).expect("objective source"),
    );
    let registry = strip_enfusion_comments_and_strings(
        &fs::read_to_string(lane.join("TBD_ObjectiveRegistry.c")).expect("registry source"),
    );
    let golden = read_json(&mission_fixtures_valid_dir(&root).join("schema-1_3-wire-fields.json"))
        .expect("staged golden");
    let row = &golden["objectives"][0];
    let mut program = String::from(
        r#"
#include <iostream>
#include <string>
#include <vector>
struct string : std::string {
    using std::string::string;
    bool IsEmpty() const { return empty(); }
    static const string Empty;
    template<class... Args> static string Format(Args...) { return {}; }
};
const string string::Empty;
enum class TBD_EObjectiveRole { NEUTRAL, ATTACKER, DEFENDER };
enum class TBD_EObjectiveKind { NONE, CAPTURE, DESTROY, HOLD_UNTIL };
struct TBD_Log {
    template<class... Args> static void Warn(Args...) {}
    template<class... Args> static void Kv(Args...) {}
};
struct Row { string side; operator bool() const { return true; } };
struct TBD_ObjectiveEntityReader {
    static bool FactionExists(string key) {
        for (const auto& faction : factions) if (faction == key) return true;
        return false;
    }
    static std::vector<string> factions;
};
struct Objective {
    TBD_EObjectiveKind m_eKind = TBD_EObjectiveKind::CAPTURE;
    string DisplayName() { return m_sLabel; }
"#,
    );
    // Use the live declarations/defaults, so deleting or forgetting the validity field is
    // observable too. Remaining engine state is unnecessary to execute these methods.
    let fields = regex::Regex::new(r"(?m)^\s*(string|bool) (m_\w+);").unwrap();
    for field in fields.captures_iter(&objective) {
        program.push_str(&format!("{} {}{{}};\n", &field[1], &field[2]));
    }
    for signature in [
        "TBD_EObjectiveRole RoleOf(string viewerFaction)",
        "string TitleFor(string viewerFaction)",
        "string TaskTextFor(string viewerFaction)",
        "bool MayOwn(string factionKey)",
    ] {
        program.push_str(signature);
        program.push_str(&body(&objective, signature));
    }
    program.push_str("};\nstruct Registry { static inline string CH;\n");
    program.push_str("static void CheckTypedSide(Objective& objective, Row row, string subject)");
    program.push_str(&body(&registry, "void CheckTypedSide("));
    program.push_str("};\nstd::vector<string> TBD_ObjectiveEntityReader::factions = {");
    for faction in golden["factions"].as_array().expect("golden factions") {
        program.push_str(&format!("{},", faction["key"]));
    }
    program.push_str("};\nint main() {\n");
    for (name, value) in [
        ("label", &row["label"]),
        ("attackTitle", &row["framing"]["attacker"]["title"]),
        ("attackText", &row["framing"]["attacker"]["text"]),
        ("defendTitle", &row["framing"]["defender"]["title"]),
        ("defendText", &row["framing"]["defender"]["text"]),
    ] {
        assert!(!value.as_str().expect("golden framing string").is_empty());
        program.push_str(&format!("const string {name} = {value};\n"));
    }
    program.push_str(
        r#"
int failures = 0, checked = 0;
for (const string task : {"capture", "destroy", "hold", "defend"})
for (const string side : {"nonexistent", "", "blufor", "opfor"})
for (const string zone : {"", "blufor", "opfor"}) {
    Objective objective;
    objective.m_sSide = side;
    objective.m_sFaction = zone;
    objective.m_sLabel = label;
    objective.m_bSideDefends = task == "hold" || task == "defend";
    objective.m_sAttackerTitle = attackTitle;
    objective.m_sAttackerText = attackText;
    objective.m_sDefenderTitle = defendTitle;
    objective.m_sDefenderText = defendText;
    Registry::CheckTypedSide(objective, Row{side}, "fixture-zone");
    for (const string viewer : {"blufor", "opfor", ""}) {
        auto expected = TBD_EObjectiveRole::NEUTRAL;
        const string owner = side.empty() ? zone : side;
        if (!viewer.empty() && side != "nonexistent" && !owner.empty())
            expected = ((viewer == owner) == (task == "hold" || task == "defend"))
                ? TBD_EObjectiveRole::DEFENDER : TBD_EObjectiveRole::ATTACKER;
        const string title = expected == TBD_EObjectiveRole::ATTACKER ? attackTitle
            : expected == TBD_EObjectiveRole::DEFENDER ? defendTitle : label;
        const string text = expected == TBD_EObjectiveRole::ATTACKER ? attackText
            : expected == TBD_EObjectiveRole::DEFENDER ? defendText : string::Empty;
        const bool mayOwn = !viewer.empty() && (zone.empty() || zone == viewer);
        ++checked;
        if (objective.RoleOf(viewer) != expected || objective.TitleFor(viewer) != title
            || objective.TaskTextFor(viewer) != text || objective.m_sFaction != zone
            || objective.MayOwn(viewer) != mayOwn) {
            ++failures;
            std::cout << "task=" << task << " side='" << side << "' zone='" << zone
                << "' viewer='" << viewer << "' expected_role=" << int(expected)
                << " actual_role=" << int(objective.RoleOf(viewer))
                << " title='" << objective.TitleFor(viewer) << "' text='"
                << objective.TaskTextFor(viewer) << "'\n";
        }
    }
}
std::cout << "T-212 source simulation: " << checked << " cases, " << failures << " failures\n";
return failures == 0 ? 0 : 1;
}
"#,
    );
    for namespace in [
        "TBD_EObjectiveRole",
        "TBD_ObjectiveEntityReader",
        "TBD_Log",
        "string",
    ] {
        program = program.replace(&format!("{namespace}."), &format!("{namespace}::"));
    }
    let dir = root
        .join("target")
        .join(format!("t212-side-fallback-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("probe directory");
    let input = dir.join("probe.cpp");
    let binary = dir.join("probe");
    fs::write(&input, program).expect("source probe");
    let host = crate::core::host_execution::Host::detect();
    let compile = host.run(&[
        "c++",
        "-std=c++17",
        "-o",
        binary.to_str().expect("probe path"),
        input.to_str().expect("source path"),
    ]);
    assert_eq!(compile, 0, "C++ source simulation must compile");
    let result = host.run(&[binary.to_str().expect("probe path")]);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(result, 0, "source simulation must pass");
}
