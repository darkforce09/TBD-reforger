//! T-181.11.2 - CLIENT side of the admin menu: the last snapshot this client received, the last
//! answer the server gave to an action, and the notifications the screen binds to.
//!
//! Static for the same reason `TBD_BriefingClient` is: the menu manager creates and destroys the
//! screen, so parking state on the screen would lose it on every close. The screen still asks for
//! a fresh snapshot on open and on a timer - an admin panel showing a two-minute-old roster is
//! worse than useless - but it always has something to draw in the meantime.
//!
//! **Nothing here is authoritative.** `m_bAuthorised` on the cached payload is the server's answer
//! being remembered, not a permission this class grants. Every request and every action goes back
//! over the wire and is re-checked there; there is no local path from this class to
//! `TBD_SpawnManager` or `TBD_FrameworkManager`, and on a dedicated server those are not even
//! present in the client's process.
class TBD_AdminClient
{
	protected static ref TBD_AdminPayload s_Payload;

	//! Last line the server sent back about an action, and whether it worked.
	protected static string s_sLastResult;
	protected static bool s_bLastResultOk;

	//! (TBD_AdminPayload payload)
	protected static ref ScriptInvoker s_OnPayloadChanged;

	//! (string message, bool ok)
	protected static ref ScriptInvoker s_OnActionResult;

	//------------------------------------------------------------------------------------------------
	static TBD_AdminPayload GetPayload()
	{
		return s_Payload;
	}

	//------------------------------------------------------------------------------------------------
	static string GetLastResult()
	{
		return s_sLastResult;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsLastResultOk()
	{
		return s_bLastResultOk;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_AdminPayload) - lazily created.
	static ScriptInvoker GetOnPayloadChanged()
	{
		if (!s_OnPayloadChanged)
			s_OnPayloadChanged = new ScriptInvoker();

		return s_OnPayloadChanged;
	}

	//------------------------------------------------------------------------------------------------
	//! (string message, bool ok) - lazily created.
	static ScriptInvoker GetOnActionResult()
	{
		if (!s_OnActionResult)
			s_OnActionResult = new ScriptInvoker();

		return s_OnActionResult;
	}

	// -- Outbound ----------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Ask the server for a fresh snapshot. No-op without a local player controller.
	static void Request()
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return;

		controller.TBD_RequestAdminSnapshot();
	}

	//------------------------------------------------------------------------------------------------
	//! Ask the server to run one admin power. The server decides; this only asks.
	static void Act(TBD_EAdminAction action, int targetId)
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return;

		controller.TBD_RequestAdminAction(action, targetId);
	}

	// -- Inbound -----------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! A snapshot arrived (or was built locally on a listen host).
	static void Accept(TBD_AdminPayload payload)
	{
		s_Payload = payload;

		if (s_OnPayloadChanged)
			s_OnPayloadChanged.Invoke(s_Payload);
	}

	//------------------------------------------------------------------------------------------------
	//! The server's verdict on an action. Always shown verbatim - an admin acting under pressure
	//! needs the authority's own words, not a client-side guess at what probably happened.
	static void AcceptActionResult(string message, bool ok)
	{
		s_sLastResult = message;
		s_bLastResultOk = ok;

		if (s_OnActionResult)
			s_OnActionResult.Invoke(message, ok);
	}

	// -- Screen lifecycle --------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Raise the admin screen. Safe to call from anywhere on a client: a dedicated server does
	//! nothing, and a non-admin gets a screen that shows only the refusal the server sends back.
	//!
	//! T-181.49 - the guard below was `if (!GetGame().GetWorkspace())`, which does NOT mean "no
	//! screen": `GetGame().GetWorkspace()` is MEASURED NON-NULL on a headless dedicated server
	//! (engine 1.7.0.54), so a server reaching `#tbd menu` would have tried to open a menu. The
	//! test both oracles use, and the one the rest of this addon already uses, is the replication
	//! mode.
	static void Open()
	{
		if (RplSession.Mode() == RplMode.Dedicated)
			return;

		// Already up (a second `#tbd menu`, say): refresh it rather than blanking the panel out
		// from under the admin's hands, which is what Reset would do to a live screen.
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIAdmin))
		{
			Request();
			return;
		}

		Reset();
		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIAdmin);
	}

	//------------------------------------------------------------------------------------------------
	static void Toggle()
	{
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIAdmin))
		{
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UIAdmin);
			return;
		}

		Open();
	}

	//------------------------------------------------------------------------------------------------
	//! Forget the last session's answers. Called on open so a stale roster cannot be mistaken for a
	//! live one during the beat before the first snapshot lands.
	static void Reset()
	{
		s_Payload = null;
		s_sLastResult = string.Empty;
		s_bLastResultOk = false;
	}
}
