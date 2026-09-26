/**
 * @file TBD_ZoneVolumeBounds.c
 * @brief The objective half of each zone's `zoneRules`: height bounds, counts, starting owner.
 *
 * Role: copies `attackerCount`, `defenderCount`, `advantagePercent`, `minHeight`, `maxHeight` and
 * `startingOwner` off every loaded zone and answers the per-zone counts with their defaults.
 * Position: filled from `TBD_MissionLoader.GetZones` when `TBD_ObjectiveRegistry` builds (through
 * `TBD_ZoneVolume.Read`); read by `TBD_ZoneVolume` and `TBD_ZoneContestResolver`.
 * State: static bound list for the life of the script VM, replaced by each `Read`.
 * Invariants: absent keys keep the loader's ABSENT sentinels; an absent count reads as 1 and an authored 0
 * stays 0; min above max is logged and contains nobody.
 */

//! One zone's volume keys, copied off the loader struct with its ABSENT sentinels kept.
class TBD_ZoneVolumeBound
{
	string zoneId; //!< `zones[].id`
	int attackerCount; //!< `attackerCount`; `TBD_MissionZoneRulesStruct.ABSENT_INT` when absent
	int defenderCount; //!< `defenderCount`; `TBD_MissionZoneRulesStruct.ABSENT_INT` when absent
	float advantagePercent; //!< `advantagePercent`; `TBD_MissionZoneRulesStruct.ABSENT` when absent
	float minHeight; //!< `minHeight` in metres above ground; ABSENT = open below
	float maxHeight; //!< `maxHeight` in metres above ground; ABSENT = open above
	string startingOwner; //!< `startingOwner` faction key; empty when absent
}

//! The per-zone volume bounds of the loaded mission.
class TBD_ZoneVolumeBounds
{
	protected static ref array<ref TBD_ZoneVolumeBound> s_aBounds; //!< one bound per loaded zone; null until `Read`

	//! Drop every bound, so a new mission cannot inherit the previous one's.
	static void Clear()
	{
		s_aBounds = null;
	}

	//! Replace the bounds with the six volume keys of every loaded zone, logging each inverted
	//! height band. Called once per world when the objective registry builds.
	static void Read()
	{
		Clear();
		s_aBounds = new array<ref TBD_ZoneVolumeBound>();

		array<ref TBD_MissionZoneStruct> zones = TBD_MissionLoader.GetZones();
		if (!zones)
			return;

		foreach (TBD_MissionZoneStruct zone : zones)
		{
			if (!zone)
				continue;

			TBD_ZoneVolumeBound bound = FromRules(zone.id, zone.rules);
			if (!bound)
				continue;

			s_aBounds.Insert(bound);
			WarnInverted(bound);
		}
	}

	//! The bound of this zone.
	//! @param zoneId a `zones[].id`
	//! @return the bound, or null when unread, the id is empty or unknown
	static TBD_ZoneVolumeBound Find(string zoneId)
	{
		if (!s_aBounds)
			return null;

		if (zoneId.IsEmpty())
			return null;

		foreach (TBD_ZoneVolumeBound bound : s_aBounds)
		{
			if (bound && bound.zoneId == zoneId)
				return bound;
		}

		return null;
	}

	//! Bodies of the acting side a capture needs in the volume.
	//! @param zoneId the objective's zone
	//! @return `attackerCount`; 1 when absent (anyone present); an authored 0 stays 0 (gate off)
	static int AttackerNeed(string zoneId)
	{
		TBD_ZoneVolumeBound bound = Find(zoneId);
		if (!bound)
			return 1;

		if (bound.attackerCount == TBD_MissionZoneRulesStruct.ABSENT_INT)
			return 1;

		return bound.attackerCount;
	}

	//! Bodies a side needs in the volume to contest a capture, or to hold.
	//! @param zoneId the objective's zone
	//! @return `defenderCount`; 1 when absent; an authored 0 stays 0 (nobody contests or holds)
	static int DefenderNeed(string zoneId)
	{
		TBD_ZoneVolumeBound bound = Find(zoneId);
		if (!bound)
			return 1;

		if (bound.defenderCount == TBD_MissionZoneRulesStruct.ABSENT_INT)
			return 1;

		return bound.defenderCount;
	}

	//! Copy the volume keys of one zone's rules; with no rules every key stays ABSENT.
	//! @param zoneId the zone's id
	//! @param rules the wire rules; may be null
	//! @return a new bound
	protected static TBD_ZoneVolumeBound FromRules(string zoneId, TBD_MissionZoneRulesStruct rules)
	{
		TBD_ZoneVolumeBound bound = new TBD_ZoneVolumeBound();
		bound.zoneId = zoneId;
		bound.attackerCount = TBD_MissionZoneRulesStruct.ABSENT_INT;
		bound.defenderCount = TBD_MissionZoneRulesStruct.ABSENT_INT;
		bound.advantagePercent = TBD_MissionZoneRulesStruct.ABSENT;
		bound.minHeight = TBD_MissionZoneRulesStruct.ABSENT;
		bound.maxHeight = TBD_MissionZoneRulesStruct.ABSENT;

		if (!rules)
			return bound;

		bound.attackerCount = rules.attackerCount;
		bound.defenderCount = rules.defenderCount;
		bound.advantagePercent = rules.advantagePercent;
		bound.minHeight = rules.minHeight;
		bound.maxHeight = rules.maxHeight;
		bound.startingOwner = rules.startingOwner;
		return bound;
	}

	//! Log a zone whose authored `minHeight` is above its `maxHeight`: that volume contains nobody.
	//! @param bound the bound to check; null does nothing
	protected static void WarnInverted(TBD_ZoneVolumeBound bound)
	{
		if (!bound)
			return;

		if (bound.minHeight == TBD_MissionZoneRulesStruct.ABSENT)
			return;

		if (bound.maxHeight == TBD_MissionZoneRulesStruct.ABSENT)
			return;

		if (bound.minHeight <= bound.maxHeight)
			return;

		TBD_Log.Warn(TBD_ZoneVolume.CH, string.Format("zone '%1' rules.minHeight=%2 is above maxHeight=%3 -- the volume contains nobody",
			bound.zoneId, bound.minHeight, bound.maxHeight));
	}

	//! Whether any volume key is authored on this bound.
	//! @param bound the bound
	//! @return true when a count, advantage, height or starting owner is present
	static bool HasAny(TBD_ZoneVolumeBound bound)
	{
		if (bound.attackerCount != TBD_MissionZoneRulesStruct.ABSENT_INT)
			return true;
		if (bound.defenderCount != TBD_MissionZoneRulesStruct.ABSENT_INT)
			return true;
		if (bound.advantagePercent != TBD_MissionZoneRulesStruct.ABSENT)
			return true;
		if (bound.minHeight != TBD_MissionZoneRulesStruct.ABSENT)
			return true;
		if (bound.maxHeight != TBD_MissionZoneRulesStruct.ABSENT)
			return true;
		if (!bound.startingOwner.IsEmpty())
			return true;
		return false;
	}
}
