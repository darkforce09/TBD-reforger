/**
 * @file TBD_DropdownMenu.c
 * @brief The open menu of a TBD dropdown: layout, pooled rows, select-all buttons, placement.
 *
 * Role: creates `TBD_DropdownMenu.layout` into the owning screen's overlay dock, fills its pooled
 * `TBD_ListBox` from the dropdown's items and anchors it under the trigger.
 * Position: owned by `TBD_DropdownComponent` while the menu is open; the component subscribes to
 * the list and buttons this class finds and destroys the menu on close.
 * State: the menu's widget references, its scrollbar and its scrim bridge, on the client, from
 * `Build` until `Destroy`.
 * Invariants: widget contract `Scrim` (full-bleed button, click outside closes), `Menu`,
 * `MenuBorder`, `MenuBG`, `MenuTitle`, `SelectAll`, `DeselectAll`, `MenuRule`, `MenuList`
 * (a `TBD_ListBox`); rows are pooled `TBD_ListRow`s, title = label, detail = badge or count.
 */

//! One open dropdown menu, built into a host widget and removed by `Destroy`.
class TBD_DropdownMenu : Managed
{
	static const int ROW_HEIGHT = 46;   //!< TBD_ListRow: 44 body + 2 padding
	static const int HEADER_HEIGHT = 36; //!< menu header height, reference pixels
	static const int MENU_PADDING = 8; //!< menu padding above and below the rows, reference pixels
	static const int MENU_GAP = 6;      //!< space between trigger and menu, reference pixels

	protected TBD_DropdownComponent m_Owner; //!< the dropdown this menu belongs to; weak, it owns the menu
	protected Widget m_wMenuRoot; //!< the created menu layout root; null before Build and after Destroy
	protected Widget m_wMenu; //!< the positioned `Menu` frame
	protected TBD_ListBox m_MenuList; //!< the pooled row list; null when the layout lacks it
	protected ref TBD_UIScrollBar m_MenuScrollBar; //!< the list's scrollbar
	protected TBD_UIButton m_SelectAll; //!< header button, shown in multi-select only
	protected TBD_UIButton m_DeselectAll; //!< header button, shown in multi-select only
	protected ref TBD_DropdownMenuBridge m_Bridge; //!< scrim click forwarder on the menu root

	//! Bind the menu to its dropdown.
	void TBD_DropdownMenu(TBD_DropdownComponent owner)
	{
		m_Owner = owner;
	}

	//! Create the menu layout under `host`, find its widgets, title and paint it.
	//! @param host the widget the layout is created into
	//! @param title the header text, written in upper case
	//! @param multi true shows the Select All and Deselect All buttons
	//! @return false when the layout cannot be created (no workspace or no resource)
	bool Build(Widget host, string title, bool multi)
	{
		m_wMenuRoot = TBD_UILayouts.Create(TBD_UILayouts.DROPDOWN_MENU, host);
		if (!m_wMenuRoot)
			return false;

		m_wMenu = m_wMenuRoot.FindAnyWidget("Menu");
		m_MenuList = TBD_ListBox.Cast(FindHandlerIn(m_wMenuRoot, "MenuList", TBD_ListBox));
		m_MenuScrollBar = TBD_UIScrollBar.Mount(m_wMenuRoot.FindAnyWidget("ScrollBarDock"), ScrollLayoutWidget.Cast(m_wMenuRoot.FindAnyWidget("Scroll")), m_wMenuRoot.FindAnyWidget("Content"), TBD_UITheme.PanelGround());
		m_SelectAll = TBD_UIButton.Cast(FindHandlerIn(m_wMenuRoot, "SelectAll", TBD_UIButton));
		m_DeselectAll = TBD_UIButton.Cast(FindHandlerIn(m_wMenuRoot, "DeselectAll", TBD_UIButton));

		m_Bridge = new TBD_DropdownMenuBridge(m_Owner);
		m_wMenuRoot.AddHandler(m_Bridge);

		if (m_SelectAll)
			TBD_UITheme.Show(m_SelectAll.GetRootWidget(), multi);

		if (m_DeselectAll)
			TBD_UITheme.Show(m_DeselectAll.GetRootWidget(), multi);

		TextWidget titleText = TextWidget.Cast(m_wMenuRoot.FindAnyWidget("MenuTitle"));
		string shout = title;
		shout.ToUpper();
		TBD_UITheme.Write(titleText, shout);
		TBD_UITheme.Paint(titleText, TBD_UITheme.MUTED_INK);

		// The menu floats over whatever is under the trigger; a glass panel is the honest guess.
		Widget menuBorder = m_wMenuRoot.FindAnyWidget("MenuBorder");
		Widget menuBG = m_wMenuRoot.FindAnyWidget("MenuBG");
		TBD_UILayouts.MountRounded(menuBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(menuBG, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.PaintOver(menuBorder, TBD_UITheme.GLASS_BORDER_STRONG, TBD_UITheme.PanelGround());
		TBD_UITheme.PaintOver(menuBG, TBD_UITheme.PANEL_FILL, TBD_UITheme.PanelGround());
		TBD_UITheme.Paint(m_wMenuRoot.FindAnyWidget("MenuRule"), TBD_UITheme.GLASS_BORDER);
		return true;
	}

	//! @return the pooled row list, or null when the layout has none
	TBD_ListBox GetList()
	{
		return m_MenuList;
	}

	//! @return the Select All button, or null when the layout has none
	TBD_UIButton GetSelectAll()
	{
		return m_SelectAll;
	}

	//! @return the Deselect All button, or null when the layout has none
	TBD_UIButton GetDeselectAll()
	{
		return m_DeselectAll;
	}

	//! Give gamepad focus to the first row.
	void FocusFirst()
	{
		if (m_MenuList)
			m_MenuList.FocusFirst();
	}

	//! Remove the menu layout, stop its scrollbar and drop every widget reference.
	//! @return the widget the menu was created under, or null when it was never built
	Widget Destroy()
	{
		if (!m_wMenuRoot)
			return null;

		Widget host = m_wMenuRoot.GetParent();
		m_wMenuRoot.RemoveFromHierarchy();
		m_wMenuRoot = null;
		if (m_MenuScrollBar)
			m_MenuScrollBar.Destroy();

		m_MenuScrollBar = null;
		m_wMenu = null;
		m_MenuList = null;
		m_SelectAll = null;
		m_DeselectAll = null;
		m_Bridge = null;
		return host;
	}

	//! Re-bind the pooled rows to `items`; rows are pooled by the list, so this is O(items)
	//! property writes. A checked item (multi) or the selected tag (single) paints ACTIVE.
	//! @param items the dropdown's items, in display order
	//! @param multi true for the checklist, false for a single choice
	//! @param selectedTag the single-select choice; ignored in multi-select
	void Fill(notnull array<ref TBD_DropdownItem> items, bool multi, int selectedTag)
	{
		if (!m_MenuList)
			return;

		m_MenuList.BeginUpdate();
		foreach (TBD_DropdownItem item : items)
		{
			TBD_EUIState state = TBD_EUIState.NORMAL;
			string detail = item.m_sBadge;

			if (multi)
			{
				if (item.m_bChecked)
					state = TBD_EUIState.ACTIVE;
			}
			else if (item.m_iTag == selectedTag)
			{
				state = TBD_EUIState.ACTIVE;
			}

			m_MenuList.AddItem(item.m_sLabel, detail, item.m_iTag, state, true);
		}
		m_MenuList.EndUpdate();

		// The list's own "last clicked" echo would fight the checkmarks in multi mode.
		if (multi)
			m_MenuList.SetSelectedTag(-1);
		else
			m_MenuList.SetSelectedTag(selectedTag);
	}

	//! Anchor the menu under `trigger` inside `host`. Screen coordinates come back in real
	//! pixels; FrameSlot wants reference pixels, hence DPIUnscale. Does nothing without a workspace.
	//! @param trigger the dropdown's trigger widget
	//! @param host the widget the menu was created under
	//! @param menuWidth the menu width in reference pixels
	//! @param alignRight true aligns the menu's right edge to the trigger's right edge
	//! @param rows the item count, which sets the menu height
	void Place(Widget trigger, notnull Widget host, int menuWidth, bool alignRight, int rows)
	{
		if (!m_wMenu || !trigger)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return;

		float tx, ty, tw, th, hx, hy;
		trigger.GetScreenPos(tx, ty);
		trigger.GetScreenSize(tw, th);
		host.GetScreenPos(hx, hy);

		float x = workspace.DPIUnscale(tx - hx);
		float y = workspace.DPIUnscale(ty - hy + th) + MENU_GAP;

		if (alignRight)
			x = workspace.DPIUnscale(tx + tw - hx) - menuWidth;

		float height = HEADER_HEIGHT + 2 + rows * ROW_HEIGHT + MENU_PADDING * 2;

		FrameSlot.SetPos(m_wMenu, x, y);
		FrameSlot.SetSize(m_wMenu, menuWidth, height);
	}

	//! Find the handler of class `handler` on the widget `name` under `root`.
	//! @return the handler, or null when the root, the widget or the handler is missing
	protected static ScriptedWidgetComponent FindHandlerIn(Widget root, string name, typename handler)
	{
		if (!root)
			return null;

		Widget w = root.FindAnyWidget(name);
		if (!w)
			return null;

		return ScriptedWidgetComponent.Cast(w.FindHandler(handler));
	}
}
