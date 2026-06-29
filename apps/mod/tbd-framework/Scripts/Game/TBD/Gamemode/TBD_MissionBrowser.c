//! Client<->server transport for the admin mission browser. Works on dedicated
//! servers (no chat dependency): the admin's client RPCs the server, the server
//! validates the player is a listed admin and drives TBD_FrameworkManager.
//!
//! Added as methods on the player controller (which is replicated and owned by
//! one client), so server->owner replies route to the requesting admin only.
modded class SCR_PlayerController
{
	//! Client-side cache of the last received mission list (display lines).
	protected ref array<string> m_TBD_MissionLines;
	protected int m_TBD_CycleIndex = 0;
	protected bool m_TBD_ListenersRegistered = false;

	//------------------------------------------------------------------------------------------------
	//! Register the admin keybinds on the local client once it owns this controller.
	//! Input actions "TBD_MissionCycle" / "TBD_MissionLoad" are defined in the mod's
	//! input config (bind keys in Workbench); listeners are a no-op until they exist.
	override void OnControlledEntityChanged(IEntity from, IEntity to)
	{
		super.OnControlledEntityChanged(from, to);
		TBD_TryRegisterListeners();
	}

	//------------------------------------------------------------------------------------------------
	protected void TBD_TryRegisterListeners()
	{
		if (m_TBD_ListenersRegistered)
			return;
		if (GetGame().GetPlayerController() != this)
			return; // local client's controller only

		InputManager im = GetGame().GetInputManager();
		if (!im)
			return;

		// Our keybinds live in a dedicated context (Configs/System/ActionContext/
		// TBD_BrowserContext.conf) so they never collide with gameplay binds and
		// can be toggled. Must be active before its actions will fire.
		im.ActivateContext("TBD_BrowserContext");

		im.AddActionListener("TBD_MissionCycle", EActionTrigger.DOWN, TBD_OnCycleAction);
		im.AddActionListener("TBD_MissionLoad", EActionTrigger.DOWN, TBD_OnLoadAction);
		m_TBD_ListenersRegistered = true;
		Print("[TBD][browser] admin mission keybinds registered (TBD_MissionCycle / TBD_MissionLoad).");
	}

	//------------------------------------------------------------------------------------------------
	//! Cycle key: first press fetches the list; subsequent presses step through it.
	protected void TBD_OnCycleAction(float value, EActionTrigger trigger)
	{
		if (!m_TBD_MissionLines || m_TBD_MissionLines.IsEmpty())
		{
			Print("[TBD][browser] fetching mission list…");
			TBD_RequestMissionList();
			return;
		}

		m_TBD_CycleIndex = m_TBD_CycleIndex + 1;
		if (m_TBD_CycleIndex >= m_TBD_MissionLines.Count())
			m_TBD_CycleIndex = 0;

		Print(string.Format("[TBD][browser] > %1   (press Load to apply)", m_TBD_MissionLines[m_TBD_CycleIndex]));
	}

	//------------------------------------------------------------------------------------------------
	//! Load key: request the server to switch to the currently highlighted mission.
	protected void TBD_OnLoadAction(float value, EActionTrigger trigger)
	{
		if (!m_TBD_MissionLines || m_TBD_MissionLines.IsEmpty())
		{
			Print("[TBD][browser] no mission selected — press Cycle first.");
			return;
		}
		Print(string.Format("[TBD][browser] loading mission #%1…", m_TBD_CycleIndex + 1));
		TBD_RequestSelectMission(m_TBD_CycleIndex + 1);
	}

	//------------------------------------------------------------------------------------------------
	// CLIENT (owner) -> SERVER: ask for the current mission list.
	void TBD_RequestMissionList()
	{
		Rpc(TBD_RpcAsk_MissionList);
	}

	//! @authority server — executes on the server (RplRcver.Server): builds and returns the list.
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_MissionList()
	{
		string payload = TBD_MissionBrowserService.BuildListPayload();
		Rpc(TBD_RpcDo_ReceiveMissionList, payload);
	}

	//! @authority owner — executes on the requesting admin's client only (RplRcver.Owner).
	//! @rpc Reliable Owner
	// SERVER -> CLIENT (owner): deliver the mission list to the requester only.
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_ReceiveMissionList(string payload)
	{
		m_TBD_MissionLines = TBD_MissionBrowserService.ParseListPayload(payload);
		foreach (string line : m_TBD_MissionLines)
			Print("[TBD][browser] " + line);
	}

	//------------------------------------------------------------------------------------------------
	// CLIENT (owner) -> SERVER: select mission by 1-based number.
	void TBD_RequestSelectMission(int number)
	{
		Rpc(TBD_RpcAsk_SelectMission, number);
	}

	//! @authority server — executes on the server; validates the caller is a listed admin before acting.
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_SelectMission(int number)
	{
		int playerId = GetPlayerId();

		SCR_PlayerListedAdminManagerComponent admins = SCR_PlayerListedAdminManagerComponent.GetInstance();
		if (!admins || !admins.IsPlayerOnAdminList(playerId))
		{
			Print(string.Format("[TBD][browser] non-admin player %1 tried to select mission %2 — denied.", playerId, number), LogLevel.WARNING);
			return;
		}

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		string status = fm.SelectMissionByNumber(number);
		Print(string.Format("[TBD][browser] admin %1 -> %2", playerId, status));
	}

	//------------------------------------------------------------------------------------------------
	//! Client-side accessor for the cached list (for a future menu/HUD).
	array<string> TBD_GetMissionLines()
	{
		return m_TBD_MissionLines;
	}
}

//! Serializes the server mission list to a newline-delimited payload and parses
//! it back on the client. Keeps the RPC signature to a single string.
class TBD_MissionBrowserService
{
	//------------------------------------------------------------------------------------------------
	//! Server: build "n) name [terrain] N slots" lines from the cached list.
	static string BuildListPayload()
	{
		array<ref TBD_MissionListEntry> entries = TBD_MissionListLoader.GetEntries();
		if (!entries || entries.IsEmpty())
			return "No missions loaded yet.";

		string result;
		for (int i = 0; i < entries.Count(); i++)
		{
			TBD_MissionListEntry e = entries[i];
			if (i > 0)
				result = result + "\n";
			result = result + string.Format("%1) %2 [%3] %4 slots", i + 1, e.name, e.terrain, e.slotCount);
		}
		return result;
	}

	//------------------------------------------------------------------------------------------------
	//! Client: split the payload back into display lines.
	static array<string> ParseListPayload(string payload)
	{
		array<string> lines = new array<string>();
		payload.Split("\n", lines, false);
		return lines;
	}
}
