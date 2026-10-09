/**
 * @file TBD_BriefingPrimaryNav.c
 * @brief The Briefing primary navigation panel: Map, Briefing, Players and Markers.
 *
 * Role: builds the glass panel of TBD_PrimaryNav.layout with four TBD_PrimaryNavItemComponent
 * items, tracks the active one and raises the selection.  Position: TBD_BriefingScreen builds it
 * in LeftDock and binds `GetOnSelected()`; the Players item shows TBD_PlayersCatalog's slotted counts.
 * State: the item list and active index on the client, owned by the screen.  Invariants: indices
 * are TBD_EBriefingMode; it is a stacked panel, not a TBD_TabStrip, which is the top bar's
 * shrink-wrapping segmented control.
 */

//! The primary navigation panel; raises `GetOnSelected()` with (nav, index).
class TBD_BriefingPrimaryNav : Managed
{
	protected Widget m_wRoot; //!< the panel root
	protected Widget m_wItems; //!< `Items`, the item column
	protected ref array<TBD_PrimaryNavItemComponent> m_aItems; //!< items in TBD_EBriefingMode order
	protected int m_iActive = -1; //!< the active index; -1 for none
	protected int m_iGround; //!< ARGB ground colour under the items
	protected ref ScriptInvoker m_OnSelected; //!< (TBD_BriefingPrimaryNav nav, int index); index is a TBD_EBriefingMode

	//! Build the panel and its four items into `dock`.
	//! @param dock the dock to build into
	//! @param players the counts for the Players chips; null shows none
	//! @return false when the dock or the layout is missing
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

	//! Drop the items and widget references; the dock owns the widgets.
	void Destroy()
	{
		if (m_aItems)
			m_aItems.Clear();

		m_wRoot = null;
		m_wItems = null;
	}

	//! Create and bind one item at the end of the column.
	//! @return the item, or null when the column or the layout is missing
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

	//! Mark the item at `index` active and every other item inactive.
	//! @param index a TBD_EBriefingMode
	void SetActive(int index)
	{
		m_iActive = index;
		foreach (int i, TBD_PrimaryNavItemComponent item : m_aItems)
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
	//! @param index the item's TBD_EBriefingMode
	void OnItemActivated(int index)
	{
		if (m_OnSelected)
			m_OnSelected.Invoke(this, index);
	}
}
