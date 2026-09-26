/**
 * @file TBD_ParamInfo.c
 * @brief One row of the Parameters page.
 *
 * Role: icon, label, value and value tint of one mission parameter.  Position: TBD_BriefingMock fills it into TBD_BriefingCatalog; the Briefing pages read it.
 * State: plain data.  Invariants: the value renders in a mono face; the tint defaults to NEUTRAL.
 */

//! One parameters row: icon - label - mono value.
class TBD_ParamInfo
{
	string m_sIcon;   //!< TBD_UIIcons key
	string m_sLabel; //!< row label
	string m_sValue; //!< row value
	TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL; //!< WARNING for the amber Safe Start value

	//! One parameter row; the parameters fill the fields of the same names.
	void TBD_ParamInfo(string icon, string label, string value, TBD_EUITint tint = TBD_EUITint.NEUTRAL)
	{
		m_sIcon = icon;
		m_sLabel = label;
		m_sValue = value;
		m_eTint = tint;
	}
}
