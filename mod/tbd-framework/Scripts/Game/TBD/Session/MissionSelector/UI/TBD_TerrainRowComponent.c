/**
 * @file TBD_TerrainRowComponent.c
 * @brief One pooled row of the Mission Selector's TERRAINS list.
 *
 * Role: widget handler of `TBD_TerrainRow.layout`: shows a terrain's icon, name and mission count
 * and paints the selected, hovered and idle states.  Position: TBD_TerrainSelectorPanel creates,
 * binds and pools the rows; activation calls back TBD_TerrainSelectorPanel.OnRowActivated.
 * State: widget references, the count chip, the owner, the row index and the selected flag.
 * Invariants: widget contract `Border`, `Background`, `Accent`, `Icon`, `Title`, `CountBadgeDock`,
 * `Chevron`; the chevron shows only on the selected row.
 */

//! One terrain row.
class TBD_TerrainRowComponent : TBD_UIInteractive
{
	protected Widget m_wBorder; //!< `Border`
	protected Widget m_wBackground; //!< `Background`
	protected Widget m_wAccent; //!< `Accent`: the glow bar of the selected row
	protected ImageWidget m_wIcon; //!< `Icon`
	protected TextWidget m_wTitle; //!< `Title`
	protected Widget m_wCountDock; //!< `CountBadgeDock`
	protected TextWidget m_wChevron; //!< `Chevron`
	protected TBD_ChipComponent m_CountChip; //!< mission count chip; mounted on the first Bind

	protected TBD_TerrainSelectorPanel m_Owner; //!< weak; the panel outlives its rows only by a frame
	protected int m_iIndex = -1; //!< live-row index in the owner; -1 before Bind
	protected bool m_bSelected; //!< this row is the selected terrain
	protected bool m_bIconShown; //!< the terrain icon loaded

	//! Find the row widgets and round the border and background.
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wAccent = w.FindAnyWidget("Accent");
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("Icon"));
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wCountDock = w.FindAnyWidget("CountBadgeDock");
		m_wChevron = TextWidget.Cast(w.FindAnyWidget("Chevron"));

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
	}

	//! Bind the row to a terrain and repaint.
	//! @param owner the panel to call back on activation
	//! @param index the row's live index
	//! @param terrain the terrain shown
	//! @param missionCount the missions on it
	void Bind(TBD_TerrainSelectorPanel owner, int index, TBD_TerrainInfo terrain, int missionCount)
	{
		m_Owner = owner;
		m_iIndex = index;

		TBD_UITheme.Write(m_wTitle, terrain.m_sName);
		m_bIconShown = TBD_UIIcons.Load(m_wIcon, terrain.m_sIcon);

		if (!m_CountChip)
			m_CountChip = TBD_ChipComponent.Mount(m_wCountDock, missionCount.ToString(), TBD_EUITint.NEUTRAL);
		else
			m_CountChip.SetText(missionCount.ToString());

		Repaint();
	}

	//! Mark the row selected or not; repaints on a change.
	void SetSelected(bool selected)
	{
		if (m_bSelected == selected)
			return;

		m_bSelected = selected;
		Repaint();
	}

	//! Paint the fill, border, accent, inks, chevron and count chip for the selected, hovered or
	//! idle state.
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int fill;
		int border;
		int accent;
		int ink;
		int iconInk;

		if (m_bSelected)
		{
			fill    = TBD_UITheme.CARD_SELECTED_FILL;
			border  = TBD_UITheme.ROW_SELECTED_BORDER;
			accent  = TBD_UITheme.ROW_ACTIVE_GLOW;
			ink     = TBD_UITheme.BRIGHT_INK;
			iconInk = TBD_UITheme.ROW_ACTIVE_GLOW;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill    = TBD_UITheme.NAV_HOVER_FILL;
			border  = TBD_UITheme.TRANSPARENT;
			accent  = TBD_UITheme.TRANSPARENT;
			ink     = TBD_UITheme.ON_SURFACE;
			iconInk = TBD_UITheme.MUTED_INK;
		}
		else
		{
			fill    = TBD_UITheme.TRANSPARENT;
			border  = TBD_UITheme.TRANSPARENT;
			accent  = TBD_UITheme.TRANSPARENT;
			ink     = TBD_UITintColours.ChipInk(TBD_EUITint.NEUTRAL);
			iconInk = TBD_UITheme.DIM_INK;
		}

		// Rows sit on the TERRAINS panel; the chip sits on the row's own fill.
		int ground = TBD_UITheme.PanelGround();
		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.Paint(m_wAccent, accent);
		TBD_UITheme.Paint(m_wTitle, ink);
		TBD_UITheme.Paint(m_wIcon, iconInk);
		TBD_UITheme.Show(m_wChevron, m_bSelected);

		if (m_CountChip)
		{
			m_CountChip.SetGround(TBD_UITheme.Over(fill, ground));
			if (m_bSelected)
				m_CountChip.SetTint(TBD_EUITint.PRIMARY);
			else
				m_CountChip.SetTint(TBD_EUITint.NEUTRAL);
		}
	}

	//! Tell the owner this row was activated.
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnRowActivated(m_iIndex);
	}

	//! Show or hide the row; hidden rows stay pooled.
	void SetRowVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
	}
}
