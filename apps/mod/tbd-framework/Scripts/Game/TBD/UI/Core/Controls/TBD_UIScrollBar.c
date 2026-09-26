/**
 * @file TBD_UIScrollBar.c
 * @brief The 4 px scrollbar every TBD list wears, drawn in the list's `ScrollBarDock`.
 *
 * Role: draws a track and a thumb whose height and position follow a `ScrollLayoutWidget`, hides
 * when nothing scrolls and brightens under the pointer.
 * Position: mounted by the owner of the scroll widget (`TBD_ScrollList`, `TBD_DropdownMenu`, the
 * screen lists) and destroyed with it; not a widget handler.
 * State: the widget references, the ground, the hover and shown flags, and a repeating 30 Hz
 * call-queue tick, on the client, from `Mount` until `Destroy`.
 * Invariants: the engine bar is clipped away by the layout (the scroll widget extends past its
 * clipping parent, as vanilla `SCR_PooledListComponent.ShowScrollbar` does); the dock recipe lives
 * in apps/mod/tbd-framework/UI/layouts/Common/README.md.
 */

//! Custom scrollbar for one scroll widget.
//!
//! Enfusion's `ScrollLayoutWidget` draws its own scrollbar: unstyled, ~10 px, white, and it
//! overlaps the right edge of the content. The vanilla UI hides it the same way we do
//! (`SCR_PooledListComponent.ShowScrollbar` -- "clip scroll bar to hide"): the scroll widget
//! extends past its clipping parent, so the engine bar lands outside the clip. This class draws
//! the mockup's bar in a `ScrollBarDock` instead: a track and a thumb whose height and position
//! follow the scroll widget.
//!
//! Not a widget handler -- `Mount()` it from the owner that already holds the scroll widget and
//! `Destroy()` it with the owner. It ticks at 30 Hz through the call queue; there is no scroll
//! event to subscribe to, and reading two widget sizes per tick is free.
class TBD_UIScrollBar : Managed
{
	static const int TICK_MS = 33; //!< tick period, ms (30 Hz)
	static const int THUMB_MIN = 24; //!< smallest thumb height, reference px
	static const int WIDTH = 4; //!< bar width, reference px

	protected Widget m_wRoot; //!< the created `TBD_ScrollBar` layout root, the track area
	protected Widget m_wTrack; //!< `Track`
	protected Widget m_wThumb; //!< `Thumb`
	protected ScrollLayoutWidget m_wScroll; //!< the watched scroll widget
	protected Widget m_wContent; //!< the scroll widget's single child; its height is the scroll range
	protected int m_iGround; //!< opaque ARGB under the dock
	protected bool m_bHovered; //!< true while the pointer is within the hover halo
	protected bool m_bShown = true; //!< true while the bar is visible

	//! `dock` is the 4 px `ScrollBarDock` frame; `scroll` the list's ScrollLayoutWidget; `content`
	//! its single child (the layout whose desired height is the scroll range). `ground` is the
	//! opaque colour under the dock. Null when the layout or any widget is missing.
	//! @return the bar, ticking; null when the layout or any widget is missing
	static TBD_UIScrollBar Mount(Widget dock, ScrollLayoutWidget scroll, Widget content, int ground)
	{
		if (!dock || !scroll || !content)
			return null;

		Widget root = TBD_UILayouts.Create(TBD_UILayouts.SCROLL_BAR, dock);
		if (!root)
			return null;

		TBD_UIScrollBar bar = new TBD_UIScrollBar();
		bar.m_wRoot = root;
		bar.m_wTrack = root.FindAnyWidget("Track");
		bar.m_wThumb = root.FindAnyWidget("Thumb");
		bar.m_wScroll = scroll;
		bar.m_wContent = content;
		bar.m_iGround = ground;
		bar.Repaint();
		bar.Update();

		GetGame().GetCallqueue().CallLater(bar.Update, TICK_MS, true);
		return bar;
	}

	//! Stop the tick and drop every widget reference.
	void Destroy()
	{
		GetGame().GetCallqueue().Remove(Update);
		m_wScroll = null;
		m_wContent = null;
		m_wRoot = null;
		m_wTrack = null;
		m_wThumb = null;
	}

	//! Set the opaque colour under the dock and repaint.
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
	}

	//! Paint the track and the thumb (brighter while hovered) over the ground.
	protected void Repaint()
	{
		int thumb = TBD_UITheme.SCROLL_THUMB;
		if (m_bHovered)
			thumb = TBD_UITheme.SCROLL_THUMB_HOVER;

		TBD_UITheme.PaintOver(m_wTrack, TBD_UITheme.SCROLL_TRACK, m_iGround);
		TBD_UITheme.PaintOver(m_wThumb, thumb, TBD_UITheme.Over(TBD_UITheme.SCROLL_TRACK, m_iGround));
	}

	//! One tick: size the thumb to viewport/content, place it by the slider, hide when nothing
	//! scrolls, brighten under the pointer.
	protected void Update()
	{
		if (!m_wRoot || !m_wScroll || !m_wContent)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return;

		float viewW, viewH, contentW, contentH, trackW, trackH;
		m_wScroll.GetScreenSize(viewW, viewH);
		m_wContent.GetScreenSize(contentW, contentH);
		m_wRoot.GetScreenSize(trackW, trackH);

		bool show = contentH > viewH + 1 && viewH > 0 && trackH > 0;
		if (show != m_bShown)
		{
			m_bShown = show;
			TBD_UITheme.Show(m_wRoot, show);
		}

		if (!show)
			return;

		float track = workspace.DPIUnscale(trackH);
		float thumb = track * viewH / contentH;
		if (thumb < THUMB_MIN)
			thumb = THUMB_MIN;
		if (thumb > track)
			thumb = track;

		float sliderX, sliderY;
		m_wScroll.GetSliderPos(sliderX, sliderY);
		float y = sliderY * (track - thumb);

		FrameSlot.SetPos(m_wThumb, 0, y);
		FrameSlot.SetSize(m_wThumb, WIDTH, thumb);

		UpdateHover();
	}

	//! Light the thumb while the pointer is within 6 px of the bar.
	protected void UpdateHover()
	{
		int mouseX, mouseY;
		WidgetManager.GetMousePos(mouseX, mouseY);

		float x, y, w, h;
		m_wRoot.GetScreenPos(x, y);
		m_wRoot.GetScreenSize(w, h);

		// A 4 px bar is a poor target; the hover halo is 6 px each side.
		bool hovered = mouseX >= x - 6 && mouseX <= x + w + 6 && mouseY >= y && mouseY <= y + h;
		if (hovered == m_bHovered)
			return;

		m_bHovered = hovered;
		Repaint();
	}
}
