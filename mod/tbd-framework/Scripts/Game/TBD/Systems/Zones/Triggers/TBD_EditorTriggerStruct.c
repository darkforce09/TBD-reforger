/**
 * @file TBD_EditorTriggerStruct.c
 * @brief The `editorTriggers[]` wire records a second typed JSON pass reads.
 *
 * Role: the typed shape of `editorTriggers[]`, its `activation` and its `effects[]`.
 * Position: filled by `TBD_TriggerCompiler.ReadWire` from `TBD_MissionJsonPass`; read by
 * `TBD_TriggerCompiler` alone.
 * State: none.  Invariants: every optional number carries an ABSENT sentinel and every nested
 * `ref` is allocated by `JsonLoadContext` whether or not its key is present, so presence is read
 * from field values, never from null references.
 */

//! The open `effects[].params` bag as the trigger runtime reads it: a key not declared here is
//! invisible, so every effect names the param it wanted when it goes INERT.
//! @contract mission.schema.json#/$defs/editorTrigger/properties/effects/items/properties/params
class TBD_TriggerParamsStruct
{
	static const float ABSENT = -1000000; //!< sentinel for an absent number key; 0 is a real coordinate

	static const int ABSENT_INT = -1; //!< sentinel for an absent `count`; 0 is an authorable value

	string alias;        //!< `spawn` / `delete`. Empty = absent.
	string text;         //!< `hint`. Empty = absent.
	string sound;        //!< `play_sound`. Empty = absent.
	string audience;     //!< `hint` / `play_sound`. Empty = absent (means "all").
	string objectiveId;  //!< `set_objective`. Empty = absent.
	string state;        //!< `set_objective`. Empty = absent (means "complete").
	string owner;        //!< `set_objective`. Empty = absent (leave the owner alone).
	string winner;       //!< `end_mission`. Empty = absent (no winner named).
	string variantId;    //!< `set_variant`. Empty = absent.

	float x = ABSENT; //!< `spawn` world X in metres; ABSENT = the trigger zone's centre
	float z = ABSENT; //!< `spawn` world Z in metres; ABSENT = the trigger zone's centre
	float headingDeg = ABSENT; //!< `spawn` heading in degrees; ABSENT = 0

	int count = ABSENT_INT; //!< `spawn` copies; ABSENT_INT = 1
}

//! One `effects[]` record on the wire.
//! @contract mission.schema.json#/$defs/editorTrigger/properties/effects/items
class TBD_TriggerEffectStruct
{
	string type;   //!< Schema enum: spawn|delete|end_mission|set_objective|hint|play_sound|set_variant.
	ref TBD_TriggerParamsStruct params; //!< `params`; allocated even when absent, so read its fields
}

//! The `activation` object; its presence is decided by `condition` being non-empty.
//! @contract mission.schema.json#/$defs/editorTrigger/properties/activation
class TBD_TriggerActivationStruct
{
	string condition;     //!< present|not_present|detected_by|seized_by|timer|objective_complete.
	string ownerSide;     //!< `factionKey`. Empty = absent; meaning is per condition.
	bool repeat;          //!< Schema default false. Absent and authored-false are the same value.
	float timeoutSeconds = TBD_TriggerParamsStruct.ABSENT; //!< dwell in seconds; ABSENT = 0
}

//! One `editorTriggers[]` entry on the wire.
//! @contract mission.schema.json#/$defs/editorTrigger
class TBD_EditorTriggerStruct
{
	string id; //!< `id`; empty = named by its array index
	string zoneId; //!< `zoneId`, a `zones[].id`; empty = a document-wide condition
	ref TBD_TriggerActivationStruct activation; //!< `activation`; allocated even when absent
	ref array<ref TBD_TriggerEffectStruct> effects; //!< `effects`; null or empty makes the trigger INERT
	string variantId; //!< `variantId`; empty = ungated (see `TBD_TriggerRuntime.SelectVariant`)
}

//! The document root of the trigger pass: declares `editorTriggers` and nothing else.
//! @contract mission.schema.json#/ partial
class TBD_TriggerDocStruct
{
	ref array<ref TBD_EditorTriggerStruct> editorTriggers; //!< `editorTriggers`; null when the key is absent
}
