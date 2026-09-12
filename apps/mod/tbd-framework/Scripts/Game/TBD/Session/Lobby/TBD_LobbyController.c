//! T-181.9.1 - the lobby wire: `SCR_PlayerController` (modded) - three asks, ONE reply.
//!
//! UI reorg 2026-09-12: this file used to also hold `TBD_LobbyClient` and `TBD_LobbyStage`;
//! they now live in `TBD_LobbyClient.c` (CLIENT roster cache + reconciliation) and
//! `TBD_LobbyStage.c` (CLIENT stage watcher that raises/drops the screen). Models are in
//! `TBD_LobbyData.c`, the server builder + wire in `TBD_LobbyService.c`.
//!
//! -- Why the transport hangs off SCR_PlayerController ----------------------------------------
//! The player controller is replicated and owned by exactly one client, so `RplRcver.Owner`
//! delivers a reply to the requester and to nobody else. Two precedents already in the tree do
//! this: `TBD_MissionBrowser.c` (admin mission list) and `TBD_BriefingController.c` (briefing
//! payload). This is the third `modded class SCR_PlayerController` block in the addon - see the
//! slice report; the compile gate proves it compiles, nothing here can prove how three blocks
//! behave at runtime.
//!
//! -- ONE reply, not three -------------------------------------------------------------------
//! Claim, release, deploy and a plain refresh all answer with the SAME message: a whole fresh
//! roster, optionally carrying a `V` verdict record saying what just happened. That is what makes
//! optimistic reconciliation a **wholesale replace** instead of a merge - the client never has to
//! reason about whether a partial update arrived before or after the refresh that overlapped it.
//! Out-of-order replies converge on the truth because every one of them IS the truth.
//!
//! -- Host vs dedicated ----------------------------------------------------------------------
//! On a listen host the requester IS the authority, so the request short-circuits and builds the
//! payload in place rather than round-tripping an RPC to itself. It still goes through
//! `Serialise` -> `Accept` -> `Parse`, so both topologies run one code path and a serialisation
//! bug cannot hide on the host.
modded class SCR_PlayerController
{
	// -- Roster ------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "what does the board look like right now".
	void TBD_RequestLobbyRoster()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_LobbyRoster);
			return;
		}

		TBD_LobbyClient.Accept(TBD_LobbyService.Serialise(TBD_LobbyService.BuildForPlayer(GetPlayerId())));
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_LobbyRoster()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_LobbyService.Serialise(TBD_LobbyService.BuildForPlayer(GetPlayerId())));
	}

	//! @authority owner - the ONE reply. Executes on the requesting client only (RplRcver.Owner).
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_LobbyRoster(string wire)
	{
		TBD_LobbyClient.Accept(wire);
	}

	// -- Claim -------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "I want this seat."
	//!
	//! The slot key is client-supplied and that is safe: `ClaimSlot` resolves it against the
	//! mission's own slot list and refuses anything it does not recognise, anything held by
	//! somebody else, and anything asked for by a dead player. A modified client can name any
	//! string it likes and gets a refusal.
	void TBD_RequestClaimSlot(string slotKey)
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_ClaimSlot, slotKey);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildClaimReply(GetPlayerId(), slotKey));
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ClaimSlot(string slotKey)
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildClaimReply(GetPlayerId(), slotKey));
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server - do it, then answer with the board as it stands AFTER doing it.
	//! Building the roster second is what makes a rejection self-explaining: the same message that
	//! says "no" already shows who does hold the seat.
	protected string TBD_BuildClaimReply(int playerId, string slotKey)
	{
		bool accepted;
		string reason = TBD_LobbyService.ApplyClaim(playerId, slotKey, accepted);

		TBD_LobbyRoster roster = TBD_LobbyService.BuildForPlayer(playerId);
		roster.m_sAction = TBD_LobbyService.ACTION_CLAIM;
		roster.m_bActionOk = accepted;
		roster.m_sActionReason = reason;
		roster.m_sActionKey = slotKey;

		return TBD_LobbyService.Serialise(roster);
	}

	// -- Release -----------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "I have changed my mind." No argument - you can only give up the
	//! seat you hold, and the server already knows which one that is.
	void TBD_RequestReleaseSlot()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_ReleaseSlot);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildReleaseReply(GetPlayerId()));
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ReleaseSlot()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildReleaseReply(GetPlayerId()));
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected string TBD_BuildReleaseReply(int playerId)
	{
		bool accepted;
		string reason = TBD_LobbyService.ApplyRelease(playerId, accepted);

		TBD_LobbyRoster roster = TBD_LobbyService.BuildForPlayer(playerId);
		roster.m_sAction = TBD_LobbyService.ACTION_RELEASE;
		roster.m_bActionOk = accepted;
		roster.m_sActionReason = reason;

		return TBD_LobbyService.Serialise(roster);
	}

	// -- Deploy ------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: the one consequential click. Takes no argument for the same
	//! reason release does not - the seat is server state, and a client cannot name one.
	void TBD_RequestDeploy()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_Deploy);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildDeployReply(GetPlayerId()));
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Deploy()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildDeployReply(GetPlayerId()));
	}

	//------------------------------------------------------------------------------------------------
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

		return TBD_LobbyService.Serialise(roster);
	}
}
