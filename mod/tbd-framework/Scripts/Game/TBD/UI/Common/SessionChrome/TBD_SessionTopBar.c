/**
 * @file TBD_SessionTopBar.c
 * @brief The bar every pre-game screen wears: title, screen tabs, identity and player count.
 *
 * Role: handler of `Session/Shared/TBD_SessionTopBar.layout`; shows the title, echoes the active
 * `TBD_ESessionTab`, shows who is looking and how many are here, and reports tab clicks.
 * Position: mounted by `TBD_DockScreen` into `TopDock`; the screen sets title, tab and identity and
 * listens to `GetOnTabSelected()`; which screen a tab opens is `TBD_DockScreen`'s decision.
 * State: the widget references, the tab strip and the role chip, on the client.
 * Invariants: widget contract `BarBorder`, `BarBG`, `Title`, `TabStripDock`, `IdentityBox`,
 * `IdentityBorder`, `IdentityBG`, `IdentityName`, `IdentityRoleDock`, `CountBox`, `CountBorder`,
 * `CountBG`, `CountIcon`, `CountText`; the bar sits on the backdrop and its boxes on the bar.
 */
//! Who is looking at the screen and how many are here: the top bar's right cluster. Built by the
//! mock catalogs and the screens that fill a dock screen's identity.
class TBD_SessionIdentity
{
	string m_sName; //!< the viewer's display name
	string m_sRoleLabel;    //!< "ADMIN", "MISSION MAKER", empty = no chip
	TBD_EUITint m_eRoleTint = TBD_EUITint.WARNING; //!< the role chip tint; default WARNING
	int m_iConnected; //!< players connected
	int m_iCapacity = -1;   //!< -1 = show the count alone

	//! Describe the viewer and the head count.
	//! @param name display name; @param roleLabel role chip text, empty for none
	//! @param connected players connected; @param capacity slot capacity, -1 shows the count alone
	void TBD_SessionIdentity(string name, string roleLabel, int connected, int capacity = -1)
	{
		m_sName = name;
		m_sRoleLabel = roleLabel;
		m_iConnected = connected;
		m_iCapacity = capacity;
	}
}

//! Top bar handler.
class TBD_SessionTopBar : ScriptedWidgetComponent
{
	protected Widget m_wRoot; //!< the layout root this handler sits on
	protected Widget m_wBorder; //!< `BarBorder` frame dock
	protected Widget m_wBackground; //!< `BarBG` frame dock
	protected TextWidget m_wTitle; //!< `Title`
	protected Widget m_wTabStripDock; //!< `TabStripDock`, where the tab strip is created
	protected Widget m_wIdentityBox; //!< `IdentityBox`, hidden without an identity
	protected Widget m_wIdentityBorder; //!< `IdentityBorder` frame dock
	protected Widget m_wIdentityBG; //!< `IdentityBG` frame dock
	protected TextWidget m_wIdentityName; //!< `IdentityName`
	protected Widget m_wIdentityRoleDock; //!< `IdentityRoleDock`, hidden without a role label
	protected Widget m_wCountBox; //!< `CountBox`, hidden without an identity
	protected Widget m_wCountBorder; //!< `CountBorder` frame dock
	protected Widget m_wCountBG; //!< `CountBG` frame dock
	protected ImageWidget m_wCountIcon; //!< `CountIcon`
	protected TextWidget m_wCountText; //!< `CountText`: `1` or `1 / 48`

	protected TBD_TabStripComponent m_Strip; //!< the screen tabs, created at attach
	protected TBD_ChipComponent m_RoleChip; //!< the identity role chip, mounted on first use

	protected ref ScriptInvoker m_OnTabSelected; //!< (TBD_SessionTopBar bar, TBD_ESessionTab tab)

	//! Find the widgets, round and paint the bar and its boxes, load the count icon, mount the strip.
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;

		m_wBorder = w.FindAnyWidget("BarBorder");
		m_wBackground = w.FindAnyWidget("BarBG");
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wTabStripDock = w.FindAnyWidget("TabStripDock");
		m_wIdentityBox = w.FindAnyWidget("IdentityBox");
		m_wIdentityBorder = w.FindAnyWidget("IdentityBorder");
		m_wIdentityBG = w.FindAnyWidget("IdentityBG");
		m_wIdentityName = TextWidget.Cast(w.FindAnyWidget("IdentityName"));
		m_wIdentityRoleDock = w.FindAnyWidget("IdentityRoleDock");
		m_wCountBox = w.FindAnyWidget("CountBox");
		m_wCountBorder = w.FindAnyWidget("CountBorder");
		m_wCountBG = w.FindAnyWidget("CountBG");
		m_wCountIcon = ImageWidget.Cast(w.FindAnyWidget("CountIcon"));
		m_wCountText = TextWidget.Cast(w.FindAnyWidget("CountText"));

		// Mockup: the bar is a rounded-xl card; identity + count are rounded-lg boxes on it.
		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UILayouts.MountRounded(m_wIdentityBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wIdentityBG, TBD_UITheme.RADIUS_ROW - 1);
		TBD_UILayouts.MountRounded(m_wCountBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wCountBG, TBD_UITheme.RADIUS_ROW - 1);

		// The bar sits on the backdrop; the identity / count boxes sit on the bar.
		int ground = TBD_UITheme.Ground();
		int barFill = TBD_UITheme.Over(TBD_UITheme.PANEL_FILL, ground);
		TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.STRIP_BORDER, ground);
		TBD_UITheme.PaintOver(m_wBackground, barFill, ground);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.PaintOver(m_wIdentityBorder, TBD_UITheme.STRIP_BORDER, barFill);
		TBD_UITheme.PaintOver(m_wIdentityBG, TBD_UITheme.SURFACE_CONTAINER_LOW, barFill);
		TBD_UITheme.Paint(m_wIdentityName, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.PaintOver(m_wCountBorder, TBD_UITheme.STRIP_BORDER, barFill);
		TBD_UITheme.PaintOver(m_wCountBG, TBD_UITheme.SURFACE_CONTAINER_LOW, barFill);
		TBD_UITheme.Paint(m_wCountText, TBD_UITheme.BRIGHT_INK);

		if (m_wCountIcon)
		{
			TBD_UIIcons.Load(m_wCountIcon, "group");
			TBD_UITheme.Paint(m_wCountIcon, TBD_UITheme.MUTED_INK);
		}

		MountStrip();
	}

	//! Unsubscribe from the strip and drop the references when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		if (m_Strip)
			m_Strip.GetOnSelected().Remove(OnStripSelected);

		m_Strip = null;
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//! `mono = true` shows a scenario id as-is (`wog_187_chollima_on_the_wing_10`); `false` shows a
	//! screen name shouted (`SCENARIO BROWSER`). Same widget, two voices -- exactly the mockup.
	void SetTitle(string title, bool mono)
	{
		string shown = title;
		if (!mono)
			shown.ToUpper();

		TBD_UITheme.Write(m_wTitle, shown);
	}

	//! Visual echo of the screen that is open. Does not fire OnTabSelected.
	void SetActiveTab(TBD_ESessionTab tab)
	{
		if (m_Strip)
			m_Strip.SetActive(tab);
	}

	//! Show the identity box and count, or hide both when `identity` is null.
	void SetIdentity(TBD_SessionIdentity identity)
	{
		if (!identity)
		{
			TBD_UITheme.Show(m_wIdentityBox, false);
			TBD_UITheme.Show(m_wCountBox, false);
			return;
		}

		TBD_UITheme.Show(m_wIdentityBox, true);
		TBD_UITheme.Show(m_wCountBox, true);
		TBD_UITheme.Write(m_wIdentityName, identity.m_sName);

		if (identity.m_sRoleLabel.IsEmpty())
		{
			TBD_UITheme.Show(m_wIdentityRoleDock, false);
		}
		else
		{
			if (!m_RoleChip)
				m_RoleChip = TBD_ChipComponent.Mount(m_wIdentityRoleDock, identity.m_sRoleLabel, identity.m_eRoleTint);
			else
				m_RoleChip.Set(identity.m_sRoleLabel, identity.m_eRoleTint);

			TBD_UITheme.Show(m_wIdentityRoleDock, m_RoleChip != null);
		}

		SetPlayerCount(identity.m_iConnected, identity.m_iCapacity);
	}

	//! `1` or `1 / 48`.
	void SetPlayerCount(int connected, int capacity = -1)
	{
		if (capacity < 0)
			TBD_UITheme.Write(m_wCountText, connected.ToString());
		else
			TBD_UITheme.Write(m_wCountText, string.Format("%1 / %2", connected, capacity));
	}

	//! (TBD_SessionTopBar bar, TBD_ESessionTab tab)
	ScriptInvoker GetOnTabSelected()
	{
		if (!m_OnTabSelected)
			m_OnTabSelected = new ScriptInvoker();

		return m_OnTabSelected;
	}

	//! @return the screen tab strip, or null when the dock or layout is missing
	TBD_TabStripComponent GetStrip()
	{
		return m_Strip;
	}

	//! @return the layout root this handler sits on
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! Create the tab strip on the bar's fill with the three session tabs and subscribe to it.
	protected void MountStrip()
	{
		if (!m_wTabStripDock)
			return;

		m_Strip = TBD_TabStripComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.TAB_STRIP, m_wTabStripDock, TBD_TabStripComponent));
		if (!m_Strip)
			return;

		// The strip sits on the bar's fill, not on the backdrop.
		m_Strip.SetGround(TBD_UITheme.Over(TBD_UITheme.PANEL_FILL, TBD_UITheme.Ground()));

		array<ref TBD_NavItemData> tabs = {};
		tabs.Insert(new TBD_NavItemData("Scenario Browser", "grid_view"));
		tabs.Insert(new TBD_NavItemData("Lobby", "meeting_room"));
		tabs.Insert(new TBD_NavItemData("Briefing", "description"));
		m_Strip.SetItems(tabs);

		m_Strip.GetOnSelected().Insert(OnStripSelected);
	}

	//! Forward a strip click as OnTabSelected with the tab index.
	protected void OnStripSelected(TBD_TabStripComponent strip, int index)
	{
		if (m_OnTabSelected)
			m_OnTabSelected.Invoke(this, index);
	}
}
