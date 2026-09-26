/**
 * @file TBD_MissionOrbatQuery.c
 * @brief Answers ORBAT questions about the loaded mission: a squad by callsign and its leader.
 *
 * Role: squad and squad-leader lookups over the validated document.  Position: reads
 * `TBD_MissionLoader.GetMission` and `GetSlotById`; called by `TBD_MissionLoader.IsSquadLeader`
 * (for `TBD_SpawnManager`) and any system that needs a squad's group row.
 * State: none.  Invariants: answers null or false unless a valid mission is loaded; a leader is
 * returned only when `leaderSlotId` resolves to a seat of the same faction and squad, and no
 * fallback leader is ever chosen, so "the author named a leader" stays distinguishable from a guess.
 */

//! Static ORBAT lookups over the loaded mission.
class TBD_MissionOrbatQuery
{
	//! The ORBAT group a squad callsign names. A callsign is unique only within one faction.
	//! @param factionKey the faction to search; empty searches every faction and the first match wins
	//! @param groupCallsign the squad callsign
	//! @return the group, or null when none matches or no valid mission is loaded
	static TBD_MissionOrbatGroupStruct GetOrbatGroup(string factionKey, string groupCallsign)
	{
		TBD_MissionDocumentStruct mission = TBD_MissionLoader.GetMission();
		if (!TBD_MissionLoader.IsValid() || !mission || !mission.orbat || groupCallsign.IsEmpty())
			return null;

		foreach (string key, TBD_MissionOrbatFactionStruct faction : mission.orbat)
		{
			if (!faction || !faction.groups)
				continue;
			if (!factionKey.IsEmpty() && key != factionKey)
				continue;

			foreach (TBD_MissionOrbatGroupStruct group : faction.groups)
			{
				if (group && group.callsign == groupCallsign)
					return group;
			}
		}

		return null;
	}

	//! The slot that leads `slot`'s squad. `leaderSlotId` carries a slot uid, so it resolves
	//! through the uid-aware `TBD_MissionLoader.GetSlotById`. A dangling id (possible in a
	//! hand-edited profile file) and a seat in another squad (uids are unique document-wide, so a
	//! copied id resolves) both read as no leader.
	//! @param slot the seat whose squad is asked about; null returns null
	//! @return the leading slot, or null when the mission names no valid leader for that squad
	static TBD_MissionSlotStruct GetSquadLeaderSlot(TBD_MissionSlotStruct slot)
	{
		if (!slot)
			return null;

		TBD_MissionOrbatGroupStruct group = GetOrbatGroup(slot.faction, slot.groupCallsign);
		if (!group || group.leaderSlotId.IsEmpty())
			return null;

		TBD_MissionSlotStruct leader = TBD_MissionLoader.GetSlotById(group.leaderSlotId);
		if (!leader)
			return null;

		// A cross-squad reference is not this squad's leader.
		if (leader.faction != slot.faction || leader.groupCallsign != slot.groupCallsign)
			return null;

		return leader;
	}

	//! Whether `slot` leads its own squad. Compares `Key()` (uid when present, derived id
	//! otherwise), the identity bodies, seat leases and rosters are keyed on.
	//! @param slot the seat; null returns false
	//! @return true when `slot` is its squad's authored leader
	static bool IsSquadLeader(TBD_MissionSlotStruct slot)
	{
		TBD_MissionSlotStruct leader = GetSquadLeaderSlot(slot);
		if (!leader)
			return false;

		return leader.Key() == slot.Key();
	}
}
