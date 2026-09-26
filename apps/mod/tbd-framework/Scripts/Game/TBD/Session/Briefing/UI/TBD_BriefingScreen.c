/**
 * @file TBD_BriefingScreen.c
 * @brief The Briefing screen: docked navigation, pages and actions floating over the live map.
 *
 * Role: lays out TBD_BriefingScreen.layout over a full-screen map: the top bar (mono mission id,
 * BRIEFING tab, identity, count), the primary navigation in LeftDock, the topic navigation or the
 * markers panel in CenterDock, the players panel in WideDock, the page column in RightDock, and the
 * bottom bar (Lock Lobby, Ready & Continue).  Position: opened through TBD_MenuStack on the
 * TBD_UIBriefing preset by SCR_PlayerController.TBD_OnStageChanged; reads TBD_BriefingCatalog,
 * TBD_LobbyCatalog and TBD_PlayersCatalog; the map runs through TBD_BriefingMapLauncher.
 * State: the current mode, page, lock toggle and the owned navs, page and panels, on the client.
 * Invariants: the shell has no backdrop and the map is never resized; MapContext is armed only
 * while the cursor is off the chrome, so the wheel scrolls the panel under it; a page, the markers
 * panel and the players panel are destroyed before the next mode or page is built; a deployed
 * answer closes every pre-game screen one frame later, never from inside the invoker.
 */

//! The Briefing screen; the class name is referenced by chimeraMenus.conf.
class TBD_BriefingScreen : TBD_DockScreen
{
	static const string ACTION_LOCK = "lock_lobby"; //!< bottom-bar action id of Lock Lobby
	static const string ACTION_READY = "ready"; //!< bottom-bar action id of Ready & Continue

	protected ref TBD_BriefingMapLauncher m_Map; //!< the map lifecycle; created in OnMenuInit
	protected TBD_BriefingCatalog m_Catalog; //!< the briefing data
	protected TBD_LobbyCatalog m_Lobby; //!< the lobby data (identity, ORBAT)
	protected TBD_PlayersCatalog m_Players; //!< the player counts

	protected ref TBD_BriefingPrimaryNav m_PrimaryNav; //!< the primary navigation
	protected ref TBD_BriefingTopicNav m_TopicNav; //!< the topic navigation
	protected ref TBD_BriefingPage m_Page; //!< the shown page; null outside BRIEFING mode
	protected ref TBD_BriefingMarkersPanel m_Markers; //!< the markers panel; null outside MARKERS mode
	protected ref TBD_PlayersPanel m_PlayersPanel; //!< the players panel; null outside PLAYERS mode

	protected TBD_EBriefingMode m_eMode = TBD_EBriefingMode.BRIEFING; //!< the current mode; default BRIEFING
	protected TBD_EBriefingPage m_ePage = TBD_EBriefingPage.FREQUENCIES; //!< the page BRIEFING mode shows; default FREQUENCIES
	protected bool m_bLocked; //!< the Lock Lobby toggle; default false

	//! Create the map launcher.
	override void OnMenuInit()
	{
		super.OnMenuInit();
		m_Map = new TBD_BriefingMapLauncher(this);
	}

	//! @return the map launcher, or null before OnMenuInit
	TBD_BriefingMapLauncher GetMapLauncher()
	{
		return m_Map;
	}

	//! TBD_MenuBase arms this context every tick before OnScreenUpdate.
	//! @return `MapContext` while the cursor is off the chrome; empty over it, so the wheel scrolls
	//! the panel under the cursor instead of reaching SCR_MapCursorModule's zoom listeners
	override protected string GetInputContext()
	{
		if (CursorOverChrome())
			return string.Empty;

		return "MapContext";
	}

	//! Build the navigations and bottom-bar actions, bind the deploy answer, show BRIEFING mode
	//! and open the map.
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

		TBD_SpawnClient.GetOnDeployResult().Insert(OnDeployResult);

		SetMode(TBD_EBriefingMode.BRIEFING);

		if (m_Map)
			m_Map.Open();

		Print("[TBD][briefing] Briefing opened (dock shell over the map).");
	}

	//! Unbind, cancel the deferred close and the map open, close the map and destroy every
	//! navigation, page and panel.
	override protected void OnScreenClose()
	{
		TBD_SpawnClient.GetOnDeployResult().Remove(OnDeployResult);
		GetGame().GetCallqueue().Remove(CloseAfterDeploy);
		if (m_Map)
			m_Map.Close();

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

	//! Focus the active primary navigation item, else the base default.
	override void FocusDefault()
	{
		if (m_PrimaryNav && m_PrimaryNav.FocusActive())
			return;

		super.FocusDefault();
	}

	//! Switch to the selected mode.
	//! @param nav the primary navigation
	//! @param index a TBD_EBriefingMode
	protected void OnPrimarySelected(TBD_BriefingPrimaryNav nav, int index)
	{
		SetMode(index);
	}

	//! Show the selected page.
	//! @param nav the topic navigation
	//! @param index a TBD_EBriefingPage
	protected void OnTopicSelected(TBD_BriefingTopicNav nav, int index)
	{
		ShowPage(index);
	}

	//! Switch mode: show the docks the mode uses, destroy the previous page and panels, and build
	//! the page, the markers panel or the players panel.
	//! @param mode the new mode
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

	//! Show `page` in the page column at its width; logs a WARNING when it fails to build.
	//! @param page the page
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

	//! True when the pointer is over a visible piece of the chrome. Probes the panels, not the docks:
	//! LeftDock and CenterDock run the full height between the bars while the navs and the markers
	//! panel are short, so a dock rect would also swallow the map under them. Rule: a dock's visible
	//! children are chrome; a panel whose root outruns its glass names the glass `PanelBorder` (the
	//! navs stretch it over a shrink-wrapped root, the markers panel's is the 132 px box). RightDock
	//! probes `PageHost`, the page column -- ORBAT mounts two panels side by side, so one border
	//! would under-cover.
	//! @return true over the chrome
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

	//! @param w the widget to test; null never contains
	//! @param mouseX the cursor X, screen pixels
	//! @param mouseY the cursor Y, screen pixels
	//! @return true when the cursor is inside `w`'s screen rectangle
	protected static bool Contains(Widget w, int mouseX, int mouseY)
	{
		if (!w)
			return false;

		float x, y, width, height;
		w.GetScreenPos(x, y);
		w.GetScreenSize(width, height);
		return mouseX >= x && mouseX <= x + width && mouseY >= y && mouseY <= y + height;
	}

	//! Set the page column width through `PageHost`'s width override and relayout at once.
	//! @param width the width in pixels
	protected void SetRightWidth(int width)
	{
		SizeLayoutWidget host = SizeLayoutWidget.Cast(Find("PageHost"));
		if (!host)
			return;

		// The host's frame slot is SizeToContent, so the override IS the column width; Update()
		// forces the relayout now; without it the width stays at its previous value for a frame.
		host.EnableWidthOverride(true);
		host.SetWidthOverride(width);
		host.Update();
		Print(string.Format("[TBD][briefing] page width %1", width));
	}

	//! The page column inside RightDock: `PageHost` is a SizeLayout whose width override IS the
	//! page width (RightDock runs to the screen edge so any page fits); `PageFrame` is the
	//! FrameWidget inside it that page roots (FrameWidgetSlot) mount into -- a frame-slotted root
	//! straight under a SizeLayout gets no rectangle.
	//! @return `PageFrame`, or null
	protected Widget PageHost()
	{
		return Find("PageFrame");
	}

	//! Destroy the page and clear the page column.
	protected void DestroyPage()
	{
		if (m_Page)
			m_Page.Destroy();

		m_Page = null;
		TBD_UILayouts.Clear(PageHost());
	}

	//! Destroy the players panel.
	protected void DestroyPlayers()
	{
		if (m_PlayersPanel)
			m_PlayersPanel.Destroy();

		m_PlayersPanel = null;
	}

	//! Destroy the markers panel.
	protected void DestroyMarkers()
	{
		if (m_Markers)
			m_Markers.Destroy();

		m_Markers = null;
	}

	//! @return the catalog's mission id, else `Briefing`
	override protected string GetScreenTitle()
	{
		if (m_Catalog)
			return m_Catalog.GetMissionId();

		return "Briefing";
	}

	//! @return true: the title is a mono mission id
	override protected bool IsTitleMono()
	{
		return true;
	}

	//! @return the BRIEFING session tab
	override protected int GetSessionTab()
	{
		return TBD_ESessionTab.BRIEFING;
	}

	//! @return the lobby identity with the total and capacity from TBD_PlayersCatalog, or null
	//! without lobby data
	override protected TBD_SessionIdentity GetSessionIdentity()
	{
		if (!m_Lobby)
			return null;

		TBD_SessionIdentity identity = m_Lobby.GetIdentity();
		if (!identity || !m_Players)
			return identity;

		return new TBD_SessionIdentity(identity.m_sName, identity.m_sRoleLabel, m_Players.Total(), m_Players.CapacityAll());
	}

	//! Lock Lobby flips its label and tint and logs only; Ready & Continue reports readiness and
	//! asks the authority for a body (TBD_SpawnClient -> TBD_SpawnManager.DeployOnReady).
	//! @param bar the bottom bar
	//! @param id the action id
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
			// Ready & Continue = "I have read my orders, put me in a body". The tally is the existing
			// one (TBD_ReportReady); the body comes from TBD_SpawnManager.DeployOnReady through
			// TBD_SpawnClient, and the answer lands in OnDeployResult.
			bar.SetActionLabel(ACTION_READY, "Deploying...");
			bar.SetActionEnabled(ACTION_READY, false);
			TBD_BriefingClient.ReportReady();
			TBD_SpawnClient.Request();
			Print("[TBD][briefing] READY -> deploy requested");
		}
	}

	//! The authority's answer to Ready & Continue. Deployed: every pre-game screen goes (the map
	//! closes with this one; TBD_LobbyStage does not re-raise -- the stage is off LOBBY or the
	//! player now controls a body). Deferred one tick so the stack never tears this screen down
	//! from inside the invoker it is bound to. Refused: the button carries the reason and can be
	//! pressed again.
	//! @param ok the player was deployed
	//! @param why the refusal reason when not
	protected void OnDeployResult(bool ok, string why)
	{
		if (ok)
		{
			Print("[TBD][briefing] deployed -- closing the pre-game screens");
			GetGame().GetCallqueue().Call(CloseAfterDeploy);
			return;
		}

		Print(string.Format("[TBD][briefing] deploy refused: %1", why), LogLevel.WARNING);
		if (!m_BottomBar)
			return;

		m_BottomBar.SetActionLabel(ACTION_READY, why);
		m_BottomBar.SetActionEnabled(ACTION_READY, true);
		TBD_UIButton button = m_BottomBar.GetAction(ACTION_READY);
		if (button)
			button.SetTint(TBD_EUITint.WARNING);
	}

	//! Close every pre-game screen.
	protected void CloseAfterDeploy()
	{
		TBD_MenuStack.CloseAll();
	}
}
