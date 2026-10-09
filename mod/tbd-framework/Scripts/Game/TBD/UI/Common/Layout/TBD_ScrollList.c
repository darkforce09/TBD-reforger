/**
 * @file TBD_ScrollList.c
 * @brief A scrolling vertical list wearing the TBD scrollbar, ready to fill.
 *
 * Role: creates `TBD_ScrollList.layout` into a frame dock and mounts a `TBD_UIScrollBar` on it.
 * Position: owned by the briefing pages and the players panel, which add rows to `GetContent()`,
 * call `ResetScroll()` after a rebuild and `Destroy()` with themselves.
 * State: the widget references and the scrollbar, on the client, until `Destroy`.
 * Invariants: the layout is the clip recipe every TBD list uses: `ListFrame` clips, `Scroll`
 * overhangs 24 px so the engine bar is cut off, `Content` pads 34, `ScrollBarDock` hosts the bar.
 */

//! One mounted scroll list and its scrollbar.
class TBD_ScrollList : Managed
{
	protected Widget m_wRoot; //!< the created layout root
	protected Widget m_wListFrame; //!< `ListFrame`, the clipping frame
	protected ScrollLayoutWidget m_wScroll; //!< `Scroll`, the engine scroll layout
	protected Widget m_wContent; //!< `Content`, the row container
	protected ref TBD_UIScrollBar m_Bar; //!< the TBD scrollbar, ticking at 30 Hz until Destroy

	//! Mount a list into `dock` over `ground`. The list sits 12 px inside the dock (the layout's
	//! `ListFrame` offsets); `inset` is accepted but not applied, because `FrameSlot.SetSize(frame, 0, 0)`
	//! zeroes an anchored frame, so another inset is a layout change.
	//! @return the list, or null without a dock or when the layout cannot be created
	static TBD_ScrollList Mount(Widget dock, int ground, int inset = 12)
	{
		if (!dock)
			return null;

		Widget root = TBD_UILayouts.Create(TBD_UILayouts.SCROLL_LIST, dock);
		if (!root)
			return null;

		TBD_ScrollList list = new TBD_ScrollList();
		list.m_wRoot = root;
		list.m_wListFrame = root.FindAnyWidget("ListFrame");
		list.m_wScroll = ScrollLayoutWidget.Cast(root.FindAnyWidget("Scroll"));
		list.m_wContent = root.FindAnyWidget("Content");

		list.m_Bar = TBD_UIScrollBar.Mount(root.FindAnyWidget("ScrollBarDock"), list.m_wScroll, list.m_wContent, ground);
		return list;
	}

	//! Stop the scrollbar and drop every widget reference.
	void Destroy()
	{
		if (m_Bar)
			m_Bar.Destroy();

		m_Bar = null;
		m_wRoot = null;
		m_wListFrame = null;
		m_wScroll = null;
		m_wContent = null;
	}

	//! @return the row container
	Widget GetContent()
	{
		return m_wContent;
	}

	//! @return the created layout root
	Widget GetRoot()
	{
		return m_wRoot;
	}

	//! Drop every row.
	void Clear()
	{
		TBD_UILayouts.Clear(m_wContent);
	}

	//! Back to the top: a rebuilt shorter list under an old slider offset shows nothing.
	void ResetScroll()
	{
		if (m_wScroll)
			m_wScroll.SetSliderPos(0, 0);

		if (m_wContent)
			m_wContent.Update();
	}
}
