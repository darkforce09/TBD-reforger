/**
 * @file TBD_NumberedCardComponent.c
 * @brief A numbered card: `(1) Title [chip]`, an optional paragraph, a body dock and a footer dock.
 *
 * Role: handler of `TBD_NumberedCard.layout`; writes number, title, paragraph and chip, and hands
 * out its body dock (a stat grid) and its right-aligned footer dock (a Locate button).
 * Position: mounted by the briefing objectives and rules pages through the static `Mount`.
 * State: the widget references, the chip and the ground colour, on the client.
 * Invariants: widget contract `Border`, `Background`, `NumberPill`, `Number`, `Title`, `ChipDock`,
 * `Body` (wrapping text, hidden when empty), `BodyDock`, `FooterRuleSize`, `FooterRule`,
 * `FooterDock` (its `FooterSpacer` pushes children right); the footer shows once it is asked for.
 */

//! Numbered card handler.
class TBD_NumberedCardComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot; //!< the layout root this handler sits on
	protected Widget m_wBorder; //!< `Border` frame dock
	protected Widget m_wBackground; //!< `Background` frame dock
	protected Widget m_wNumberPill; //!< `NumberPill` frame dock, a round pill
	protected TextWidget m_wNumber; //!< `Number`
	protected TextWidget m_wTitle; //!< `Title`
	protected Widget m_wChipDock; //!< `ChipDock`
	protected TextWidget m_wBody; //!< `Body` paragraph, hidden while empty
	protected Widget m_wBodyDock; //!< `BodyDock`, for caller content
	protected Widget m_wFooterRuleSize; //!< `FooterRuleSize`, hidden with the footer
	protected Widget m_wFooterRule; //!< `FooterRule`
	protected Widget m_wFooterDock; //!< `FooterDock`, hidden until GetFooterDock

	protected TBD_ChipComponent m_Chip; //!< the title chip, mounted on first use
	protected int m_iGround; //!< opaque ARGB under the card; the panel ground by default

	//! Find the widgets, round the docks, hide the body and footer, paint over the panel ground.
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wNumberPill = w.FindAnyWidget("NumberPill");
		m_wNumber = TextWidget.Cast(w.FindAnyWidget("Number"));
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wChipDock = w.FindAnyWidget("ChipDock");
		m_wBody = TextWidget.Cast(w.FindAnyWidget("Body"));
		m_wBodyDock = w.FindAnyWidget("BodyDock");
		m_wFooterRuleSize = w.FindAnyWidget("FooterRuleSize");
		m_wFooterRule = w.FindAnyWidget("FooterRule");
		m_wFooterDock = w.FindAnyWidget("FooterDock");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
		TBD_UILayouts.MountRounded(m_wNumberPill, TBD_UITheme.RADIUS_PILL);

		TBD_UITheme.Show(m_wBody, false);
		ShowFooter(false);
		m_iGround = TBD_UITheme.PanelGround();
		Repaint();
	}

	//! Drop the root when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//! Create a card stretched under `parent`, set its ground, number and title.
	//! @return the card, or null when the layout or its handler is missing
	static TBD_NumberedCardComponent Mount(Widget parent, int number, string title, int ground)
	{
		Widget w = TBD_UILayouts.CreateStretched(TBD_UILayouts.NUMBERED_CARD, parent);
		if (!w)
			return null;

		TBD_NumberedCardComponent card = TBD_NumberedCardComponent.Cast(w.FindHandler(TBD_NumberedCardComponent));
		if (!card)
			return null;

		card.SetGround(ground);
		card.Set(number, title);
		return card;
	}

	//! Write the number and the title.
	void Set(int number, string title)
	{
		TBD_UITheme.Write(m_wNumber, number.ToString());
		TBD_UITheme.Write(m_wTitle, title);
	}

	//! Paragraph under the title; empty hides it.
	void SetBody(string text)
	{
		TBD_UITheme.Write(m_wBody, text);
		TBD_UITheme.Show(m_wBody, !text.IsEmpty());
	}

	//! Show `text` in a pill chip after the title; empty hides the chip.
	void SetChip(string text, TBD_EUITint tint)
	{
		if (text.IsEmpty())
		{
			if (m_Chip)
				m_Chip.SetChipVisible(false);
			return;
		}

		if (!m_Chip)
		{
			m_Chip = TBD_ChipComponent.Mount(m_wChipDock, text, tint, GetGround());
			if (m_Chip)
				m_Chip.SetPill(true);
		}
		else
		{
			m_Chip.Set(text, tint);
		}

		if (m_Chip)
			m_Chip.SetChipVisible(true);
	}

	//! Show or hide the footer rule and dock.
	void ShowFooter(bool shown)
	{
		TBD_UITheme.Show(m_wFooterRuleSize, shown);
		TBD_UITheme.Show(m_wFooterDock, shown);
	}

	//! Set the opaque colour under the card and repaint it and its chip.
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		if (m_Chip)
			m_Chip.SetGround(GetGround());
		Repaint();
	}

	//! Opaque colour of the card's own fill (for children).
	int GetGround()
	{
		return TBD_UITheme.Over(TBD_UITheme.KIT_CARD_FILL, m_iGround);
	}

	//! @return the dock under the title for caller content
	Widget GetBodyDock()
	{
		return m_wBodyDock;
	}

	//! Show the footer and return its dock.
	//! @return the footer dock
	Widget GetFooterDock()
	{
		ShowFooter(true);
		return m_wFooterDock;
	}

	//! @return the layout root this handler sits on
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! Paint border, fill, number pill, footer rule and the three texts.
	protected void Repaint()
	{
		if (!m_wRoot)
			return;

		int cardGround = GetGround();
		TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.KIT_CARD_BORDER, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.KIT_CARD_FILL, m_iGround);
		TBD_UITheme.PaintOver(m_wNumberPill, TBD_UITheme.KIT_CELL_BORDER, cardGround);
		TBD_UITheme.PaintOver(m_wFooterRule, TBD_UITheme.KIT_CARD_BORDER, cardGround);
		TBD_UITheme.Paint(m_wNumber, TBD_UITheme.MUTED_INK);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.Paint(m_wBody, TBD_UITheme.MUTED_INK);
	}
}
