/**
 * @file TBD_ZoneContestResolver.c
 * @brief Which side acts on a capture objective when volume counts are authored.
 *
 * Role: resolves the acting side of a capture from per-side presence counts under
 * `attackerCount`, `defenderCount` and `advantagePercent`, for contestable and weight-of-numbers
 * objectives.  Position: called by `TBD_ZoneVolume.ResolveActingFaction`; reads
 * `TBD_ZoneVolumeBounds`.
 * State: none.  Invariants: a contested or tied objective resolves to no side and is marked
 * contested; with no counts authored the answer equals presence-only capture.
 */

//! Capture acting-side resolution under volume counts.
class TBD_ZoneContestResolver
{
	//! Contestable capture: every side meeting `needAtk` may act and any side meeting `needDef`
	//! contests; exactly one acting side and no contester acts.
	//! @param objective the capture objective with this tick's per-side presence
	//! @param needAtk bodies a side needs to act
	//! @param needDef bodies a side needs to contest; 0 = nobody contests
	//! @return the acting side, or empty (with `m_bContested` set on a contest)
	static string ResolveContestable(notnull TBD_Objective objective, int needAtk, int needDef)
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

	//! Weight-of-numbers capture: the single largest side meeting `needAtk` acts when it also
	//! passes the `advantagePercent` gate over the others; a tie or a failed gate is contested.
	//! @param objective the capture objective with this tick's per-side presence
	//! @param needAtk bodies a side needs to act
	//! @param needDef bodies a non-acting side needs to count against the acting one
	//! @return the acting side, or empty (with `m_bContested` set on a tie or failed gate)
	static string ResolveByWeight(notnull TBD_Objective objective, int needAtk, int needDef)
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

	//! The `advantagePercent` gate: the acting side needs `acting * 100 >= others * (100 +
	//! percent)`. An absent or negative percent, or no others, passes.
	//! @param zoneId the objective's zone
	//! @param actingCount bodies of the acting side
	//! @param othersCount counted bodies of every other side
	//! @return true when the acting side may progress
	protected static bool AdvantageOk(string zoneId, int actingCount, int othersCount)
	{
		TBD_ZoneVolumeBound bound = TBD_ZoneVolumeBounds.Find(zoneId);
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
}
