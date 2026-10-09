/**
 * @file TBD_AdminClient.c
 * @brief Client side of the admin menu: the cached snapshot and verdict, the requests, open and close.
 *
 * Role: remembers the last snapshot and action verdict the server sent, asks for new ones, and
 * opens or toggles the admin screen.  Position: TBD_AdminScreen reads it and binds its invokers;
 * the admin RPCs on SCR_PlayerController feed Accept and AcceptActionResult; the `TBD_AdminMenu`
 * key and `#tbd menu` call Toggle and Open.
 * State: static, on the client, so it survives the menu manager destroying the screen.
 * Invariants: nothing here is authoritative; m_bAuthorised on the cached payload is the server's
 * remembered answer, and every request and action is re-checked on the server.
 */

//! Static client cache and request surface of the admin menu.
class TBD_AdminClient
{
	protected static ref TBD_AdminPayload s_Payload; //!< last snapshot received; null until one arrives
	protected static string s_sLastResult; //!< last line the server sent about an action
	protected static bool s_bLastResultOk; //!< whether that action worked
	protected static ref ScriptInvoker s_OnPayloadChanged; //!< (TBD_AdminPayload payload); created on first use
	protected static ref ScriptInvoker s_OnActionResult; //!< (string message, bool ok); created on first use

	//! @return the last snapshot received, or null
	static TBD_AdminPayload GetPayload()
	{
		return s_Payload;
	}

	//! @return the last line the server sent about an action
	static string GetLastResult()
	{
		return s_sLastResult;
	}

	//! @return whether the last action worked
	static bool IsLastResultOk()
	{
		return s_bLastResultOk;
	}

	//! @return the invoker raised with (TBD_AdminPayload) when a snapshot arrives
	static ScriptInvoker GetOnPayloadChanged()
	{
		if (!s_OnPayloadChanged)
			s_OnPayloadChanged = new ScriptInvoker();

		return s_OnPayloadChanged;
	}

	//! @return the invoker raised with (string message, bool ok) when a verdict arrives
	static ScriptInvoker GetOnActionResult()
	{
		if (!s_OnActionResult)
			s_OnActionResult = new ScriptInvoker();

		return s_OnActionResult;
	}

	//! Ask the server for a fresh snapshot. No-op without a local player controller.
	//! @authority client
	static void Request()
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return;

		controller.TBD_RequestAdminSnapshot();
	}

	//! Ask the server to run one admin power. The server decides; this only asks.
	//! @param action the power
	//! @param targetId the player it acts on; 0 for STAGE_ADVANCE
	//! @authority client
	static void Act(TBD_EAdminAction action, int targetId)
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return;

		controller.TBD_RequestAdminAction(action, targetId);
	}

	//! Cache a snapshot that arrived (or was built in place on a listen host) and raise the invoker.
	//! @param payload the snapshot
	static void Accept(TBD_AdminPayload payload)
	{
		s_Payload = payload;

		if (s_OnPayloadChanged)
			s_OnPayloadChanged.Invoke(s_Payload);
	}

	//! Cache the server's verdict on an action and raise the invoker; the screen shows it verbatim.
	//! @param message the server's line
	//! @param ok whether the action worked
	static void AcceptActionResult(string message, bool ok)
	{
		s_sLastResult = message;
		s_bLastResultOk = ok;

		if (s_OnActionResult)
			s_OnActionResult.Invoke(message, ok);
	}

	//! Raise the admin screen, or refresh it when it is already open. Does nothing on a dedicated
	//! server, which has a workspace but no screen. A non-admin gets a screen showing only the
	//! server's refusal.
	//! @authority client
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

	//! Close the admin screen when it is open, else Open it.
	//! @authority client
	static void Toggle()
	{
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIAdmin))
		{
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UIAdmin);
			return;
		}

		Open();
	}

	//! Forget the cached snapshot and verdict. Open calls it so a stale roster is never shown as
	//! live before the first snapshot lands.
	static void Reset()
	{
		s_Payload = null;
		s_sLastResult = string.Empty;
		s_bLastResultOk = false;
	}
}
