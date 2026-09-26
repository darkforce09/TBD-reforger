/**
 * @file TBD_ChipComponent.c
 * @brief The mono chip: `ADMIN`, `30 AVAILABLE`, `PVP`, `v2.14.99`, `LATEST`, `48 SLOTS`, `2x`.
 *
 * Role: handler of `TBD_Chip.layout`; writes the text, rounds the tag or pill shape and paints
 * fill, border, ink and the optional dot from a `TBD_EUITint`.
 * Position: mounted by the pre-game panels and pages, the dropdown badge, key-value rows,
 * numbered cards, sections and the session top bar, mostly through the static `Mount`.
 * State: the text, tint, shape, uppercase flag, ground colour and widget references, on the client.
 * Invariants: widget contract `ChipBorder`, `ChipBG`, `ChipDot`, `ChipText`; the root is an
 * OverlayWidget whose stretched images take the size of the text, so the chip shrink-wraps; colour
 * is a tint through `TBD_UITintColours`, never a literal.
 */

//! Chip handler: text, tint, tag or pill shape, and the status dot.
class TBD_ChipComponent : ScriptedWidgetComponent
{
	[Attribute("", UIWidgets.EditBox, desc: "Chip text, if the owning screen does not set one")]
	protected string m_sText; //!< chip text; default empty

	[Attribute("0", UIWidgets.ComboBox, desc: "Tint", enums: ParamEnumArray.FromEnum(TBD_EUITint))]
	protected TBD_EUITint m_eTint; //!< the tint; default NEUTRAL

	[Attribute("1", UIWidgets.CheckBox, desc: "Uppercase the text (mono badge style)")]
	protected bool m_bUppercase; //!< true writes the text upper case; default true

	protected bool m_bPill; //!< rounded-full instead of rounded-md (count pills). SetPill() flips it at runtime.
	protected int m_iGround; //!< Opaque colour under the chip; 0 = glass panel (TBD_UITheme.PanelGround()).

	protected Widget m_wRoot; //!< the layout root this handler sits on
	protected Widget m_wBorder; //!< `ChipBorder` frame dock
	protected Widget m_wBackground; //!< `ChipBG` frame dock
	protected Widget m_wDot; //!< `ChipDot`, the optional status dot; hidden by default
	protected TextWidget m_wText; //!< `ChipText`

	//! Find the widgets, mount the shape, hide the dot and show the attribute text and tint.
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

	//! rounded-md tag by default; rounded-full pill on request (count pills).
	protected void MountShape()
	{
		int radius = TBD_UITheme.RADIUS_TAG;
		if (m_bPill)
			radius = TBD_UITheme.RADIUS_PILL;

		TBD_UILayouts.MountRounded(m_wBorder, radius);
		TBD_UILayouts.MountRounded(m_wBackground, radius - 1);
	}

	//! Switch between the tag shape and the fully round pill (`N AVAILABLE`, `Modes 5`).
	void SetPill(bool pill)
	{
		if (m_bPill == pill)
			return;

		m_bPill = pill;
		MountShape();
		Repaint();
	}

	//! Opaque colour under the chip. Defaults to a glass panel; a chip on a selected card or in a
	//! faction column is told so by whoever mounts it.
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
	}

	//! Drop the root when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

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

	//! Set the text, keeping the tint.
	void SetText(string text)
	{
		Set(text, m_eTint);
	}

	//! Set the tint, keeping the text.
	void SetTint(TBD_EUITint tint)
	{
		Set(m_sText, tint);
	}

	//! Set the uppercase flag and rewrite the text.
	void SetUppercase(bool uppercase)
	{
		m_bUppercase = uppercase;
		Set(m_sText, m_eTint);
	}

	//! The little status dot the "SYNCED & ACTIVE" pill carries. Painted in the tint's ink.
	void SetDotVisible(bool visible)
	{
		TBD_UITheme.Show(m_wDot, visible);
	}

	//! Show or hide the whole chip.
	void SetChipVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
	}

	//! @return the layout root this handler sits on
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! Paint border, fill, text and dot from the tint over the ground; a PRIMARY pill uses the pill tokens.
	protected void Repaint()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.PanelGround();

		int fill = TBD_UITintColours.ChipFill(m_eTint);
		int border = TBD_UITintColours.ChipBorder(m_eTint);
		if (m_bPill && m_eTint == TBD_EUITint.PRIMARY)
		{
			fill = TBD_UITheme.PILL_FILL;
			border = TBD_UITheme.PILL_BORDER;
		}

		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.Paint(m_wText, TBD_UITintColours.ChipInk(m_eTint));
		TBD_UITheme.Paint(m_wDot, TBD_UITintColours.ChipInk(m_eTint));
	}

	//! Mount a chip into `dock`, set it and return its handler. `ground` is the opaque colour under the
	//! dock; 0 keeps the glass-panel default.
	//! @return the chip, or null without a dock or when the layout cannot be created
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
