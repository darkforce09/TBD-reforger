//! Briefing pass (2026-09-14) — a scrolling vertical list with the TBD scrollbar, ready to fill.
//!
//! `TBD_ScrollList.layout` is the clip recipe every TBD list uses (`ListFrame` clips, `Scroll`
//! overhangs 24 px so the engine bar is cut off, `Content` pads 34, `ScrollBarDock` hosts
//! `TBD_UIScrollBar`). Mount it into a frame dock (a `TBD_PanelFill` body, a player lane), add
//! rows to `GetContent()`, `ResetScroll()` after a rebuild, `Destroy()` with the owner.
class TBD_ScrollList : Managed
{
	protected Widget m_wRoot;
	protected Widget m_wListFrame;
	protected ScrollLayoutWidget m_wScroll;
	protected Widget m_wContent;
	protected ref TBD_UIScrollBar m_Bar;

	//------------------------------------------------------------------------------------------------
	//! The list sits 12 px inside the dock (the layout's `ListFrame` offsets). `inset` is kept for
	//! callers but NOT applied at runtime: `FrameSlot.SetSize(frame, 0, 0)` zeroes an anchored frame
	//! (MEASURED 2026-09-14, empty player lanes) — change the layout if another inset is needed.
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	Widget GetContent()
	{
		return m_wContent;
	}

	Widget GetRoot()
	{
		return m_wRoot;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop every row.
	void Clear()
	{
		TBD_UILayouts.Clear(m_wContent);
	}

	//------------------------------------------------------------------------------------------------
	//! Back to the top — a rebuilt shorter list under an old slider offset shows nothing (Pass 5).
	void ResetScroll()
	{
		if (m_wScroll)
			m_wScroll.SetSliderPos(0, 0);

		if (m_wContent)
			m_wContent.Update();
	}
}
