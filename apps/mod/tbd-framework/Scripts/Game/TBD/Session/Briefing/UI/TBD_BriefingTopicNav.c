//! Briefing rebuild (2026-09-14) — the topic navigation panel (`briefing_navigation_panel`).
//!
//! A glass panel (`TBD_TopicNav.layout`: `PanelBorder`, `PanelBG`, `Items`) directly right of the
//! primary nav, holding the ten topics of `TBD_BriefingNav.TopicItems` in three groups; an item
//! (`TBD_TopicNavItem.layout`: `SeparatorSize`/`Separator`, `Border`, `Background`, `Icon`,
//! `Label`) shows the rule above it when its `TBD_NavItemData.m_bSeparatorBefore` is set. Active =
//! solid tactical blue, hover = slate-800/50 with a faint border. Indices are `TBD_EBriefingPage`.
class TBD_TopicNavItemComponent : TBD_UIInteractive
{
	protected Widget m_wSeparatorSize;
	protected Widget m_wSeparator;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected ImageWidget m_wIcon;
	protected TextWidget m_wLabel;

	protected TBD_BriefingTopicNav m_Owner; //!< weak — the nav owns its items
	protected int m_iIndex = -1;
	protected bool m_bActive;
	protected bool m_bHasIcon;
	protected int m_iGround;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wSeparatorSize = w.FindAnyWidget("SeparatorSize");
		m_wSeparator = w.FindAnyWidget("Separator");
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("Icon"));
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("Label"));

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.Show(m_wSeparatorSize, false);
	}

	//------------------------------------------------------------------------------------------------
	void Bind(TBD_BriefingTopicNav owner, int index, TBD_NavItemData data, int ground)
	{
		m_Owner = owner;
		m_iIndex = index;
		m_iGround = ground;
		TBD_UITheme.Write(m_wLabel, data.m_sLabel);
		m_bHasIcon = TBD_UIIcons.Load(m_wIcon, data.m_sIcon);
		TBD_UITheme.Show(m_wSeparatorSize, data.m_bSeparatorBefore);
		SetInteractive(data.m_bEnabled);
		Repaint();
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

		if (m_bActive)
		{
			fill = TBD_UITheme.TOPIC_ITEM_ACTIVE_FILL;
			border = TBD_UITheme.TRANSPARENT;
			ink = TBD_UITheme.ON_ACTION;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill = TBD_UITheme.TOPIC_ITEM_HOVER_FILL;
			border = TBD_UITheme.TOPIC_ITEM_HOVER_BORDER;
			ink = TBD_UITheme.BRIGHT_INK;
		}
		else
		{
			fill = TBD_UITheme.TRANSPARENT;
			border = TBD_UITheme.TRANSPARENT;
			ink = TBD_UITheme.ON_SURFACE_VARIANT;
		}

		if (!m_bInteractive)
			ink = TBD_UITheme.ROW_DISABLED_TEXT;

		TBD_UITheme.PaintOver(m_wBorder, border, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, fill, m_iGround);
		TBD_UITheme.PaintOver(m_wSeparator, TBD_UITheme.STRIP_BORDER, m_iGround);
		TBD_UITheme.Paint(m_wLabel, ink);
		if (m_bHasIcon)
			TBD_UITheme.Paint(m_wIcon, ink);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnItemActivated(m_iIndex);
	}
}

//! The panel: builds the items from `TBD_BriefingNav.TopicItems`, raises `GetOnSelected()(nav, index)`.
class TBD_BriefingTopicNav : Managed
{
	protected Widget m_wRoot;
	protected Widget m_wItems;
	protected ref array<TBD_TopicNavItemComponent> m_aItems;
	protected int m_iActive = -1;
	protected int m_iGround;

	//! (TBD_BriefingTopicNav nav, int index) — index is a TBD_EBriefingPage
	protected ref ScriptInvoker m_OnSelected;

	//------------------------------------------------------------------------------------------------
	bool Build(Widget dock)
	{
		m_aItems = {};
		if (!dock)
			return false;

		m_wRoot = TBD_UILayouts.Create(TBD_UILayouts.BRIEFING_TOPIC_NAV, dock);
		if (!m_wRoot)
			return false;

		Widget border = m_wRoot.FindAnyWidget("PanelBorder");
		Widget background = m_wRoot.FindAnyWidget("PanelBG");
		m_wItems = m_wRoot.FindAnyWidget("Items");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.PaintOver(border, TBD_UITheme.STRIP_BORDER, TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(background, TBD_UITheme.TOPIC_NAV_FILL, TBD_UITheme.Ground());
		m_iGround = TBD_UITheme.Over(TBD_UITheme.TOPIC_NAV_FILL, TBD_UITheme.Ground());

		array<ref TBD_NavItemData> topics = {};
		TBD_BriefingNav.TopicItems(topics);
		foreach (int i, TBD_NavItemData topic : topics)
		{
			TBD_TopicNavItemComponent item = TBD_TopicNavItemComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.BRIEFING_TOPIC_NAV_ITEM, m_wItems, TBD_TopicNavItemComponent));
			if (!item)
				continue;

			item.Bind(this, i, topic, m_iGround);
			m_aItems.Insert(item);
		}

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
	void SetActive(int index)
	{
		m_iActive = index;
		foreach (int i, TBD_TopicNavItemComponent item : m_aItems)
		{
			if (item)
				item.SetActive(i == index);
		}
	}

	int GetActive()
	{
		return m_iActive;
	}

	//! Hidden while the Markers panel borrows the column.
	void SetVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
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
