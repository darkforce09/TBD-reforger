/**
 * @file TBD_RadioBridgeStub.c
 * @brief Hook surface for a partner VOIP bridge, and the live stage hook of the native radio.
 *
 * Role: the documented subscription points of the external-bridge contract
 * (`documentation/contracts/definitions/bridge_messages.md`) plus two delegates into
 * `TBD_RadioService`.  Position: `TBD_FrameworkManager` calls `OnStageChanged` on every stage
 * transition; nothing calls the other hooks.
 * State: none.  Invariants: `tbd-framework` takes no workshop dependencies
 * (`documentation/mod/tbd-framework/mod_design.md` sections 2 and 6), so there is no partner
 * bridge and `OnPlayerSpawned`, `OnPlayerKilled`, `OnRadioRetune` and `OnPTT` are empty; the nets
 * are read, served, displayed and tuned natively by `TBD_RadioPlan`, `TBD_RadioService`,
 * `TBD_RadioClient` and `TBD_RadioTuner`.
 */

//! Bridge hooks and the native radio's stage delegate.
//! @authority server
class TBD_RadioBridgeStub
{
	//! Bridge hook: a player entered the world in a slot. Empty; has no call site.
	//! @param identityId the player's platform identity
	//! @param radioNetIds the caller's idea of the player's nets; never trusted
	//! @authority server
	static void OnPlayerSpawned(string identityId, array<string> radioNetIds)
	{
		// Partner bridge (if one ever exists) subscribes here.
	}

	//! Serve and tune a spawned player's nets through `TBD_RadioService.BuildForPlayer`, which
	//! resolves them from the assigned slot. Does nothing on a client; has no call site.
	//! @param playerId the spawned player
	//! @authority server
	static void OnPlayerSpawnedById(int playerId)
	{
		if (TBD_Authority.IsClient())
			return;

		TBD_RadioService.BuildForPlayer(playerId);
	}

	//! Bridge hook: a player died. Empty; moving the dead to a voice channel is a bridge concern.
	//! @param identityId the player's platform identity
	static void OnPlayerKilled(string identityId)
	{
	}

	//! Bridge hook: a player changed radio or frequency. Empty; has no call site.
	//! @param identityId the player's platform identity
	//! @param netId the caller's idea of the net; never trusted
	//! @authority server
	static void OnRadioRetune(string identityId, string netId)
	{
	}

	//! Bridge hook: push-to-talk. Empty; push-to-talk routing belongs to a bridge.
	//! @param identityId the player's platform identity
	//! @param netId the net keyed
	//! @param pressed true on press, false on release
	static void OnPTT(string identityId, string netId, bool pressed)
	{
	}

	//! Delegate a stage transition to `TBD_RadioService.OnStageChanged`, which serves and tunes
	//! every connected player at SAFE_START and LIVE.
	//! @param stage the stage entered
	//! @authority server
	static void OnStageChanged(TBD_EGameStage stage)
	{
		TBD_RadioService.OnStageChanged(stage);
	}
}
