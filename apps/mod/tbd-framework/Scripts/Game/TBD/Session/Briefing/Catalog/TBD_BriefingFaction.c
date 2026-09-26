/**
 * @file TBD_BriefingFaction.c
 * @brief One side as the Briefing screen names it.
 *
 * Role: key, display name, role chip and tint of one side.  Position: TBD_BriefingMock fills it into TBD_BriefingCatalog; the Briefing pages read it.
 * State: plain data.  Invariants: the tint picks the side's colours, so an enemy page is the
 * friendly builder painted in the enemy tint.
 */

//! One side as the briefing names it.
class TBD_BriefingFaction
{
	string m_sKey;        //!< "BLUFOR"
	string m_sName;       //!< "BLUFOR"
	string m_sRoleLabel;  //!< "DEFENDERS" / "ATTACKERS" -- the header chip
	TBD_EUITint m_eTint; //!< side tint, such as BLUFOR or OPFOR

	//! One side.
	//! @param key the faction key
	//! @param name the display name
	//! @param roleLabel the header chip text
	//! @param tint the side tint
	void TBD_BriefingFaction(string key, string name, string roleLabel, TBD_EUITint tint)
	{
		m_sKey = key;
		m_sName = name;
		m_sRoleLabel = roleLabel;
		m_eTint = tint;
	}
}
