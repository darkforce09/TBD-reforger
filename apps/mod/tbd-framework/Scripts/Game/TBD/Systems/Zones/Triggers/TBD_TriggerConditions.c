/**
 * @file TBD_TriggerConditions.c
 * @brief Whether a trigger's `activation.condition` holds this tick.
 *
 * Role: answers the six conditions against the tick's player snapshot and the objective registry.
 * Position: called by `TBD_TriggerRuntime.Evaluate`; reads `TBD_TriggerPlayerSnapshot` and
 * `TBD_ObjectiveRegistry`.
 * State: none.  Invariants: `not_present` never holds over an unidentified body in the area;
 * `detected_by` is proximity within `DETECT_RADIUS_M`, not perception; a `timer` always holds.
 */

//! Trigger condition predicates.
//! @authority server
class TBD_TriggerConditions
{
	static const float DETECT_RADIUS_M = 250.0; //!< metres an observer may be from an intruder for `detected_by`; no line of sight is traced

	//  CONDITIONS

	//! Whether this trigger's condition holds now.
	//! @param trigger an armed trigger
	//! @param snapshot the tick's captured player snapshot
	//! @return true when the condition holds; false for `NONE`
	static bool Holds(notnull TBD_Trigger trigger, notnull TBD_TriggerPlayerSnapshot snapshot)
	{
		if (trigger.m_eCondition == TBD_ETriggerCondition.TIMER)
			return true;

		if (trigger.m_eCondition == TBD_ETriggerCondition.OBJECTIVE_COMPLETE)
			return ObjectiveComplete(trigger);

		if (trigger.m_eCondition == TBD_ETriggerCondition.PRESENT)
			return snapshot.CountInside(trigger.m_Zone, trigger.m_sOwnerSide, true) > 0;

		if (trigger.m_eCondition == TBD_ETriggerCondition.NOT_PRESENT)
		{
			// An unidentified body in the area means absence cannot be asserted: answered false.
			if (snapshot.CountInside(trigger.m_Zone, trigger.m_sOwnerSide, true) > 0)
				return false;

			return snapshot.CountUnknownInside(trigger.m_Zone) == 0;
		}

		if (trigger.m_eCondition == TBD_ETriggerCondition.SEIZED_BY)
		{
			if (snapshot.CountInside(trigger.m_Zone, trigger.m_sOwnerSide, true) == 0)
				return false;

			return snapshot.CountInside(trigger.m_Zone, trigger.m_sOwnerSide, false) == 0;
		}

		if (trigger.m_eCondition == TBD_ETriggerCondition.DETECTED_BY)
		{
			if (!trigger.m_Zone)
				return false;

			return snapshot.IsIntruderSpotted(trigger.m_Zone, trigger.m_sOwnerSide, DETECT_RADIUS_M);
		}

		return false;
	}

	//! `objective_complete`: with a `zoneId`, that one objective; without one, every usable
	//! objective, and at least one must exist. Unusable objectives are excluded, as
	//! `TBD_ObjectiveRegistry.AreAllObjectivesCaptured` excludes them.
	//! @param trigger the trigger whose `m_sZoneId` names the objective, or is empty
	//! @return true when the named objective, or every usable one, is complete
	protected static bool ObjectiveComplete(notnull TBD_Trigger trigger)
	{
		array<ref TBD_Objective> objectiveList = TBD_ObjectiveRegistry.GetAll();
		if (!objectiveList)
			return false;

		if (!trigger.m_sZoneId.IsEmpty())
		{
			foreach (TBD_Objective one : objectiveList)
			{
				if (one && one.m_sId == trigger.m_sZoneId)
					return one.m_bComplete;
			}

			return false;
		}

		int usable = 0;
		foreach (TBD_Objective objective : objectiveList)
		{
			if (!objective || !objective.m_bUsable)
				continue;

			usable++;
			if (!objective.m_bComplete)
				return false;
		}

		return usable > 0;
	}
}
