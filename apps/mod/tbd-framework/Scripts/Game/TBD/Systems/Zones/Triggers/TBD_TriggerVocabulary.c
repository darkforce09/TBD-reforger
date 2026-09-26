/**
 * @file TBD_TriggerVocabulary.c
 * @brief The closed `activation.condition` and `effects[].type` vocabularies and their enums.
 *
 * Role: maps the schema's trigger strings onto the condition and effect enums, and names the
 * `params.audience` and `params.state` words.  Position: read by `TBD_TriggerCompiler`,
 * `TBD_TriggerEffectValidator`, `TBD_TriggerEffects` and `TBD_TriggerFlowEffects`.
 * State: none.  Invariants: an absent or unknown string maps to `NONE`, never to a guess.
 */

//! What makes a trigger fire. `NONE` is not a condition - it is what an unrecognised or unauthored
//! `activation.condition` resolves to, and a trigger that resolves to it is INERT and says so.
enum TBD_ETriggerCondition
{
	NONE,                //!< Absent or unrecognised; the trigger is INERT.
	PRESENT,             //!< At least one live player of `ownerSide` is inside the area.
	NOT_PRESENT,         //!< No live player of `ownerSide` is inside the area.
	DETECTED_BY,         //!< A side that is NOT `ownerSide` is inside and `ownerSide` is near it.
	SEIZED_BY,           //!< `ownerSide` is inside and no other side is.
	TIMER,               //!< Holds from the moment the runtime arms; `timeoutSeconds` is the whole rule.
	OBJECTIVE_COMPLETE   //!< The named objective (or every objective) is complete.
}

//! What an effect does. `NONE` is an unrecognised `effects[].type`; such an effect is reported
//! at load with its index named and never runs.
enum TBD_ETriggerEffect
{
	NONE,          //!< Absent or unrecognised `type`; the effect is unusable.
	SPAWN,         //!< `spawn`
	DELETE,        //!< `delete`
	END_MISSION,   //!< `end_mission`
	SET_OBJECTIVE, //!< `set_objective`
	HINT,          //!< `hint`
	PLAY_SOUND,    //!< `play_sound`
	SET_VARIANT    //!< `set_variant`
}

//! The trigger string vocabulary and its parsers.
class TBD_TriggerVocabulary
{
	static const string COND_PRESENT = "present"; //!< `activation.condition` value
	static const string COND_NOT_PRESENT = "not_present"; //!< `activation.condition` value
	static const string COND_DETECTED_BY = "detected_by"; //!< `activation.condition` value
	static const string COND_SEIZED_BY = "seized_by"; //!< `activation.condition` value
	static const string COND_TIMER = "timer"; //!< `activation.condition` value
	static const string COND_OBJECTIVE_COMPLETE = "objective_complete"; //!< `activation.condition` value

	static const string FX_SPAWN = "spawn"; //!< `effects[].type` value
	static const string FX_DELETE = "delete"; //!< `effects[].type` value
	static const string FX_END_MISSION = "end_mission"; //!< `effects[].type` value
	static const string FX_SET_OBJECTIVE = "set_objective"; //!< `effects[].type` value
	static const string FX_HINT = "hint"; //!< `effects[].type` value
	static const string FX_PLAY_SOUND = "play_sound"; //!< `effects[].type` value
	static const string FX_SET_VARIANT = "set_variant"; //!< `effects[].type` value

	static const string AUD_ALL = "all"; //!< `params.audience` value; any other value is a faction key
	static const string AUD_OWNER = "owner"; //!< `params.audience` value; any other value is a faction key
	static const string AUD_ENEMY = "enemy"; //!< `params.audience` value; any other value is a faction key

	static const string STATE_COMPLETE = "complete"; //!< `set_objective` `params.state` value
	static const string STATE_INCOMPLETE = "incomplete"; //!< `set_objective` `params.state` value

	//! Map an `activation.condition` string onto its enum.
	//! @param raw the authored string; may be empty
	//! @return the condition, or `NONE` for an absent or unrecognised string
	static TBD_ETriggerCondition ConditionFromString(string raw)
	{
		if (raw == COND_PRESENT)
			return TBD_ETriggerCondition.PRESENT;
		if (raw == COND_NOT_PRESENT)
			return TBD_ETriggerCondition.NOT_PRESENT;
		if (raw == COND_DETECTED_BY)
			return TBD_ETriggerCondition.DETECTED_BY;
		if (raw == COND_SEIZED_BY)
			return TBD_ETriggerCondition.SEIZED_BY;
		if (raw == COND_TIMER)
			return TBD_ETriggerCondition.TIMER;
		if (raw == COND_OBJECTIVE_COMPLETE)
			return TBD_ETriggerCondition.OBJECTIVE_COMPLETE;

		return TBD_ETriggerCondition.NONE;
	}

	//! Map an `effects[].type` string onto its enum.
	//! @param raw the authored string; may be empty
	//! @return the effect kind, or `NONE` for an absent or unrecognised string
	static TBD_ETriggerEffect EffectFromString(string raw)
	{
		if (raw == FX_SPAWN)
			return TBD_ETriggerEffect.SPAWN;
		if (raw == FX_DELETE)
			return TBD_ETriggerEffect.DELETE;
		if (raw == FX_END_MISSION)
			return TBD_ETriggerEffect.END_MISSION;
		if (raw == FX_SET_OBJECTIVE)
			return TBD_ETriggerEffect.SET_OBJECTIVE;
		if (raw == FX_HINT)
			return TBD_ETriggerEffect.HINT;
		if (raw == FX_PLAY_SOUND)
			return TBD_ETriggerEffect.PLAY_SOUND;
		if (raw == FX_SET_VARIANT)
			return TBD_ETriggerEffect.SET_VARIANT;

		return TBD_ETriggerEffect.NONE;
	}
}
