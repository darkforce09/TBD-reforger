/**
 * @file TBD_TabStripComponent.c
 * @brief The segmented control: a row or column of pooled `TBD_NavItemComponent` tabs.
 *
 * Role: handler of `TBD_TabStrip.layout`; builds its tabs from a `TBD_NavItemData` table, echoes
 * the active tab and fires `GetOnSelected()(strip, index)` on a click.
 * Position: mounted by `TBD_SessionTopBar` for the Scenario Browser, Lobby and Briefing switch;
 * the owning screen decides what a tab means.
 * State: the widget references, the item pool, the live count, the active index and the ground
 * colour, on the client.
 * Invariants: widget contract `StripBorder`, `StripBG`, `ItemsRow` (horizontal), `ItemsColumn`
 * (vertical); `m_bVertical` picks the container and the other stays hidden; item widgets are
 * created once per index and surplus ones are hidden.
 */

//! Tab strip handler: pooled tabs, the active echo and the selection event.
class TBD_TabStripComponent : ScriptedWidgetComponent
{
	[Attribute("0", UIWidgets.CheckBox, desc: "Stack items vertically instead of in a row")]
	protected bool m_bVertical; //!< true stacks the tabs in `ItemsColumn`; default false

	[Attribute("1", UIWidgets.CheckBox, desc: "Draw the strip's own pill background (off for a bare vertical list)")]
	protected bool m_bChrome; //!< true draws the strip border and fill; default true

	protected Widget m_wRoot; //!< the layout root this handler sits on
	protected Widget m_wBorder; //!< `StripBorder` frame dock, rounded at attach
	protected Widget m_wBackground; //!< `StripBG` frame dock, rounded at attach
	protected Widget m_wItemsRow; //!< `ItemsRow`, the horizontal container
	protected Widget m_wItemsColumn; //!< `ItemsColumn`, the vertical container

	//! Pool, index-stable. Weak elements: each item handler is owned by its widget.
	protected ref array<TBD_NavItemComponent> m_aItems; //!< index-stable pool; weak elements, each item handler is owned by its widget
	protected int m_iLive; //!< how many pooled tabs are shown
	protected int m_iActive = -1; //!< the echoed tab; -1 = none
	protected int m_iGround; //!< Opaque colour under the strip; 0 = the backdrop.

	protected ref ScriptInvoker m_OnSelected; //!< (TBD_TabStripComponent strip, int index)

	//! Find the strip widgets, show the container `m_bVertical` picks and paint the chrome.
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_aItems = {};

		m_wBorder = w.FindAnyWidget("StripBorder");
		m_wBackground = w.FindAnyWidget("StripBG");
		m_wItemsRow = w.FindAnyWidget("ItemsRow");
		m_wItemsColumn = w.FindAnyWidget("ItemsColumn");

		TBD_UITheme.Show(m_wItemsRow, !m_bVertical);
		TBD_UITheme.Show(m_wItemsColumn, m_bVertical);

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
		RepaintChrome();
	}

	//! Stack the items vertically (call before SetItems; the layout attribute is the default).
	void SetVertical(bool vertical)
	{
		m_bVertical = vertical;
		TBD_UITheme.Show(m_wItemsRow, !vertical);
		TBD_UITheme.Show(m_wItemsColumn, vertical);
	}

	//! Opaque colour under the strip (the top bar sits on the backdrop; a vertical list may sit on a
	//! panel). Items recompute their ground from it.
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		RepaintChrome();
		foreach (TBD_NavItemComponent item : m_aItems)
		{
			if (item)
				item.Repaint();
		}
	}

	//! What the items sit on: the strip's own fill when it draws chrome, else the strip's ground.
	int GetItemGround()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.Ground();

		if (m_bChrome)
			return TBD_UITheme.Over(TBD_UITheme.STRIP_FILL, ground);

		return ground;
	}

	//! Paint the strip border and fill over the ground, or clear them when chrome is off.
	protected void RepaintChrome()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.Ground();

		if (m_bChrome)
		{
			TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.STRIP_BORDER, ground);
			TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.STRIP_FILL, ground);
		}
		else
		{
			TBD_UITheme.Paint(m_wBorder, TBD_UITheme.TRANSPARENT);
			TBD_UITheme.Paint(m_wBackground, TBD_UITheme.TRANSPARENT);
		}
	}

	//! Drop the pool and the root when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		if (m_aItems)
			m_aItems.Clear();

		m_wRoot = null;
		m_iLive = 0;
		super.HandlerDeattached(w);
	}

	//! Rebuild the strip from a table. Items are pooled: widgets are created only for indices the
	//! strip has never reached, surplus ones are hidden.
	void SetItems(notnull array<ref TBD_NavItemData> items)
	{
		Widget container = GetContainer();
		if (!container)
			return;

		int count = items.Count();
		for (int i = 0; i < count; i++)
		{
			TBD_NavItemComponent item = AcquireItem(i, container);
			if (!item)
				continue;

			item.Bind(this, i, items[i]);
			TBD_UITheme.Show(item.GetRootWidget(), true);
			item.SetActive(i == m_iActive);
		}

		m_iLive = count;
		for (int j = count; j < m_aItems.Count(); j++)
		{
			TBD_UITheme.Show(m_aItems[j].GetRootWidget(), false);
		}
	}

	//! Visual echo only -- does not fire OnSelected. Use it to reflect the screen that is open.
	void SetActive(int index)
	{
		m_iActive = index;
		for (int i = 0; i < m_aItems.Count(); i++)
		{
			m_aItems[i].SetActive(i == index);
		}
	}

	//! @return the echoed tab index; -1 = none
	int GetActive()
	{
		return m_iActive;
	}

	//! @return how many tabs are shown
	int GetItemCount()
	{
		return m_iLive;
	}

	//! (TBD_TabStripComponent strip, int index)
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	//! Focus the active tab (or the first) so a gamepad user lands on the strip.
	bool FocusActive()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace || m_iLive == 0)
			return false;

		int index = m_iActive;
		if (index < 0 || index >= m_iLive)
			index = 0;

		Widget target = m_aItems[index].GetRootWidget();
		if (!target)
			return false;

		workspace.SetFocusedWidget(target);
		return true;
	}

	//! @return the layout root this handler sits on
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! Called by a tab on click: echo it and fire OnSelected; an index outside the shown tabs is ignored.
	void OnItemActivated(int index)
	{
		if (index < 0 || index >= m_iLive)
			return;

		SetActive(index);

		if (m_OnSelected)
			m_OnSelected.Invoke(this, index);
	}

	//! @return the container the tabs go into: the column when vertical, else the row, else the root
	protected Widget GetContainer()
	{
		if (m_bVertical && m_wItemsColumn)
			return m_wItemsColumn;

		if (m_wItemsRow)
			return m_wItemsRow;

		return m_wRoot;
	}

	//! @return the pooled tab at `index`, created into `container` on first reach; null when the
	//! layout cannot be created
	protected TBD_NavItemComponent AcquireItem(int index, Widget container)
	{
		if (index < m_aItems.Count())
			return m_aItems[index];

		TBD_NavItemComponent item = TBD_NavItemComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.NAV_ITEM, container, TBD_NavItemComponent));
		if (!item)
			return null;

		// Vertical strips want every tab the full column width; a row keeps tabs text-sized.
		if (m_bVertical)
			AlignableSlot.SetHorizontalAlign(item.GetRootWidget(), LayoutHorizontalAlign.Stretch);

		m_aItems.Insert(item);
		return item;
	}
}
