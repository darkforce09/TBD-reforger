/**
 * @file TBD_TriggerEffectValidator.c
 * @brief Decides at load whether each prepared trigger effect can ever run.
 *
 * Role: the per-effect checks on `type` and `params`, so a defect is a boot-log line rather than
 * an effect that fires into nothing.  Position: called by `TBD_TriggerCompiler.PrepareEffect`.
 * State: none.  Invariants: an unusable effect carries a reason naming the param it wanted (the
 * open `params` bag means a misspelled key arrives as an absent one); an out-of-range `count` or
 * `state` is logged and clamped, never obeyed.
 */

//! Load-time effect validation.
class TBD_TriggerEffectValidator
{
	static const int MAX_SPAWN_COUNT = 32; //!< ceiling on one `spawn` effect's `count`; above it is clamped and logged

	//! Mark `effect` unusable, with a reason naming the param it wanted, when it can never run;
	//! clamp `count` into 1..`MAX_SPAWN_COUNT` and an unknown `state` to complete, logging each.
	//! @param trigger the owning trigger, for its zone and id
	//! @param effect the prepared effect, updated in place
	static void Validate(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		if (effect.m_eKind == TBD_ETriggerEffect.NONE)
		{
			effect.m_bUsable = false;
			if (effect.m_sRawType.IsEmpty())
			{
				effect.m_sInertReason = "its `type` is absent. The schema's vocabulary is spawn | delete | end_mission | set_objective | hint | play_sound | set_variant.";
			}
			else
			{
				effect.m_sInertReason = "its `type` is outside the schema vocabulary spawn | delete | end_mission | set_objective | hint | play_sound | set_variant.";
			}
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SPAWN)
		{
			if (effect.m_sAlias.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.alias` is absent, so there is no prefab to spawn";
				return;
			}

			// With a zone, its centre is the default position; without one, x and z are required.
			if (!trigger.m_Zone && (effect.m_fX == TBD_TriggerParamsStruct.ABSENT || effect.m_fZ == TBD_TriggerParamsStruct.ABSENT))
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "the trigger has no `zoneId` to take a position from and `params.x`/`params.z` are absent, so there is nowhere to spawn";
				return;
			}

			if (effect.m_iCount < 1)
			{
				TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' spawn effect authors params.count=%2 - using 1",
					trigger.m_sId, effect.m_iCount));
				effect.m_iCount = 1;
			}
			else if (effect.m_iCount > MAX_SPAWN_COUNT)
			{
				TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' spawn effect authors params.count=%2, above the %3 ceiling - clamped",
					trigger.m_sId, effect.m_iCount, MAX_SPAWN_COUNT));
				effect.m_iCount = MAX_SPAWN_COUNT;
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.DELETE)
		{
			if (effect.m_sAlias.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.alias` is absent, so there is nothing to identify for deletion";
				return;
			}

			// A delete with no zone would remove every matching object on the terrain; refused.
			if (!trigger.m_Zone)
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "the trigger has no `zoneId`, and a world-wide delete is refused - give the trigger a zone so the deletion has a boundary";
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SET_OBJECTIVE)
		{
			if (effect.m_sObjectiveId.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.objectiveId` is absent, so there is no objective to set";
				return;
			}

			if (!effect.m_sState.IsEmpty() && effect.m_sState != TBD_TriggerVocabulary.STATE_COMPLETE && effect.m_sState != TBD_TriggerVocabulary.STATE_INCOMPLETE)
			{
				TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' set_objective authors params.state='%2' - the vocabulary is complete | incomplete; using complete",
					trigger.m_sId, effect.m_sState));
				effect.m_sState = TBD_TriggerVocabulary.STATE_COMPLETE;
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.HINT)
		{
			if (effect.m_sText.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.text` is absent, so the hint would be an empty message";
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.PLAY_SOUND)
		{
			if (effect.m_sSound.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.sound` is absent, so there is no sound event to raise";
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SET_VARIANT)
		{
			if (effect.m_sVariantId.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.variantId` is absent, so there is no variant to select";
			}
		}

		// END_MISSION needs nothing: `params.winner` is optional.
	}
}
