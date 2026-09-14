//! Briefing pass (2026-09-14) — a numbered card: `(1) Title [chip]`, optional paragraph, a dock for
//! anything else (a stat grid), and a footer dock right-aligned (a Locate button).
//!
//! Widget contract of `TBD_NumberedCard.layout`: `Border`, `Background`, `NumberPill`, `Number`,
//! `Title`, `ChipDock`, `Body` (wrapping text, hidden when empty), `BodyDock`, `FooterRuleSize` /
//! `FooterRule`, `FooterDock` (its `FooterSpacer` pushes children right).
//! Used by: objectives, rules.
class TBD_NumberedCardComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wNumberPill;
	protected TextWidget m_wNumber;
	protected TextWidget m_wTitle;
	protected Widget m_wChipDock;
	protected TextWidget m_wBody;
	protected Widget m_wBodyDock;
	protected Widget m_wFooterRuleSize;
	protected Widget m_wFooterRule;
	protected Widget m_wFooterDock;

	protected TBD_ChipComponent m_Chip;
	protected int m_iGround;

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
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

	// ── API ─────────────────────────────────────────────────────────────────────────────────

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

	void ShowFooter(bool shown)
	{
		TBD_UITheme.Show(m_wFooterRuleSize, shown);
		TBD_UITheme.Show(m_wFooterDock, shown);
	}

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

	Widget GetBodyDock()
	{
		return m_wBodyDock;
	}

	Widget GetFooterDock()
	{
		ShowFooter(true);
		return m_wFooterDock;
	}

	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//------------------------------------------------------------------------------------------------
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
