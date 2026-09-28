use super::super::enfusion_script_lexer::{CommentMarker, split_script_lines, strip_c_comments};
use super::*;

const CLEAN_NAME: &str = "TBD_SampleWidget.c";

/// A script that meets every rule of the card.
const CLEAN: &str = r#"/**
 * @file TBD_SampleWidget.c
 * @brief Sample widget component the comment rules are proven on.
 *
 * Role: counts events.  Position: fed by the game mode, read by the HUD.
 * State: m_iCount on the server.  Invariants: the count never drops below zero.
 */

//! States a sample widget moves through.
enum TBD_SampleWidgetState
{
	IDLE,	//!< default state
	ACTIVE	//!< after Start
}

//! Editor class of the sample widget component.
[ComponentEditorProps(category: "TBD/Framework", description: "Sample")]
class TBD_SampleWidgetClass : ScriptComponentClass
{
}

//! Sample component counting server events.
class TBD_SampleWidget : ScriptComponent
{
	[Attribute("1", UIWidgets.EditBox, desc: "Step per event")]
	protected int m_iStep; //!< events per call; default 1

	//! @replicated m_iCount
	[RplProp()]
	protected int m_iCount; //!< events counted; JSON key "count"

	protected string m_sUrl = "http://example//x"; //!< endpoint; default the sample host
	protected string m_sMode = "reworked slice; wave"; //!< mode name; default sample
	protected ref array<int> m_aSteps = {1, 2}; //!< step history; default {1, 2}

	//! Asks the server to count one event.
	//! @authority owner
	void RequestCount()
	{
		Rpc(RpcAsk_Count, m_iStep);
	}

	//! Counts one event on the server.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void RpcAsk_Count(int step)
	{
		if (step > 0)
		{
			m_iCount += step;
		}
	}

	//! Posts the count to the platform.
	//! @authority server
	//! @route POST /api/v1/sample
	void Report(string label,
		int attempt)
	{
		TBD_GameRuntimeHttp.Post("/api/v1/sample", m_iCount.ToString());
	}
}

//! Payload of the sample report.
//! @contract game-runtime-session.schema.json#/definitions/RuntimeHeartbeat
class TBD_SampleReportStruct : JsonApiStruct
{
	int count; //!< JSON key "count"
}
"#;

fn findings_of(name: &str, source: &str, rule: RuleId) -> Vec<Finding> {
    check_script(name, source)
        .into_iter()
        .filter(|finding| finding.rule == rule)
        .collect()
}

fn fails(source: &str, rule: RuleId) -> Vec<Finding> {
    let found = findings_of(CLEAN_NAME, source, rule);
    assert!(!found.is_empty(), "{rule} must fire on:\n{source}");
    found
}

fn edited(from: &str, to: &str) -> String {
    assert!(CLEAN.contains(from), "fixture lacks `{from}`");
    CLEAN.replacen(from, to, 1)
}

#[test]
fn clean_fixture_passes_every_rule() {
    assert_eq!(check_script(CLEAN_NAME, CLEAN), Vec::<Finding>::new());
}

#[test]
fn ascii_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::AsciiOnly).is_empty());
    let found = fails(
        &edited("\"Sample\"", "\"Sample \u{2014} widget\""),
        RuleId::AsciiOnly,
    );
    assert_eq!(found[0].line, 17);
    assert!(found[0].message.contains("U+2014"));
}

#[test]
fn file_header_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::FileHeader).is_empty());
    let wrong_name = fails(
        &edited("@file TBD_SampleWidget.c", "@file TBD_Other.c"),
        RuleId::FileHeader,
    );
    assert!(wrong_name[0].message.contains("TBD_Other.c"));
    let late = fails(&format!("\n{CLEAN}"), RuleId::FileHeader);
    assert_eq!(late.len(), 1);
    let empty_role = fails(&edited("Role: counts events.", "Role:"), RuleId::FileHeader);
    assert!(empty_role[0].message.contains("Role:"));
}

#[test]
fn declaration_banner_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::DeclarationBanner).is_empty());
    let found = fails(
        &edited(
            "\t//! @authority owner\n\tvoid RequestCount()",
            "\t//! @authority owner\n\n\tvoid RequestCount()",
        ),
        RuleId::DeclarationBanner,
    );
    assert!(found[0].message.contains("RequestCount"));
    let enum_found = fails(
        &edited("//! States a sample widget moves through.\n", ""),
        RuleId::DeclarationBanner,
    );
    assert!(
        enum_found[0]
            .message
            .contains("enum `TBD_SampleWidgetState`")
    );
}

#[test]
fn trailing_member_doc_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::TrailingMemberDoc).is_empty());
    let field = fails(
        &edited(" //!< events per call; default 1", ""),
        RuleId::TrailingMemberDoc,
    );
    assert!(field[0].message.contains("m_iStep"));
    let member = fails(&edited("\t//!< after Start", ""), RuleId::TrailingMemberDoc);
    assert!(member[0].message.contains("ACTIVE"));
}

#[test]
fn network_authority_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::NetworkAuthority).is_empty());
    let authority = fails(
        &edited("\t//! @authority owner\n", ""),
        RuleId::NetworkAuthority,
    );
    assert!(authority[0].message.contains("RequestCount"));
    let mismatch = fails(
        &edited("@rpc Reliable Server", "@rpc Unreliable Server"),
        RuleId::NetworkAuthority,
    );
    assert!(mismatch[0].message.contains("Reliable Server"));
    let missing_rpc = fails(
        &edited("\t//! @rpc Reliable Server\n", ""),
        RuleId::NetworkAuthority,
    );
    assert!(missing_rpc[0].message.contains("[RplRpc]"));
    let replicated = fails(
        &edited("\t//! @replicated m_iCount\n", ""),
        RuleId::NetworkAuthority,
    );
    assert!(replicated[0].message.contains("[RplProp]"));
    let bad_value = fails(
        &edited("@authority owner", "@authority anywhere"),
        RuleId::NetworkAuthority,
    );
    assert!(bad_value[0].message.contains("anywhere"));
}

#[test]
fn authority_helper_calls_ask_where_a_method_runs() {
    let report_code = "\t\tTBD_GameRuntimeHttp.Post(";
    for helper in ["TBD_Authority.IsClient()", "TBD_Authority.IsServer()"] {
        let asks = format!("\t\tif ({helper})\n\t\t\treturn;\n{report_code}");
        let tagged = CLEAN.replacen(report_code, &asks, 1);
        assert!(findings_of(CLEAN_NAME, &tagged, RuleId::NetworkAuthority).is_empty());
        let untagged = tagged.replacen("\t//! @authority server\n\t//! @route", "\t//! @route", 1);
        let found = fails(&untagged, RuleId::NetworkAuthority);
        assert!(
            found[0].message.contains("Report"),
            "{helper}: {}",
            found[0].message
        );
    }
}

#[test]
fn boundary_tag_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::BoundaryTags).is_empty());
    let route = fails(
        &edited("\t//! @route POST /api/v1/sample\n", ""),
        RuleId::BoundaryTags,
    );
    assert!(route[0].message.contains("Report"));
    let contract = fails(
        &edited(
            "//! @contract game-runtime-session.schema.json#/definitions/RuntimeHeartbeat",
            "//! Carries no schema citation.",
        ),
        RuleId::BoundaryTags,
    );
    assert!(contract[0].message.contains("TBD_SampleReportStruct"));
    let unnamed = CLEAN.replace("TBD_SampleReportStruct", "TBD_SampleReportPayload");
    let named = fails(&unnamed, RuleId::BoundaryTags);
    assert!(named.iter().any(|f| f.message.contains("*Wire")));
}

#[test]
fn attribute_description_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::AttributeDescription).is_empty());
    let positional = fails(
        &edited("desc: \"Step per event\"", "\"desc: Step per event\""),
        RuleId::AttributeDescription,
    );
    assert!(positional[0].message.contains("[Attribute]"));
    let props = fails(
        &edited("description: \"Sample\"", "\"Sample\""),
        RuleId::AttributeDescription,
    );
    assert!(props[0].message.contains("ComponentEditorProps"));
}

#[test]
fn context_free_prose_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::ContextFreeProse).is_empty());
    let history = fails(
        &edited(
            "//! Asks the server to count one event.",
            concat!(
                "//! Asks the server (T-1092.3, 2026-09-26); previous",
                "ly a Slice TODO."
            ),
        ),
        RuleId::ContextFreeProse,
    );
    let messages: Vec<&str> = history.iter().map(|f| f.message.as_str()).collect();
    let history_word = concat!("`previous", "ly`");
    for expected in ["ticket id", "date", history_word, "`slice`", "`TODO`"] {
        assert!(
            messages.iter().any(|m| m.contains(expected)),
            "{expected} in {messages:?}"
        );
    }
    let separator = fails(
        &edited(
            "//! Sample component",
            "//------------\n//! Sample component",
        ),
        RuleId::ContextFreeProse,
    );
    assert!(separator[0].message.contains("separator"));
    let dead = fails(
        &edited(
            "\t\tRpc(RpcAsk_Count, m_iStep);",
            "\t\t// Rpc(RpcAsk_Count, 1);\n\t\tRpc(RpcAsk_Count, m_iStep);",
        ),
        RuleId::ContextFreeProse,
    );
    assert!(dead[0].message.contains("commented-out"));
    let prose = edited(
        "//! Posts the count to the platform.",
        "//! Posts the count to the platform:\n\t//!   * `kick` {arma_id}: arma_id of 1 to 128 bytes;\n\t//!     (TBD_PlayerIdentity.GetArmaId);",
    );
    assert!(findings_of(CLEAN_NAME, &prose, RuleId::ContextFreeProse).is_empty());
    let reference = fails(
        &edited(
            "//! Posts the count",
            "//! Mirrors TBD_Other.c:42 and posts the count",
        ),
        RuleId::ContextFreeProse,
    );
    assert!(reference[0].message.contains("file:line"));
}

#[test]
fn primary_type_rule_passes_and_fails() {
    assert!(findings_of(CLEAN_NAME, CLEAN, RuleId::FileNamesPrimaryType).is_empty());
    let renamed = findings_of("TBD_Wrong.c", CLEAN, RuleId::FileNamesPrimaryType);
    assert_eq!(renamed.len(), 1);
    assert!(renamed[0].message.contains("TBD_Wrong"));
    let padding = "\tint filler; //!< padding\n".repeat(primary_type_rule::COMPANION_MAX_LINES);
    let big = edited("\tint count; //!< JSON key \"count\"\n", &padding);
    let found = fails(&big, RuleId::FileNamesPrimaryType);
    assert!(found[0].message.contains("TBD_SampleReportStruct"));
}

#[test]
fn string_literals_are_never_comments() {
    assert_eq!(
        strip_c_comments("s = \"a//b\"; // c\nt"),
        "s = \"a//b\"; \nt"
    );
    assert_eq!(strip_c_comments("a /* x\ny */ b"), "a \n b");
    let lines = split_script_lines("x = \"//!< no\"; //!< yes\n");
    assert_eq!(lines[0].comments.len(), 1);
    assert_eq!(lines[0].comments[0].marker, CommentMarker::TrailingDoc);
    assert_eq!(lines[0].comments[0].text, "yes");
    assert_eq!(lines[0].code.trim(), "x = \"\";");
}

#[test]
fn outline_reads_methods_fields_and_attributes() {
    let script = CheckedScript::new(CLEAN_NAME, CLEAN);
    let names = |kind: script_outline::ItemKind| -> Vec<String> {
        script
            .outline
            .items
            .iter()
            .filter(|item| item.kind == kind)
            .map(|item| item.name.clone())
            .collect()
    };
    assert_eq!(
        names(script_outline::ItemKind::Method),
        ["RequestCount", "RpcAsk_Count", "Report"]
    );
    assert_eq!(
        names(script_outline::ItemKind::Field),
        [
            "m_iStep", "m_iCount", "m_sUrl", "m_sMode", "m_aSteps", "count"
        ]
    );
    assert_eq!(
        names(script_outline::ItemKind::EnumMember),
        ["IDLE", "ACTIVE"]
    );
    let report = script
        .outline
        .items
        .iter()
        .find(|item| item.name == "Report")
        .unwrap();
    assert!(report.body_code.contains("TBD_GameRuntimeHttp.Post"));
    assert_eq!(script.item_banner(report).len(), 3);
}

/// A throwaway repository root holding an `apps/mod` tree.
struct TemporaryRepository(PathBuf);

impl TemporaryRepository {
    fn new(label: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("enfusion-comments-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("apps/mod")).unwrap();
        TemporaryRepository(root)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

impl Drop for TemporaryRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(repo: &TemporaryRepository, paths: &[&str]) -> u8 {
    let owned: Vec<String> = paths.iter().map(|path| (*path).to_string()).collect();
    verify_enfusion_comments(&repo.0, &owned)
}

#[test]
fn path_narrows_the_walk() {
    let repo = TemporaryRepository::new("narrow");
    repo.write("apps/mod/clean/TBD_SampleWidget.c", CLEAN);
    repo.write("apps/mod/dirty/TBD_Wrong.c", CLEAN);
    assert_eq!(run(&repo, &["apps/mod/clean"]), 0);
    assert_eq!(run(&repo, &["apps/mod/clean/TBD_SampleWidget.c"]), 0);
    assert_eq!(run(&repo, &["apps/mod/dirty"]), 1);
    assert_eq!(run(&repo, &["apps/mod"]), 1);
    let absolute = repo.0.join("apps/mod/clean");
    assert_eq!(run(&repo, &[absolute.to_str().unwrap()]), 0);
}

#[test]
fn a_path_outside_the_mod_tree_does_not_run() {
    let repo = TemporaryRepository::new("outside");
    repo.write("tools/TBD_SampleWidget.c", CLEAN);
    assert_eq!(run(&repo, &["tools"]), 2);
    assert_eq!(run(&repo, &["apps/mod/../../tools"]), 2);
}

#[test]
fn a_missing_root_does_not_run() {
    let repo = TemporaryRepository::new("missing");
    assert_eq!(run(&repo, &["apps/mod/absent"]), 2);
    assert_eq!(run(&repo, &[]), 2);
}

#[test]
fn an_empty_walk_does_not_run() {
    let repo = TemporaryRepository::new("empty");
    repo.write("apps/mod/notes/readme.txt", "no scripts here");
    assert_eq!(run(&repo, &["apps/mod/notes"]), 2);
}

#[test]
fn pinned_roots_sit_under_the_mod_tree() {
    for root in PINNED_ROOTS {
        assert!(root.starts_with("apps/mod/"), "{root}");
    }
}
