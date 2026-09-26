/**
 * @file SCR_VehicleDamageManagerComponent.c
 * @brief Reports each vehicle that reaches the destroyed damage state to match event capture.
 *
 * Role: forwards a vehicle's transition into `EDamageState.DESTROYED`, with its last instigator,
 * to `TBD_MatchEventCapture.OnVehicleDestroyed`.  Position: a modded vanilla damage manager on
 * every vehicle; capture records only in a framework world with a LIVE registered round.
 * State: none.  Invariants: runs on the authority only; one report per transition into the
 * destroyed state, never for a replicated join-in-progress state or a vehicle already destroyed.
 */

//! Destroyed-vehicle hook of detailed match events.
modded class SCR_VehicleDamageManagerComponent
{
	//! Calls super, then reports a fresh transition into the destroyed state on the authority.
	//! @param newState the damage state entered
	//! @param previousDamageState the damage state left
	//! @param isJIP true when the state arrives with a join-in-progress snapshot
	//! @authority server
	override void OnDamageStateChanged(EDamageState newState, EDamageState previousDamageState, bool isJIP)
	{
		super.OnDamageStateChanged(newState, previousDamageState, isJIP);

		if (newState != EDamageState.DESTROYED || previousDamageState == EDamageState.DESTROYED || isJIP)
			return;

		if (TBD_Authority.IsClient())
			return;

		TBD_MatchEventCapture.OnVehicleDestroyed(GetOwner(), GetInstigator());
	}
}
