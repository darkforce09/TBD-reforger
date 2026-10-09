/**
 * @file TBD_ObjectiveInfo.c
 * @brief One objective card of the Objectives page.
 *
 * Role: number, title, role pill, stats and map position of one objective.  Position: TBD_BriefingMock fills it into TBD_BriefingCatalog; the Briefing pages read it.
 * State: plain data.  Invariants: `m_fX` and `m_fZ` are world metres; the Locate button pans
 * the map to them.
 */

//! One objective card.
class TBD_ObjectiveInfo
{
	int m_iIndex; //!< the card number
	string m_sTitle;        //!< "Southern Zone"
	string m_sRoleLabel;    //!< "Defend" / "Capture" -- the pill
	string m_sType;         //!< "Sector"
	string m_sCaptureTime;  //!< "60s"
	string m_sRetake;       //!< "Permanent (Locked)"
	float m_fX;             //!< world X for Locate (map pan)
	float m_fZ; //!< world Z for Locate, metres

	//! One objective card; the parameters fill the fields of the same names.
	void TBD_ObjectiveInfo(int index, string title, string roleLabel, string type, string captureTime, string retake, float x, float z)
	{
		m_iIndex = index;
		m_sTitle = title;
		m_sRoleLabel = roleLabel;
		m_sType = type;
		m_sCaptureTime = captureTime;
		m_sRetake = retake;
		m_fX = x;
		m_fZ = z;
	}
}
