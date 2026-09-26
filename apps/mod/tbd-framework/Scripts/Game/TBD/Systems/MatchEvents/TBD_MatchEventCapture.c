/**
 * @file TBD_MatchEventCapture.c
 * @brief Turns engine kill, life-state, seat and vehicle facts into detailed events and tally credits.
 *
 * Role: resolves the identities, relation, distance, weapon, cause, prefab and seat of an engine
 * fact, credits `TBD_MatchTelemetryTally` and captures the matching event through
 * `TBD_MatchEventRecorder`.  Position: called by `TBD_MatchTelemetryComponent` (kills and deaths),
 * `TBD_MatchCharacterWatch` (life states and seats) and the modded
 * `SCR_VehicleDamageManagerComponent` (destroyed vehicles); builds payloads with
 * `TBD_MatchEventWire`.
 * State: none.  Invariants: nothing is credited or captured unless the recorder records; an event
 * whose required identity is missing is skipped, and an optional identity that is missing is
 * omitted; identities come from `TBD_PlayerIdentity.GetArmaId`, the same bytes the results lines
 * carry; a suicide is a `self` death and never a kill.
 */

//! Engine facts to detailed events and tally credits.
//! @authority server
class TBD_MatchEventCapture
{
	//! A character's death with a player killer who is not the victim: credits the killer and
	//! captures `combat.kill`.
	//! @param context the engine's kill context
	//! @param victimPlayerId the victim's player id, or 0 or less for an AI character
	//! @authority server
	static void OnKill(notnull SCR_InstigatorContextData context, int victimPlayerId)
	{
		if (!TBD_MatchEventRecorder.IsRecording())
			return;

		int killerPlayerId = context.GetKillerPlayerID();
		IEntity killer = context.GetKillerEntity();
		IEntity victim = context.GetVictimEntity();
		bool teamKill = context.GetVictimKillerRelation() == SCR_ECharacterDeathStatusRelations.KILLED_BY_FRIENDLY_PLAYER;
		int distanceM = DistanceM(killer, victim);

		if (teamKill)
			TBD_MatchTelemetryTally.CreditTeamKill(killerPlayerId);
		else
			TBD_MatchTelemetryTally.CreditKill(killerPlayerId, distanceM);

		string killerArmaId = ArmaIdOf(killerPlayerId);
		if (killerArmaId.IsEmpty())
			return;

		string victimArmaId;
		if (victimPlayerId > 0)
			victimArmaId = ArmaIdOf(victimPlayerId);

		string weapon = WeaponOf(killer);
		TBD_MatchEventRecorder.Capture(TBD_MatchEventWire.KIND_COMBAT_KILL, TBD_MatchEventWire.CombatKill(killerArmaId,
			victimArmaId, victimPlayerId > 0, teamKill, distanceM, weapon));
	}

	//! A player's death without a player killer, or by their own hand: captures `combat.death`.
	//! @param context the engine's kill context
	//! @param victimPlayerId the dead player
	//! @authority server
	static void OnPlayerDeath(notnull SCR_InstigatorContextData context, int victimPlayerId)
	{
		if (!TBD_MatchEventRecorder.IsRecording())
			return;

		string victimArmaId = ArmaIdOf(victimPlayerId);
		if (victimArmaId.IsEmpty())
			return;

		TBD_MatchEventRecorder.Capture(TBD_MatchEventWire.KIND_COMBAT_DEATH,
			TBD_MatchEventWire.CombatDeath(victimArmaId, DeathCause(context, victimPlayerId)));
	}

	//! A player's character changed life state: ALIVE or DEAD to INCAPACITATED captures
	//! `medical.incapacitated`, INCAPACITATED to ALIVE captures `medical.revived`.
	//! @authority server
	static void OnLifeStateChanged(int playerId, ECharacterLifeState previousState, ECharacterLifeState newState)
	{
		string kind;
		if (newState == ECharacterLifeState.INCAPACITATED && previousState != ECharacterLifeState.INCAPACITATED)
			kind = TBD_MatchEventWire.KIND_MEDICAL_INCAPACITATED;
		else if (previousState == ECharacterLifeState.INCAPACITATED && newState == ECharacterLifeState.ALIVE)
			kind = TBD_MatchEventWire.KIND_MEDICAL_REVIVED;
		else
			return;

		if (!TBD_MatchEventRecorder.IsRecording())
			return;

		string armaId = ArmaIdOf(playerId);
		if (armaId.IsEmpty())
			return;

		TBD_MatchEventRecorder.Capture(kind, TBD_MatchEventWire.MedicalSubject(armaId));
	}

	//! A player took or left a vehicle seat: captures `vehicle.entered` or `vehicle.exited`.
	//! @param vehicle the entity the seat belongs to
	//! @param slot the seat, or null when the engine names none
	//! @param entered true for a seat taken, false for a seat left
	//! @authority server
	static void OnSeat(int playerId, IEntity vehicle, BaseCompartmentSlot slot, bool entered)
	{
		if (!TBD_MatchEventRecorder.IsRecording())
			return;

		string armaId = ArmaIdOf(playerId);
		if (armaId.IsEmpty())
			return;

		if (slot && slot.GetVehicle())
			vehicle = slot.GetVehicle();

		string prefab = TBD_LoadoutInventoryUtil.PrefabOf(vehicle);
		if (!TBD_MatchEventWire.Fits(prefab, TBD_MatchEventWire.PREFAB_MAX))
			return;

		string kind = TBD_MatchEventWire.KIND_VEHICLE_EXITED;
		if (entered)
			kind = TBD_MatchEventWire.KIND_VEHICLE_ENTERED;

		TBD_MatchEventRecorder.Capture(kind, TBD_MatchEventWire.VehicleSeat(armaId, prefab, CompartmentOf(slot)));
	}

	//! A vehicle reached the destroyed damage state: credits the instigating player and captures
	//! `vehicle.destroyed`.
	//! @param vehicle the destroyed vehicle
	//! @param instigator the damage manager's last instigator
	//! @authority server
	static void OnVehicleDestroyed(IEntity vehicle, notnull Instigator instigator)
	{
		if (!TBD_MatchEventRecorder.IsRecording())
			return;

		int playerId = instigator.GetInstigatorPlayerID();
		TBD_MatchTelemetryTally.CreditVehicleDestroyed(playerId);

		string prefab = TBD_LoadoutInventoryUtil.PrefabOf(vehicle);
		if (!TBD_MatchEventWire.Fits(prefab, TBD_MatchEventWire.PREFAB_MAX))
			return;

		TBD_MatchEventRecorder.Capture(TBD_MatchEventWire.KIND_VEHICLE_DESTROYED,
			TBD_MatchEventWire.VehicleDestroyed(prefab, ArmaIdOf(playerId)));
	}

	//! The `combat.death` cause: `self` for the victim's own hand, `ai` for an AI killer,
	//! `environment` when no entity instigated it, else `unknown`.
	//! @return one of the `TBD_MatchEventWire.CAUSE_` names
	static string DeathCause(notnull SCR_InstigatorContextData context, int victimPlayerId)
	{
		if (context.GetKillerPlayerID() == victimPlayerId)
			return TBD_MatchEventWire.CAUSE_SELF;

		SCR_ECharacterDeathStatusRelations relation = context.GetVictimKillerRelation();
		if (relation == SCR_ECharacterDeathStatusRelations.KILLED_BY_ENEMY_AI || relation == SCR_ECharacterDeathStatusRelations.KILLED_BY_FRIENDLY_AI)
			return TBD_MatchEventWire.CAUSE_AI;

		if (!context.GetKillerEntity())
			return TBD_MatchEventWire.CAUSE_ENVIRONMENT;

		return TBD_MatchEventWire.CAUSE_UNKNOWN;
	}

	//! The identity a payload names a player by: `TBD_PlayerIdentity.GetArmaId` when it fits the
	//! schema's bound.
	//! @return the identity, or empty for no player or no identity
	static string ArmaIdOf(int playerId)
	{
		if (playerId <= 0)
			return string.Empty;

		string armaId = TBD_PlayerIdentity.GetArmaId(playerId);
		if (!TBD_MatchEventWire.Fits(armaId, TBD_MatchEventWire.ARMA_ID_MAX))
			return string.Empty;

		return armaId;
	}

	//! Killer to victim distance.
	//! @return whole metres, 0 when either entity is gone
	static int DistanceM(IEntity killer, IEntity victim)
	{
		if (!killer || !victim)
			return 0;

		return TBD_Rounding.RoundToInt(vector.Distance(killer.GetOrigin(), victim.GetOrigin()));
	}

	//! The prefab of the weapon a killing character holds; a character seated in a vehicle fires
	//! the vehicle's weapon, which its hands do not tell, so none is named.
	//! @return the weapon prefab, or empty when it cannot be told
	static string WeaponOf(IEntity killer)
	{
		ChimeraCharacter character = ChimeraCharacter.Cast(killer);
		if (!character || character.IsInVehicle())
			return string.Empty;

		BaseWeaponManagerComponent weapons = BaseWeaponManagerComponent.Cast(character.FindComponent(BaseWeaponManagerComponent));
		if (!weapons)
			return string.Empty;

		BaseWeaponComponent current = weapons.GetCurrentWeapon();
		if (!current)
			return string.Empty;

		string prefab = TBD_LoadoutInventoryUtil.PrefabOf(current.GetOwner());
		if (!TBD_MatchEventWire.Fits(prefab, TBD_MatchEventWire.PREFAB_MAX))
			return string.Empty;

		return prefab;
	}

	//! The seat kind of `slot`.
	//! @return `pilot`, `turret`, `cargo`, or `other` when the slot is unknown
	static string CompartmentOf(BaseCompartmentSlot slot)
	{
		if (!slot)
			return "other";

		ECompartmentType type = slot.GetType();
		if (type == ECompartmentType.PILOT)
			return "pilot";

		if (type == ECompartmentType.TURRET)
			return "turret";

		if (type == ECompartmentType.CARGO)
			return "cargo";

		return "other";
	}
}
