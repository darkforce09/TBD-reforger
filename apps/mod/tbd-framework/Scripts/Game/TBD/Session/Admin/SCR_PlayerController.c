/**
 * @file SCR_PlayerController.c
 * @brief The admin transport on the player controller: snapshot and action request and reply RPCs.
 *
 * Role: carries admin snapshot requests, admin action requests and the open-menu push between a
 * player's client and the server.  Position: TBD_AdminClient calls TBD_RequestAdminSnapshot and
 * TBD_RequestAdminAction; TBD_AdminCommands calls TBD_OpenAdminMenuOnOwner for `#tbd menu`; the
 * server side calls TBD_AdminSnapshotService and TBD_AdminService; replies land in TBD_AdminClient.
 * State: none on the controller.  Invariants: every server handler takes the caller from
 * GetPlayerId() of this replicated controller, never from an argument; the read gate is
 * TBD_AdminSnapshotService.BuildForAdmin and the write gate TBD_AdminService.Execute; every reply
 * goes to the owning client only; a listen host builds or runs in place instead of RPCing itself.
 */

//! Admin request and reply RPCs on the player controller. RplRcver.Owner answers exactly the one
//! client that asked, which a game-mode component cannot.
modded class SCR_PlayerController
{
	//! Ask the server for this player's admin snapshot. Off the authority it RPCs; on a listen host
	//! it builds in place and hands the result to TBD_AdminClient.
	//! @authority owner
	void TBD_RequestAdminSnapshot()
	{
		// Authority only -- the snapshot reads server-owned state (slot map, life ledger, mission
		// document), none of which exists in a client's process. Off the authority, ask for it.
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_AdminSnapshot);
			return;
		}

		TBD_AdminClient.Accept(TBD_AdminSnapshotService.BuildForAdmin(GetPlayerId()));
	}

	//! Build the snapshot the caller is entitled to and send it back. A non-admin's payload holds
	//! only the refusal, so the reply is safe to send unconditionally.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_AdminSnapshot()
	{
		Rpc(TBD_RpcDo_AdminSnapshot, TBD_AdminSnapshotService.Serialise(
			TBD_AdminSnapshotService.BuildForAdmin(GetPlayerId())));
	}

	//! Parse a snapshot on the requesting client and hand it to TBD_AdminClient.
	//! @param wire the serialised snapshot
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_AdminSnapshot(string wire)
	{
		TBD_AdminClient.Accept(TBD_AdminSnapshotService.Parse(wire));
	}

	//! Ask the server to run one admin power. The action crosses the wire as an int. On a listen
	//! host it runs in place and hands the verdict and a fresh snapshot to TBD_AdminClient.
	//! @param action the power
	//! @param targetId the player it acts on
	//! @authority owner
	void TBD_RequestAdminAction(TBD_EAdminAction action, int targetId)
	{
		int actionId = action;

		// Authority only -- the power itself runs server-side. Off the authority, ask for it.
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_AdminAction, actionId, targetId);
			return;
		}

		bool ok;
		string message = TBD_AdminService.Execute(GetPlayerId(), action, targetId, ok);
		TBD_AdminClient.AcceptActionResult(message, ok);
		TBD_AdminClient.Accept(TBD_AdminSnapshotService.BuildForAdmin(GetPlayerId()));
	}

	//! Run one admin power for the caller (GetPlayerId() of this controller) through
	//! TBD_AdminService.Execute, then send the verdict and a fresh snapshot back.
	//! @param actionId the action as sent; normalised by TBD_AdminService.FromWire
	//! @param targetId the player it acts on
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_AdminAction(int actionId, int targetId)
	{
		int playerId = GetPlayerId();

		bool ok;
		// The int came off the wire: normalise it to a known action (or NONE) before it is used as
		// one. Enfusion assigns any int to an enum without complaint, so this is the boundary.
		string message = TBD_AdminService.Execute(playerId, TBD_AdminService.FromWire(actionId), targetId, ok);

		Rpc(TBD_RpcDo_AdminActionResult, message, ok);

		// A fresh snapshot on the same round trip: the admin sees the outcome AND the world it
		// produced, without a poll interval of staleness in between.
		Rpc(TBD_RpcDo_AdminSnapshot, TBD_AdminSnapshotService.Serialise(
			TBD_AdminSnapshotService.BuildForAdmin(playerId)));
	}

	//! Hand the server's verdict to TBD_AdminClient on the requesting client.
	//! @param message the server's line
	//! @param ok whether the action worked
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_AdminActionResult(string message, bool ok)
	{
		TBD_AdminClient.AcceptActionResult(message, ok);
	}

	//! Raise the admin screen on this player's client, for `#tbd menu`, so the screen is reachable
	//! with no key bound. On a listen host, where this is the local controller, it opens in place.
	//! @authority server
	void TBD_OpenAdminMenuOnOwner()
	{
		// Listen host: the requesting admin is this machine, and an owner RPC from the authority to
		// itself is not relied on. "Is this the local controller" is true on a host and false on a
		// dedicated server.
		if (GetGame().GetWorkspace() && GetGame().GetPlayerController() == this)
		{
			TBD_AdminClient.Open();
			return;
		}

		Rpc(TBD_RpcDo_OpenAdminMenu);
	}

	//! Open the admin screen on the owning client.
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_OpenAdminMenu()
	{
		TBD_AdminClient.Open();
	}
}
