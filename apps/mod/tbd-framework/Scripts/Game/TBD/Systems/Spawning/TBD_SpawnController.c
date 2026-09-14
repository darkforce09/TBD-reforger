//! Ready & Continue (2026-09-14) — the wire between the briefing's primary button and
//! TBD_SpawnManager.DeployOnReady. One request/reply RPC pair on the player controller, the
//! precedent TBD_BriefingController.c documents: the controller is replicated and owned by exactly
//! one client, so `RplRcver.Owner` answers the requester and nobody else. A further
//! `modded class SCR_PlayerController` block alongside the existing ones compiles clean (probed by
//! T-181.9.2); method names are unique across blocks.
//!
//! On a listen host or in PIE the requester IS the authority, so the request runs in place — the
//! same short-circuit TBD_ReportReady takes.
//!
//! Named "ReadyDeploy" because the lobby picker already owns `TBD_RequestDeploy` /
//! `TBD_RpcAsk_Deploy` (TBD_LobbyController.c → TBD_LobbyService.ApplyDeploy: seat required, no
//! stage advance, no walk-on); a second modded block may not reuse a method name.
modded class SCR_PlayerController
{
	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "put me in a body now" (Ready & Continue).
	void TBD_RequestReadyDeploy()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_ReadyDeploy);
			return;
		}

		string why;
		bool ok = TBD_ReadyDeployHere(why);
		TBD_SpawnClient.AcceptResult(ok, why);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ReadyDeploy()
	{
		string why;
		bool ok = TBD_ReadyDeployHere(why);
		Rpc(TBD_RpcDo_ReadyDeployResult, ok, why);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_ReadyDeployResult(bool ok, string why)
	{
		TBD_SpawnClient.AcceptResult(ok, why);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the one call both entry paths make.
	protected bool TBD_ReadyDeployHere(out string why)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			why = "no spawn manager on this game mode";
			return false;
		}

		return spawn.DeployOnReady(GetPlayerId(), why);
	}
}
