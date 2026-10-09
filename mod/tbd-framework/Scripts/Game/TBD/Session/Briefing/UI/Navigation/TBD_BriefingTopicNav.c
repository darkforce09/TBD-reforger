/**
 * @file TBD_BriefingTopicNav.c
 * @brief The Briefing topic navigation panel: ten topics in three groups.
 *
 * Role: builds the glass panel of TBD_TopicNav.layout with one TBD_TopicNavItemComponent per
 * TBD_BriefingNav.TopicItems entry, tracks the active one and raises the selection.
 * Position: TBD_BriefingScreen builds it in CenterDock, right of the primary navigation, and binds `GetOnSelected()`.
 * State: the item list and active index on the client, owned by the screen.  Invariants: indices
 * are TBD_EBriefingPage; it is hidden while the Markers panel uses the column.
 */

//! The topic navigation panel; raises `GetOnSelected()` with (nav, index).
class TBD_BriefingTopicNav : Managed
{
	protected Widget m_wRoot; //!< the panel root
	protected Widget m_wItems; //!< `Items`, the item column
	protected ref array<TBD_TopicNavItemComponent> m_aItems; //!< items in TBD_EBriefingPage order
	protected int m_iActive = -1; //!< the active index; -1 for none
	protected int m_iGround; //!< ARGB ground colour under the items

	protected ref ScriptInvoker m_OnSelected; //!< (TBD_BriefingTopicNav nav, int index) -- index is a TBD_EBriefingPage

	//! Build the panel and its items into `dock`.
	//! @param dock the dock to build into
	//! @return false when the dock or the layout is missing
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

	//! Drop the items and widget references; the dock owns the widgets.
	void Destroy()
	{
		if (m_aItems)
			m_aItems.Clear();

		m_wRoot = null;
		m_wItems = null;
	}

	//! Mark the item at `index` active and every other item inactive.
	//! @param index a TBD_EBriefingPage
	void SetActive(int index)
	{
		m_iActive = index;
		foreach (int i, TBD_TopicNavItemComponent item : m_aItems)
		{
			if (item)
				item.SetActive(i == index);
		}
	}

	//! @return the active index, or -1
	int GetActive()
	{
		return m_iActive;
	}

	//! Show or hide the panel; it is hidden while the Markers panel uses the column.
	//! @param visible the new visibility
	void SetVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
	}

	//! Give keyboard focus to the active item, else the first.
	//! @return false when there is no workspace, no item or no root widget
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

	//! @return the selection invoker; created on first use
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	//! Raise the selection for the activated item.
	//! @param index the item's TBD_EBriefingPage
	void OnItemActivated(int index)
	{
		if (m_OnSelected)
			m_OnSelected.Invoke(this, index);
	}
}
