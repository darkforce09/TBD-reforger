/**
 * @file TBD_SessionSelection.c
 * @brief What the player picked in the scenario browser, kept for the screens after it.
 *
 * Role: holds the chosen mission id, title, terrain and version label.  Position: TBD_MissionSelectorScreen sets it on Select Scenario; the lobby and briefing catalogs read the
 * mission id for their titles.
 * State: static, on the client, so it outlives the selector screen.  Invariants: Set with a null
 * mission changes nothing.
 */

//! What the player picked in the scenario browser, for the screens after it (the lobby's mono
//! title). Static because the selector screen is gone by the time the lobby reads it.
class TBD_SessionSelection
{
	static string s_sMissionId; //!< chosen mission id; empty until a pick
	static string s_sTitle; //!< chosen mission title
	static string s_sTerrainKey; //!< chosen mission terrain key
	static string s_sVersionLabel; //!< chosen version label ("v2.14.99"), empty when none

	//! Remember a pick.
	//! @param mission the chosen mission; null changes nothing
	//! @param versionLabel the chosen version label
	static void Set(TBD_MissionSummary mission, string versionLabel)
	{
		if (!mission)
			return;

		s_sMissionId = mission.m_sId;
		s_sTitle = mission.m_sTitle;
		s_sTerrainKey = mission.m_sTerrainKey;
		s_sVersionLabel = versionLabel;
	}
}
