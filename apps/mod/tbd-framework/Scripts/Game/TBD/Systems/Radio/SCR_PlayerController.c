/**
 * @file SCR_PlayerController.c
 * @brief Radio wire: the net request, reply and push RPCs on the player controller.
 *
 * Role: carries a client's argument-free net request to `TBD_RadioService.BuildForPlayer`, and
 * the side-scoped answer (and the stage sweep's unprompted push) to `TBD_RadioClient.Accept`.
 * Position: `TBD_RadioClient` calls `TBD_RequestRadioNets` on the owning client;
 * `TBD_RadioService` calls `TBD_PushRadioNets` on the server.
 * State: none.  Invariants: frequencies are side-scoped intelligence and `RplRcver.Owner` delivers
 * only to the requester; the request has no faction parameter to forge; on a listen host the
 * requester is the authority and both request and push run in place, since an owner RPC is not
 * delivered to the machine that sends it; nets travel as parallel arrays, never a delimited
 * string; the reply uses the eight-parameter `Rpc()` ceiling, so the faction key stays on the
 * server; overrides no vanilla method and every symbol is `TBD_`-prefixed.
 */

//! Radio net RPCs of the player controller.
modded class SCR_PlayerController
{
	//! Ask the authority for this player's nets: by RPC from a client, in place on the authority
	//! (listen host), where the answer goes straight to `TBD_RadioClient.Accept`.
	//! @authority owner
	void TBD_RequestRadioNets()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_RadioNets);
			return;
		}

		TBD_RadioWire wire = TBD_RadioService.BuildForPlayer(GetPlayerId());
		TBD_RadioClient.Accept(wire.m_aId, wire.m_aLabel, wire.m_aFreqKHz, wire.m_aLongRange,
			wire.m_sMissionId, wire.m_sTuneResult, wire.m_iTuned, wire.m_bServed);
	}

	//! Push a wire to this controller's owner unprompted, so the net list shows the stage sweep's
	//! tune result (the client's poll has stopped once served). The local controller (listen
	//! host) takes it in place. Does nothing on a client.
	//! @param wire the answer the sweep built
	//! @authority server
	void TBD_PushRadioNets(notnull TBD_RadioWire wire)
	{
		if (TBD_Authority.IsClient())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			TBD_RadioClient.Accept(wire.m_aId, wire.m_aLabel, wire.m_aFreqKHz, wire.m_aLongRange,
				wire.m_sMissionId, wire.m_sTuneResult, wire.m_iTuned, wire.m_bServed);
			return;
		}

		Rpc(TBD_RpcDo_RadioNets, wire.m_aId, wire.m_aLabel, wire.m_aFreqKHz, wire.m_aLongRange,
			wire.m_sMissionId, wire.m_sTuneResult, wire.m_iTuned, wire.m_bServed);
	}

	//! Build (and tune) the caller's nets from server-owned slot state and answer the caller only.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_RadioNets()
	{
		TBD_RadioWire wire = TBD_RadioService.BuildForPlayer(GetPlayerId());

		Rpc(TBD_RpcDo_RadioNets, wire.m_aId, wire.m_aLabel, wire.m_aFreqKHz, wire.m_aLongRange,
			wire.m_sMissionId, wire.m_sTuneResult, wire.m_iTuned, wire.m_bServed);
	}

	//! Hand the answer to `TBD_RadioClient.Accept` on the owning client.
	//! @param ids net id per net
	//! @param labels net label per net
	//! @param freqKHz frequency in kHz per net
	//! @param longRange 1 for a long-range net, 0 otherwise, per net
	//! @param missionId the mission the nets belong to
	//! @param tuneResult the server's tune outcome name (`NO_BACKBONE` on a world without a radio
	//! manager)
	//! @param tuned how many radios the server tuned
	//! @param served false when the server had no authoritative answer
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_RadioNets(array<string> ids, array<string> labels, array<int> freqKHz,
		array<int> longRange, string missionId, string tuneResult, int tuned, bool served)
	{
		TBD_RadioClient.Accept(ids, labels, freqKHz, longRange, missionId, tuneResult, tuned, served);
	}
}
