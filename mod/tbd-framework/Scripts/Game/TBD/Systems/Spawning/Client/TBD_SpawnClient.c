/**
 * @file TBD_SpawnClient.c
 * @brief Ready and Continue on the client: send the request, hold and announce the answer.
 *
 * Role: the client end of the Ready and Continue deploy.  Position: the briefing screen calls
 * Request and binds GetOnDeployResult; the player controller RPC (SCR_PlayerController in this
 * folder) delivers the authority's answer to AcceptResult.
 * State: the last answer and its invoker (static: the screen is created and destroyed by the menu
 * manager).  Invariants: every request produces exactly one AcceptResult, locally on the authority
 * or through the owner RPC.
 */

//! Client state of the Ready and Continue deploy.
class TBD_SpawnClient
{
	protected static bool m_bLastOk; //!< last answer: the player now has a body; default false
	protected static string m_sLastWhy; //!< last refusal reason; empty on success

	protected static ref ScriptInvoker m_OnDeployResult; //!< (bool ok, string why)

	//! (bool ok, string why) -- lazily created.
	static ScriptInvoker GetOnDeployResult()
	{
		if (!m_OnDeployResult)
			m_OnDeployResult = new ScriptInvoker();

		return m_OnDeployResult;
	}

	//! Ask the authority for a body (Ready and Continue). The answer arrives through GetOnDeployResult.
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

	//! Store the authority's answer and announce it through GetOnDeployResult.
	static void AcceptResult(bool ok, string why)
	{
		m_bLastOk = ok;
		m_sLastWhy = why;
		if (m_OnDeployResult)
			m_OnDeployResult.Invoke(ok, why);
	}

	//! True when the last answer put the player in a body.
	static bool LastOk()
	{
		return m_bLastOk;
	}

	//! The last refusal reason; empty on success.
	static string LastWhy()
	{
		return m_sLastWhy;
	}

	//! Forget the last answer.
	static void Reset()
	{
		m_bLastOk = false;
		m_sLastWhy = string.Empty;
	}
}
