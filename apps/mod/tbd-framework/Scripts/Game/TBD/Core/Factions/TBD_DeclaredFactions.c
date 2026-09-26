/**
 * @file TBD_DeclaredFactions.c
 * @brief Checks a faction key against the loaded mission's declared `factions[]`.
 *
 * Role: membership test for authored faction keys.  Position: called by zone volumes and the
 * objective registry while they validate authored rows; reads `TBD_MissionLoader.GetFactions`.
 * State: none.  Invariants: an empty key, or no loaded faction list, is never declared.
 */

//! Faction-key membership over the loaded mission document.
class TBD_DeclaredFactions
{
	//! Whether `key` is one of the loaded mission's `factions[].key`.
	//! @param key the faction key to look up
	//! @return true when a non-null declared faction carries exactly `key`; false for an empty key
	//! or when no mission factions are loaded
	static bool Exists(string key)
	{
		if (key.IsEmpty())
			return false;

		array<ref TBD_MissionFactionStruct> factions = TBD_MissionLoader.GetFactions();
		if (!factions)
			return false;

		foreach (TBD_MissionFactionStruct faction : factions)
		{
			if (faction && faction.key == key)
				return true;
		}

		return false;
	}
}
