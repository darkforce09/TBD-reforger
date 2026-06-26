//! Maps a mission terrain key (mission JSON meta.terrain) to the generic TBD
//! scenario (.conf) that loads that world. Used to switch worlds when an admin
//! picks a mission on a different terrain.
//!
//! NOTE: each supported terrain needs a generic TBD scenario built on that
//! world (same TBD_GameMode prefab). Only Everon exists today; add Arland etc.
//! in Workbench and register the GUIDs here, then re-publish the mod.
class TBD_ScenarioRouter
{
	//! TBD addon (this mod) GUID — the addon list passed to a scenario change.
	protected const string TBD_ADDON_GUID = "B2C3D4E5F6A78901";

	//------------------------------------------------------------------------------------------------
	//! Scenario resource for a terrain, or empty if no TBD scenario exists for it yet.
	static string GetScenarioForTerrain(string terrain)
	{
		if (terrain == "everon")
			return "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

		// if (terrain == "arland")
		//     return "{<ARLAND_CONF_GUID>}Missions/TBD_Arland.conf";

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	static bool HasScenario(string terrain)
	{
		return !GetScenarioForTerrain(terrain).IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! Addon list string for GameStateTransitions.RequestScenarioChangeTransition.
	static string GetAddonList()
	{
		return TBD_ADDON_GUID;
	}
}
