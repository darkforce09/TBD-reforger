/**
 * @file TBD_MatchTelemetryComponent.c
 * @brief Game mode component that feeds the engine's kill, death and player-character hooks to capture.
 *
 * Role: routes each player death, each AI character death with a player killer, and each player
 * character's life-state and seat changes to `TBD_MatchEventCapture`.  Position: on the game mode
 * prefab (`TBD_GameMode.et`); the engine dispatches `OnPlayerKilled`, `OnControllableDestroyed` and
 * `OnControllableDeleted` to it, and the game mode's `GetOnPlayerSpawned` invoker reports every
 * spawn, the framework's deploys included.
 * State: one `TBD_MatchCharacterWatch` per living player character, on the server.
 * Invariants: everything runs on the authority only; a player's death is handled once, by
 * `OnPlayerKilled`, never again by `OnControllableDestroyed`; every watch is detached when its
 * character is deleted or the component goes.
 */

[ComponentEditorProps(category: "TBD/Framework", description: "TBD match telemetry -- detailed events and the round's combat tally.")]
//! Component class of `TBD_MatchTelemetryComponent`; carries no data.
class TBD_MatchTelemetryComponentClass : SCR_BaseGameModeComponentClass {}

//! The engine hooks of detailed match events.
class TBD_MatchTelemetryComponent : SCR_BaseGameModeComponent
{
	protected ref map<IEntity, ref TBD_MatchCharacterWatch> m_mWatches = new map<IEntity, ref TBD_MatchCharacterWatch>(); //!< player character -> its subscriptions

	//! Subscribe to the game mode's spawn invoker; components have no spawn virtual.
	//! @param owner the game mode entity
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(owner);
		if (gameMode)
			gameMode.GetOnPlayerSpawned().Insert(OnPlayerSpawned);
	}

	//! Unsubscribe the spawn invoker and detach every character watch.
	//! @param owner the game mode entity
	override void OnDelete(IEntity owner)
	{
		SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(owner);
		if (gameMode)
			gameMode.GetOnPlayerSpawned().Remove(OnPlayerSpawned);

		foreach (IEntity character, TBD_MatchCharacterWatch watch : m_mWatches)
		{
			watch.Detach();
		}

		m_mWatches.Clear();
		super.OnDelete(owner);
	}

	//! Watch the character a player spawned into; a second report of one character is ignored.
	//! @param playerId the spawned player
	//! @param controlledEntity the character they control
	//! @authority server
	protected void OnPlayerSpawned(int playerId, IEntity controlledEntity)
	{
		if (TBD_Authority.IsClient() || !controlledEntity || m_mWatches.Contains(controlledEntity))
			return;

		TBD_MatchCharacterWatch watch = new TBD_MatchCharacterWatch(controlledEntity, playerId);
		if (watch.Attach())
			m_mWatches.Set(controlledEntity, watch);
	}

	//! A player died: a kill by another player, or a death by AI, environment or their own hand.
	//! @param instigatorContextData the engine's kill context
	//! @authority server
	override void OnPlayerKilled(notnull SCR_InstigatorContextData instigatorContextData)
	{
		super.OnPlayerKilled(instigatorContextData);

		if (TBD_Authority.IsClient())
			return;

		int victimPlayerId = instigatorContextData.GetVictimPlayerID();
		int killerPlayerId = instigatorContextData.GetKillerPlayerID();
		if (killerPlayerId > 0 && killerPlayerId != victimPlayerId)
			TBD_MatchEventCapture.OnKill(instigatorContextData, victimPlayerId);
		else
			TBD_MatchEventCapture.OnPlayerDeath(instigatorContextData, victimPlayerId);
	}

	//! A controllable entity was destroyed: an AI character killed by a player is a kill; a player's
	//! character is left to `OnPlayerKilled`, and anything that is not a character is ignored.
	//! @param instigatorContextData the engine's destruction context
	//! @authority server
	override void OnControllableDestroyed(notnull SCR_InstigatorContextData instigatorContextData)
	{
		super.OnControllableDestroyed(instigatorContextData);

		if (TBD_Authority.IsClient())
			return;

		IEntity victim = instigatorContextData.GetVictimEntity();
		if (!ChimeraCharacter.Cast(victim) || IsPlayerCharacter(victim, instigatorContextData))
			return;

		if (instigatorContextData.GetKillerPlayerID() > 0)
			TBD_MatchEventCapture.OnKill(instigatorContextData, 0);
	}

	//! Detach the watch of a character about to be deleted.
	//! @param entity the entity being deleted
	//! @authority server
	override void OnControllableDeleted(IEntity entity)
	{
		super.OnControllableDeleted(entity);

		TBD_MatchCharacterWatch watch = m_mWatches.Get(entity);
		if (!watch)
			return;

		watch.Detach();
		m_mWatches.Remove(entity);
	}

	//! Whether `victim` is a player's character: one a player spawned into, one the context names
	//! a player for, or one a player controls.
	//! @return true for a player's character
	protected bool IsPlayerCharacter(IEntity victim, notnull SCR_InstigatorContextData instigatorContextData)
	{
		if (m_mWatches.Contains(victim) || instigatorContextData.GetVictimPlayerID() > 0)
			return true;

		return GetGame().GetPlayerManager().GetPlayerIdFromControlledEntity(victim) > 0;
	}
}
