//! Briefing rebuild (2026-09-14) — the Briefing screen on the dock shell, over the live map.
//!
//! ```
//!  MapFrame (full-screen SCR_MapEntity, never resized)                       TBD_BriefingScreen.layout
//!  ┌ TopDock ─ TBD_SessionTopBar (mono mission id · BRIEFING tab · identity · count) ───────────┐
//!  │ LeftDock 288        CenterDock 340            RightDock <page width>                       │
//!  │ primary nav         topic nav (3 groups)      topic page          — BRIEFING mode          │
//!  │ Map·Briefing·       markers panel (CenterDock)  hidden            — MARKERS mode           │
//!  │ Players·Markers     players panel (WideDock, to the right edge)   — PLAYERS mode           │
//!  │ Players·Markers     hidden                    hidden              — MAP mode               │
//!  └ BottomDock ─ TBD_SessionBottomBar (Lock Lobby · Ready & Continue, mock toggles) ──────────┘
//! ```
//! Everything floats over the map; the shell has no Backdrop. Pages are `TBD_BriefingPage`s
//! built by `TBD_BriefingNav.CreatePage` from `TBD_BriefingCatalog` (mock until the adapter
//! pass). PLAYERS is a mode too: `TBD_PlayersPanel` pops out beside the primary nav in `WideDock`.
//!
//! The map lifecycle (`OpenMap` chain, `MapContext`, mission-centre pan) is the previous screen's,
//! kept verbatim.
class TBD_BriefingScreen : TBD_DockScreen
{
	static const string ACTION_LOCK = "lock_lobby";
	static const string ACTION_READY = "ready";

	protected SCR_MapEntity m_MapEntity;
	protected TBD_BriefingCatalog m_Catalog;
	protected TBD_LobbyCatalog m_Lobby;
	protected TBD_PlayersCatalog m_Players;

	protected ref TBD_BriefingPrimaryNav m_PrimaryNav;
	protected ref TBD_BriefingTopicNav m_TopicNav;
	protected ref TBD_BriefingPage m_Page;
	protected ref TBD_BriefingMarkersPanel m_Markers;
	protected ref TBD_PlayersPanel m_PlayersPanel;

	protected TBD_EBriefingMode m_eMode = TBD_EBriefingMode.BRIEFING;
	protected TBD_EBriefingPage m_ePage = TBD_EBriefingPage.FREQUENCIES;
	protected bool m_bLocked;
	protected bool m_bReady;

	//------------------------------------------------------------------------------------------------
	override void OnMenuInit()
	{
		super.OnMenuInit();
		if (!m_MapEntity)
			m_MapEntity = SCR_MapEntity.GetMapInstance();
	}

	//------------------------------------------------------------------------------------------------
	//! MapContext only while the cursor is off our chrome. TBD_MenuBase arms this every tick BEFORE
	//! OnScreenUpdate, so a gate there never released it (the preset's ActionContext went at the
	//! rebuild and this override replaced it — unconditionally, 2026-09-14). Empty = arm nothing =
	//! the wheel scrolls the panel under the cursor instead of reaching SCR_MapCursorModule's
	//! OnInputZoomWheelUp/Down listeners.
	override protected string GetInputContext()
	{
		if (CursorOverChrome())
			return string.Empty;

		return "MapContext";
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenOpen()
	{
		m_Catalog = TBD_BriefingCatalog.Get();
		m_Lobby = TBD_LobbyCatalog.Get();
		m_Players = TBD_PlayersCatalog.Get();
		super.OnScreenOpen(); // bars are up after this

		// Primary navigation (its own glass panel, `primary_navigation_panel`).
		m_PrimaryNav = new TBD_BriefingPrimaryNav();
		if (m_PrimaryNav.Build(GetDock("LeftDock"), m_Players))
		{
			m_PrimaryNav.SetActive(TBD_EBriefingMode.BRIEFING);
			m_PrimaryNav.GetOnSelected().Insert(OnPrimarySelected);
		}

		// Topic navigation (its own glass panel, `briefing_navigation_panel`).
		m_TopicNav = new TBD_BriefingTopicNav();
		if (m_TopicNav.Build(GetDock("CenterDock")))
		{
			m_TopicNav.SetActive(TBD_EBriefingPage.FREQUENCIES);
			m_TopicNav.GetOnSelected().Insert(OnTopicSelected);
		}

		if (m_BottomBar)
		{
			m_BottomBar.AddAction(ACTION_LOCK, "Lock Lobby");
			m_BottomBar.AddAction(ACTION_READY, "Ready & Continue", true);
		}

		SetMode(TBD_EBriefingMode.BRIEFING);

		if (!m_MapEntity)
			m_MapEntity = SCR_MapEntity.GetMapInstance();

		if (m_MapEntity)
			GetGame().GetCallqueue().Call(OpenMap);

		Print("[TBD][briefing] Briefing opened (dock shell over the map).");
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		GetGame().GetCallqueue().Remove(OpenMap);
		GetGame().GetCallqueue().Remove(OpenMapWrap);
		GetGame().GetCallqueue().Remove(OpenMapWrapZoomChange);
		GetGame().GetCallqueue().Remove(OpenMapWrapZoomChangeWrap);

		if (m_MapEntity && m_MapEntity.IsOpen())
			m_MapEntity.CloseMap();

		DestroyPage();
		DestroyMarkers();
		DestroyPlayers();

		if (m_PrimaryNav)
		{
			m_PrimaryNav.GetOnSelected().Remove(OnPrimarySelected);
			m_PrimaryNav.Destroy();
		}

		if (m_TopicNav)
		{
			m_TopicNav.GetOnSelected().Remove(OnTopicSelected);
			m_TopicNav.Destroy();
		}

		m_PrimaryNav = null;
		m_TopicNav = null;

		Print("[TBD][briefing] Briefing closed.");
		super.OnScreenClose();
	}

	//------------------------------------------------------------------------------------------------
	override void FocusDefault()
	{
		if (m_PrimaryNav && m_PrimaryNav.FocusActive())
			return;

		super.FocusDefault();
	}

	// ── Modes and pages ─────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnPrimarySelected(TBD_BriefingPrimaryNav nav, int index)
	{
		SetMode(index);
	}

	//------------------------------------------------------------------------------------------------
	protected void OnTopicSelected(TBD_BriefingTopicNav nav, int index)
	{
		ShowPage(index);
	}

	//------------------------------------------------------------------------------------------------
	void SetMode(TBD_EBriefingMode mode)
	{
		m_eMode = mode;
		if (m_PrimaryNav)
			m_PrimaryNav.SetActive(mode);

		bool briefing = mode == TBD_EBriefingMode.BRIEFING;
		bool markers = mode == TBD_EBriefingMode.MARKERS;
		bool players = mode == TBD_EBriefingMode.PLAYERS;
		// Markers and Players pop out where the topic nav sits: directly right of the primary nav.
		TBD_UITheme.Show(GetDock("CenterDock"), briefing || markers);
		TBD_UITheme.Show(GetDock("RightDock"), briefing);
		TBD_UITheme.Show(GetDock("WideDock"), players);
		if (m_TopicNav)
			m_TopicNav.SetVisible(briefing);

		DestroyPage();
		DestroyMarkers();
		DestroyPlayers();

		if (briefing)
		{
			ShowPage(m_ePage);
		}
		else if (markers)
		{
			m_Markers = new TBD_BriefingMarkersPanel();
			m_Markers.Build(GetDock("CenterDock"), m_Catalog, GetOverlayDock());
		}
		else if (players)
		{
			m_PlayersPanel = new TBD_PlayersPanel();
			m_PlayersPanel.Build(GetDock("WideDock"));
		}

		Print(string.Format("[TBD][briefing] mode %1", typename.EnumToString(TBD_EBriefingMode, mode)));
	}

	//------------------------------------------------------------------------------------------------
	void ShowPage(TBD_EBriefingPage page)
	{
		m_ePage = page;
		if (m_TopicNav)
			m_TopicNav.SetActive(page);

		DestroyPage();
		SetRightWidth(TBD_BriefingNav.PageWidth(page));

		m_Page = TBD_BriefingNav.CreatePage(page);
		if (m_Page && !m_Page.Build(this, PageHost(), m_Catalog, m_Lobby))
			Print(string.Format("[TBD][briefing] page %1 failed to build", typename.EnumToString(TBD_EBriefingPage, page)), LogLevel.WARNING);
		else
			Print(string.Format("[TBD][briefing] page %1", typename.EnumToString(TBD_EBriefingPage, page)));
	}

	//------------------------------------------------------------------------------------------------
	//! Pan the map to a world position (objective / vehicle Locate).
	void LocateOnMap(float x, float z)
	{
		if (!m_MapEntity || !m_MapEntity.IsOpen())
		{
			Print(string.Format("[TBD][briefing] locate %1 %2 — map not open", x, z));
			return;
		}

		m_MapEntity.ZoomPanSmooth(0.3, x, z);
		Print(string.Format("[TBD][briefing] locate %1 %2", x, z));
	}

	//------------------------------------------------------------------------------------------------
	//! True when the pointer is over a visible piece of our chrome. Probes the panels, not the docks:
	//! LeftDock and CenterDock run the full height between the bars while the navs and the markers
	//! panel are short, so a dock rect would also swallow the map under them. Rule: a dock's visible
	//! children are chrome; a panel whose root outruns its glass names the glass `PanelBorder` (the
	//! navs stretch it over a shrink-wrapped root, the markers panel's is the 132 px box). RightDock
	//! probes `PageHost`, the page column — ORBAT mounts two panels side by side, so one border
	//! would under-cover.
	protected bool CursorOverChrome()
	{
		int mouseX, mouseY;
		WidgetManager.GetMousePos(mouseX, mouseY);

		array<string> docks = {"TopDock", "LeftDock", "CenterDock", "WideDock", "BottomDock", "OverlayDock"};
		foreach (string name : docks)
		{
			Widget dock = GetDock(name);
			if (!dock || !dock.IsVisible())
				continue;

			Widget child = dock.GetChildren();
			while (child)
			{
				if (child.IsVisible())
				{
					Widget probe = child.FindAnyWidget("PanelBorder");
					if (!probe)
						probe = child;

					if (Contains(probe, mouseX, mouseY))
						return true;
				}

				child = child.GetSibling();
			}
		}

		Widget rightDock = GetDock("RightDock");
		if (rightDock && rightDock.IsVisible() && Contains(Find("PageHost"), mouseX, mouseY))
			return true;

		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool Contains(Widget w, int mouseX, int mouseY)
	{
		if (!w)
			return false;

		float x, y, width, height;
		w.GetScreenPos(x, y);
		w.GetScreenSize(width, height);
		return mouseX >= x && mouseX <= x + width && mouseY >= y && mouseY <= y + height;
	}

	//------------------------------------------------------------------------------------------------
	protected void SetRightWidth(int width)
	{
		SizeLayoutWidget host = SizeLayoutWidget.Cast(Find("PageHost"));
		if (!host)
			return;

		// The host's frame slot is SizeToContent, so the override IS the column width; Update()
		// forces the relayout now rather than next frame (MEASURED run 4: width stuck at 448).
		host.EnableWidthOverride(true);
		host.SetWidthOverride(width);
		host.Update();
		Print(string.Format("[TBD][briefing] page width %1", width));
	}

	//------------------------------------------------------------------------------------------------
	//! The page column inside RightDock: `PageHost` is a SizeLayout whose width override IS the
	//! page width (RightDock runs to the screen edge so any page fits); `PageFrame` is the
	//! FrameWidget inside it that page roots (FrameWidgetSlot) mount into — a frame-slotted root
	//! straight under a SizeLayout gets no rect (MEASURED run 4).
	protected Widget PageHost()
	{
		return Find("PageFrame");
	}

	//------------------------------------------------------------------------------------------------
	protected void DestroyPage()
	{
		if (m_Page)
			m_Page.Destroy();

		m_Page = null;
		TBD_UILayouts.Clear(PageHost());
	}

	//------------------------------------------------------------------------------------------------
	protected void DestroyPlayers()
	{
		if (m_PlayersPanel)
			m_PlayersPanel.Destroy();

		m_PlayersPanel = null;
	}

	//------------------------------------------------------------------------------------------------
	protected void DestroyMarkers()
	{
		if (m_Markers)
			m_Markers.Destroy();

		m_Markers = null;
	}

	// ── TBD_DockScreen hooks ────────────────────────────────────────────────────────────────

	override protected string GetScreenTitle()
	{
		if (m_Catalog)
			return m_Catalog.GetMissionId();

		return "Briefing";
	}

	override protected bool IsTitleMono()
	{
		return true;
	}

	override protected int GetSessionTab()
	{
		return TBD_ESessionTab.BRIEFING;
	}

	override protected TBD_SessionIdentity GetSessionIdentity()
	{
		if (!m_Lobby)
			return null;

		TBD_SessionIdentity identity = m_Lobby.GetIdentity();
		if (!identity || !m_Players)
			return identity;

		return new TBD_SessionIdentity(identity.m_sName, identity.m_sRoleLabel, m_Players.Total(), m_Players.CapacityAll());
	}

	//------------------------------------------------------------------------------------------------
	//! Mock pass: the lobby's two toggles, one log line each (the wire step routes them).
	override protected void OnBottomAction(TBD_SessionBottomBar bar, string id)
	{
		if (id == ACTION_LOCK)
		{
			m_bLocked = !m_bLocked;
			TBD_UIButton button = bar.GetAction(ACTION_LOCK);
			if (m_bLocked)
			{
				bar.SetActionLabel(ACTION_LOCK, "Unlock Lobby");
				if (button)
					button.SetTint(TBD_EUITint.WARNING);
			}
			else
			{
				bar.SetActionLabel(ACTION_LOCK, "Lock Lobby");
				if (button)
					button.SetTint(TBD_EUITint.PRIMARY);
			}

			Print(string.Format("[TBD][briefing] LOCK LOBBY -> %1", m_bLocked));
			return;
		}

		if (id == ACTION_READY)
		{
			m_bReady = !m_bReady;
			TBD_UIButton button = bar.GetAction(ACTION_READY);
			if (m_bReady)
			{
				bar.SetActionLabel(ACTION_READY, "Ready (Waiting for Admin)");
				if (button)
					button.SetTint(TBD_EUITint.SUCCESS);
			}
			else
			{
				bar.SetActionLabel(ACTION_READY, "Ready & Continue");
				if (button)
					button.SetTint(TBD_EUITint.PRIMARY);
			}

			Print(string.Format("[TBD][briefing] READY -> %1", m_bReady));
		}
	}

	// ── Map lifecycle (kept verbatim from the previous screen) ──────────────────────────────

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
		PlayerController controller = GetGame().GetPlayerController();
		if (controller)
		{
			IEntity playerEnt = controller.GetControlledEntity();
			if (playerEnt)
				return playerEnt.GetOrigin();
		}

		BaseGameMode gm = GetGame().GetGameMode();
		if (gm)
			return gm.GetOrigin();

		return vector.Zero;
	}
}

modded enum ChimeraMenuPreset
{
	TBD_UIBriefing
}
