/**
 * @file SCR_PlayerController.c
 * @brief The lobby's roster RPCs on the player controller: three asks and one reply.
 *
 * Role: carries a roster refresh, a claim, a release and a deploy from the owning client to the
 * server, runs the seat action through TBD_LobbyService, and answers with the whole roster after
 * the action plus a `V` verdict record.  Position: called by TBD_LobbyClient on the client; the
 * reply goes to TBD_LobbyClient.Accept on the requester only (RplRcver.Owner).
 * State: none.  Invariants: the caller is always GetPlayerId() of the controller the request
 * arrived on, never a client-supplied id; a client-supplied slot key is judged by
 * TBD_SpawnManager.ClaimSlot; every reply is a whole roster, so the client replaces rather than
 * merges; a listen host builds the reply in place and still goes through Serialise, Accept and
 * Parse.
 */

//! Lobby roster RPCs on the modded player controller.
modded class SCR_PlayerController
{
	//! Ask for the roster as it stands; on a listen host the reply is built and accepted in place.
	//! @authority owner
	void TBD_RequestLobbyRoster()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_LobbyRoster);
			return;
		}

		TBD_LobbyClient.Accept(TBD_LobbyRosterWire.Serialise(TBD_LobbyService.BuildForPlayer(GetPlayerId())));
	}

	//! Answer the caller with their roster.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_LobbyRoster()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_LobbyRosterWire.Serialise(TBD_LobbyService.BuildForPlayer(GetPlayerId())));
	}

	//! The one reply, on the requesting client only: hand the wire to TBD_LobbyClient.Accept.
	//! @param wire a TBD_LobbyRosterWire string
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_LobbyRoster(string wire)
	{
		TBD_LobbyClient.Accept(wire);
	}


	//! Ask for `slotKey`. The key is client-supplied and that is safe: ClaimSlot refuses any key the mission does not have, a seat another player holds and a dead player.
	//! @authority owner
	void TBD_RequestClaimSlot(string slotKey)
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_ClaimSlot, slotKey);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildClaimReply(GetPlayerId(), slotKey));
	}

	//! Claim `slotKey` for the caller and answer with the roster and verdict.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ClaimSlot(string slotKey)
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildClaimReply(GetPlayerId(), slotKey));
	}

	//! Claim, then build the roster after the claim, so a refusal already shows who holds the seat.
	//! @return the serialised roster with a CLAIM verdict keyed on `slotKey`
	//! @authority server
	protected string TBD_BuildClaimReply(int playerId, string slotKey)
	{
		bool accepted;
		string reason = TBD_LobbyService.ApplyClaim(playerId, slotKey, accepted);

		TBD_LobbyRoster roster = TBD_LobbyService.BuildForPlayer(playerId);
		roster.m_sAction = TBD_LobbyService.ACTION_CLAIM;
		roster.m_bActionOk = accepted;
		roster.m_sActionReason = reason;
		roster.m_sActionKey = slotKey;

		return TBD_LobbyRosterWire.Serialise(roster);
	}


	//! Ask to give up the caller's seat; takes no argument because the server knows which seat that is.
	//! @authority owner
	void TBD_RequestReleaseSlot()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_ReleaseSlot);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildReleaseReply(GetPlayerId()));
	}

	//! Release the caller's seat and answer with the roster and verdict.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ReleaseSlot()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildReleaseReply(GetPlayerId()));
	}

	//! Release, then build the roster after the release.
	//! @return the serialised roster with a RELEASE verdict
	//! @authority server
	protected string TBD_BuildReleaseReply(int playerId)
	{
		bool accepted;
		string reason = TBD_LobbyService.ApplyRelease(playerId, accepted);

		TBD_LobbyRoster roster = TBD_LobbyService.BuildForPlayer(playerId);
		roster.m_sAction = TBD_LobbyService.ACTION_RELEASE;
		roster.m_bActionOk = accepted;
		roster.m_sActionReason = reason;

		return TBD_LobbyRosterWire.Serialise(roster);
	}


	//! Ask to deploy into the caller's seat; takes no argument because the seat is server state.
	//! @authority owner
	void TBD_RequestDeploy()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_Deploy);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildDeployReply(GetPlayerId()));
	}

	//! Deploy the caller and answer with the roster and verdict.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Deploy()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildDeployReply(GetPlayerId()));
	}

	//! Deploy, then build the roster after the deploy.
	//! @return the serialised roster with a DEPLOY verdict whose key is the TBD_EDeployResult name
	//! @authority server
	protected string TBD_BuildDeployReply(int playerId)
	{
		bool accepted;
		string resultName;
		string reason = TBD_LobbyService.ApplyDeploy(playerId, accepted, resultName);

		TBD_LobbyRoster roster = TBD_LobbyService.BuildForPlayer(playerId);
		roster.m_sAction = TBD_LobbyService.ACTION_DEPLOY;
		roster.m_bActionOk = accepted;
		roster.m_sActionReason = reason;
		roster.m_sActionKey = resultName;

		return TBD_LobbyRosterWire.Serialise(roster);
	}
}
