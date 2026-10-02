/**
 * @file TBD_ObjectiveEndConditions.c
 * @brief The three objective end conditions, each answered over the prepared objectives.
 *
 * Role: decides whether every capture is owned by one side, any destroy objective is complete,
 * or any hold objective ran its clock out, and names the winning side.  Position: called by
 * `TBD_ObjectiveRegistry.EvaluateEndTriggers`, which gates each on `winConditions.endOn`.
 * State: none; pure functions over the objectives passed in.  Invariants: inert objectives
 * neither fire nor block a condition; a fresh round (every capture neutral) fires nothing; a
 * split between two owners is a stalemate; these functions never end the round themselves.
 */

//! Objective end-condition predicates.
class TBD_ObjectiveEndConditions
{
	//! `all_objectives_captured`: at least one usable capture objective exists and every one is
	//! owned by the same side.
	//! @param prepared the prepared objectives; may be null
	//! @param winnerFaction set to that side, or empty when the condition is not met
	//! @return true when the condition is met
	static bool AreAllObjectivesCaptured(array<ref TBD_Objective> prepared, out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!prepared)
			return false;

		string owner;
		int considered = 0;

		foreach (TBD_Objective objective : prepared)
		{
			if (!objective || !objective.m_bUsable || objective.m_eKind != TBD_EObjectiveKind.CAPTURE)
				continue;

			considered++;

			if (objective.m_sOwner.IsEmpty())
				return false;

			if (owner.IsEmpty())
			{
				owner = objective.m_sOwner;
				continue;
			}

			if (objective.m_sOwner != owner)
				return false;
		}

		if (considered == 0)
			return false;

		winnerFaction = owner;
		return true;
	}

	//! `objective_destroyed`: any usable destroy objective is complete. The winner is its zone's
	//! `faction`, the side told to destroy it.
	//! @param prepared the prepared objectives; may be null
	//! @param winnerFaction set to that side; empty when the zone names none
	//! @return true when the condition is met
	static bool HasObjectiveBeenDestroyed(array<ref TBD_Objective> prepared, out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!prepared)
			return false;

		foreach (TBD_Objective objective : prepared)
		{
			if (!objective || !objective.m_bUsable || objective.m_eKind != TBD_EObjectiveKind.DESTROY)
				continue;

			if (!objective.m_bComplete)
				continue;

			winnerFaction = objective.m_sFaction;
			return true;
		}

		return false;
	}

	//! `hold_expired`: any usable hold objective ran its clock out. The winner is the holder.
	//! @param prepared the prepared objectives; may be null
	//! @param winnerFaction set to the holding side
	//! @return true when the condition is met
	static bool HasHoldExpired(array<ref TBD_Objective> prepared, out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!prepared)
			return false;

		foreach (TBD_Objective objective : prepared)
		{
			if (!objective || !objective.m_bUsable || objective.m_eKind != TBD_EObjectiveKind.HOLD_UNTIL)
				continue;

			if (!objective.m_bComplete)
				continue;

			winnerFaction = objective.m_sFaction;
			return true;
		}

		return false;
	}
}
