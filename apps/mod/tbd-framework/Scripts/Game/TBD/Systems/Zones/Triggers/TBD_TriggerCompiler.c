/**
 * @file TBD_TriggerCompiler.c
 * @brief Reads `editorTriggers[]` and prepares each entry against the zone registry.
 *
 * Role: turns the wire records into `TBD_Trigger`s: condition, owner side, dwell, zone and effects
 * resolved once, every defect reported by trigger id.  Position: called by
 * `TBD_TriggerRuntime.Build`; reads `TBD_MissionJsonPass` and `TBD_ZoneRegistry`; hands each effect
 * to `TBD_TriggerEffectValidator`.
 * State: none.  Invariants: a trigger that can never fire leaves here INERT with its reason; an
 * authored `zoneId` that resolves to nothing is INERT, never widened to the whole world.
 */

//! Wire-to-runtime preparation of editor triggers.
//! @authority server
class TBD_TriggerCompiler
{
	//! Second typed pass over the held mission JSON for `editorTriggers[]`.
	//! @return the wire array, or null when no document is held, it is not JSON, its root does not
	//! read (both logged as errors), or it authors no `editorTriggers` key
	static array<ref TBD_EditorTriggerStruct> ReadWire()
	{
		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (outcome == TBD_EMissionJsonPassOutcome.NO_DOCUMENT)
			return null;

		if (!ctx)
		{
			TBD_Log.Error(TBD_TriggerRuntime.CH, "the mission document did not parse as JSON on the trigger pass - no trigger is armed this round");
			return null;
		}

		TBD_TriggerDocStruct doc = new TBD_TriggerDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			TBD_Log.Error(TBD_TriggerRuntime.CH, "the mission document parsed but its root would not read on the trigger pass - no trigger is armed this round");
			return null;
		}

		return doc.editorTriggers;
	}

	//! Log one `trigger` line for an armed trigger: its condition, owner, zone, repeat, dwell,
	//! usable effect count and variant, computed from the parsed fields so the boot log shows what
	//! will fire.
	//! @param trigger the prepared trigger
	static void LogPrepared(notnull TBD_Trigger trigger)
	{
		string zoneLabel = "<document-wide>";
		if (trigger.m_Zone)
			zoneLabel = trigger.m_Zone.LogKey();

		TBD_Log.Kv(TBD_TriggerRuntime.CH, "trigger", string.Format("id=%1 condition=%2 owner='%3' zone=%4 repeat=%5 timeout=%6s effects=%7 variant='%8'",
			trigger.m_sId,
			typename.EnumToString(TBD_ETriggerCondition, trigger.m_eCondition),
			trigger.m_sOwnerSide,
			zoneLabel,
			trigger.m_bRepeat,
			trigger.m_fTimeoutSeconds,
			trigger.UsableEffectCount(),
			trigger.m_sVariantId));
	}

	//! Flatten one wire trigger into its runtime form, reporting every defect by trigger id.
	//! @param rawTrigger the wire record
	//! @param index its position in `editorTriggers[]`, used as the id when `id` is absent
	//! @return the prepared trigger, ARMED or INERT with `m_sInertReason` set
	static TBD_Trigger Prepare(notnull TBD_EditorTriggerStruct rawTrigger, int index)
	{
		TBD_Trigger trigger = new TBD_Trigger();
		trigger.m_sId = rawTrigger.id;
		if (trigger.m_sId.IsEmpty())
			trigger.m_sId = string.Format("editorTriggers[%1]", index);

		trigger.m_sZoneId = rawTrigger.zoneId;
		trigger.m_sVariantId = rawTrigger.variantId;
		trigger.m_eState = TBD_ETriggerState.ARMED;
		trigger.m_aEffects = new array<ref TBD_TriggerEffect>();

		ResolveActivation(trigger, rawTrigger.activation);
		ResolveZone(trigger);
		ResolveEffects(trigger, rawTrigger.effects);

		if (trigger.m_eState == TBD_ETriggerState.INERT)
			return trigger;

		if (trigger.UsableEffectCount() == 0)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = "it has no effect this build can run, so activating it could not change anything. Author at least one `effects[]` record whose `type` and `params` resolve.";
		}

		return trigger;
	}

	//! Resolve `activation` into condition, owner side, repeat and dwell. `timeoutSeconds` is a
	//! dwell: the condition must hold without a gap for that long. A negative dwell is logged and
	//! read as 0; an absent or unknown condition, or `seized_by`/`detected_by` without an owner
	//! side, makes the trigger INERT.
	//! @param trigger the trigger being prepared
	//! @param activation the wire `activation`; null makes the trigger INERT
	protected static void ResolveActivation(notnull TBD_Trigger trigger, TBD_TriggerActivationStruct activation)
	{
		trigger.m_eCondition = TBD_ETriggerCondition.NONE;
		trigger.m_fTimeoutSeconds = 0;

		if (!activation)
		{
			// JsonLoadContext allocates the nested ref, so this guards a hand-built struct only.
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = "it has no `activation` block, so there is nothing to make it fire";
			return;
		}

		trigger.m_sRawCondition = activation.condition;
		trigger.m_sOwnerSide = activation.ownerSide;
		trigger.m_bRepeat = activation.repeat;

		if (activation.timeoutSeconds != TBD_TriggerParamsStruct.ABSENT)
		{
			if (activation.timeoutSeconds < 0)
			{
				TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' authors activation.timeoutSeconds=%2, which is negative - using 0s (fire as soon as the condition holds)",
					trigger.m_sId, activation.timeoutSeconds));
			}
			else
			{
				trigger.m_fTimeoutSeconds = activation.timeoutSeconds;
			}
		}

		trigger.m_eCondition = TBD_TriggerVocabulary.ConditionFromString(activation.condition);
		if (trigger.m_eCondition == TBD_ETriggerCondition.NONE)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			if (activation.condition.IsEmpty())
			{
				trigger.m_sInertReason = "its `activation.condition` is absent. The schema's vocabulary is present | not_present | detected_by | seized_by | timer | objective_complete.";
			}
			else
			{
				trigger.m_sInertReason = string.Format("its `activation.condition` is '%1', which this build does not recognise. The schema's vocabulary is present | not_present | detected_by | seized_by | timer | objective_complete.",
					activation.condition);
			}
			return;
		}

		// seized_by and detected_by ask about a side; without one they are refused, not widened.
		if (trigger.m_sOwnerSide.IsEmpty())
		{
			if (trigger.m_eCondition == TBD_ETriggerCondition.SEIZED_BY || trigger.m_eCondition == TBD_ETriggerCondition.DETECTED_BY)
			{
				trigger.m_eState = TBD_ETriggerState.INERT;
				trigger.m_sInertReason = string.Format("condition '%1' asks which SIDE seized or detected, and `activation.ownerSide` is absent. Set it to a `factions[].key`.",
					activation.condition);
			}
		}
	}

	//! Bind `zoneId` to the registry's prepared zone. An absent `zoneId` means document-wide
	//! (INERT for `detected_by`, which needs an area); an id that names no zone, or a zone with no
	//! usable shape, makes the trigger INERT.
	//! @param trigger the trigger being prepared; skipped when already INERT
	protected static void ResolveZone(notnull TBD_Trigger trigger)
	{
		if (trigger.m_eState == TBD_ETriggerState.INERT)
			return;

		if (trigger.m_sZoneId.IsEmpty())
		{
			if (trigger.m_eCondition == TBD_ETriggerCondition.DETECTED_BY)
			{
				trigger.m_eState = TBD_ETriggerState.INERT;
				trigger.m_sInertReason = "condition 'detected_by' needs an area to be entered, and no `zoneId` is authored. Point it at a `zones[].id`.";
			}
			return;
		}

		if (!TBD_ZoneRegistry.GetAll())
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = "the zone registry produced no zones, so its `zoneId` cannot be resolved";
			return;
		}

		trigger.m_Zone = TBD_ZoneRegistry.FindById(trigger.m_sZoneId);

		if (!trigger.m_Zone)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = string.Format("its `zoneId` is '%1' and no `zones[]` row has that id", trigger.m_sZoneId);
			return;
		}

		if (!trigger.m_Zone.IsUsable())
		{
			trigger.m_Zone = null;
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = string.Format("zone '%1' has no usable shape (no circle, or a polygon with fewer than 3 vertices), so it can never contain anybody",
				trigger.m_sZoneId);
		}
	}

	//! Prepare `effects[]`, each validated once; a null entry is skipped and an unusable one kept
	//! and reported by index. No effects at all makes the trigger INERT.
	//! @param trigger the trigger being prepared; skipped when already INERT
	//! @param effects the wire `effects`; may be null
	protected static void ResolveEffects(notnull TBD_Trigger trigger, array<ref TBD_TriggerEffectStruct> effects)
	{
		if (trigger.m_eState == TBD_ETriggerState.INERT)
			return;

		if (!effects || effects.Count() == 0)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = "it authors no `effects[]`, so firing it would do nothing";
			return;
		}

		foreach (int index, TBD_TriggerEffectStruct rawEffect : effects)
		{
			if (!rawEffect)
			{
				TBD_Log.Warn(TBD_TriggerRuntime.CH, string.Format("trigger '%1' effects[%2] is null - skipped", trigger.m_sId, index));
				continue;
			}

			TBD_TriggerEffect effect = PrepareEffect(trigger, rawEffect);
			trigger.m_aEffects.Insert(effect);

			if (!effect.m_bUsable)
			{
				TBD_Log.Warn(TBD_TriggerRuntime.CH, string.Format("trigger '%1' effects[%2] (type '%3') is INERT: %4",
					trigger.m_sId, index, effect.m_sRawType, effect.m_sInertReason));
			}
		}
	}

	//! Flatten one effect record and decide, once, whether it can ever run.
	//! @param trigger the owning trigger, for its zone and id
	//! @param rawEffect the wire record
	//! @return the prepared effect; `m_bUsable` false with a reason when it can never run
	protected static TBD_TriggerEffect PrepareEffect(notnull TBD_Trigger trigger, notnull TBD_TriggerEffectStruct rawEffect)
	{
		TBD_TriggerEffect effect = new TBD_TriggerEffect();
		effect.m_sRawType = rawEffect.type;
		effect.m_eKind = TBD_TriggerVocabulary.EffectFromString(rawEffect.type);
		effect.m_bUsable = true;

		effect.m_fX = TBD_TriggerParamsStruct.ABSENT;
		effect.m_fZ = TBD_TriggerParamsStruct.ABSENT;
		effect.m_fHeadingDeg = 0;
		effect.m_iCount = 1;

		if (rawEffect.params)
		{
			effect.m_sAlias = rawEffect.params.alias;
			effect.m_sText = rawEffect.params.text;
			effect.m_sSound = rawEffect.params.sound;
			effect.m_sAudience = rawEffect.params.audience;
			effect.m_sObjectiveId = rawEffect.params.objectiveId;
			effect.m_sState = rawEffect.params.state;
			effect.m_sOwner = rawEffect.params.owner;
			effect.m_sWinner = rawEffect.params.winner;
			effect.m_sVariantId = rawEffect.params.variantId;

			if (rawEffect.params.x != TBD_TriggerParamsStruct.ABSENT)
				effect.m_fX = rawEffect.params.x;
			if (rawEffect.params.z != TBD_TriggerParamsStruct.ABSENT)
				effect.m_fZ = rawEffect.params.z;
			if (rawEffect.params.headingDeg != TBD_TriggerParamsStruct.ABSENT)
				effect.m_fHeadingDeg = rawEffect.params.headingDeg;
			if (rawEffect.params.count != TBD_TriggerParamsStruct.ABSENT_INT)
				effect.m_iCount = rawEffect.params.count;
		}

		TBD_TriggerEffectValidator.Validate(trigger, effect);
		return effect;
	}
}
