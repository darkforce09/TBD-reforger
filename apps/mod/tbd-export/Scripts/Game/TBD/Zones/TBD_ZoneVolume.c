//! T-685 - zone volumes: height bounds, capture counts, starting owner.
//!
//! == What was missing ========================================================================
//! T-706 put `attackerCount` / `defenderCount` / `advantagePercent` / `minHeight` / `maxHeight` /
//! `startingOwner` on `$defs/zoneRules`. `TBD_MissionZoneRulesStruct` declared only the play-area
//! three (`graceSeconds`, `warnEverySeconds`, `penalty`), so JsonLoadContext could not see the
//! six. Zones stayed XZ footprints; capture stayed 1-vs-1 presence; ownership started neutral.
//! This file consumes the six keys the loader now binds.
//!
//! == WOG caveat (INFERRED, not copied into behaviour) ========================================
//! Parameter NAMES and observed VALUES on WMT_Task_Point are evidence. The sentence that reads
//! those as "captured when CaptureCount attackers are inside between MinHeight and MaxHeight
//! while fewer than DefCount defenders contest" is marked INFERRED in wog.md: the addon that
//! implements WMT_* is absent from the corpus. TBD chooses the rules below; do not treat the
//! inferred sentence as an acceptance criterion.
//!
//! == TBD rules (chosen here) =================================================================
//!   Volume: an entity is inside the zone VOLUME when the existing XZ footprint contains it AND
//!           its AGL at its OWN ground position is inside each authored bound. AGL =
//!           origin.y - World.GetSurfaceY(x, z). Not ASL, not anchored at the zone centre: a
//!           slope makes those diverge, which is why the bounds exist. Absent bound = that
//!           side open. Authored min > max => empty volume (fail closed) and a load warning.
//!   attackerCount: living bodies of the acting side that must be in the volume to bank or
//!           neutralize. Absent => 1 (today's "anyone present"). Authored 0 => the count gate
//!           does not block (presence is still required to be the acting side).
//!   defenderCount: living bodies of an opposing side that must be in the volume to CONTEST
//!           a capture, or of the holding side to COUNT as holding. Absent => 1. Authored 0 =>
//!           nobody contests / nobody is required to hold (undefended).
//!   advantagePercent: extra capture gate, only when contestable is false. Acting side may
//!           progress only if acting * 100 >= others * (100 + percent). Absent => off. 0 means
//!           need at least as many as the others combined. contestable:true still freezes when
//!           a qualifying defender is inside -- that existing key is not reinterpreted.
//!   startingOwner: capture objectives start HELD by that factionKey (progress full). Unknown
//!           key, or a key the zone's `faction` restriction forbids, is warned and ignored.
//!
//! Existing zoneRules keys (contestable, neutralizeSeconds, onEmpty, decayRate, pauseOnEnemy,
//! resetOnEnemy, requireHolderPresent) keep their meaning. This file adds gates, it does not
//! replace those branches.
//!
//! == Presence, and the nested-ref landmine ===================================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` even when the JSON key is
//! ABSENT, so `if (zone.rules)` is always true. Presence is the scalar sentinel on the loader
//! struct (`ABSENT` / `ABSENT_INT`). Counts and heights can be authored 0 / -5, so the
//! sentinel must not be 0. startingOwner uses the empty string.
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile`. Flatten does not emit these keys (T-946.36). A
//! hand-staged schemaVersion 1.3 document reaches the reader; live `/compiled` volume checks
//! belong on the human checklist (a zone with maxHeight 30 ignores aircraft above it).
//! @contract mission.schema.json#/$defs/zoneRules

//------------------------------------------------------------------------------------------------
//! One zone's T-685 bound set, copied off the loader struct after sentinels are resolved.
class TBD_ZoneVolumeBound
{
	string zoneId;
	int attackerCount;
	int defenderCount;
	float advantagePercent;
	float minHeight;
	float maxHeight;
	string startingOwner;
}

//------------------------------------------------------------------------------------------------
//! Server-side volume + count + owner consumer. Binds from TBD_MissionLoader.GetZones().
class TBD_ZoneVolume
{
	static const string CH = "ZoneVol";
	protected static ref array<ref TBD_ZoneVolumeBound> s_aBounds;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		s_aBounds = null;
	}

	//------------------------------------------------------------------------------------------------
	//! Copy the six keys off every loaded zone. Idempotent for a world: Registry.Build calls
	//! this once after the loader has parsed. Statics outlive a world -- Registry.Clear calls
	//! Clear() so mission B cannot inherit mission A's bounds.
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

	//------------------------------------------------------------------------------------------------
	//! Capture objectives whose startingOwner is a known, allowed faction start HELD.
	static void ApplyStartingOwner(notnull TBD_Objective objective)
	{
		TBD_ZoneVolumeBound bound = Find(objective.m_sId);
		if (!bound)
			return;

		if (bound.startingOwner.IsEmpty())
			return;

		if (objective.m_eKind == TBD_EObjectiveKind.HOLD_UNTIL)
		{
			if (bound.startingOwner != objective.m_sFaction)
			{
				TBD_Log.Warn(CH, string.Format("objective '%1' rules.startingOwner='%2' is ignored on objective_hold_until (holder is zones[].faction='%3')",
					objective.m_sId, bound.startingOwner, objective.m_sFaction));
			}
			return;
		}

		if (objective.m_eKind != TBD_EObjectiveKind.CAPTURE)
			return;

		if (!FactionExists(bound.startingOwner))
		{
			TBD_Log.Warn(CH, string.Format("objective '%1' rules.startingOwner='%2' names no factions[].key -- leaving the objective NEUTRAL",
				objective.m_sId, bound.startingOwner));
			return;
		}

		if (!objective.MayOwn(bound.startingOwner))
		{
			TBD_Log.Warn(CH, string.Format("objective '%1' rules.startingOwner='%2' is excluded by zones[].faction='%3' -- leaving the objective NEUTRAL",
				objective.m_sId, bound.startingOwner, objective.m_sFaction));
			return;
		}

		objective.m_sOwner = bound.startingOwner;
		objective.m_sProgressFaction = bound.startingOwner;
		objective.m_fProgress = objective.m_fCaptureSeconds;

		TBD_Log.Kv(CH, "startingOwner", string.Format("id=%1 owner=%2",
			objective.m_sId, bound.startingOwner));
	}

	//------------------------------------------------------------------------------------------------
	//! Height gate after the XZ footprint has already accepted the body. Absent bounds pass.
	static bool ContainsAgl(string zoneId, vector origin)
	{
		TBD_ZoneVolumeBound bound = Find(zoneId);
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

	//------------------------------------------------------------------------------------------------
	//! XZ + AGL together. Used by the destroy-target query, which only had XZ before T-685.
	static bool ContainsOrigin(TBD_Zone zone, vector origin)
	{
		if (!zone)
			return false;

		if (!zone.Contains(origin[0], origin[2]))
			return false;

		return ContainsAgl(zone.m_sId, origin);
	}

	//------------------------------------------------------------------------------------------------
	//! Capture acting-side resolution. Preserves contestable / weight-of-numbers, then applies
	//! attackerCount / defenderCount / advantagePercent. No volume authored => identical to
	//! TBD_Objective.ResolveActingFaction.
	static string ResolveActingFaction(notnull TBD_Objective objective)
	{
		int sides = objective.PresentFactionCount();
		objective.m_bContested = false;

		if (sides == 0)
			return string.Empty;

		int needAtk = AttackerNeed(objective.m_sId);
		int needDef = DefenderNeed(objective.m_sId);

		if (objective.m_bContestable)
			return ResolveContestable(objective, needAtk, needDef);

		return ResolveByWeight(objective, needAtk, needDef);
	}

	//------------------------------------------------------------------------------------------------
	//! Hold: an enemy contests only when they meet defenderCount (absent => 1, matching today).
	static bool EnemyContestsHold(notnull TBD_Objective objective)
	{
		int needDef = DefenderNeed(objective.m_sId);
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

	//------------------------------------------------------------------------------------------------
	//! Hold: the holding side counts as present when they meet defenderCount (defenders needed
	//! to hold). Authored 0 => always true. Absent => PresenceOf > 0.
	static bool HolderPresent(notnull TBD_Objective objective)
	{
		int needDef = DefenderNeed(objective.m_sId);
		if (needDef <= 0)
			return true;

		return objective.PresenceOf(objective.m_sFaction) >= needDef;
	}

	//------------------------------------------------------------------------------------------------
	static void LogBound(notnull TBD_Objective objective)
	{
		TBD_ZoneVolumeBound bound = Find(objective.m_sId);
		if (!bound)
			return;

		if (!HasAny(bound))
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

		TBD_Log.Warn(CH, string.Format("zone '%1' rules.minHeight=%2 is above maxHeight=%3 -- the volume contains nobody",
			bound.zoneId, bound.minHeight, bound.maxHeight));
	}

	//------------------------------------------------------------------------------------------------
	protected static bool HasAny(TBD_ZoneVolumeBound bound)
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

	//------------------------------------------------------------------------------------------------
	protected static TBD_ZoneVolumeBound Find(string zoneId)
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

	//------------------------------------------------------------------------------------------------
	//! Absent => 1 (today's anyone-present). Authored 0 stays 0 (count gate off).
	protected static int AttackerNeed(string zoneId)
	{
		TBD_ZoneVolumeBound bound = Find(zoneId);
		if (!bound)
			return 1;

		if (bound.attackerCount == TBD_MissionZoneRulesStruct.ABSENT_INT)
			return 1;

		return bound.attackerCount;
	}

	//------------------------------------------------------------------------------------------------
	//! Absent => 1. Authored 0 stays 0 (undefended / nobody contests).
	protected static int DefenderNeed(string zoneId)
	{
		TBD_ZoneVolumeBound bound = Find(zoneId);
		if (!bound)
			return 1;

		if (bound.defenderCount == TBD_MissionZoneRulesStruct.ABSENT_INT)
			return 1;

		return bound.defenderCount;
	}

	//------------------------------------------------------------------------------------------------
	protected static string ResolveContestable(notnull TBD_Objective objective, int needAtk, int needDef)
	{
		int actingSides = 0;
		string acting = string.Empty;
		bool contested = false;

		foreach (int index, string present : objective.m_aPresentFactions)
		{
			int count = objective.m_aPresentCounts[index];
			bool canAct = false;
			if (count >= needAtk)
				canAct = true;

			bool canContest = false;
			if (needDef > 0)
			{
				if (count >= needDef)
					canContest = true;
			}

			if (canAct)
			{
				actingSides = actingSides + 1;
				if (actingSides == 1)
				{
					acting = present;
				}
				else
				{
					contested = true;
				}
			}

			if (!canAct && canContest)
				contested = true;
		}

		if (contested)
		{
			objective.m_bContested = true;
			return string.Empty;
		}

		if (actingSides == 0)
			return string.Empty;

		return acting;
	}

	//------------------------------------------------------------------------------------------------
	protected static string ResolveByWeight(notnull TBD_Objective objective, int needAtk, int needDef)
	{
		int best = -1;
		int bestCount = 0;
		bool tied = false;
		int others = 0;

		foreach (int index, int count : objective.m_aPresentCounts)
		{
			bool eligible = false;
			if (count >= needAtk)
				eligible = true;

			if (!eligible)
			{
				if (needDef > 0)
				{
					if (count >= needDef)
						others = others + count;
				}
				continue;
			}

			if (count > bestCount)
			{
				if (best != -1)
					others = others + bestCount;

				bestCount = count;
				best = index;
				tied = false;
				continue;
			}

			if (count == bestCount)
			{
				tied = true;
				others = others + count;
				continue;
			}

			others = others + count;
		}

		if (best == -1 || tied)
		{
			objective.m_bContested = true;
			return string.Empty;
		}

		if (!AdvantageOk(objective.m_sId, bestCount, others))
		{
			objective.m_bContested = true;
			return string.Empty;
		}

		return objective.m_aPresentFactions[best];
	}

	//------------------------------------------------------------------------------------------------
	//! contestable:false extra gate. Absent percent always passes. others==0 always passes.
	protected static bool AdvantageOk(string zoneId, int actingCount, int othersCount)
	{
		TBD_ZoneVolumeBound bound = Find(zoneId);
		if (!bound)
			return true;

		if (bound.advantagePercent == TBD_MissionZoneRulesStruct.ABSENT)
			return true;

		if (bound.advantagePercent < 0)
			return true;

		if (othersCount <= 0)
			return true;

		float need = othersCount * (100.0 + bound.advantagePercent);
		float have = actingCount * 100.0;
		if (have >= need)
			return true;

		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool FactionExists(string key)
	{
		if (key.IsEmpty())
			return false;

		array<ref TBD_MissionFactionStruct> factions = TBD_MissionLoader.GetFactions();
		if (!factions)
			return false;

		foreach (TBD_MissionFactionStruct faction : factions)
		{
			if (!faction)
				continue;

			if (faction.key == key)
				return true;
		}

		return false;
	}
}
