use super::*;

/// Shapes taken verbatim from real CRF/vanilla sources read this session.
const SAMPLE: &str = r#"
class CRF_SlottingManagerClass : ScriptComponentClass {}

class CRF_SlottingManager : ScriptComponent
{
	[Attribute("0", UIWidgets.Hidden), RplProp(onRplName: "OnGamemodeStateChanged")]
	int m_GamemodeState = CRF_EGamemodeState.BRIEFING;

	override void OnPostInit(IEntity owner)
	{
		if (true) { }
	}

	void UpdateSlotPlayerID(int slotId, int playerId = -1)
	{
	}

	protected bool IsValidGroupInSlot(CRF_SlotData slotData)
	{
	}
}

modded class SCR_ChatComponent
{
}

enum CRF_EGamemodeState
{
	BRIEFING,
}
"#;

#[test]
fn finds_real_declarations() {
    let s = scan_str(SAMPLE, "CRF_SlottingManager.c");
    let names: Vec<_> = s.symbols.iter().map(|x| x.name.as_str()).collect();
    assert!(names.contains(&"CRF_SlottingManager"));
    assert!(names.contains(&"CRF_SlottingManagerClass"));
    assert!(names.contains(&"SCR_ChatComponent"));
    assert!(names.contains(&"CRF_EGamemodeState"));
    // The methods that actually exist.
    assert!(names.contains(&"UpdateSlotPlayerID"));
    assert!(names.contains(&"OnPostInit"));
    assert!(names.contains(&"IsValidGroupInSlot"));
}

/// The regression that motivates the whole mechanical index.
#[test]
fn does_not_invent_apis() {
    let s = scan_str(SAMPLE, "CRF_SlottingManager.c");
    let names: Vec<_> = s.symbols.iter().map(|x| x.name.as_str()).collect();
    for hallucinated in ["RequestSlotChange", "ReleaseSlot", "GetInstance"] {
        assert!(
            !names.contains(&hallucinated),
            "scanner invented {hallucinated}"
        );
    }
}

#[test]
fn base_class_is_exact() {
    let s = scan_str(SAMPLE, "x.c");
    let m = s
        .symbols
        .iter()
        .find(|x| x.name == "CRF_SlottingManager")
        .unwrap();
    // NOT SCR_BaseGameModeComponent, which is what the LLM claimed.
    assert_eq!(m.base, "ScriptComponent");
    assert_eq!(m.kind, Kind::Class);
}

#[test]
fn control_flow_is_not_a_method() {
    let s = scan_str(SAMPLE, "x.c");
    assert!(!s.symbols.iter().any(|x| x.name == "if"));
}

#[test]
fn captures_rplprop_with_callback() {
    let s = scan_str(SAMPLE, "x.c");
    let p = s.rpl_props.first().expect("no RplProp captured");
    assert_eq!(p.prop, "m_GamemodeState");
    assert_eq!(p.on_rpl_name, "OnGamemodeStateChanged");
    assert_eq!(p.class, "CRF_SlottingManager");
}

#[test]
fn modded_class_kind_is_distinct() {
    let s = scan_str(SAMPLE, "x.c");
    let m = s
        .symbols
        .iter()
        .find(|x| x.name == "SCR_ChatComponent")
        .unwrap();
    assert_eq!(m.kind, Kind::ModdedClass);
}
