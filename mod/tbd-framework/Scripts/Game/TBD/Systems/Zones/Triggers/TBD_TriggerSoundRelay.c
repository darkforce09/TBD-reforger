/**
 * @file TBD_TriggerSoundRelay.c
 * @brief Carries a trigger's `play_sound` cue from the server to each addressed client.
 *
 * Role: the server-to-owner transport of the `play_sound` effect; the server has no audio device,
 * so the event name is played on the client that hears it.  Position: `TBD_TriggerEffects` calls
 * `PushToAudience` on the server; the modded `SCR_PlayerController` plays the cue on the owner.
 * State: none.  Invariants: a listen host plays its own cue in place, since an owner RPC is not
 * delivered to the machine that sends it.
 */

//! Server-side fan-out of a trigger sound cue.
//! @authority server
class TBD_TriggerSoundRelay
{
	//! Push `soundEvent` to each player in `audience` that has a player controller.
	//! @param players the player manager
	//! @param audience the addressed player ids
	//! @param soundEvent an `SCR_SoundEvent` name
	//! @return how many controllers the cue was handed to
	//! @authority server
	static int PushToAudience(notnull PlayerManager players, notnull array<int> audience, string soundEvent)
	{
		int sent = 0;
		foreach (int playerId : audience)
		{
			SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(playerId));
			if (!controller)
				continue;

			controller.TBD_PushTriggerSound(soundEvent);
			sent++;
		}

		return sent;
	}
}

//! Plays a trigger's sound cue on the owning client.
modded class SCR_PlayerController
{
	//! Play `soundEvent` on this controller's owner: in place on a listen host's own controller,
	//! otherwise through the owner RPC. An empty name does nothing.
	//! @param soundEvent an `SCR_SoundEvent` name
	//! @authority server
	void TBD_PushTriggerSound(string soundEvent)
	{
		if (soundEvent.IsEmpty())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			SCR_UISoundEntity.SoundEvent(soundEvent);
			return;
		}

		Rpc(TBD_RpcDo_TriggerSound, soundEvent);
	}

	//! Raise `soundEvent` as a 2D UI sound on the addressed client.
	//! @param soundEvent an `SCR_SoundEvent` name
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_TriggerSound(string soundEvent)
	{
		SCR_UISoundEntity.SoundEvent(soundEvent);
	}
}
