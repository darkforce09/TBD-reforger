//! Briefing feature module - CLIENT payload cache + invokers the screen binds to. Split out of TBD_BriefingController.c (UI reorg 2026-09-12); logic unchanged.
//!
//! CLIENT — the last briefing this player received, and the change notifications the screen
//! binds to.
//!
//! Static because the screen is created and destroyed by the menu manager: parking the payload
//! on the screen would re-request it on every open and lose it on every close. The screen still
//! re-requests on open (the ORBAT moves while players slot up), but it always has something to
//! draw in the meantime.
class TBD_BriefingClient
{
	protected static ref TBD_BriefingPayload m_Payload;
	protected static bool m_bReady;
	protected static string m_sTally;

	//! (TBD_BriefingPayload payload)
	protected static ref ScriptInvoker m_OnPayloadChanged;

	//! (string tally)
	protected static ref ScriptInvoker m_OnReadyStateChanged;

	//------------------------------------------------------------------------------------------------
	static TBD_BriefingPayload GetPayload()
	{
		return m_Payload;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsReady()
	{
		return m_bReady;
	}

	//------------------------------------------------------------------------------------------------
	static string GetReadyTally()
	{
		return m_sTally;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_BriefingPayload) — lazily created.
	static ScriptInvoker GetOnPayloadChanged()
	{
		if (!m_OnPayloadChanged)
			m_OnPayloadChanged = new ScriptInvoker();

		return m_OnPayloadChanged;
	}

	//------------------------------------------------------------------------------------------------
	//! (string tally) — lazily created.
	static ScriptInvoker GetOnReadyStateChanged()
	{
		if (!m_OnReadyStateChanged)
			m_OnReadyStateChanged = new ScriptInvoker();

		return m_OnReadyStateChanged;
	}

	//------------------------------------------------------------------------------------------------
	//! Ask the server for this player's briefing. No-op without a local controller.
	static void Request()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		pc.TBD_RequestBriefing();
	}

	//------------------------------------------------------------------------------------------------
	static void ReportReady()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		// Optimistic only. The authority's reply (AcceptTally) is what actually latches it, so a
		// refusal releases the button instead of stranding the player on a spent one.
		m_bReady = true;
		pc.TBD_ReportReady();
	}

	//------------------------------------------------------------------------------------------------
	//! A payload arrived (or was built locally on a host).
	static void Accept(TBD_BriefingPayload payload)
	{
		m_Payload = payload;

		if (m_OnPayloadChanged)
			m_OnPayloadChanged.Invoke(m_Payload);
	}

	//------------------------------------------------------------------------------------------------
	//! The authority's verdict on a readiness report. `accepted` false means the server refused
	//! (no slot), so the button is released and the reason shows in the status line.
	static void AcceptTally(string tally, bool accepted)
	{
		m_sTally = tally;
		m_bReady = accepted;

		if (m_OnReadyStateChanged)
			m_OnReadyStateChanged.Invoke(m_sTally);
	}

	//------------------------------------------------------------------------------------------------
	//! New briefing phase: forget the last round's answers.
	static void Reset()
	{
		m_Payload = null;
		m_bReady = false;
		m_sTally = string.Empty;
	}
}
