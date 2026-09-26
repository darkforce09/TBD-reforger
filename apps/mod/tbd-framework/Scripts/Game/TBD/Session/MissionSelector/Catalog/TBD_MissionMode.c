/**
 * @file TBD_MissionMode.c
 * @brief One mode tag a mission carries and the scenario browser filters on.
 *
 * Role: pairs a mode key with its label and chip tint.  Position: TBD_MissionSelectorMock builds
 * them into TBD_MissionCatalog; TBD_ScenarioBrowserPanel lists them in its Modes filter and the
 * cards and inspector tint their tag chips from them.
 * State: none beyond its fields.  Invariants: m_sKey matches TBD_MissionSummary.m_sTag.
 */

//! One mode / tag a mission carries and the browser filters on (COOP, PvP, Warlords, RHS, Zeus).
class TBD_MissionMode
{
	string m_sKey;           //!< "PVP" -- matches TBD_MissionSummary.m_sTag
	string m_sLabel;         //!< "PvP"
	TBD_EUITint m_eTint;     //!< chip colour on the card

	//! Build a mode entry.
	void TBD_MissionMode(string key, string label, TBD_EUITint tint)
	{
		m_sKey = key;
		m_sLabel = label;
		m_eTint = tint;
	}
}
