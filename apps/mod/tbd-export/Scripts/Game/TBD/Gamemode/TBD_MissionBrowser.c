//! Client<->server transport for the admin mission browser. Works on dedicated
//! servers (no chat dependency): the admin's client RPCs the server, the server
//! validates the player is a listed admin and drives TBD_FrameworkManager.
//!
//! Added as methods on the player controller (which is replicated and owned by
//! one client), so server->owner replies route to the requesting admin only.
//!
//! -- T-181.11.2 - why the admin MENU transport also lives in this block ----------------------
//! It is a third client<->server admin channel, and it belongs on the player controller for the
//! same reason the two above it do: `RplRcver.Owner` delivers to exactly one client, which is
//! what makes a targeted, non-broadcast reply possible.
//!
//! It does not add a `modded class SCR_PlayerController` block; it extends the one below.
//!
//! -- T-181.30 - THE COUNT IN THIS COMMENT WAS WRONG AND IS WORTH CORRECTING CAREFULLY ---------
//! This used to read "Two already exist in this addon (here, and `TBD_BriefingController.c`)" and
//! "**Two blocks are measured to compile and interoperate; three are not.**" Both statements are
//! now false. There are **SIX** such blocks in the addon as of today - this file,
//! `TBD_BriefingController.c`, `TBD_LobbyController.c`, `TBD_SpectatorHost.c`,
//! `TBD_MarkerController.c`, `TBD_RadioController.c` - and the program has re-measured static
//! coexistence at N=2, 3, 5 and 6.
//!
//! **What is actually known is narrower than either the old claim or the new count suggests**, and
//! the authority is the Landmines section of `docs/mod/t181_event_mod_program.md`, not this header:
//!   * N blocks COMPILE and methods declared in one are callable from the others. Measured to N=6.
//!   * RUNTIME coexistence has NEVER been observed. `world-boot.sh` boots with zero players and
//!     every one of these blocks only does anything once a client connects. "Compiles" is not
//!     "works", and settling this is the first job of T-181.25 on a real dedicated server.
//!
//! So the reason to keep extending this block rather than adding a seventh is unchanged, but it is
//! blast-radius minimisation, NOT a measured two-block ceiling: this file already overrides a
//! vanilla method, so folding the admin transport in here adds no new override and no new
//! `modded enum ChimeraMenuPreset` entry.
//!
//! The spectator slice (T-181.12) hit the same question and answered it by hosting on a game-mode
//! component instead - that route is unavailable here, because a game-mode component has no
//! per-client owner and so no way to answer one admin privately. So this file gains the methods and
//! the logic stays in `TBD_AdminService` / `TBD_AdminSnapshotService`; that is why they live here
//! rather than in the UI folder with the rest of the admin screen.
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

		// T-181.11.2 - the admin menu. Registered for every client, not just admins, because the
		// client genuinely does not know whether it is one: nothing on a client is authoritative
		// about the admin list. A non-admin who presses it gets a screen containing the server's
		// refusal and nothing else - no roster, no mission, no audit trail (see TBD_AdminData.c).
		im.AddActionListener("TBD_AdminMenu", EActionTrigger.DOWN, TBD_OnAdminMenuAction);

		m_TBD_ListenersRegistered = true;
		Print("[TBD][browser] admin keybinds registered (TBD_MissionCycle / TBD_MissionLoad / TBD_AdminMenu).");
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.11.2 - raise or drop the admin screen on this client.
	protected void TBD_OnAdminMenuAction(float value, EActionTrigger trigger)
	{
		TBD_AdminClient.Toggle();
	}

	//------------------------------------------------------------------------------------------------
	//! Cycle key: first press fetches the list; subsequent presses step through it.
	protected void TBD_OnCycleAction(float value, EActionTrigger trigger)
	{
		if (!m_TBD_MissionLines || m_TBD_MissionLines.IsEmpty())
		{
			Print("[TBD][browser] fetching mission list...");
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
			Print("[TBD][browser] no mission selected - press Cycle first.");
			return;
		}
		Print(string.Format("[TBD][browser] loading mission #%1...", m_TBD_CycleIndex + 1));
		TBD_RequestSelectMission(m_TBD_CycleIndex + 1);
	}

	//------------------------------------------------------------------------------------------------
	// CLIENT (owner) -> SERVER: ask for the current mission list.
	void TBD_RequestMissionList()
	{
		Rpc(TBD_RpcAsk_MissionList);
	}

	//! @authority server - executes on the server (RplRcver.Server): builds and returns the list.
	//! Admin-gated like TBD_RpcAsk_SelectMission - this is an admin browser tool, and the
	//! payload is server-built content that shouldn't stream to arbitrary clients (T-130.4 F1-17).
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_MissionList()
	{
		int playerId = GetPlayerId();

		SCR_PlayerListedAdminManagerComponent admins = SCR_PlayerListedAdminManagerComponent.GetInstance();
		if (!admins || !admins.IsPlayerOnAdminList(playerId))
		{
			Print(string.Format("[TBD][browser] non-admin player %1 requested the mission list - denied.", playerId), LogLevel.WARNING);
			return;
		}

		string payload = TBD_MissionBrowserService.BuildListPayload();
		Rpc(TBD_RpcDo_ReceiveMissionList, payload);
	}

	//! @authority owner - executes on the requesting admin's client only (RplRcver.Owner).
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

	//! @authority server - executes on the server; validates the caller is a listed admin before acting.
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_SelectMission(int number)
	{
		int playerId = GetPlayerId();

		SCR_PlayerListedAdminManagerComponent admins = SCR_PlayerListedAdminManagerComponent.GetInstance();
		if (!admins || !admins.IsPlayerOnAdminList(playerId))
		{
			Print(string.Format("[TBD][browser] non-admin player %1 tried to select mission %2 - denied.", playerId, number), LogLevel.WARNING);
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

	// == T-181.11.2 - admin menu transport ===================================================
	//
	// Two request/reply pairs and one push, all owner-targeted. Every one of them re-derives the
	// caller from `GetPlayerId()` on THIS replicated controller - the client cannot name a player
	// id, so it cannot claim to be somebody else. Neither RPC below decides anything: the read
	// gate lives in `TBD_AdminSnapshotService.BuildForAdmin` and the write gate in
	// `TBD_AdminService.Execute`, so a future transport cannot forget to check.

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: ask for the admin snapshot. On a listen host the caller IS the
	//! authority, so build in place instead of RPCing ourselves.
	void TBD_RequestAdminSnapshot()
	{
		// Authority only - the snapshot reads server-owned state (slot map, life ledger, mission
		// document), none of which exists in a client's process. Off the authority, ask for it.
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_AdminSnapshot);
			return;
		}

		TBD_AdminClient.Accept(TBD_AdminSnapshotService.BuildForAdmin(GetPlayerId()));
	}

	//! @authority server - builds the snapshot the caller is entitled to. A non-admin gets a
	//! payload carrying a refusal and no data at all, which is also what makes the reply safe to
	//! send unconditionally: there is nothing in it to leak.
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_AdminSnapshot()
	{
		Rpc(TBD_RpcDo_AdminSnapshot, TBD_AdminSnapshotService.Serialise(
			TBD_AdminSnapshotService.BuildForAdmin(GetPlayerId())));
	}

	//! @authority owner - executes on the requesting client only (RplRcver.Owner).
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_AdminSnapshot(string wire)
	{
		TBD_AdminClient.Accept(TBD_AdminSnapshotService.Parse(wire));
	}

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: run one admin power. The enum crosses the wire as a plain int;
	//! an unknown value falls through `TBD_AdminService.Execute` to "unknown admin action".
	void TBD_RequestAdminAction(TBD_EAdminAction action, int targetId)
	{
		int actionId = action;

		// Authority only - the power itself runs server-side. Off the authority, ask for it.
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_AdminAction, actionId, targetId);
			return;
		}

		bool ok;
		string message = TBD_AdminService.Execute(GetPlayerId(), action, targetId, ok);
		TBD_AdminClient.AcceptActionResult(message, ok);
		TBD_AdminClient.Accept(TBD_AdminSnapshotService.BuildForAdmin(GetPlayerId()));
	}

	//! @authority server - the caller is `GetPlayerId()` of this controller, never an argument.
	//! `TBD_AdminService.Execute` re-checks the admin list before touching anything and audits the
	//! attempt either way.
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_AdminAction(int actionId, int targetId)
	{
		int playerId = GetPlayerId();

		bool ok;
		// The int came off the wire: normalise it to a known action (or NONE) before it is used as
		// one. Enfusion assigns any int to an enum without complaint, so this is the boundary.
		string message = TBD_AdminService.Execute(playerId, TBD_AdminService.FromWire(actionId), targetId, ok);

		Rpc(TBD_RpcDo_AdminActionResult, message, ok);

		// A fresh snapshot on the same round trip: the admin sees the outcome AND the world it
		// produced, without a poll interval of staleness in between.
		Rpc(TBD_RpcDo_AdminSnapshot, TBD_AdminSnapshotService.Serialise(
			TBD_AdminSnapshotService.BuildForAdmin(playerId)));
	}

	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_AdminActionResult(string message, bool ok)
	{
		TBD_AdminClient.AcceptActionResult(message, ok);
	}

	//------------------------------------------------------------------------------------------------
	//! SERVER -> owner: raise the admin screen on this player's client. Driven by `#tbd menu`, so
	//! an admin can reach the panel with no keybind bound and no ActionContext registered - the
	//! chat path is the one surface that works on a stock client today.
	//! @authority server
	void TBD_OpenAdminMenuOnOwner()
	{
		// Listen host: the requesting admin IS this machine. An owner-targeted RPC from the
		// authority to itself is not a delivery worth relying on, so open in place - same
		// short-circuit the snapshot request uses, expressed as "is this the local controller"
		// because that is the question that is true on a host and false on a dedicated server.
		if (GetGame().GetWorkspace() && GetGame().GetPlayerController() == this)
		{
			TBD_AdminClient.Open();
			return;
		}

		Rpc(TBD_RpcDo_OpenAdminMenu);
	}

	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_OpenAdminMenu()
	{
		TBD_AdminClient.Open();
	}
}

//! Serializes the server mission list to a newline-delimited payload and parses
//! it back on the client. Keeps the RPC signature to a single string.
class TBD_MissionBrowserService
{
	//! Cap on list lines in one RPC payload - a runaway mission list must not become an
	//! unbounded reliable-channel string (T-130.4 F1-17). Selection stays 1-based over the
	//! full list; only the display payload is clipped.
	protected static const int MAX_LIST_LINES = 100;

	//------------------------------------------------------------------------------------------------
	//! Server: build "n) name [terrain] N slots" lines from the cached list.
	static string BuildListPayload()
	{
		array<ref TBD_MissionListEntry> entries = TBD_MissionListLoader.GetEntries();
		if (!entries || entries.IsEmpty())
			return "No missions loaded yet.";

		int shown = entries.Count();
		if (shown > MAX_LIST_LINES)
			shown = MAX_LIST_LINES;

		string result;
		for (int i = 0; i < shown; i++)
		{
			TBD_MissionListEntry e = entries[i];
			if (i > 0)
				result = result + "\n";
			result = result + string.Format("%1) %2 [%3] %4 slots", i + 1, e.name, e.terrain, e.slotCount);
		}
		if (entries.Count() > shown)
			result = result + string.Format("\n... and %1 more (list capped at %2).", entries.Count() - shown, MAX_LIST_LINES);
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
