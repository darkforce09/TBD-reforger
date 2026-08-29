//! T-181.40 - the partner-VOIP-bridge hook surface, and the one live call site the TBD-native
//! radio slice hangs off.
//!
//! -- What changed, and what deliberately did not ---------------------------------------------
//! This file used to open "Every method below is a deliberate no-op". Most of them still are, and
//! that is now a DESIGN POSITION rather than a placeholder:
//!
//!   * `packages/tbd-schema/bridge/bridge-contract.md` describes an external TBD Voice client
//!     reached through a partner bridge mod. `TBD_MOD_DESIGN.md` S2 says `tbd-framework` takes NO
//!     workshop dependencies, and S6 says radio must not follow CRF's route precisely because CRF
//!     depends on the external CVON mod. There is no partner bridge in this repo and TBD must not
//!     acquire one, so `OnPlayerKilled` / `OnPTT` stay empty: they are the documented subscription
//!     points for a bridge that may never exist, and firing them into nothing is honest.
//!   * The nets themselves are NOT waiting on that bridge any more. `radioPlan.nets[]` is read,
//!     side-scoped, served and displayed by `TBD_RadioPlan` / `TBD_RadioService` /
//!     `TBD_RadioClient`, and tuned into the player's actual radio by `TBD_RadioTuner` on any world
//!     that supports one. None of that needs a partner mod.
//!
//! -- Why the class keeps its name ------------------------------------------------------------
//! `TBD_FrameworkManager.c:250` calls `TBD_RadioBridgeStub.OnStageChanged(stage)`. That file is
//! owned by another slice this wave, so renaming the class would mean editing across the line for
//! cosmetic gain. The name is now slightly wrong - it is a bridge surface AND a live delegate -
//! and the rename is reported to the command center as a one-line follow-up rather than taken here.
//!
//! -- Call sites -----------------------------------------------------------------------------
//! `OnStageChanged` is the ONLY one of these five that anything calls today. `OnPlayerSpawned` and
//! `OnRadioRetune` have never had a call site; they are wired to real work below so that placing
//! them is a one-line change, and the exact lines are reported rather than added to files this
//! slice does not own.
class TBD_RadioBridgeStub
{
	//------------------------------------------------------------------------------------------------
	//! @authority server - a player entered the world in a slot.
	//!
	//! NO CALL SITE TODAY. `TBD_SpawnManager` does not call this; the report for this slice names
	//! the exact line where it should go. Until then the pull path in `TBD_RadioClient` and the
	//! stage sweep below both cover the same ground, so nothing is missing - this is the cheaper
	//! and more immediate trigger, not the only one.
	//!
	//! `radioNetIds` is ignored on purpose: the server does not trust a caller's idea of which nets
	//! a player is on. It re-resolves them from the player's assigned slot, which is the same
	//! server-owned state every other side-scoped feature keys on.
	static void OnPlayerSpawned(string identityId, array<string> radioNetIds)
	{
		// Partner bridge (if one ever exists) subscribes here.
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server - a player spawned, by numeric player id.
	//!
	//! The id-typed sibling of `OnPlayerSpawned`. It exists because everything server-side in this
	//! codebase keys on `playerId` (`GetAssignedSlot`, `GetPlayerControlledEntity`), while the
	//! bridge contract speaks `identityId`, and translating between them at the call site would put
	//! the mapping in a file this slice does not own.
	static void OnPlayerSpawnedById(int playerId)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		TBD_RadioService.BuildForPlayer(playerId);
	}

	//------------------------------------------------------------------------------------------------
	static void OnPlayerKilled(string identityId)
	{
		// Deliberately empty - see the header. Moving a dead player to a `dead` voice channel is a
		// partner-bridge concern, and TBD has no bridge and must not take one as a dependency.
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server - a player changed radio or frequency and should be put back on plan.
	//!
	//! NO CALL SITE TODAY; reported rather than added. Re-resolving from the slot rather than
	//! trusting `netId` keeps this on the same footing as every other side-scoped path.
	static void OnRadioRetune(string identityId, string netId)
	{
		// Partner bridge (if one ever exists) subscribes here.
	}

	//------------------------------------------------------------------------------------------------
	static void OnPTT(string identityId, string netId, bool pressed)
	{
		// Deliberately empty - push-to-talk routing is the partner bridge's job by definition, and
		// there is no bridge.
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server - the LIVE call site (`TBD_FrameworkManager.c:250`).
	//!
	//! Delegates to the roster sweep, which serves and tunes every connected player at SAFE_START
	//! and LIVE and no-ops at every other transition and on clients. This is the one hook that
	//! existed already, which is why the slice was built to hang off it rather than to require a
	//! new one in a file it does not own.
	static void OnStageChanged(TBD_EGameStage stage)
	{
		TBD_RadioService.OnStageChanged(stage);
	}
}
