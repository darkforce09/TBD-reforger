//! Pre-game rebuild (2026-09-12) — the mono pill: `ADMIN`, `30 AVAILABLE`, `PVP`, `v2.14.99`,
//! `LATEST`, `48 SLOTS`, `BLUFOR`, `2x`. Twenty-two mockup panels use it; one layout serves them.
//!
//! Widget contract on `TBD_Chip.layout`: `ChipBorder`, `ChipBG`, `ChipDot` (optional pulse dot),
//! `ChipText`. The chip shrink-wraps its text: the layout's root is an OverlayWidget whose
//! stretched images take the size of the text child.
//!
//! Colour is a tint, never a literal — TBD_UITheme.ChipFill / ChipBorder / ChipInk.
class TBD_ChipComponent : ScriptedWidgetComponent
{
	[Attribute("", UIWidgets.EditBox, "Chip text, if the owning screen does not set one")]
	protected string m_sText;

	[Attribute("0", UIWidgets.ComboBox, "Tint", "", ParamEnumArray.FromEnum(TBD_EUITint))]
	protected TBD_EUITint m_eTint;

	[Attribute("1", UIWidgets.CheckBox, "Uppercase the text (mono badge style)")]
	protected bool m_bUppercase;

	//! rounded-full instead of rounded-md (count pills). SetPill() flips it at runtime.
	protected bool m_bPill;
	//! Opaque colour under the chip; 0 = glass panel (TBD_UITheme.PanelGround()).
	protected int m_iGround;

	protected Widget m_wRoot;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wDot;
	protected TextWidget m_wText;

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBorder = w.FindAnyWidget("ChipBorder");
		m_wBackground = w.FindAnyWidget("ChipBG");
		m_wDot = w.FindAnyWidget("ChipDot");
		m_wText = TextWidget.Cast(w.FindAnyWidget("ChipText"));

		MountShape();
		TBD_UITheme.Show(m_wDot, false);
		Set(m_sText, m_eTint);
	}

	//------------------------------------------------------------------------------------------------
	//! rounded-md tag by default; rounded-full pill on request (count pills).
	protected void MountShape()
	{
		int radius = TBD_UITheme.RADIUS_TAG;
		if (m_bPill)
			radius = TBD_UITheme.RADIUS_PILL;

		TBD_UILayouts.MountRounded(m_wBorder, radius);
		TBD_UILayouts.MountRounded(m_wBackground, radius - 1);
	}

	//------------------------------------------------------------------------------------------------
	//! Switch between the tag shape and the fully round pill (`N AVAILABLE`, `Modes 5`).
	void SetPill(bool pill)
	{
		if (m_bPill == pill)
			return;

		m_bPill = pill;
		MountShape();
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	//! Opaque colour under the chip. Defaults to a glass panel; a chip on a selected card or in a
	//! faction column is told so by whoever mounts it.
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	//! Text + tint in one call; this is the whole API a screen needs.
	void Set(string text, TBD_EUITint tint)
	{
		m_sText = text;
		m_eTint = tint;

		string shown = text;
		if (m_bUppercase)
			shown.ToUpper();

		TBD_UITheme.Write(m_wText, shown);
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	void SetText(string text)
	{
		Set(text, m_eTint);
	}

	//------------------------------------------------------------------------------------------------
	void SetTint(TBD_EUITint tint)
	{
		Set(m_sText, tint);
	}

	//------------------------------------------------------------------------------------------------
	void SetUppercase(bool uppercase)
	{
		m_bUppercase = uppercase;
		Set(m_sText, m_eTint);
	}

	//------------------------------------------------------------------------------------------------
	//! The little status dot the "SYNCED & ACTIVE" pill carries. Painted in the tint's ink.
	void SetDotVisible(bool visible)
	{
		TBD_UITheme.Show(m_wDot, visible);
	}

	//------------------------------------------------------------------------------------------------
	void SetChipVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
	}

	//------------------------------------------------------------------------------------------------
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//------------------------------------------------------------------------------------------------
	protected void Repaint()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.PanelGround();

		int fill = TBD_UITheme.ChipFill(m_eTint);
		int border = TBD_UITheme.ChipBorder(m_eTint);
		if (m_bPill && m_eTint == TBD_EUITint.PRIMARY)
		{
			fill = TBD_UITheme.PILL_FILL;
			border = TBD_UITheme.PILL_BORDER;
		}

		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.Paint(m_wText, TBD_UITheme.ChipInk(m_eTint));
		TBD_UITheme.Paint(m_wDot, TBD_UITheme.ChipInk(m_eTint));
	}

	//------------------------------------------------------------------------------------------------
	//! Mount a chip into `dock`, set it, return its handler. The one-liner every screen wants.
	//! `ground` is the opaque colour under the dock; 0 keeps the glass-panel default.
	static TBD_ChipComponent Mount(Widget dock, string text, TBD_EUITint tint, int ground = 0)
	{
		if (!dock)
			return null;

		TBD_ChipComponent chip = TBD_ChipComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.CHIP, dock, TBD_ChipComponent));
		if (!chip)
			return null;

		if (ground != 0)
			chip.SetGround(ground);

		chip.Set(text, tint);
		return chip;
	}
}
