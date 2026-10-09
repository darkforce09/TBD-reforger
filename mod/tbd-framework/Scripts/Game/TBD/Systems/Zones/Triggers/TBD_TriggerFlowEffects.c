/**
 * @file TBD_TriggerFlowEffects.c
 * @brief The trigger effects that steer the round: objectives, variants and the end.
 *
 * Role: runs `set_objective`, `set_variant` and `end_mission`.  Position: called by
 * `TBD_TriggerEffects.Run`; writes `TBD_Objective` state, `TBD_TriggerRuntime`'s variant
 * selection and the stage through `TBD_FrameworkManager.SetStage`.
 * State: none.  Invariants: the round ends only through the stage machine's `SetStage`; a winner is
 * reported in the `[TBD][Win]` line, not stored.
 */

//! Round-flow trigger effects.
//! @authority server
class TBD_TriggerFlowEffects
{
	//! `set_objective`: set one objective's `m_bComplete` (complete unless `state` is
	//! incomplete), the same flag the objective system sets, and write `params.owner` as its
	//! capture owner when authored. An unbuilt registry or an unknown id changes nothing (logged).
	//! @param trigger the firing trigger
	//! @param effect the prepared `set_objective` effect
	static void SetObjective(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		array<ref TBD_Objective> objectiveList = TBD_ObjectiveRegistry.GetAll();
		if (!objectiveList)
		{
			TBD_Log.Warn(TBD_TriggerRuntime.CH, string.Format("trigger '%1' set_objective '%2' but the objective registry has not been built - nothing changed",
				trigger.m_sId, effect.m_sObjectiveId));
			return;
		}

		TBD_Objective target;
		foreach (TBD_Objective objective : objectiveList)
		{
			if (objective && objective.m_sId == effect.m_sObjectiveId)
			{
				target = objective;
				break;
			}
		}

		if (!target)
		{
			TBD_Log.Warn(TBD_TriggerRuntime.CH, string.Format("trigger '%1' set_objective '%2' names no objective in this mission - nothing changed",
				trigger.m_sId, effect.m_sObjectiveId));
			return;
		}

		bool complete = effect.m_sState != TBD_TriggerVocabulary.STATE_INCOMPLETE;
		target.m_bComplete = complete;

		if (!effect.m_sOwner.IsEmpty())
			target.m_sOwner = effect.m_sOwner;

		TBD_Log.Kv(TBD_TriggerRuntime.CH, "setObjective", string.Format("id=%1 objective=%2 complete=%3 owner='%4'",
			trigger.m_sId, target.m_sId, complete, target.m_sOwner));
	}

	//! `set_variant`: add `params.variantId` to the runtime's selected variants.
	//! @param trigger the firing trigger
	//! @param effect the prepared `set_variant` effect
	static void SetVariant(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		int selected = TBD_TriggerRuntime.SelectVariant(effect.m_sVariantId);
		TBD_Log.Kv(TBD_TriggerRuntime.CH, "setVariant", string.Format("id=%1 variant='%2' selected=%3",
			trigger.m_sId, effect.m_sVariantId, selected));
	}

	//! `end_mission`: log `[TBD][Win] trigger:<id> - winner=<winner>` and move the stage to END
	//! through `TBD_FrameworkManager.SetStage`, so the end runs every hook any other end runs. No
	//! manager, or a refusal by the stage machine, is logged as an error.
	//! @param trigger the firing trigger
	//! @param effect the prepared `end_mission` effect
	static void EndMission(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
		{
			TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' end_mission but there is no TBD_FrameworkManager on this world - the round was NOT ended",
				trigger.m_sId));
			return;
		}

		Print(string.Format("[TBD][Win] trigger:%1 - winner=%2", trigger.m_sId, effect.m_sWinner));

		fm.SetStage(TBD_EGameStage.END);

		string refusal = fm.GetLastStageRefusal();
		if (!refusal.IsEmpty())
		{
			TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' end_mission was REFUSED by the stage machine: %2",
				trigger.m_sId, refusal));
			return;
		}

		TBD_Log.Kv(TBD_TriggerRuntime.CH, "endMission", string.Format("id=%1 winner='%2'", trigger.m_sId, effect.m_sWinner));
	}
}
