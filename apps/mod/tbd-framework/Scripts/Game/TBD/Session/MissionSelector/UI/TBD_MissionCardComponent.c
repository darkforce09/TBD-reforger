/**
 * @file TBD_MissionCardComponent.c
 * @brief One pooled mission card of the Mission Selector's scenario browser.
 *
 * Role: widget handler of `TBD_MissionCard.layout`: shows a mission's mode tag, terrain, slot
 * count and title and paints the selected, hovered and idle states.  Position: TBD_ScenarioBrowserPanel creates, binds and pools the cards; activation calls back
 * TBD_ScenarioBrowserPanel.OnCardActivated.
 * State: widget references, the tag chip, the owner, the card index, the selected flag and the
 * tag tint.  Invariants: widget contract `Border`, `Background`, `TagChipDock`, `TerrainText`,
 * `SlotCount`, `PulseDot`, `Title`, `Indicator` (image), `IndicatorGlyph` (text fallback); the
 * indicator shows its icon or its glyph, never both and never neither.
 */

//! One mission card.
class TBD_MissionCardComponent : TBD_UIInteractive
{
	protected Widget m_wBorder; //!< `Border`
	protected Widget m_wBackground; //!< `Background`
	protected Widget m_wTagDock; //!< `TagChipDock`
	protected TextWidget m_wTerrain; //!< `TerrainText`
	protected TextWidget m_wSlots; //!< `SlotCount`
	protected Widget m_wPulse; //!< `PulseDot`, shown on the selected card
	protected TextWidget m_wTitle; //!< `Title`
	protected ImageWidget m_wIndicator; //!< `Indicator` icon
	protected TextWidget m_wIndicatorGlyph; //!< `IndicatorGlyph`, the text fallback of the icon
	protected TBD_ChipComponent m_TagChip; //!< mode tag chip; mounted on the first Bind

	protected TBD_ScenarioBrowserPanel m_Owner; //!< weak; the panel that pools this card
	protected int m_iIndex = -1; //!< live-card index in the owner; -1 before Bind
	protected bool m_bSelected; //!< this card is the selected mission
	protected TBD_EUITint m_eTagTint = TBD_EUITint.NEUTRAL; //!< the mode's tint, used when not selected; default NEUTRAL

	//! Find the card widgets and round the border and background.
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wTagDock = w.FindAnyWidget("TagChipDock");
		m_wTerrain = TextWidget.Cast(w.FindAnyWidget("TerrainText"));
		m_wSlots = TextWidget.Cast(w.FindAnyWidget("SlotCount"));
		m_wPulse = w.FindAnyWidget("PulseDot");
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wIndicator = ImageWidget.Cast(w.FindAnyWidget("Indicator"));
		m_wIndicatorGlyph = TextWidget.Cast(w.FindAnyWidget("IndicatorGlyph"));

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
	}

	//! Bind the card to a mission and repaint.
	//! @param owner the panel to call back on activation
	//! @param index the card's live index
	//! @param mission the mission shown
	//! @param terrainName the terrain display name
	//! @param tagLabel the mode label
	//! @param tagTint the mode tint
	void Bind(TBD_ScenarioBrowserPanel owner, int index, TBD_MissionSummary mission, string terrainName, string tagLabel, TBD_EUITint tagTint)
	{
		m_Owner = owner;
		m_iIndex = index;
		m_eTagTint = tagTint;

		if (!m_TagChip)
			m_TagChip = TBD_ChipComponent.Mount(m_wTagDock, tagLabel, tagTint);
		else
			m_TagChip.Set(tagLabel, tagTint);

		TBD_UITheme.Write(m_wTerrain, terrainName);
		TBD_UITheme.Write(m_wSlots, string.Format("%1 SLOTS", mission.GetSlotTotal()));
		TBD_UITheme.Write(m_wTitle, mission.m_sTitle);

		Repaint();
	}

	//! Mark the card selected or not; repaints on a change.
	void SetSelected(bool selected)
	{
		if (m_bSelected == selected)
			return;

		m_bSelected = selected;
		Repaint();
	}

	//! Paint the fill, border, inks, pulse dot, tag chip and indicator for the selected, hovered or
	//! idle state.
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int fill;
		int border;
		int titleInk;
		int slotInk;
		int terrainInk;
		int indicatorInk;
		string indicatorIcon;
		string glyph;

		if (m_bSelected)
		{
			fill         = TBD_UITheme.CARD_SELECTED_FILL;
			border       = TBD_UITheme.CARD_SELECTED_BORDER;
			titleInk     = TBD_UITheme.BRIGHT_INK;
			slotInk      = TBD_UITheme.CARD_SELECTED_INK;
			terrainInk   = TBD_UITheme.PRIMARY_FIXED;
			indicatorInk = TBD_UITheme.CARD_SELECTED_INK;
			indicatorIcon = "check_circle";
			glyph = "*";
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill         = TBD_UITheme.CARD_HOVER_FILL;
			border       = TBD_UITheme.CARD_HOVER_BORDER;
			titleInk     = TBD_UITheme.BRIGHT_INK;
			slotInk      = TBD_UITheme.ON_SURFACE;
			terrainInk   = TBD_UITheme.MUTED_INK;
			indicatorInk = TBD_UITheme.ON_SURFACE;
			indicatorIcon = "chevron_right";
			glyph = ">";
		}
		else
		{
			fill         = TBD_UITheme.CARD_IDLE_FILL;
			border       = TBD_UITheme.CARD_IDLE_BORDER;
			titleInk     = TBD_UITheme.ON_SURFACE;
			slotInk      = TBD_UITheme.ChipInk(TBD_EUITint.NEUTRAL);
			terrainInk   = TBD_UITheme.MUTED_INK;
			indicatorInk = TBD_UITheme.DIM_INK;
			indicatorIcon = "chevron_right";
			glyph = ">";
		}

		// Cards sit on the MISSIONS panel; the tag chip sits on the card's own fill.
		int ground = TBD_UITheme.PanelGround();
		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.Paint(m_wTitle, titleInk);
		TBD_UITheme.Paint(m_wSlots, slotInk);
		TBD_UITheme.Paint(m_wTerrain, terrainInk);
		TBD_UITheme.Show(m_wPulse, m_bSelected);
		TBD_UITheme.Paint(m_wPulse, TBD_UITheme.CARD_BORDER);

		if (m_TagChip)
		{
			m_TagChip.SetGround(TBD_UITheme.Over(fill, ground));
			if (m_bSelected)
				m_TagChip.SetTint(TBD_EUITint.SOLID);
			else
				m_TagChip.SetTint(m_eTagTint);
		}

		// Icon when the imageset has one, glyph otherwise -- never both, never neither.
		bool iconShown = TBD_UIIcons.Load(m_wIndicator, indicatorIcon);
		TBD_UITheme.Paint(m_wIndicator, indicatorInk);
		TBD_UITheme.Show(m_wIndicatorGlyph, !iconShown);
		TBD_UITheme.Write(m_wIndicatorGlyph, glyph);
		TBD_UITheme.Paint(m_wIndicatorGlyph, indicatorInk);
	}

	//! Tell the owner this card was activated.
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnCardActivated(m_iIndex);
	}

	//! Show or hide the card; hidden cards stay pooled.
	void SetCardVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
	}
}
