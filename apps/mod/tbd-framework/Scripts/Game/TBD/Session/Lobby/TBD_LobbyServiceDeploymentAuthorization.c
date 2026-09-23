//! The lobby's words for a deploy the TBD platform decides (TBD_SpawnManagerDeploymentAuthorization).
//! @authority server
modded class TBD_LobbyService
{
	//------------------------------------------------------------------------------------------------
	//! An AUTHORIZING or UNAUTHORIZED deploy is not `accepted`: the screen latches a deploy as done,
	//! and the platform may still refuse. `resultName` is set only when this click reached
	//! DeployPlayerEx, so an earlier deploy never colours the answer.
	override static string ApplyDeploy(int playerId, out bool accepted, out string resultName)
	{
		string sentence = super.ApplyDeploy(playerId, accepted, resultName);

		if (resultName == typename.EnumToString(TBD_EDeployResult, TBD_EDeployResult.AUTHORIZING))
			return "Checking your seat with the TBD platform - you deploy once it allows it; a refusal arrives in chat.";

		if (resultName == typename.EnumToString(TBD_EDeployResult, TBD_EDeployResult.UNAUTHORIZED))
			return "Your seat cannot be authorized right now - the reason is in your chat. The seat stays yours; deploy again shortly.";

		return sentence;
	}
}
