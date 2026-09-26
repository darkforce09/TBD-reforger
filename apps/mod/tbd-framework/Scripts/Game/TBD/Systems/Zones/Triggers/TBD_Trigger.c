/**
 * @file TBD_Trigger.c
 * @brief One prepared editor trigger, its prepared effects and its life-cycle state.
 *
 * Role: the runtime form of one `editorTriggers[]` entry, flattened and validated once at load.
 * Position: built by `TBD_TriggerCompiler`; advanced by `TBD_TriggerRuntime`; read by the task
 * state machine, audio emitters and spawn modules through `TBD_TriggerRuntime.GetAll`.
 * State: the state machine fields, owned by the server's trigger runtime.  Invariants: an INERT
 * trigger carries its reason and is never ticked; `m_Zone` is a reference into the zone registry,
 * never a copy.
 */

//! Where a trigger is in its life: four states and no flags, so "has it fired" and "is it
//! counting down" can never disagree.
enum TBD_ETriggerState
{
	INERT,     //!< Could never fire. Reported at load; never ticked.
	ARMED,     //!< Waiting for the condition to hold.
	PENDING,   //!< The condition is holding; the timeout is counting.
	FIRED      //!< The effects have run. Terminal unless `repeat`.
}

//! One prepared effect: the wire record with its type resolved and its params flattened,
//! validated once at load.
class TBD_TriggerEffect
{
	TBD_ETriggerEffect m_eKind; //!< resolved `type`
	string m_sRawType; //!< the authored `type`, for the unknown-type diagnostic

	string m_sAlias; //!< `params.alias`; empty = absent
	string m_sText; //!< `params.text`; empty = absent
	string m_sSound; //!< `params.sound`, an `SCR_SoundEvent` name; empty = absent
	string m_sAudience; //!< `params.audience`; empty = everybody
	string m_sObjectiveId; //!< `params.objectiveId`; empty = absent
	string m_sState; //!< `params.state`; empty = complete
	string m_sOwner; //!< `params.owner`; empty = leave the owner alone
	string m_sWinner; //!< `params.winner`; empty = no winner named
	string m_sVariantId; //!< `params.variantId`; empty = absent

	float m_fX; //!< world X in metres; `TBD_TriggerParamsStruct.ABSENT` = the zone's centre
	float m_fZ; //!< world Z in metres; `TBD_TriggerParamsStruct.ABSENT` = the zone's centre
	float m_fHeadingDeg; //!< spawn heading in degrees; default 0
	int m_iCount; //!< spawn copies, 1 to `TBD_TriggerEffectValidator.MAX_SPAWN_COUNT`; default 1

	bool m_bUsable; //!< false = this effect can never run
	string m_sInertReason; //!< why `m_bUsable` is false; empty otherwise
}

//! One prepared trigger and its state machine fields.
class TBD_Trigger
{
	string m_sId; //!< `id`, or `editorTriggers[<index>]` when absent
	string m_sZoneId; //!< `zoneId`; empty = document-wide
	TBD_Zone m_Zone; //!< the registry's zone for `m_sZoneId`, or null for a document-wide condition

	TBD_ETriggerCondition m_eCondition; //!< resolved `activation.condition`
	string m_sRawCondition; //!< the authored condition, for the unknown-condition diagnostic
	string m_sOwnerSide; //!< `activation.ownerSide`; empty = absent
	bool m_bRepeat; //!< `activation.repeat`; default false
	float m_fTimeoutSeconds; //!< `activation.timeoutSeconds`, the dwell in seconds; default 0
	string m_sVariantId; //!< `variantId`; empty = ungated

	ref array<ref TBD_TriggerEffect> m_aEffects; //!< prepared effects in authored order

	TBD_ETriggerState m_eState; //!< where the trigger is in its life
	float m_fHeldSeconds; //!< seconds the condition has held without a gap; 0 when it stops holding
	int m_iFireCount; //!< how many times the effects have run
	string m_sInertReason; //!< why the trigger is INERT; empty otherwise

	//! Stable identifier for logs, built in steps (a long `+` chain is `Formula too complex`).
	//! @return `trigger:<id>`
	string LogKey()
	{
		string key = "trigger:";
		key += m_sId;
		return key;
	}

	//! How many prepared effects this trigger can run.
	//! @return the count of effects with `m_bUsable`, 0 when there are none
	int UsableEffectCount()
	{
		if (!m_aEffects)
			return 0;

		int usable = 0;
		foreach (TBD_TriggerEffect effect : m_aEffects)
		{
			if (effect && effect.m_bUsable)
				usable++;
		}

		return usable;
	}
}
