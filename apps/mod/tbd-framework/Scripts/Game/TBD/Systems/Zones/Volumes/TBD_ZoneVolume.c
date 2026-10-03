/**
 * @file TBD_ZoneVolume.c
 * @brief The objective system's questions about a zone volume: height, capture, hold, owner.
 *
 * Role: answers whether a body is inside a zone's height band, which side acts on a capture,
 * whether an enemy contests or the holder holds, and hands `startingOwner` to the objective's kind
 * behaviour.  Position: called by `TBD_ObjectiveRegistry`, `TBD_ObjectivesComponent`,
 * `TBD_EntityQuery` and the capture and hold behaviours on the server; reads
 * `TBD_ZoneVolumeBounds`, resolves captures through `TBD_ZoneContestResolver`, and applies
 * `startingOwner` through `TBD_ObjectiveKindBehaviour.ApplyStartingOwner`.
 * State: none of its own; the bounds live in `TBD_ZoneVolumeBounds`.  Invariants: height is
 * above ground at the body's own XZ, not sea level; absent bounds and counts leave presence-only
 * behaviour unchanged.
 */

//! Zone volume queries for the objective system.
//! @authority server
class TBD_ZoneVolume
{
	static const string CH = "ZoneVol"; //!< log channel

	//! Hand the zone's non-empty `startingOwner` to the objective's kind behaviour: a capture
	//! objective starts HELD by it when that is a declared faction the objective may be owned by,
	//! a hold-until objective logs and ignores one other than the holder, and every other kind
	//! ignores it. An absent bound or an empty `startingOwner` changes nothing.
	//! @param objective the objective being prepared
	static void ApplyStartingOwner(notnull TBD_Objective objective)
	{
		TBD_ZoneVolumeBound bound = TBD_ZoneVolumeBounds.Find(objective.m_sId);
		if (!bound)
			return;

		if (bound.startingOwner.IsEmpty())
			return;

		TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.For(objective.m_eKind);
		behaviour.ApplyStartingOwner(objective, bound.startingOwner);
	}

	//! Whether `origin` is inside the zone's height band: height above the ground at its own XZ,
	//! checked after the XZ footprint accepted it. An absent bound is open.
	//! @param zoneId the zone
	//! @param origin the body's world position
	//! @return true inside the band, with no bounds, or with no bound record; false with no world
	static bool ContainsAgl(string zoneId, vector origin)
	{
		TBD_ZoneVolumeBound bound = TBD_ZoneVolumeBounds.Find(zoneId);
		if (!bound)
			return true;

		bool hasMin = bound.minHeight != TBD_MissionZoneRulesStruct.ABSENT;
		bool hasMax = bound.maxHeight != TBD_MissionZoneRulesStruct.ABSENT;
		if (!hasMin && !hasMax)
			return true;

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return false;

		float surfaceY = world.GetSurfaceY(origin[0], origin[2]);
		float agl = origin[1] - surfaceY;

		if (hasMin)
		{
			if (agl < bound.minHeight)
				return false;
		}

		if (hasMax)
		{
			if (agl > bound.maxHeight)
				return false;
		}

		return true;
	}

	//! Whether `origin` is inside the zone's XZ shape and its height band.
	//! @param zone the zone; null returns false
	//! @param origin the world position
	//! @return true when inside both
	static bool ContainsOrigin(TBD_Zone zone, vector origin)
	{
		if (!zone)
			return false;

		if (!zone.Contains(origin[0], origin[2]))
			return false;

		return ContainsAgl(zone.m_sId, origin);
	}

	//! The side acting on a capture objective this tick, applying the zone's counts and advantage;
	//! with no volume keys authored one side inside acts, two or more freeze a contestable objective,
	//! and a non-contestable one goes to the larger side with ties frozen. Clears and may set
	//! `m_bContested`.
	//! @param objective the capture objective with this tick's presence
	//! @return the acting faction key, or empty when nobody or a contest
	static string ResolveActingFaction(notnull TBD_Objective objective)
	{
		int sides = objective.PresentFactionCount();
		objective.m_bContested = false;

		if (sides == 0)
			return string.Empty;

		int needAtk = TBD_ZoneVolumeBounds.AttackerNeed(objective.m_sId);
		int needDef = TBD_ZoneVolumeBounds.DefenderNeed(objective.m_sId);

		if (objective.m_bContestable)
			return TBD_ZoneContestResolver.ResolveContestable(objective, needAtk, needDef);

		return TBD_ZoneContestResolver.ResolveByWeight(objective, needAtk, needDef);
	}

	//! Whether an enemy side contests a hold: some non-holder side meets `defenderCount`
	//! (absent = 1; authored 0 = nobody contests).
	//! @param objective the hold objective with this tick's presence
	//! @return true when contested
	static bool EnemyContestsHold(notnull TBD_Objective objective)
	{
		int needDef = TBD_ZoneVolumeBounds.DefenderNeed(objective.m_sId);
		if (needDef <= 0)
			return false;

		if (!objective.m_aPresentFactions)
			return false;

		foreach (int index, string present : objective.m_aPresentFactions)
		{
			if (present == objective.m_sFaction)
				continue;

			if (objective.m_aPresentCounts[index] >= needDef)
				return true;
		}

		return false;
	}

	//! Whether the holding side counts as present: it meets `defenderCount` (absent = 1; authored
	//! 0 = always).
	//! @param objective the hold objective with this tick's presence
	//! @return true when the holder holds
	static bool HolderPresent(notnull TBD_Objective objective)
	{
		int needDef = TBD_ZoneVolumeBounds.DefenderNeed(objective.m_sId);
		if (needDef <= 0)
			return true;

		return objective.PresenceOf(objective.m_sFaction) >= needDef;
	}

	//! Log a `volume` line for an objective whose zone authors any volume key.
	//! @param objective the prepared objective
	static void LogBound(notnull TBD_Objective objective)
	{
		TBD_ZoneVolumeBound bound = TBD_ZoneVolumeBounds.Find(objective.m_sId);
		if (!bound)
			return;

		if (!TBD_ZoneVolumeBounds.HasAny(bound))
			return;

		TBD_Log.Kv(CH, "volume", string.Format("id=%1 atk=%2 def=%3 adv=%4 minH=%5 maxH=%6 owner='%7'",
			objective.m_sId,
			bound.attackerCount,
			bound.defenderCount,
			bound.advantagePercent,
			bound.minHeight,
			bound.maxHeight,
			bound.startingOwner));
	}

}
