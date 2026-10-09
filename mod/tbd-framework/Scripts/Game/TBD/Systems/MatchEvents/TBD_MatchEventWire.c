/**
 * @file TBD_MatchEventWire.c
 * @brief The JSON of one detailed match event and of each event kind's payload.
 *
 * Role: builds the `MatchEvent` object around a captured payload and one payload object per event
 * kind, byte for byte.  Position: the payload builders are called by `TBD_MatchEventCapture` at
 * capture time; `BuildEvent` by `TBD_MatchEventRecorder` when it numbers a batch.
 * State: none; pure functions.  Invariants: every string goes through
 * `TBD_BackendText.JsonEscape`; an optional member is omitted, never sent empty; every
 * `string.Format` takes at most nine arguments; the kind names are the schema's constants.
 */

//! Builders of the `MatchEvent` object and its seven payloads.
//! @authority server
class TBD_MatchEventWire
{
	static const string KIND_COMBAT_KILL = "combat.kill"; //!< a player killed a player or an AI character
	static const string KIND_COMBAT_DEATH = "combat.death"; //!< a player died without a player killer
	static const string KIND_MEDICAL_INCAPACITATED = "medical.incapacitated"; //!< a player went unconscious
	static const string KIND_MEDICAL_REVIVED = "medical.revived"; //!< an unconscious player came round
	static const string KIND_VEHICLE_DESTROYED = "vehicle.destroyed"; //!< a vehicle reached the destroyed damage state
	static const string KIND_VEHICLE_ENTERED = "vehicle.entered"; //!< a player got into a vehicle seat
	static const string KIND_VEHICLE_EXITED = "vehicle.exited"; //!< a player got out of a vehicle seat

	static const string CAUSE_AI = "ai"; //!< `combat.death` cause: an AI character killed the player
	static const string CAUSE_ENVIRONMENT = "environment"; //!< `combat.death` cause: nothing instigated the death
	static const string CAUSE_SELF = "self"; //!< `combat.death` cause: the player instigated their own death
	static const string CAUSE_UNKNOWN = "unknown"; //!< `combat.death` cause: none of the above could be told

	static const int ARMA_ID_MAX = 128; //!< longest `arma_id` value the schema accepts, in bytes
	static const int PREFAB_MAX = 256; //!< longest prefab or weapon name the schema accepts, in bytes
	static const int COMPARTMENT_MAX = 64; //!< longest compartment name the schema accepts, in bytes

	//! The complete event object; `event_id` is the sequence written as a decimal string.
	//! @param sequence the event's sequence, at least 1
	//! @param kind one of the `KIND_` names
	//! @param missionTimeMs milliseconds since the round went LIVE, at least 0
	//! @param occurredAtUtc the capture time, RFC 3339 UTC
	//! @param payload the kind's payload object
	//! @return the JSON object
	//! @contract match-telemetry.schema.json#/definitions/MatchEvent
	static string BuildEvent(int sequence, string kind, int missionTimeMs, string occurredAtUtc, string payload)
	{
		string json = string.Format("{\"event_id\":\"%1\",\"sequence\":%1", sequence);
		json += string.Format(",\"kind\":\"%1\",\"mission_time_ms\":%2", kind, missionTimeMs);
		json += string.Format(",\"occurred_at\":\"%1\",\"payload\":", occurredAtUtc);
		json += payload;
		json += "}";
		return json;
	}

	//! The `combat.kill` payload.
	//! @param victimArmaId the victim's identity, omitted when empty
	//! @param distanceM killer to victim distance in whole metres, at least 0
	//! @param weapon the killer's weapon prefab, omitted when empty
	//! @return the JSON object
	//! @contract match-telemetry.schema.json#/definitions/CombatKillPayload
	static string CombatKill(string killerArmaId, string victimArmaId, bool victimIsPlayer, bool teamKill, int distanceM,
		string weapon)
	{
		string json = string.Format("{\"killer_arma_id\":\"%1\"", TBD_BackendText.JsonEscape(killerArmaId));
		if (!victimArmaId.IsEmpty())
			json += string.Format(",\"victim_arma_id\":\"%1\"", TBD_BackendText.JsonEscape(victimArmaId));

		json += string.Format(",\"victim_is_player\":%1,\"team_kill\":%2", JsonBool(victimIsPlayer), JsonBool(teamKill));
		json += string.Format(",\"distance_m\":%1", distanceM);
		if (!weapon.IsEmpty())
			json += string.Format(",\"weapon\":\"%1\"", TBD_BackendText.JsonEscape(weapon));

		json += "}";
		return json;
	}

	//! The `combat.death` payload.
	//! @param cause one of the `CAUSE_` names
	//! @return the JSON object
	//! @contract match-telemetry.schema.json#/definitions/CombatDeathPayload
	static string CombatDeath(string victimArmaId, string cause)
	{
		return string.Format("{\"victim_arma_id\":\"%1\",\"cause\":\"%2\"}", TBD_BackendText.JsonEscape(victimArmaId), cause);
	}

	//! The `medical.incapacitated` and `medical.revived` payload, which share one shape.
	//! @return the JSON object
	//! @contract match-telemetry.schema.json#/definitions/MedicalIncapacitatedPayload
	static string MedicalSubject(string subjectArmaId)
	{
		return string.Format("{\"subject_arma_id\":\"%1\"}", TBD_BackendText.JsonEscape(subjectArmaId));
	}

	//! The `vehicle.destroyed` payload.
	//! @param instigatorArmaId the destroying player's identity, omitted when empty
	//! @return the JSON object
	//! @contract match-telemetry.schema.json#/definitions/VehicleDestroyedPayload
	static string VehicleDestroyed(string vehiclePrefab, string instigatorArmaId)
	{
		string json = string.Format("{\"vehicle_prefab\":\"%1\"", TBD_BackendText.JsonEscape(vehiclePrefab));
		if (!instigatorArmaId.IsEmpty())
			json += string.Format(",\"instigator_arma_id\":\"%1\"", TBD_BackendText.JsonEscape(instigatorArmaId));

		json += "}";
		return json;
	}

	//! The `vehicle.entered` and `vehicle.exited` payload, which share one shape.
	//! @param compartment the seat kind (`pilot`, `turret`, `cargo` or `other`)
	//! @return the JSON object
	//! @contract match-telemetry.schema.json#/definitions/VehicleEnteredPayload
	static string VehicleSeat(string armaId, string vehiclePrefab, string compartment)
	{
		return string.Format("{\"arma_id\":\"%1\",\"vehicle_prefab\":\"%2\",\"compartment\":\"%3\"}",
			TBD_BackendText.JsonEscape(armaId), TBD_BackendText.JsonEscape(vehiclePrefab), TBD_BackendText.JsonEscape(compartment));
	}

	//! Whether `value` is non-empty and at most `maxBytes` long, the schema's bound on an
	//! identity, prefab, weapon or compartment string.
	//! @return true when the value may be sent
	static bool Fits(string value, int maxBytes)
	{
		return !value.IsEmpty() && value.Length() <= maxBytes;
	}

	//! A JSON boolean literal.
	//! @return `true` or `false`
	static string JsonBool(bool value)
	{
		if (value)
			return "true";

		return "false";
	}
}
