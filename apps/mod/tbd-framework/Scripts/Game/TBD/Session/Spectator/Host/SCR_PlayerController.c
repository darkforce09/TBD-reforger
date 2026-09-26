/**
 * @file SCR_PlayerController.c
 * @brief Spectator streaming host transport on the player controller: the camera-position RPC.
 *
 * Role: carries the owning client's spectator camera position to the server, where
 * TBD_SpectatorHost.MoveTo moves that player's streaming host.
 * Position: TBD_SpectatorHostReporter calls TBD_ReportSpectatorCamera on the owning client; the
 * server-side handler feeds TBD_SpectatorHost.MoveTo.
 * State: none.
 * Invariants: the RPC names no player, so the server moves only the host of GetPlayerId(); the
 * position is untrusted and MoveTo validates and clamps it; on a listen host the requester is the
 * authority and the call runs in place; the method names differ from every other modded block of
 * SCR_PlayerController, which share one namespace.
 */

//! Spectator streaming host transport of the player controller.
modded class SCR_PlayerController
{
	//! Owner to server: "my spectator camera is here; put my streaming host there". The position is
	//! client-supplied and untrusted: TBD_SpectatorHost.MoveTo refuses a player who is not dead, has
	//! no host or whose connection epoch has moved on, and clamps the rest to the world box and the
	//! range leash, so a modified client only chooses what is streamed to itself. Unreliable, since a
	//! dropped sample is corrected by the next one and a reliable channel would replay stale ones.
	//! @param position the camera position, world metres
	//! @authority owner
	void TBD_ReportSpectatorCamera(vector position)
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_SpectatorHostAt, position);
			return;
		}

		// Listen host: the requester is the authority, so the move runs in place rather than as an
		// RPC to itself; both topologies share one path.
		TBD_SpectatorHost.MoveTo(GetPlayerId(), position);
	}

	//! Server handler: move the host of the player who owns this controller. GetPlayerId() is the
	//! authority's own answer, so the RPC cannot move somebody else's host.
	//! @param position the client-supplied camera position, world metres
	//! @authority server
	//! @rpc Unreliable Server
	[RplRpc(RplChannel.Unreliable, RplRcver.Server)]
	protected void TBD_RpcAsk_SpectatorHostAt(vector position)
	{
		TBD_SpectatorHost.MoveTo(GetPlayerId(), position);
	}
}
