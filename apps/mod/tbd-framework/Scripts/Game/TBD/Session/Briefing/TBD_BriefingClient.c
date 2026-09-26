/**
 * @file TBD_BriefingClient.c
 * @brief Client cache of the last briefing and ready tally, with the invokers the screen binds to.
 *
 * Role: holds this player's last briefing payload and readiness answer and notifies on change;
 * sends the briefing and ready requests through the local player controller.  Position: the
 * Briefing screen and SCR_PlayerController's stage handler call it; SCR_PlayerController's reply
 * RPCs feed `Accept` and `AcceptTally`.
 * State: static payload, ready flag, tally and two invokers on the client, for the life of the
 * script VM; static because the menu manager creates and destroys the screen.  Invariants: the
 * ready flag set by `ReportReady` is optimistic, and the authority's `AcceptTally` decides it; a
 * request without a local player controller does nothing; `Reset` clears everything for a new
 * briefing phase.
 */

//! Client-side briefing cache and change notifications.
class TBD_BriefingClient
{
	protected static ref TBD_BriefingPayload m_Payload; //!< last payload received; null until one arrives
	protected static bool m_bReady; //!< this player is ready; default false
	protected static string m_sTally; //!< last tally text from the server

	protected static ref ScriptInvoker m_OnPayloadChanged; //!< (TBD_BriefingPayload payload)

	protected static ref ScriptInvoker m_OnReadyStateChanged; //!< (string tally)

	//! @return the last payload received, or null
	static TBD_BriefingPayload GetPayload()
	{
		return m_Payload;
	}

	//! @return true when this player is marked ready
	static bool IsReady()
	{
		return m_bReady;
	}

	//! @return the last tally text, or empty
	static string GetReadyTally()
	{
		return m_sTally;
	}

	//! @return the invoker raised with (TBD_BriefingPayload payload) when a payload arrives; created on first use
	static ScriptInvoker GetOnPayloadChanged()
	{
		if (!m_OnPayloadChanged)
			m_OnPayloadChanged = new ScriptInvoker();

		return m_OnPayloadChanged;
	}

	//! @return the invoker raised with (string tally) when a readiness answer arrives; created on first use
	static ScriptInvoker GetOnReadyStateChanged()
	{
		if (!m_OnReadyStateChanged)
			m_OnReadyStateChanged = new ScriptInvoker();

		return m_OnReadyStateChanged;
	}

	//! Ask the server for this player's briefing; does nothing without a local player controller.
	//! @authority client
	static void Request()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		pc.TBD_RequestBriefing();
	}

	//! Report this player ready: set the flag optimistically and send the report; does nothing
	//! without a local player controller.
	//! @authority client
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

	//! Store a payload that arrived, or was built in place on a listen host, and notify.
	//! @param payload the new payload
	static void Accept(TBD_BriefingPayload payload)
	{
		m_Payload = payload;

		if (m_OnPayloadChanged)
			m_OnPayloadChanged.Invoke(m_Payload);
	}

	//! The authority's verdict on a readiness report. `accepted` false means the server refused
	//! (no slot), so the button is released and the reason shows in the status line.
	//! @param tally the tally text, or the refusal reason
	//! @param accepted the server recorded the readiness
	static void AcceptTally(string tally, bool accepted)
	{
		m_sTally = tally;
		m_bReady = accepted;

		if (m_OnReadyStateChanged)
			m_OnReadyStateChanged.Invoke(m_sTally);
	}

	//! Forget the last payload and readiness for a new briefing phase.
	static void Reset()
	{
		m_Payload = null;
		m_bReady = false;
		m_sTally = string.Empty;
	}
}
