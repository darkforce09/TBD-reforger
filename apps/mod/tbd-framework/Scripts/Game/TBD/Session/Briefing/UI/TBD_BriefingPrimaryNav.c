//! Briefing rebuild (2026-09-14) — the primary navigation panel (`primary_navigation_panel`).
//!
//! A glass panel (`TBD_PrimaryNav.layout`: `PanelBorder`, `PanelBG`, `Items`) of four full-width
//! items (`TBD_PrimaryNavItem.layout`: `Border`, `Background`, `Accent`, `IconBoxBorder`,
//! `IconBoxBG`, `Icon`, `Label`, `BadgeDock`): Map · Briefing · Players [36] [48] · Markers.
//! Active = the blue fill with the white right bar and a lit icon box; hover = white/5; Players
//! carries the BLUFOR / OPFOR slotted counts as tinted chips. Not a `TBD_TabStrip`: the strip is
//! the top bar's horizontal segmented control and shrink-wraps, this is a stacked panel.
class TBD_PrimaryNavItemComponent : TBD_UIInteractive
{
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wAccentSize;
	protected Widget m_wAccent;
	protected Widget m_wIconBoxBorder;
	protected Widget m_wIconBoxBG;
	protected ImageWidget m_wIcon;
	protected TextWidget m_wLabel;
	protected Widget m_wBadgeDock;

	protected TBD_BriefingPrimaryNav m_Owner; //!< weak — the nav owns its items
	protected int m_iIndex = -1;
	protected bool m_bActive;
	protected bool m_bHasIcon;
	protected int m_iGround;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wAccentSize = w.FindAnyWidget("AccentSize");
		m_wAccent = w.FindAnyWidget("Accent");
		m_wIconBoxBorder = w.FindAnyWidget("IconBoxBorder");
		m_wIconBoxBG = w.FindAnyWidget("IconBoxBG");
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("Icon"));
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("Label"));
		m_wBadgeDock = w.FindAnyWidget("BadgeDock");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UILayouts.MountRounded(m_wIconBoxBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wIconBoxBG, TBD_UITheme.RADIUS_ROW - 1);
		TBD_UITheme.Show(m_wAccentSize, false);
	}

	//------------------------------------------------------------------------------------------------
	void Bind(TBD_BriefingPrimaryNav owner, int index, string label, string icon, int ground)
	{
		m_Owner = owner;
		m_iIndex = index;
		m_iGround = ground;
		TBD_UITheme.Write(m_wLabel, label);
		m_bHasIcon = TBD_UIIcons.Load(m_wIcon, icon);
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	//! A mono count chip after the label (the Players item's 36 / 48).
	void AddBadge(string text, TBD_EUITint tint)
	{
		TBD_ChipComponent chip = TBD_ChipComponent.Mount(m_wBadgeDock, text, tint, m_iGround);
		if (!chip)
			return;

		chip.SetUppercase(false);
		AlignableSlot.SetPadding(chip.GetRootWidget(), 4, 0, 0, 0);
	}

	//------------------------------------------------------------------------------------------------
	void SetActive(bool active)
	{
		if (m_bActive == active)
			return;

		m_bActive = active;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int fill;
		int border;
		int ink;
		int boxFill;
		int boxBorder;
		int iconInk;

		if (m_bActive)
		{
			fill = TBD_UITheme.NAV_ITEM_ACTIVE_FILL;
			border = TBD_UITheme.NAV_ITEM_ACTIVE_BORDER;
			ink = TBD_UITheme.ON_ACTION;
			boxFill = TBD_UITheme.ICON_BOX_ACTIVE_FILL;
			boxBorder = TBD_UITheme.ICON_BOX_ACTIVE_BORDER;
			iconInk = TBD_UITheme.ON_ACTION;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill = TBD_UITheme.NAV_ITEM_HOVER_FILL;
			border = TBD_UITheme.TRANSPARENT;
			ink = TBD_UITheme.BRIGHT_INK;
			boxFill = TBD_UITheme.ICON_BOX_HOVER_FILL;
			boxBorder = TBD_UITheme.ICON_BOX_HOVER_BORDER;
			iconInk = TBD_UITheme.ACTION;
		}
		else
		{
			fill = TBD_UITheme.TRANSPARENT;
			border = TBD_UITheme.TRANSPARENT;
			ink = TBD_UITheme.ON_SURFACE;
			boxFill = TBD_UITheme.ICON_BOX_FILL;
			boxBorder = TBD_UITheme.ICON_BOX_BORDER;
			iconInk = TBD_UITheme.MUTED_INK;
		}

		if (!m_bInteractive)
			ink = TBD_UITheme.ROW_DISABLED_TEXT;

		TBD_UITheme.PaintOver(m_wBorder, border, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, fill, m_iGround);
		int itemGround = TBD_UITheme.Over(fill, m_iGround);
		TBD_UITheme.PaintOver(m_wIconBoxBorder, boxBorder, itemGround);
		TBD_UITheme.PaintOver(m_wIconBoxBG, boxFill, itemGround);
		TBD_UITheme.Paint(m_wLabel, ink);
		if (m_bHasIcon)
			TBD_UITheme.Paint(m_wIcon, iconInk);

		TBD_UITheme.Show(m_wAccentSize, m_bActive);
		if (m_bActive)
			TBD_UITheme.PaintOver(m_wAccent, TBD_UITheme.NAV_ACCENT, itemGround);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnItemActivated(m_iIndex);
	}

	//------------------------------------------------------------------------------------------------
	int GetIndex()
	{
		return m_iIndex;
	}
}

//! The panel: builds the four items, tracks the active one, raises `GetOnSelected()(nav, index)`.
class TBD_BriefingPrimaryNav : Managed
{
	protected Widget m_wRoot;
	protected Widget m_wItems;
	protected ref array<TBD_PrimaryNavItemComponent> m_aItems;
	protected int m_iActive = -1;
	protected int m_iGround;

	//! (TBD_BriefingPrimaryNav nav, int index) — index is a TBD_EBriefingMode
	protected ref ScriptInvoker m_OnSelected;

	//------------------------------------------------------------------------------------------------
	bool Build(Widget dock, TBD_PlayersCatalog players)
	{
		m_aItems = {};
		if (!dock)
			return false;

		m_wRoot = TBD_UILayouts.Create(TBD_UILayouts.BRIEFING_PRIMARY_NAV, dock);
		if (!m_wRoot)
			return false;

		Widget border = m_wRoot.FindAnyWidget("PanelBorder");
		Widget background = m_wRoot.FindAnyWidget("PanelBG");
		m_wItems = m_wRoot.FindAnyWidget("Items");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.PaintOver(border, TBD_UITheme.GLASS_PANEL_BORDER, TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(background, TBD_UITheme.GLASS_PANEL_FILL, TBD_UITheme.Ground());
		m_iGround = TBD_UITheme.Over(TBD_UITheme.GLASS_PANEL_FILL, TBD_UITheme.Ground());

		AddItem("Map", "map");
		AddItem("Briefing", "description");
		TBD_PrimaryNavItemComponent playersItem = AddItem("Players", "groups");
		if (playersItem && players)
		{
			playersItem.AddBadge(players.CountSlotted("BLUFOR").ToString(), TBD_EUITint.BLUFOR);
			playersItem.AddBadge(players.CountSlotted("OPFOR").ToString(), TBD_EUITint.OPFOR);
		}
		AddItem("Markers", "edit_location_alt");
		return true;
	}

	//------------------------------------------------------------------------------------------------
	void Destroy()
	{
		if (m_aItems)
			m_aItems.Clear();

		m_wRoot = null;
		m_wItems = null;
	}

	//------------------------------------------------------------------------------------------------
	protected TBD_PrimaryNavItemComponent AddItem(string label, string icon)
	{
		if (!m_wItems)
			return null;

		TBD_PrimaryNavItemComponent item = TBD_PrimaryNavItemComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.BRIEFING_PRIMARY_NAV_ITEM, m_wItems, TBD_PrimaryNavItemComponent));
		if (!item)
			return null;

		item.Bind(this, m_aItems.Count(), label, icon, m_iGround);
		m_aItems.Insert(item);
		return item;
	}

	//------------------------------------------------------------------------------------------------
	void SetActive(int index)
	{
		m_iActive = index;
		foreach (int i, TBD_PrimaryNavItemComponent item : m_aItems)
		{
			if (item)
				item.SetActive(i == index);
		}
	}

	int GetActive()
	{
		return m_iActive;
	}

	//------------------------------------------------------------------------------------------------
	bool FocusActive()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace || m_aItems.IsEmpty())
			return false;

		int index = m_iActive;
		if (index < 0 || index >= m_aItems.Count())
			index = 0;

		Widget target = m_aItems[index].GetRootWidget();
		if (!target)
			return false;

		workspace.SetFocusedWidget(target);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	//------------------------------------------------------------------------------------------------
	void OnItemActivated(int index)
	{
		if (m_OnSelected)
			m_OnSelected.Invoke(this, index);
	}
}
