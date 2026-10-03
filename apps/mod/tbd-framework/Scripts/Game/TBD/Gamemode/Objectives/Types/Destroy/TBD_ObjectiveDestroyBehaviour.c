/**
 * @file TBD_ObjectiveDestroyBehaviour.c
 * @brief The `objective_destroy` behaviour: targets inside a zone a side must destroy.
 *
 * Role: answers every hook of the destroy kind: its zone type, end trigger and task type, its
 * rules, load log, the `objective_destroyed` condition, the target search on the LIVE edge, the
 * per-tick completion, its status text and HUD glyph.  Position: found through
 * `TBD_ObjectiveKindBehaviour.For` and its sibling lookups; the target search and count live in
 * `TBD_ObjectiveDestroyTargets` beside it; logs on the registry's `Obj` channel.
 * State: none; every objective's state lives on `TBD_Objective`.  Invariants: a destroy objective
 * without `targetAlias` is inert; the targets are searched once, on the first LIVE tick, because a
 * target placed by another subsystem may not exist before; the winner is the zone's `faction`,
 * the side told to destroy it, and may be empty; a `startingOwner` is ignored silently.
 */

//! Destroy objective behaviour.
class TBD_ObjectiveDestroyBehaviour : TBD_ObjectiveKindBehaviour
{
	static const string ZONE_TYPE   = "objective_destroy"; //!< `zones[].type` of a destroy objective
	static const string END_TRIGGER = "objective_destroyed"; //!< `winConditions.endOn`: any usable destroy objective completed
	static const string TASK_DESTROY = "destroy"; //!< `objectives[].type` value

	//! @return DESTROY
	override TBD_EObjectiveKind Kind() { return TBD_EObjectiveKind.DESTROY; }

	//! @return `objective_destroy`
	override string ZoneType() { return ZONE_TYPE; }

	//! @return `objective_destroyed`
	override string EndTrigger() { return END_TRIGGER; }

	//! `destroy` belongs on a destroy zone.
	//! @param taskType an `objectives[].type` value
	//! @return true for `destroy`
	override bool AcceptsTaskType(string taskType)
	{
		return taskType == TASK_DESTROY;
	}

	//! @return `destroy`
	override string CountLabel() { return "destroy"; }

	//! The note that the destroy objectives are tracked and end nothing.
	//! @param usableCount how many objectives of this kind are usable; above 0
	//! @return the note text
	override string UndeclaredTriggerNote(int usableCount)
	{
		return string.Format("%1 destroy objective(s) but endOn does not declare '%2' -- tracked, will not end the round", usableCount, END_TRIGGER);
	}

	//! `objective_destroy`: requires `targetAlias`, else inert; takes `targetCount`. Finding the
	//! targets waits for LIVE (`TBD_ObjectiveDestroyTargets.ArmDestroyTargets`).
	//! @param objective the objective being prepared
	//! @param rules the zone's rules from the rules pass; null makes the objective inert
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	override void ResolveRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		if (!rules || rules.targetAlias.IsEmpty())
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = "objective_destroy has no readable rules.targetAlias, so there is nothing to watch. Author `rules.targetAlias` with a registry alias (e.g. \"comp:ammo_cache\").";
			return;
		}

		objective.m_sTargetAlias = rules.targetAlias;

		if (rules.targetCount != TBD_ObjectiveRulesStruct.ABSENT_INT)
		{
			if (rules.targetCount < 0)
			{
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.targetCount=%2 is negative -- using 0 (destroy everything found)",
					subject, rules.targetCount));
			}
			else
			{
				objective.m_iTargetCount = rules.targetCount;
			}
		}

		if (objective.m_sFaction.IsEmpty())
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' (%2) names no `faction` -- if it completes, '%3' will fire with no winning side named. Set `faction` to the side that must destroy it.",
				subject, ZONE_TYPE, END_TRIGGER));
	}

	//! Log the target alias, the target count and `points`.
	//! @param objective a usable prepared objective
	override void LogRules(notnull TBD_Objective objective)
	{
		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "objectiveRules", string.Format("id=%1 targetAlias='%2' targetCount=%3 points=%4",
			objective.m_sId,
			objective.m_sTargetAlias,
			objective.m_iTargetCount,
			objective.m_fPoints));
	}

	//! `objective_destroyed`: any usable destroy objective is complete. The winner is its zone's
	//! `faction`, the side told to destroy it.
	//! @param prepared the prepared objectives
	//! @param winnerFaction set to that side; empty when the zone names none
	//! @return true when the condition is met
	//! @authority server
	override bool HasEnded(notnull array<ref TBD_Objective> prepared, out string winnerFaction)
	{
		winnerFaction = string.Empty;

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

	//! Arm the target search once: a search at load could run before
	//! `TBD_MissionWorldApplier.SpawnMissionEntities` and other subsystems place the targets.
	//! @param objective a usable destroy objective
	//! @authority server
	override void OnEnterLive(notnull TBD_Objective objective)
	{
		if (!objective.m_bArmed)
			TBD_ObjectiveDestroyTargets.ArmDestroyTargets(objective);
	}

	//! Evaluate the targets and hand back the DESTROYED line on the tick the objective completes.
	//! The counting lives in `TBD_ObjectiveDestroyTargets.EvaluateDestroy`.
	//! @param objective a usable destroy objective
	//! @return the DESTROYED chat line on the completing tick, else empty
	//! @authority server
	override string Advance(notnull TBD_Objective objective)
	{
		if (!TBD_ObjectiveDestroyTargets.EvaluateDestroy(objective))
			return "";

		string msg = "TBD: ";
		msg += objective.DisplayName();
		msg += " has been DESTROYED.";

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "destroyed", string.Format("id=%1 alias='%2' destroyed=%3/%4 by=%5 points=%6",
			objective.m_sId,
			objective.m_sTargetAlias,
			objective.m_iTargetsDestroyed,
			objective.RequiredKills(),
			objective.m_sFaction,
			objective.m_fPoints));

		return msg;
	}

	//! DESTROY status: `DESTROYED`, or `intact <destroyed>/<required>`.
	//! @param objective a usable destroy objective
	//! @param viewerFaction the viewer's side; the destroy status reads the same for every side
	//! @return the status text
	override string StatusText(notnull TBD_Objective objective, string viewerFaction)
	{
		if (objective.m_bComplete)
			return "DESTROYED";

		string text = "intact ";
		text += objective.m_iTargetsDestroyed.ToString();
		text += "/";
		text += objective.RequiredKills().ToString();
		return text;
	}

	//! `#` for a destroy objective.
	//! @param objective a usable destroy objective, neither complete nor contested
	//! @param viewerFaction the viewer's side; the glyph reads the same for every side
	//! @return the glyph
	override string HudIcon(notnull TBD_Objective objective, string viewerFaction)
	{
		return "#";
	}
}
