/**
 * @file SCR_PlayerController.c
 * @brief The mission browser on the player controller: session key binds and the list and select RPCs.
 *
 * Role: registers the TBD key actions on the local client and carries the admin mission browser's
 * list request and mission selection to the server.  Position: vanilla OnControlledEntityChanged
 * triggers the key registration; the keys call TBD_MissionSelectorScreen.Toggle,
 * TBD_AdminClient.Toggle and the list and select requests here; the server side calls
 * TBD_MissionBrowserService and TBD_MissionDeploymentRelay and tells the admin in chat.
 * State: per controller, on the owning client: the cached list lines, the cycle cursor and the
 * listeners-registered latch.  Invariants: listeners register once, on the local controller only;
 * both server handlers refuse a caller not on the vanilla admin list; the list reply goes to the
 * requesting client only.
 */

//! Mission browser keys and RPCs on the player controller.
modded class SCR_PlayerController
{
	protected ref array<string> m_TBD_MissionLines; //!< last mission list received, as display lines; null until one arrives
	protected int m_TBD_CycleIndex = 0; //!< 0-based cursor into m_TBD_MissionLines; default 0
	protected bool m_TBD_ListenersRegistered = false; //!< key listeners are registered; default false

	//! Register the TBD key listeners once this controller controls an entity.
	//! @param from the entity controlled before
	//! @param to the newly controlled entity
	//! @authority owner
	override void OnControlledEntityChanged(IEntity from, IEntity to)
	{
		super.OnControlledEntityChanged(from, to);
		TBD_TryRegisterListeners();
	}

	//! Activate `TBD_BrowserContext` and add the `TBD_MissionCycle`, `TBD_MissionLoad`,
	//! `TBD_AdminMenu` and `TBD_MissionSelector` listeners, once, on the local controller only.
	//! @authority owner
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

		// The admin menu, registered for every client: no client knows whether it is an admin. A
		// non-admin who presses it gets a screen holding only the server's refusal (see
		// TBD_AdminPayload).
		im.AddActionListener("TBD_AdminMenu", EActionTrigger.DOWN, TBD_OnAdminMenuAction);

		// Mission Selector (F9) -- opens through TBD_MenuStack like every TBD screen.
		im.AddActionListener("TBD_MissionSelector", EActionTrigger.DOWN, TBD_OnMissionSelectorAction);

		m_TBD_ListenersRegistered = true;
		Print("[TBD][browser] admin keybinds registered (TBD_MissionCycle / TBD_MissionLoad / TBD_AdminMenu / TBD_MissionSelector). Press F6 or F9 for Mission Selector!");
	}

	//! `TBD_MissionSelector` key: toggle the Mission Selector screen.
	//! @authority owner
	protected void TBD_OnMissionSelectorAction(float value, EActionTrigger trigger)
	{
		TBD_MissionSelectorScreen.Toggle();
	}

	//! `TBD_AdminMenu` key: raise or drop the admin screen on this client.
	//! @authority owner
	protected void TBD_OnAdminMenuAction(float value, EActionTrigger trigger)
	{
		TBD_AdminClient.Toggle();
	}

	//! `TBD_MissionCycle` key: the first press fetches the list; later presses step through it and
	//! print the highlighted line.
	//! @authority owner
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

	//! `TBD_MissionLoad` key: ask the server to request a deployment of the highlighted mission.
	//! @authority owner
	protected void TBD_OnLoadAction(float value, EActionTrigger trigger)
	{
		if (!m_TBD_MissionLines || m_TBD_MissionLines.IsEmpty())
		{
			Print("[TBD][browser] no mission selected -- press Cycle first.");
			return;
		}
		Print(string.Format("[TBD][browser] requesting a deployment of mission #%1", m_TBD_CycleIndex + 1));
		TBD_RequestSelectMission(m_TBD_CycleIndex + 1);
	}

	//! Ask the server for the deployable mission list.
	//! @authority owner
	void TBD_RequestMissionList()
	{
		Rpc(TBD_RpcAsk_MissionList);
	}

	//! Build the list payload and send it to the requesting client, for listed admins only; any other
	//! caller is refused with a WARNING and gets no reply.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_MissionList()
	{
		int playerId = GetPlayerId();

		SCR_PlayerListedAdminManagerComponent admins = SCR_PlayerListedAdminManagerComponent.GetInstance();
		if (!admins || !admins.IsPlayerOnAdminList(playerId))
		{
			Print(string.Format("[TBD][browser] non-admin player %1 requested the mission list -- denied.", playerId), LogLevel.WARNING);
			return;
		}

		string payload = TBD_MissionBrowserService.BuildListPayload();
		Rpc(TBD_RpcDo_ReceiveMissionList, payload);
	}

	//! Cache the mission list on the requesting admin's client and print each line.
	//! @param payload the string TBD_MissionBrowserService.BuildListPayload produced
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_ReceiveMissionList(string payload)
	{
		m_TBD_MissionLines = TBD_MissionBrowserService.ParseListPayload(payload);
		foreach (string line : m_TBD_MissionLines)
			Print("[TBD][browser] " + line);
	}

	//! Ask the server to request a deployment of mission `number`.
	//! @param number the 1-based list number
	//! @authority owner
	void TBD_RequestSelectMission(int number)
	{
		Rpc(TBD_RpcAsk_SelectMission, number);
	}

	//! Relay a listed admin's selection to the platform through TBD_MissionDeploymentRelay and tell
	//! them the reply in chat. Nothing restarts here: the platform decides and runs the deployment.
	//! @param number the 1-based list number
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_SelectMission(int number)
	{
		int playerId = GetPlayerId();

		SCR_PlayerListedAdminManagerComponent admins = SCR_PlayerListedAdminManagerComponent.GetInstance();
		if (!admins || !admins.IsPlayerOnAdminList(playerId))
		{
			Print(string.Format("[TBD][browser] non-admin player %1 tried to select mission %2 -- denied.", playerId, number), LogLevel.WARNING);
			return;
		}

		string reply = TBD_MissionDeploymentRelay.RequestByNumber(playerId, number);
		Print(string.Format("[TBD][browser] admin %1 -> %2", playerId, reply));
		TBD_PlayerChat.Tell(playerId, reply);
	}

	//! @return the cached mission list lines, or null before the first reply
	//! @authority owner
	array<string> TBD_GetMissionLines()
	{
		return m_TBD_MissionLines;
	}
}
