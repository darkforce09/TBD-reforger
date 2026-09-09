//! TBD Reforger Platform - Interactive Map-Centric Tactical Briefing Screen.
//! Provides a full-screen interactive terrain map (SCR_MapEntity + MapFullscreen.conf)
//! with Reforger native drawing/planning capabilities, plus a collapsible tactical
//! briefing drawer displaying mission orders, ORBAT, AO, and kit specifications.

class TBD_BriefingScreen : TBD_MenuBase
{
	protected SCR_MapEntity m_MapEntity;
	protected ref TBD_BriefingPayload m_Payload;

	protected int m_iActiveTab = 0; // 0: ORDERS, 1: ORBAT, 2: AO & ROE, 3: MY KIT
	protected bool m_bDrawerCollapsed = false;

	// --- Header Widgets ---
	protected TextWidget m_wHeaderTitle;
	protected TextWidget m_wHeaderSubtitle;
	protected TextWidget m_wHeaderTimeText;
	protected TextWidget m_wHeaderWeatherText;

	// --- Drawer Widgets ---
	protected Widget m_wBriefingDrawer;
	protected TextWidget m_wDrawerTitle;
	protected ButtonWidget m_wToggleDrawerBtn;
	protected TextWidget m_wToggleDrawerText;
	protected ButtonWidget m_wCollapsedDrawerBtn;

	// --- Tab Buttons & Labels ---
	protected ButtonWidget m_wTabOrdersBtn;
	protected ImageWidget m_wTabOrdersBG;
	protected TextWidget m_wTabOrdersText;

	protected ButtonWidget m_wTabOrbatBtn;
	protected ImageWidget m_wTabOrbatBG;
	protected TextWidget m_wTabOrbatText;

	protected ButtonWidget m_wTabAoBtn;
	protected ImageWidget m_wTabAoBG;
	protected TextWidget m_wTabAoText;

	protected ButtonWidget m_wTabKitBtn;
	protected ImageWidget m_wTabKitBG;
	protected TextWidget m_wTabKitText;

	// --- Content Widgets ---
	protected TextWidget m_wContentTitle;
	protected TextWidget m_wContentBody;

	// --- Footer Widgets ---
	protected TextWidget m_wReadinessStatusText;
	protected TextWidget m_wFooterMapHint;
	protected ButtonWidget m_wReadyBtn;
	protected ImageWidget m_wReadyBtnBG;
	protected TextWidget m_wReadyBtnText;
	protected ButtonWidget m_wLobbyReturnBtn;

	//------------------------------------------------------------------------------------------------
	override void OnMenuInit()
	{
		super.OnMenuInit();

		if (!m_MapEntity)
			m_MapEntity = SCR_MapEntity.GetMapInstance();
	}

	//------------------------------------------------------------------------------------------------
	override protected string GetInputContext()
	{
		return "MapContext";
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();

		BindWidgets();

		// Bind client events
		TBD_BriefingClient.GetOnPayloadChanged().Insert(OnPayloadChanged);
		TBD_BriefingClient.GetOnReadyStateChanged().Insert(OnReadyStateChanged);

		AdoptPayload(TBD_BriefingClient.GetPayload());

		// Initialize interactive map
		if (!m_MapEntity)
			m_MapEntity = SCR_MapEntity.GetMapInstance();

		if (m_MapEntity)
			GetGame().GetCallqueue().Call(OpenMap);

		// Always ask server for freshest briefing payload
		TBD_BriefingClient.Request();
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		// Cancel any pending map calls
		GetGame().GetCallqueue().Remove(OpenMap);
		GetGame().GetCallqueue().Remove(OpenMapWrap);
		GetGame().GetCallqueue().Remove(OpenMapWrapZoomChange);
		GetGame().GetCallqueue().Remove(OpenMapWrapZoomChangeWrap);

		if (m_MapEntity && m_MapEntity.IsOpen())
			m_MapEntity.CloseMap();

		TBD_BriefingClient.GetOnPayloadChanged().Remove(OnPayloadChanged);
		TBD_BriefingClient.GetOnReadyStateChanged().Remove(OnReadyStateChanged);

		super.OnScreenClose();
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenUpdate(float tDelta)
	{
		super.OnScreenUpdate(tDelta);

		// Keep MapContext actively armed for map interaction
		if (m_MapEntity && m_MapEntity.IsOpen())
		{
			InputManager inputMgr = GetGame().GetInputManager();
			if (inputMgr)
				inputMgr.ActivateContext("MapContext");
		}

		UpdateTimeAndWeather();
	}

	// -- Widget Binding --------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	protected void BindWidgets()
	{
		Widget root = GetRoot();
		if (!root)
			return;

		m_wHeaderTitle = TextWidget.Cast(root.FindAnyWidget("HeaderTitle"));
		m_wHeaderSubtitle = TextWidget.Cast(root.FindAnyWidget("HeaderSubtitle"));
		m_wHeaderTimeText = TextWidget.Cast(root.FindAnyWidget("HeaderTimeText"));
		m_wHeaderWeatherText = TextWidget.Cast(root.FindAnyWidget("HeaderWeatherText"));

		m_wBriefingDrawer = root.FindAnyWidget("BriefingDrawer");
		m_wDrawerTitle = TextWidget.Cast(root.FindAnyWidget("DrawerTitle"));
		m_wToggleDrawerBtn = ButtonWidget.Cast(root.FindAnyWidget("ToggleDrawerBtn"));
		m_wToggleDrawerText = TextWidget.Cast(root.FindAnyWidget("ToggleDrawerText"));
		m_wCollapsedDrawerBtn = ButtonWidget.Cast(root.FindAnyWidget("CollapsedDrawerBtn"));

		m_wTabOrdersBtn = ButtonWidget.Cast(root.FindAnyWidget("TabOrdersBtn"));
		m_wTabOrdersBG = ImageWidget.Cast(root.FindAnyWidget("TabOrdersBG"));
		m_wTabOrdersText = TextWidget.Cast(root.FindAnyWidget("TabOrdersText"));

		m_wTabOrbatBtn = ButtonWidget.Cast(root.FindAnyWidget("TabOrbatBtn"));
		m_wTabOrbatBG = ImageWidget.Cast(root.FindAnyWidget("TabOrbatBG"));
		m_wTabOrbatText = TextWidget.Cast(root.FindAnyWidget("TabOrbatText"));

		m_wTabAoBtn = ButtonWidget.Cast(root.FindAnyWidget("TabAoBtn"));
		m_wTabAoBG = ImageWidget.Cast(root.FindAnyWidget("TabAoBG"));
		m_wTabAoText = TextWidget.Cast(root.FindAnyWidget("TabAoText"));

		m_wTabKitBtn = ButtonWidget.Cast(root.FindAnyWidget("TabKitBtn"));
		m_wTabKitBG = ImageWidget.Cast(root.FindAnyWidget("TabKitBG"));
		m_wTabKitText = TextWidget.Cast(root.FindAnyWidget("TabKitText"));

		m_wContentTitle = TextWidget.Cast(root.FindAnyWidget("ContentTitle"));
		m_wContentBody = TextWidget.Cast(root.FindAnyWidget("ContentBody"));

		m_wReadinessStatusText = TextWidget.Cast(root.FindAnyWidget("ReadinessStatusText"));
		m_wFooterMapHint = TextWidget.Cast(root.FindAnyWidget("FooterMapHint"));
		m_wReadyBtn = ButtonWidget.Cast(root.FindAnyWidget("ReadyBtn"));
		m_wReadyBtnBG = ImageWidget.Cast(root.FindAnyWidget("ReadyBtnBG"));
		m_wReadyBtnText = TextWidget.Cast(root.FindAnyWidget("ReadyBtnText"));
		m_wLobbyReturnBtn = ButtonWidget.Cast(root.FindAnyWidget("LobbyReturnBtn"));
	}

	// -- Map Lifecycle ---------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	void OpenMap()
	{
		GetGame().GetCallqueue().Call(OpenMapWrap);
	}

	//------------------------------------------------------------------------------------------------
	void OpenMapWrap()
	{
		if (!m_MapEntity)
			m_MapEntity = SCR_MapEntity.GetMapInstance();

		if (!m_MapEntity)
			return;

		MapConfiguration mapConfigFullscreen = m_MapEntity.SetupMapConfig(
			EMapEntityMode.FULLSCREEN,
			"{1B8AC767E06A0ACD}Configs/Map/MapFullscreen.conf",
			GetRoot()
		);

		if (mapConfigFullscreen)
			m_MapEntity.OpenMap(mapConfigFullscreen);

		GetGame().GetCallqueue().Call(OpenMapWrapZoomChange);
	}

	//------------------------------------------------------------------------------------------------
	void OpenMapWrapZoomChange()
	{
		GetGame().GetCallqueue().Call(OpenMapWrapZoomChangeWrap);
	}

	//------------------------------------------------------------------------------------------------
	void OpenMapWrapZoomChangeWrap()
	{
		if (!m_MapEntity)
			return;

		m_MapEntity.ZoomOut();

		vector center = GetMissionCenter();
		if (center != vector.Zero)
			m_MapEntity.ZoomPanSmooth(0.3, center[0], center[2]);
	}

	//------------------------------------------------------------------------------------------------
	protected vector GetMissionCenter()
	{
		IEntity playerEnt = GetGame().GetPlayerController().GetControlledEntity();
		if (playerEnt)
			return playerEnt.GetOrigin();

		BaseGameMode gm = GetGame().GetGameMode();
		if (gm)
			return gm.GetOrigin();

		return vector.Zero;
	}

	// -- Data & Rendering ------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	protected void OnPayloadChanged(TBD_BriefingPayload payload)
	{
		AdoptPayload(payload);
	}

	//------------------------------------------------------------------------------------------------
	protected void OnReadyStateChanged(string tally)
	{
		RefreshFooter();
	}

	//------------------------------------------------------------------------------------------------
	protected void AdoptPayload(TBD_BriefingPayload payload)
	{
		m_Payload = payload;

		RefreshHeader();
		RefreshDrawer();
		RefreshFooter();
	}

	//------------------------------------------------------------------------------------------------
	protected void RefreshHeader()
	{
		if (!m_wHeaderTitle)
			return;

		if (m_Payload && !m_Payload.m_sMissionName.IsEmpty())
			m_wHeaderTitle.SetText(m_Payload.m_sMissionName);
		else
			m_wHeaderTitle.SetText("MISSION BRIEFING");

		if (m_wHeaderSubtitle)
		{
			if (!m_Payload)
			{
				m_wHeaderSubtitle.SetText("CONNECTING TO TACTICAL NETWORK...");
			}
			else
			{
				string faction = m_Payload.m_sFactionName;
				if (faction.IsEmpty())
					faction = "UNASSIGNED";
				else
					faction.ToUpper();

				string terrain = m_Payload.m_sTerrain;
				if (terrain.IsEmpty())
					terrain = "EVERON";
				else
					terrain.ToUpper();

				m_wHeaderSubtitle.SetText(string.Format("%1 - %2 - BRIEFING PHASE", faction, terrain));
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void RefreshDrawer()
	{
		// Update tab highlights
		UpdateTabStyles();

		// Render tab content
		RenderActiveTab();
	}

	//------------------------------------------------------------------------------------------------
	protected void UpdateTabStyles()
	{
		SetTabStyle(m_wTabOrdersBG, m_wTabOrdersText, m_iActiveTab == 0);
		SetTabStyle(m_wTabOrbatBG, m_wTabOrbatText, m_iActiveTab == 1);
		SetTabStyle(m_wTabAoBG, m_wTabAoText, m_iActiveTab == 2);
		SetTabStyle(m_wTabKitBG, m_wTabKitText, m_iActiveTab == 3);
	}

	//------------------------------------------------------------------------------------------------
	protected void SetTabStyle(ImageWidget bg, TextWidget text, bool active)
	{
		if (bg)
		{
			if (active)
				bg.SetColorInt(0x331E86E5);
			else
				bg.SetColorInt(0x0DFFFFFF);
		}

		if (text)
		{
			if (active)
				text.SetColorInt(0xFFFFFFFF);
			else
				text.SetColorInt(0x99A0AEC0);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void RenderActiveTab()
	{
		if (!m_wContentTitle || !m_wContentBody)
			return;

		if (!m_Payload)
		{
			m_wContentTitle.SetText("STATUS");
			m_wContentBody.SetText("Requesting operational briefing from server...");
			return;
		}

		if (!m_Payload.IsAvailable())
		{
			m_wContentTitle.SetText("BRIEFING UNAVAILABLE");
			m_wContentBody.SetText(m_Payload.m_sUnavailableReason);
			return;
		}

		switch (m_iActiveTab)
		{
			case 0:
			{
				m_wContentTitle.SetText("OPERATIONAL ORDERS");
				m_wContentBody.SetText(BuildOrdersText());
				break;
			}
			case 1:
			{
				m_wContentTitle.SetText("FRIENDLY ORBAT & ROSTER");
				m_wContentBody.SetText(BuildOrbatText());
				break;
			}
			case 2:
			{
				m_wContentTitle.SetText("AREA OF OPERATIONS & ROE");
				m_wContentBody.SetText(BuildAoText());
				break;
			}
			case 3:
			{
				m_wContentTitle.SetText("YOUR ASSIGNED KIT");
				m_wContentBody.SetText(BuildKitText());
				break;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected string BuildOrdersText()
	{
		string outText = "";

		if (m_Payload.m_aSituation && m_Payload.m_aSituation.Count() > 0)
		{
			outText = outText + "[ 1. SITUATION ]\n";
			foreach (string line : m_Payload.m_aSituation)
			{
				outText = outText + line + "\n\n";
			}
		}

		if (m_Payload.m_aMission && m_Payload.m_aMission.Count() > 0)
		{
			outText = outText + "[ 2. MISSION ]\n";
			foreach (string line : m_Payload.m_aMission)
			{
				outText = outText + line + "\n\n";
			}
		}

		if (m_Payload.m_aExecution && m_Payload.m_aExecution.Count() > 0)
		{
			outText = outText + "[ 3. EXECUTION ]\n";
			foreach (string line : m_Payload.m_aExecution)
			{
				outText = outText + line + "\n\n";
			}
		}

		if (outText.IsEmpty())
			outText = "No specific written orders authored for this faction.";

		return outText;
	}

	//------------------------------------------------------------------------------------------------
	protected string BuildOrbatText()
	{
		string outText = "";

		if (!m_Payload.m_sFactionName.IsEmpty())
			outText = string.Format("FACTION: %1\n\n", m_Payload.m_sFactionName);

		if (m_Payload.m_aGroups && m_Payload.m_aGroups.Count() > 0)
		{
			foreach (TBD_BriefingGroup group : m_Payload.m_aGroups)
			{
				string groupTag = "";
				if (group.m_bIsOwn)
					groupTag = "  [YOUR SQUAD]";

				outText = outText + string.Format("- SQUAD: %1 (%2 seats)%3\n", group.m_sCallsign, group.m_iSeats, groupTag);

				if (group.m_aRoles)
				{
					foreach (TBD_BriefingRole role : group.m_aRoles)
					{
						string youTag = "";
						if (role.m_bIsOwn)
							youTag = " <YOU>";

						outText = outText + string.Format("   * %1 x%2%3\n", role.m_sRole, role.m_iCount, youTag);
					}
				}
				outText = outText + "\n";
			}
		}
		else
		{
			outText = outText + "No ORBAT data available.";
		}

		return outText;
	}

	//------------------------------------------------------------------------------------------------
	protected string BuildAoText()
	{
		string outText = "[ OBJECTIVES & ZONES ]\n";

		if (m_Payload.m_aZones && m_Payload.m_aZones.Count() > 0)
		{
			foreach (TBD_BriefingZone zone : m_Payload.m_aZones)
			{
				outText = outText + string.Format("- %1\n  %2\n\n", zone.m_sTitle, zone.m_sDetail);
			}
		}
		else
		{
			outText = outText + "No specific zones configured.\n\n";
		}

		outText = outText + "[ RULES OF ENGAGEMENT & END CONDITIONS ]\n";
		if (!m_Payload.m_sWinMode.IsEmpty())
			outText = outText + string.Format("Win Mode: %1\n", m_Payload.m_sWinMode);

		if (m_Payload.m_aEndConditions && m_Payload.m_aEndConditions.Count() > 0)
		{
			foreach (string cond : m_Payload.m_aEndConditions)
			{
				outText = outText + string.Format("- %1\n", cond);
			}
		}
		else
		{
			outText = outText + "Standard mission victory conditions apply.\n";
		}

		return outText;
	}

	//------------------------------------------------------------------------------------------------
	protected string BuildKitText()
	{
		string outText = "";

		if (!m_Payload.m_bHasSlot)
		{
			return "You have not claimed an operational slot yet. Return to the lobby to select a role.";
		}

		outText = string.Format("ASSIGNED SEAT: %1\nROLE: %2\nKIT: %3\n\n[ LOADOUT SPECIFICATION ]\n",
			m_Payload.m_sOwnGroup, m_Payload.m_sOwnRole, m_Payload.m_sOwnKit);

		if (m_Payload.m_aKit && m_Payload.m_aKit.Count() > 0)
		{
			foreach (TBD_BriefingKitLine kit : m_Payload.m_aKit)
			{
				outText = outText + string.Format("- %1: %2\n", kit.m_sLabel, kit.m_sValue);
			}
		}
		else
		{
			outText = outText + "Standard kit loadout assigned.\n";
		}

		return outText;
	}

	//------------------------------------------------------------------------------------------------
	protected void RefreshFooter()
	{
		if (!m_wReadinessStatusText || !m_wReadyBtn)
			return;

		string tally = TBD_BriefingClient.GetReadyTally();
		if (tally.IsEmpty())
			tally = "Read briefing and coordinate on map before marking ready.";

		m_wReadinessStatusText.SetText(tally);

		bool ready = TBD_BriefingClient.IsReady();
		if (ready)
		{
			if (m_wReadyBtnBG)
				m_wReadyBtnBG.SetColorInt(0xFF22C55E); // Green
			if (m_wReadyBtnText)
				m_wReadyBtnText.SetText("READY");
		}
		else
		{
			if (m_wReadyBtnBG)
				m_wReadyBtnBG.SetColorInt(0xFF1E86E5); // Blue
			if (m_wReadyBtnText)
				m_wReadyBtnText.SetText("I'M READY");
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void UpdateTimeAndWeather()
	{
		if (!m_wHeaderTimeText || !m_wHeaderWeatherText)
			return;

		ChimeraWorld world = ChimeraWorld.CastFrom(GetGame().GetWorld());
		if (!world)
			return;

		TimeAndWeatherManagerEntity twMgr = world.GetTimeAndWeatherManager();
		if (!twMgr)
			return;

		TimeContainer tc = twMgr.GetTime();
		string hourStr = tc.m_iHours.ToString();
		if (tc.m_iHours < 10)
			hourStr = "0" + hourStr;

		string minStr = tc.m_iMinutes.ToString();
		if (tc.m_iMinutes < 10)
			minStr = "0" + minStr;

		m_wHeaderTimeText.SetText(string.Format("%1:%2", hourStr, minStr));

		WeatherState ws = twMgr.GetCurrentWeatherState();
		if (ws)
			m_wHeaderWeatherText.SetText("Weather: " + ws.GetStateName());
	}

	// -- User Input & Actions --------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	override bool OnClick(Widget w, int x, int y, int button)
	{
		string name = "";
		if (w)
			name = w.GetName();

		if (w == m_wReadyBtn || name == "ReadyBtn" || name == "ReadyBtnBG" || name == "ReadyBtnText")
		{
			OnReadyClicked();
			return true;
		}

		if (w == m_wLobbyReturnBtn || name == "LobbyReturnBtn" || name == "LobbyReturnBG" || name == "LobbyReturnBorder" || name == "LobbyReturnText")
		{
			OnLobbyReturnClicked();
			return true;
		}

		if (w == m_wToggleDrawerBtn || name == "ToggleDrawerBtn" || name == "ToggleDrawerBG" || name == "ToggleDrawerText")
		{
			ToggleDrawer();
			return true;
		}

		if (w == m_wCollapsedDrawerBtn || name == "CollapsedDrawerBtn" || name == "CollapsedDrawerBG" || name == "CollapsedDrawerBorder" || name == "CollapsedDrawerIcon")
		{
			ToggleDrawer();
			return true;
		}

		if (w == m_wTabOrdersBtn || name == "TabOrdersBtn" || name == "TabOrdersBG" || name == "TabOrdersText")
		{
			SwitchTab(0);
			return true;
		}

		if (w == m_wTabOrbatBtn || name == "TabOrbatBtn" || name == "TabOrbatBG" || name == "TabOrbatText")
		{
			SwitchTab(1);
			return true;
		}

		if (w == m_wTabAoBtn || name == "TabAoBtn" || name == "TabAoBG" || name == "TabAoText")
		{
			SwitchTab(2);
			return true;
		}

		if (w == m_wTabKitBtn || name == "TabKitBtn" || name == "TabKitBG" || name == "TabKitText")
		{
			SwitchTab(3);
			return true;
		}

		return super.OnClick(w, x, y, button);
	}

	//------------------------------------------------------------------------------------------------
	protected void SwitchTab(int tab)
	{
		m_iActiveTab = tab;
		RefreshDrawer();
	}

	//------------------------------------------------------------------------------------------------
	protected void ToggleDrawer()
	{
		m_bDrawerCollapsed = !m_bDrawerCollapsed;

		if (m_wBriefingDrawer)
			m_wBriefingDrawer.SetVisible(!m_bDrawerCollapsed);

		if (m_wCollapsedDrawerBtn)
			m_wCollapsedDrawerBtn.SetVisible(m_bDrawerCollapsed);
	}

	//------------------------------------------------------------------------------------------------
	protected void OnReadyClicked()
	{
		TBD_BriefingClient.ReportReady();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnLobbyReturnClicked()
	{
		TBD_MenuStack.Close(m_iPreset);
		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UILobby);
	}
}

modded enum ChimeraMenuPreset
{
	TBD_UIBriefing
}
