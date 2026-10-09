/**
 * @file SCR_PlayerController.c
 * @brief Audio wire: the emitter and music cue RPCs on the player controller.
 *
 * Role: carries server-armed emitters and fired cues to the client that plays them; the server has
 * no audio device.  Position: `TBD_AudioEmitter` calls `TBD_PushAudioEmitter` and
 * `TBD_PushAudioCue` on the server; the owner spawns a source through `TBD_AudioLocalSources` or
 * plays the cue through `SCR_UISoundEntity`.
 * State: none.  Invariants: overrides no vanilla method and every symbol is `TBD_`-prefixed; a
 * listen host runs its own controller's delivery in place, since an owner RPC is not delivered to
 * the machine that sends it.
 */

//! Audio emitter and music cue RPCs of the player controller.
modded class SCR_PlayerController
{
	//! Start one positional emitter on this controller's owning client: in place when this is the
	//! local controller (listen host), otherwise by owner RPC. Empty `id` or `sound` sends nothing.
	//! @param id emitter id
	//! @param x world X in metres
	//! @param y metres above sea level, or `TBD_AudioEmitter.ABSENT`
	//! @param z world Z in metres
	//! @param radiusM listener radius in metres
	//! @param loop true to repeat inside the radius
	//! @param sound `SCR_SoundEvent` name
	//! @authority server
	void TBD_PushAudioEmitter(string id, float x, float y, float z, float radiusM, bool loop, string sound)
	{
		if (id.IsEmpty() || sound.IsEmpty())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			TBD_AudioLocalSources.Spawn(id, x, y, z, radiusM, loop, sound);
			return;
		}

		Rpc(TBD_RpcDo_AudioEmitter, id, x, y, z, radiusM, loop, sound);
	}

	//! Spawn the emitter's local source on the owning client.
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_AudioEmitter(string id, float x, float y, float z, float radiusM, bool loop, string sound)
	{
		TBD_AudioLocalSources.Spawn(id, x, y, z, radiusM, loop, sound);
	}

	//! Play a 2D music cue on this controller's owning client: in place when this is the local
	//! controller (listen host), otherwise by owner RPC. An empty `track` sends nothing.
	//! @param track `SCR_SoundEvent` name
	//! @authority server
	void TBD_PushAudioCue(string track)
	{
		if (track.IsEmpty())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			SCR_UISoundEntity.SoundEvent(track);
			return;
		}

		Rpc(TBD_RpcDo_AudioCue, track);
	}

	//! Play the cue on the owning client.
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_AudioCue(string track)
	{
		SCR_UISoundEntity.SoundEvent(track);
	}
}
