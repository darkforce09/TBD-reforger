/**
 * @file TBD_ZoneRegistry.c
 * @brief The prepared zones of the loaded mission and the in-bounds questions asked of them.
 *
 * Role: builds every `zones[]` row into a `TBD_Zone` once per world and answers which boundary
 * applies to a side, whether a position is inside it, which zone governs a violation and which
 * base-protection zone a player intrudes on.  Position: built by `TBD_PlayAreaComponent` and
 * `TBD_ObjectiveRegistry` from `TBD_MissionLoader.GetZones`; read by the play area, triggers,
 * objectives, the win-condition evaluator and the dynamic spawner.
 * State: static zone list and counts for the life of the script VM, cleared by
 * `TBD_PlayAreaComponent.OnDelete`.  Invariants: "no boundary applies" and "outside the boundary"
 * stay separate verdicts, so a mission without a boundary confines nobody; a player is in bounds
 * inside any one applicable boundary (union).
 */

//! The zone registry of the loaded mission.
//! @authority server
class TBD_ZoneRegistry
{
	static const string CH = "Zones"; //!< log channel

	static const string TYPE_BOUNDARY = "boundary"; //!< `zones[].type` of a play-area boundary
	static const string TYPE_BASE_PROTECTION = "base_protection"; //!< `zones[].type` of a side's protected ground

	protected static ref array<ref TBD_Zone> s_aZones; //!< every prepared zone; null until built
	protected static bool s_bBuilt; //!< true once `Build` succeeded for this world
	protected static int s_iBoundaryCount; //!< usable boundary zones
	protected static int s_iBaseProtectionCount; //!< usable base-protection zones that name a faction

	//! Whether the registry is built for this world.
	//! @return true once `Build` has succeeded since the last `Clear`
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//! How many usable boundary zones the mission has.
	//! @return the count, 0 before `Build`
	static int GetBoundaryCount()
	{
		return s_iBoundaryCount;
	}

	//! How many usable base-protection zones name a faction.
	//! @return the count, 0 before `Build`
	static int GetBaseProtectionCount()
	{
		return s_iBaseProtectionCount;
	}

	//! Every prepared zone, unusable ones included.
	//! @return the zone list, or null until `Build` has run
	static array<ref TBD_Zone> GetAll()
	{
		return s_aZones;
	}

	//! The prepared zone with this id.
	//! @param zoneId a `zones[].id`
	//! @return the first zone carrying `zoneId`, usable or not; null when the registry is unbuilt,
	//! the id is empty or no zone has it
	static TBD_Zone FindById(string zoneId)
	{
		if (!s_aZones || zoneId.IsEmpty())
			return null;

		foreach (TBD_Zone zone : s_aZones)
		{
			if (zone && zone.m_sId == zoneId)
				return zone;
		}

		return null;
	}

	//! Drop every zone and the vehicle-class filters. Called on world teardown, since statics
	//! outlive a world when a mission restarts in-process.
	static void Clear()
	{
		s_aZones = null;
		s_bBuilt = false;
		s_iBoundaryCount = 0;
		s_iBaseProtectionCount = 0;
		TBD_PlayAreaVehicleAxis.Clear();
	}

	//! Prepare every zone in the loaded mission and log one `built` summary; only the first call
	//! after `Clear` does work.
	//! @return false while no mission zones are loaded, so the caller keeps waiting
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		TBD_PlayAreaVehicleAxis.Clear();

		array<ref TBD_MissionZoneStruct> raw = TBD_MissionLoader.GetZones();
		if (!raw)
			return false;

		s_aZones = new array<ref TBD_Zone>();
		s_iBoundaryCount = 0;
		s_iBaseProtectionCount = 0;

		int usable = 0;
		int circles = 0;
		int polygons = 0;

		foreach (int index, TBD_MissionZoneStruct rawZone : raw)
		{
			if (!rawZone)
			{
				TBD_Log.Warn(CH, string.Format("zones[%1] is null -- skipped", index));
				continue;
			}

			TBD_Zone zone = TBD_ZoneCompiler.Prepare(rawZone, index);
			s_aZones.Insert(zone);

			if (zone.IsUsable())
			{
				usable++;
				if (zone.m_eShape == TBD_EZoneShapeKind.CIRCLE)
					circles++;
				else
					polygons++;
			}

			if (zone.m_sType == TYPE_BOUNDARY && zone.IsUsable())
			{
				s_iBoundaryCount++;
			}
			else if (zone.m_sType == TYPE_BASE_PROTECTION && zone.IsUsable())
			{
				if (zone.m_sFaction.IsEmpty())
				{
					// A protection zone with no faction has no side to protect and is never enforced.
					TBD_Log.Warn(CH, string.Format("zone '%1' is base_protection but names no faction -- it protects nobody and is not enforced. Set `faction` to the side whose ground this is.",
						zone.m_sId));
				}
				else
				{
					s_iBaseProtectionCount++;
				}
			}

			if (TBD_ZoneCompiler.EnforcesType(zone.m_sType))
				TBD_ZoneCompiler.LogPrepared(zone);
		}

		s_bBuilt = true;

		// polygon= also shows the typed reader populated the nested vertex arrays.
		TBD_Log.Kv(CH, "built", string.Format("zones=%1 usable=%2 circle=%3 polygon=%4 boundary=%5 baseProtection=%6",
			raw.Count(), usable, circles, polygons, s_iBoundaryCount, s_iBaseProtectionCount));

		return true;
	}


	//! Whether any usable boundary zone applies to this side, asked apart from containment so a
	//! mission with no boundary confines nobody.
	//! @param factionKey the player's side; empty matches only zones that name no faction
	//! @return true when at least one applicable usable boundary exists
	static bool HasBoundaryFor(string factionKey)
	{
		if (!s_aZones)
			return false;

		foreach (TBD_Zone zone : s_aZones)
		{
			if (zone && zone.m_sType == TYPE_BOUNDARY && zone.IsUsable() && AppliesToFaction(zone, factionKey))
				return true;
		}

		return false;
	}

	//! Whether this position is inside at least one boundary zone that applies to `factionKey`,
	//! or its occupant is off the governing zone's vehicle-class axis (the aircraft exemption).
	//! Asked only when `HasBoundaryFor` is true.
	//! @param factionKey the player's side
	//! @param px world X in metres
	//! @param pz world Z in metres
	//! @return true when in bounds; true when the registry is unbuilt
	static bool IsInsideBoundary(string factionKey, float px, float pz)
	{
		if (!s_aZones)
			return true;

		foreach (TBD_Zone zone : s_aZones)
		{
			if (!zone || zone.m_sType != TYPE_BOUNDARY || !zone.IsUsable())
				continue;
			if (!AppliesToFaction(zone, factionKey))
				continue;
			if (zone.Contains(px, pz))
				return true;
		}

		// An occupant off the governing zone's vehicleClasses axis is not in violation; the axis
		// finds the body by the exact XZ the play area sampled.
		TBD_Zone governing = GoverningBoundary(factionKey);
		if (governing && !TBD_PlayAreaVehicleAxis.OccupantConfinedByZone(governing, px, pz))
			return true;

		return false;
	}

	//! The first base-protection zone that names a faction other than the player's and contains
	//! the player, whose occupant class it confines. A player with no side counts as an outsider.
	//! @param factionKey the player's side; may be empty
	//! @param px world X in metres
	//! @param pz world Z in metres
	//! @return the violated zone, or null
	static TBD_Zone FindViolatedProtection(string factionKey, float px, float pz)
	{
		if (!s_aZones)
			return null;

		foreach (TBD_Zone zone : s_aZones)
		{
			if (!zone || zone.m_sType != TYPE_BASE_PROTECTION || !zone.IsUsable())
				continue;
			if (zone.m_sFaction.IsEmpty())
				continue;
			if (zone.m_sFaction == factionKey)
				continue;
			if (zone.Contains(px, pz) && TBD_PlayAreaVehicleAxis.OccupantConfinedByZone(zone, px, pz))
				return zone;
		}

		return null;
	}

	//! The applicable boundary zone whose rules govern a violation: the strictest penalty wins
	//! (KILL over WARN over NONE, by `TBD_EZonePenalty` order), then the shortest grace.
	//! @param factionKey the player's side
	//! @return the governing zone, or null when none applies
	static TBD_Zone GoverningBoundary(string factionKey)
	{
		if (!s_aZones)
			return null;

		TBD_Zone strictest;
		foreach (TBD_Zone zone : s_aZones)
		{
			if (!zone || zone.m_sType != TYPE_BOUNDARY || !zone.IsUsable())
				continue;
			if (!AppliesToFaction(zone, factionKey))
				continue;

			if (!strictest)
			{
				strictest = zone;
				continue;
			}

			if (zone.m_ePenalty > strictest.m_ePenalty)
			{
				strictest = zone;
				continue;
			}

			if (zone.m_ePenalty == strictest.m_ePenalty && zone.m_fGraceSeconds < strictest.m_fGraceSeconds)
				strictest = zone;
		}

		return strictest;
	}

	//! Whether a zone applies to a side: it names no faction, or names that one.
	//! @param zone the zone
	//! @param factionKey the player's side
	//! @return true when the zone applies
	protected static bool AppliesToFaction(notnull TBD_Zone zone, string factionKey)
	{
		if (zone.m_sFaction.IsEmpty())
			return true;

		return zone.m_sFaction == factionKey;
	}
}
