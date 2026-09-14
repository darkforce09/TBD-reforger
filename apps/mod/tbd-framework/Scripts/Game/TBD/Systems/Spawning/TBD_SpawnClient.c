//! Ready & Continue (2026-09-14) — CLIENT: the authority's answer to the last deploy request and
//! the invoker the briefing screen binds to. Static for the same reason TBD_BriefingClient is:
//! the screen is created and destroyed by the menu manager.
class TBD_SpawnClient
{
	protected static bool m_bLastOk;
	protected static string m_sLastWhy;

	//! (bool ok, string why)
	protected static ref ScriptInvoker m_OnDeployResult;

	//------------------------------------------------------------------------------------------------
	//! (bool ok, string why) — lazily created.
	static ScriptInvoker GetOnDeployResult()
	{
		if (!m_OnDeployResult)
			m_OnDeployResult = new ScriptInvoker();

		return m_OnDeployResult;
	}

	//------------------------------------------------------------------------------------------------
	//! Ask the authority for a body (Ready & Continue). The answer arrives through GetOnDeployResult.
	static void Request()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
		{
			AcceptResult(false, "no local player controller");
			return;
		}

		pc.TBD_RequestReadyDeploy();
	}

	//------------------------------------------------------------------------------------------------
	static void AcceptResult(bool ok, string why)
	{
		m_bLastOk = ok;
		m_sLastWhy = why;
		if (m_OnDeployResult)
			m_OnDeployResult.Invoke(ok, why);
	}

	//------------------------------------------------------------------------------------------------
	static bool LastOk()
	{
		return m_bLastOk;
	}

	//------------------------------------------------------------------------------------------------
	static string LastWhy()
	{
		return m_sLastWhy;
	}

	//------------------------------------------------------------------------------------------------
	static void Reset()
	{
		m_bLastOk = false;
		m_sLastWhy = string.Empty;
	}
}
