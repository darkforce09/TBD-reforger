/**
 * @file SCR_PlayerController.c
 * @brief The HUD transport on the player controller: objective and assigned-task snapshots.
 *
 * Role: carries the two HUD RPC pairs, each a Server ask and an Owner answer, so a snapshot
 * reaches exactly one client.
 * Position: `TBD_ObjectiveHudPublisher` pushes objective snapshots and `TBD_TaskHud` pushes and
 * requests task snapshots through here; the answers land in `TBD_ObjectiveHud.Accept` and
 * `TBD_TaskHud.Accept`.
 * State: none.
 * Invariants: a push is ignored on a client; the host's own controller applies its snapshot
 * locally instead of sending an RPC to itself.
 */

//! HUD snapshot transport: objective board and assigned-task markers.
modded class SCR_PlayerController
{
	//! Ask for this player's objective snapshot: a client asks the server over RPC; the server (or a
	//! listen host) has the objectives runner push it directly.
	//! @authority owner
	void TBD_RequestObjectiveHud()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_ObjectiveHud);
			return;
		}

		TBD_ObjectivesComponent runner = TBD_ObjectivesComponent.GetInstance();
		if (runner)
			runner.PushHudTo(GetPlayerId());
	}

	//! Deliver one snapshot to this controller's player: accepted locally for the host's own
	//! controller, else sent over the Owner RPC. Does nothing on a client.
	//! @authority server
	void TBD_PushObjectiveHud(array<string> icons, array<string> titles, array<string> details,
		string barLabel, int barPercent, int barVisible, int show)
	{
		if (TBD_Authority.IsClient())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			TBD_ObjectiveHud.Accept(icons, titles, details, barLabel, barPercent, barVisible, show);
			return;
		}

		Rpc(TBD_RpcDo_ObjectiveHud, icons, titles, details, barLabel, barPercent, barVisible, show);
	}

	//! The server answers a client's ask with a push of its snapshot.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ObjectiveHud()
	{
		TBD_ObjectivesComponent runner = TBD_ObjectivesComponent.GetInstance();
		if (runner)
			runner.PushHudTo(GetPlayerId());
	}

	//! The owning client applies the snapshot.
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_ObjectiveHud(array<string> icons, array<string> titles,
		array<string> details, string barLabel, int barPercent, int barVisible, int show)
	{
		TBD_ObjectiveHud.Accept(icons, titles, details, barLabel, barPercent, barVisible, show);
	}

	//! Ask for this player's task snapshot: a client asks the server over RPC; the server (or a
	//! listen host) builds it and applies it locally.
	//! @authority owner
	void TBD_RequestTaskHud()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_TaskHud);
			return;
		}

		array<int> xs;
		array<int> zs;
		array<string> icons;
		array<string> labels;
		array<string> ids;
		array<string> states;
		TBD_TaskHud.BuildSnapshot(xs, zs, icons, labels, ids, states);
		TBD_TaskHud.Accept(xs, zs, icons, labels, ids, states);
	}

	//! Deliver one snapshot to this controller's player: applied locally for the host's own
	//! controller, else sent over the Owner RPC. Does nothing on a client.
	//! @authority server
	void TBD_PushTaskHud(array<int> xs, array<int> zs, array<string> icons, array<string> labels,
		array<string> ids, array<string> states)
	{
		if (TBD_Authority.IsClient())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			TBD_TaskHud.Accept(xs, zs, icons, labels, ids, states);
			return;
		}

		Rpc(TBD_RpcDo_TaskHud, xs, zs, icons, labels, ids, states);
	}

	//! The server answers a client's ask with the current snapshot.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_TaskHud()
	{
		array<int> xs;
		array<int> zs;
		array<string> icons;
		array<string> labels;
		array<string> ids;
		array<string> states;
		TBD_TaskHud.BuildSnapshot(xs, zs, icons, labels, ids, states);
		Rpc(TBD_RpcDo_TaskHud, xs, zs, icons, labels, ids, states);
	}

	//! The owning client draws the snapshot.
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_TaskHud(array<int> xs, array<int> zs, array<string> icons,
		array<string> labels, array<string> ids, array<string> states)
	{
		TBD_TaskHud.Accept(xs, zs, icons, labels, ids, states);
	}
}
