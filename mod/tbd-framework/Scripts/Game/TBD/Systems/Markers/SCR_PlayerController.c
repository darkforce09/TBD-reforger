/**
 * @file SCR_PlayerController.c
 * @brief Marker wire: one request and reply RPC pair on the player controller.
 *
 * Role: carries a client's argument-free marker request to `TBD_MarkerService.BuildForPlayer`
 * and the side-scoped answer back.  Position: `TBD_MarkerClient.Request` calls
 * `TBD_RequestMarkers` on the owning client; the answer lands in `TBD_MarkerClient.Accept`.
 * State: none.  Invariants: the controller is owned by exactly one client, so `RplRcver.Owner`
 * delivers the answer to the requester and nobody else; the request has no faction parameter to
 * forge; on a listen host the requester is the authority and the answer is built in place; the
 * rows travel as parallel arrays, never a delimited string; overrides no vanilla method and every
 * symbol is `TBD_`-prefixed.
 */

//! Marker request and reply RPCs of the player controller.
modded class SCR_PlayerController
{
	//! Ask the authority for this player's markers: by RPC from a client, in place on the
	//! authority (listen host), where the answer goes straight to `TBD_MarkerClient.Accept`.
	//! @authority owner
	void TBD_RequestMarkers()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_Markers);
			return;
		}

		TBD_MarkerWire wire = TBD_MarkerService.BuildForPlayer(GetPlayerId());
		TBD_MarkerStyleCodec.PackIntoX(wire);
		TBD_MarkerClient.Accept(wire.m_aX, wire.m_aZ, wire.m_aIcon, wire.m_aLabel,
			wire.m_sFactionKey, wire.m_sMissionId, wire.m_bServed);
	}

	//! Build the caller's marker set from server-owned slot state and answer the caller only.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Markers()
	{
		TBD_MarkerWire wire = TBD_MarkerService.BuildForPlayer(GetPlayerId());

		TBD_MarkerStyleCodec.PackIntoX(wire);
		Rpc(TBD_RpcDo_Markers, wire.m_aX, wire.m_aZ, wire.m_aIcon, wire.m_aLabel,
			wire.m_sFactionKey, wire.m_sMissionId, wire.m_bServed);
	}

	//! Hand the answer to `TBD_MarkerClient.Accept` on the requesting client.
	//! @param xs world X per marker, then the style trailer
	//! @param zs world Z per marker
	//! @param icons authored icon per marker
	//! @param labels caption per marker
	//! @param factionKey the side the rows belong to
	//! @param missionId the mission the rows belong to
	//! @param served false when the server had no authoritative answer
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_Markers(array<int> xs, array<int> zs, array<string> icons,
		array<string> labels, string factionKey, string missionId, bool served)
	{
		TBD_MarkerClient.Accept(xs, zs, icons, labels, factionKey, missionId, served);
	}
}
