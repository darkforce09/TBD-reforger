//! The admin powers' words for a respawn or deploy the TBD platform decides
//! (TBD_SpawnManagerDeploymentAuthorization). The audit line keeps the plain result name; only the
//! sentence the admin reads says what happens next.
//! @authority server
modded class TBD_AdminService
{
	//------------------------------------------------------------------------------------------------
	override protected static string Respawn(int callerId, int targetId, out bool ok)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			spawn.ForgetDeployResult(targetId);

		string message = super.Respawn(callerId, targetId, ok);
		if (!spawn)
			return message;

		TBD_EDeployResult result = spawn.LastDeployResult(targetId);
		if (result == TBD_EDeployResult.AUTHORIZING)
			return string.Format("TBD: respawn player=%1 -> AUTHORIZING - the TBD platform is deciding on their seat; they stay dead until it allows the new life.", targetId);

		if (result == TBD_EDeployResult.UNAUTHORIZED)
			return string.Format("TBD: respawn player=%1 -> UNAUTHORIZED - their seat cannot be authorized right now (see the audit trail); they stay dead.", targetId);

		return message;
	}

	//------------------------------------------------------------------------------------------------
	override protected static string Deploy(int callerId, int targetId, out bool ok)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			spawn.ForgetDeployResult(targetId);

		string message = super.Deploy(callerId, targetId, ok);
		if (!spawn)
			return message;

		TBD_EDeployResult result = spawn.LastDeployResult(targetId);
		if (result == TBD_EDeployResult.AUTHORIZING)
			return string.Format("TBD: deploy player=%1 -> AUTHORIZING - the TBD platform is deciding on their seat; they deploy once it allows it.", targetId);

		if (result == TBD_EDeployResult.UNAUTHORIZED)
			return string.Format("TBD: deploy player=%1 -> UNAUTHORIZED - their seat cannot be authorized right now (see the audit trail); they were told why and keep the seat.", targetId);

		return message;
	}
}
