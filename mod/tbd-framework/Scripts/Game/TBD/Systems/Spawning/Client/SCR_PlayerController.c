/**
 * @file SCR_PlayerController.c
 * @brief Ready and Continue wire: one request and reply RPC pair on the player controller.
 *
 * Role: carries the briefing's primary button to TBD_SpawnManager.DeployOnReady and the answer
 * back.  Position: TBD_SpawnClient.Request calls TBD_RequestReadyDeploy on the owning client; the
 * answer lands in TBD_SpawnClient.AcceptResult.
 * State: none.  Invariants: the controller is owned by exactly one client, so RplRcver.Owner answers
 * the requester only; on a listen host or in PIE the requester is the authority and the request runs
 * in place; the method names differ from the lobby picker's TBD_RequestDeploy pair, because modded
 * blocks of one class share one method namespace.
 */

//! Ready and Continue RPCs of the player controller.
modded class SCR_PlayerController
{
	//! Ask the authority to put this player in a body now; runs in place on the authority.
	//! @authority owner
	void TBD_RequestReadyDeploy()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_ReadyDeploy);
			return;
		}

		string why;
		bool ok = TBD_ReadyDeployHere(why);
		TBD_SpawnClient.AcceptResult(ok, why);
	}

	//! Server end of the request: deploy and answer the owner.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ReadyDeploy()
	{
		string why;
		bool ok = TBD_ReadyDeployHere(why);
		Rpc(TBD_RpcDo_ReadyDeployResult, ok, why);
	}

	//! Owner end of the answer.
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_ReadyDeployResult(bool ok, string why)
	{
		TBD_SpawnClient.AcceptResult(ok, why);
	}

	//! The deploy both entry paths make.
	//! @param why set to the refusal reason
	//! @return true when the player now has a body
	//! @authority server
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
