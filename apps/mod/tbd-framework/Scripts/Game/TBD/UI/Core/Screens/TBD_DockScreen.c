/**
 * @file TBD_DockScreen.c
 * @brief The base of every dock screen: a shell of named docks that the script fills at open.
 *
 * Role: mounts sub-layouts into the shell's named docks at open and removes them at close, and
 * mounts the shared session top and bottom bars and routes their tab and action events.
 * Position: base of `TBD_MissionSelectorScreen`, `TBD_LobbyScreen` and `TBD_BriefingScreen`;
 * derives from `TBD_MenuBase`; `TBD_DropdownComponent` opens its menus in `GetOverlayDock()`.
 * State: the mounted roots and the two bar handlers, on the client, from open to close.
 * Invariants: dock names `TopDock`, `LeftDock`, `CenterDock`, `RightDock`, `BottomDock` and
 * `OverlayDock` are a contract with the shells, the bars and the dropdown; the overlay dock is
 * hidden while empty; a mount failure logs a WARNING naming the dock or layout.
 */

//! Dock screen base. Layout: `TopDock` (56, the session top bar) over `LeftDock`, `CenterDock` and
//! `RightDock` (the per-screen panels) over `BottomDock` (64, the session bottom bar), with the
//! full-bleed `OverlayDock` as the last child. Column widths are each shell's own business.
//! A subclass overrides `OnScreenOpen()` (calling `super` first, so the bars are up), mounts its
//! panels with `Mount()` or `MountHandler()` and overrides `GetScreenTitle`, `GetSessionTab`,
//! `GetSessionIdentity` and `OnBottomAction`; `OnScreenClose` unmounts everything, so an override
//! releases only its own listeners and calls `super` last.
class TBD_DockScreen : TBD_MenuBase
{
	protected ref array<Widget> m_aMounted; //!< every root mounted since open; removed on close
	protected TBD_SessionTopBar m_TopBar; //!< the session top bar; null when unused or unmounted
	protected TBD_SessionBottomBar m_BottomBar; //!< the session bottom bar; null when unused or unmounted

	//! Paint the backdrop, hide the overlay dock and mount the bars.
	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();
		m_aMounted = {};

		TBD_UITheme.Paint(Find("Backdrop"), TBD_UITheme.SURFACE_CONTAINER_LOWEST);
		TBD_UITheme.Show(GetOverlayDock(), false);

		MountChrome();
	}

	//! Unsubscribe from the bars and unmount every mounted root.
	override protected void OnScreenClose()
	{
		if (m_TopBar)
			m_TopBar.GetOnTabSelected().Remove(OnTabSelected);

		if (m_BottomBar)
			m_BottomBar.GetOnAction().Remove(OnBottomAction);

		m_TopBar = null;
		m_BottomBar = null;
		UnmountAll();
		super.OnScreenClose();
	}

	//! Land on the primary action when a subclass has nothing better; subclasses usually do.
	override void FocusDefault()
	{
		if (m_BottomBar && m_BottomBar.FocusPrimary())
			return;

		if (m_TopBar && m_TopBar.GetStrip() && m_TopBar.GetStrip().FocusActive())
			return;

		super.FocusDefault();
	}

	//! Top-bar title. Screen-name style (`Scenario Browser`) unless IsTitleMono() says otherwise.
	protected string GetScreenTitle()
	{
		return "TBD";
	}

	//! True when the title is a scenario id to show verbatim (lobby, briefing).
	protected bool IsTitleMono()
	{
		return false;
	}

	//! Which top-bar tab this screen is. -1 = none highlighted.
	protected int GetSessionTab()
	{
		return -1;
	}

	//! Who is here. Null hides the identity cluster.
	protected TBD_SessionIdentity GetSessionIdentity()
	{
		return null;
	}

	//! Mount the shared top bar into TopDock. Off for screens with their own header (briefing map).
	protected bool UsesTopBar()
	{
		return true;
	}

	//! Mount the shared bottom bar into BottomDock.
	protected bool UsesBottomBar()
	{
		return true;
	}

	//! A bottom-bar action fired. `id` is what the screen passed to AddAction.
	protected void OnBottomAction(TBD_SessionBottomBar bar, string id) {}

	//! A top-bar tab was clicked. Default: swap this screen for the one the tab names. A screen
	//! that must confirm first (unsaved slot?) overrides and calls super when it is happy.
	protected void OnTabSelected(TBD_SessionTopBar bar, int tab)
	{
		if (tab == GetSessionTab())
			return;

		int preset = PresetForTab(tab);
		if (preset < 0)
		{
			// Nothing to open for that tab yet -- put the highlight back where the screen is.
			if (m_TopBar)
				m_TopBar.SetActiveTab(GetSessionTab());
			return;
		}

		TBD_MenuStack.Replace(preset);
	}

	//! Instantiate `layout` inside the dock named `dockName`. The root is remembered and removed
	//! on close. Null when the dock is missing or the layout does not resolve -- and a WARNING says
	//! which, because a silent empty dock is the hardest UI bug to see.
	protected Widget Mount(string dockName, ResourceName layout)
	{
		Widget dock = Find(dockName);
		if (!dock)
		{
			Print(string.Format("[TBD][ui] %1: shell has no dock named '%2'", ClassName(), dockName), LogLevel.WARNING);
			return null;
		}

		Widget created = TBD_UILayouts.Create(layout, dock);
		if (!created)
		{
			Print(string.Format("[TBD][ui] %1: could not mount %2 into %3", ClassName(), TBD_UILayouts.StripGuid(layout), dockName), LogLevel.WARNING);
			return null;
		}

		m_aMounted.Insert(created);
		return created;
	}

	//! Mount() and pull the layout's handler off the new root in one step.
	protected ScriptedWidgetComponent MountHandler(string dockName, ResourceName layout, typename handler)
	{
		Widget created = Mount(dockName, layout);
		if (!created)
			return null;

		ScriptedWidgetComponent found = ScriptedWidgetComponent.Cast(created.FindHandler(handler));
		if (!found)
			Print(string.Format("[TBD][ui] %1: %2 carries no %3 handler", ClassName(), TBD_UILayouts.StripGuid(layout), handler), LogLevel.ERROR);

		return found;
	}

	//! Remove every mounted root from the hierarchy.
	protected void UnmountAll()
	{
		if (!m_aMounted)
			return;

		foreach (Widget w : m_aMounted)
		{
			if (w)
				w.RemoveFromHierarchy();
		}

		m_aMounted.Clear();
	}

	//! The named dock, or null. Panels that build their own children use this.
	Widget GetDock(string dockName)
	{
		return Find(dockName);
	}

	//! Full-bleed host for popovers and modals. Hidden while empty (see TBD_DropdownComponent).
	Widget GetOverlayDock()
	{
		return Find("OverlayDock");
	}

	//! @return the session top bar, or null
	TBD_SessionTopBar GetTopBar()
	{
		return m_TopBar;
	}

	//! @return the session bottom bar, or null
	TBD_SessionBottomBar GetBottomBar()
	{
		return m_BottomBar;
	}

	//! Tab -> preset. -1 when the tab has no screen yet.
	static int PresetForTab(int tab)
	{
		switch (tab)
		{
			case TBD_ESessionTab.SCENARIO_BROWSER: return ChimeraMenuPreset.TBD_UIMissionSelector;
			case TBD_ESessionTab.LOBBY:            return ChimeraMenuPreset.TBD_UILobby;
			case TBD_ESessionTab.BRIEFING:         return ChimeraMenuPreset.TBD_UIBriefing;
		}

		return -1;
	}

	//! Mount the top bar (title, tab, identity, tab events) and the bottom bar (action events) when used.
	protected void MountChrome()
	{
		if (UsesTopBar())
		{
			m_TopBar = TBD_SessionTopBar.Cast(MountHandler("TopDock", TBD_UILayouts.SESSION_TOP_BAR, TBD_SessionTopBar));
			if (m_TopBar)
			{
				m_TopBar.SetTitle(GetScreenTitle(), IsTitleMono());
				m_TopBar.SetActiveTab(GetSessionTab());
				m_TopBar.SetIdentity(GetSessionIdentity());
				m_TopBar.GetOnTabSelected().Insert(OnTabSelected);
			}
		}

		if (UsesBottomBar())
		{
			m_BottomBar = TBD_SessionBottomBar.Cast(MountHandler("BottomDock", TBD_UILayouts.SESSION_BOTTOM_BAR, TBD_SessionBottomBar));
			if (m_BottomBar)
				m_BottomBar.GetOnAction().Insert(OnBottomAction);
		}
	}
}
