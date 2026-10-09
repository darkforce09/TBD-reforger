/**
 * @file TBD_MissionFactionNames.c
 * @brief Display names for the mission document's faction keys.
 *
 * Role: maps a `factions[].key` to its authored `displayName`.  Position: called by the briefing
 * and lobby services while they build their payloads; reads a `TBD_MissionDocumentStruct`.
 * State: none.  Invariants: never returns empty for a non-empty key: a faction without a display
 * name, or a key the document does not declare, falls back to the key itself.
 */

//! Faction display-name lookup over one mission document.
class TBD_MissionFactionNames
{
	//! The authored display name of `factionKey`. The value is raw text; wire builders pass it
	//! through `TBD_WireCodec.Sanitise`.
	//! @param doc the mission document; may be null
	//! @param factionKey the faction key to name
	//! @return the first matching faction's non-empty `displayName`, else `factionKey`
	static string DisplayName(TBD_MissionDocumentStruct doc, string factionKey)
	{
		if (doc && doc.factions)
		{
			foreach (TBD_MissionFactionStruct faction : doc.factions)
			{
				if (faction && faction.key == factionKey && !faction.displayName.IsEmpty())
					return faction.displayName;
			}
		}

		return factionKey;
	}
}
