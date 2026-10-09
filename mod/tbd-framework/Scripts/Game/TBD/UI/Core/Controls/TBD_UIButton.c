/**
 * @file TBD_UIButton.c
 * @brief The TBD button: one loud primary variant and one quiet variant, with toggle tints.
 *
 * Role: handler of `Common/TBD_Button.layout` and of button widgets in screen layouts; paints the
 * primary, quiet, success, warning and disabled looks and fires `GetOnActivate()(button)` on click.
 * Position: created by `TBD_SessionBottomBar.AddAction`, found by `TBD_DropdownMenu` and the
 * screens that nest buttons; derives from `TBD_UIInteractive`.
 * State: the label, primary flag, tint, ground and widget references, on the client.
 * Invariants: design law allows exactly one obvious primary action per screen (filled ACTION
 * blue); the quiet variant has no fill until touched; every colour is a `TBD_UITheme` token, and
 * the primary hover reuses the PRIMARY / ON_PRIMARY pair so the palette stays closed. The root is
 * a `ButtonWidgetClass` with an image `Background`, a text `Label` and an optional hairline `Border`.
 */

//! Primary or quiet button handler.
class TBD_UIButton : TBD_UIInteractive
{
	[Attribute("0", UIWidgets.CheckBox, desc: "Primary action styling -- at most one per screen")]
	protected bool m_bPrimary; //!< true = the filled primary look; default false

	[Attribute("", UIWidgets.EditBox, desc: "Label text, if the layout does not already carry it")]
	protected string m_sLabel; //!< label text written at bind when not empty; default empty

	protected Widget m_wBackground; //!< `Background` image or frame dock
	protected Widget m_wBorder; //!< `Border` hairline, optional
	protected TextWidget m_wLabel; //!< `Label`

	//! (TBD_UIButton button) -- fires on the click itself. There is no separate confirm step.
	protected ref ScriptInvoker m_OnActivate; //!< created on first GetOnActivate

	//! Opaque colour under the button; 0 = the backdrop (bottom bars). See TBD_UITheme colour law.
	protected int m_iGround; //!< 0 = the backdrop (bottom bars)
	//! PRIMARY (blue) by default; SUCCESS (emerald) and WARNING (amber) for the toggled states of the
	//! lobby's Ready / Lock buttons. Other tints fall back to PRIMARY.
	protected TBD_EUITint m_eTint = TBD_EUITint.PRIMARY; //!< default PRIMARY

	//! Find the widgets, round the frame docks and write the attribute label.
	override protected void OnBind(Widget w)
	{
		m_wBackground = w.FindAnyWidget("Background");
		m_wBorder = w.FindAnyWidget("Border");
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("Label"));

		// rounded-lg. No-op on layouts whose Border/Background are still plain images.
		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);

		if (!m_sLabel.IsEmpty())
			TBD_UITheme.Write(m_wLabel, m_sLabel);
	}

	//! The opaque colour this button sits on (a panel, a menu, the backdrop).
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
	}

	//! A click fires OnActivate with this button.
	override protected void OnActivated()
	{
		if (m_OnActivate)
			m_OnActivate.Invoke(this);
	}

	//! Highlight is not a commitment, so a button has nothing to preview.
	override protected void OnHighlighted() {}

	//! Paint disabled, success, warning, primary or quiet, lit while highlighted, over the ground.
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int background;
		int border;
		int ink;

		if (!m_bInteractive)
		{
			background = TBD_UITheme.SURFACE_CONTAINER;
			border     = TBD_UITheme.GLASS_BORDER;
			ink        = TBD_UITheme.ROW_DISABLED_TEXT;
		}
		else if (m_bPrimary && m_eTint == TBD_EUITint.SUCCESS)
		{
			background = TBD_UITheme.BTN_SUCCESS_FILL;
			border     = TBD_UITheme.BTN_SUCCESS_BORDER;
			ink        = TBD_UITheme.ON_ACTION;
		}
		else if (m_eTint == TBD_EUITint.WARNING)
		{
			background = TBD_UITheme.BTN_WARNING_FILL;
			border     = TBD_UITheme.BTN_WARNING_BORDER;
			ink        = TBD_UITheme.BTN_WARNING_INK;
		}
		else if (m_bPrimary)
		{
			// Touched -> the lighter Aegis primary pair. Untouched -> the one action blue.
			if (IsHighlighted())
			{
				background = TBD_UITheme.PRIMARY;
				border     = TBD_UITheme.PRIMARY_FIXED;
				ink        = TBD_UITheme.ON_PRIMARY;
			}
			else
			{
				background = TBD_UITheme.ACTION;
				border     = TBD_UITheme.ACTION_BORDER;
				ink        = TBD_UITheme.ON_ACTION;
			}
		}
		else
		{
			// Quiet by default: nothing but ink until the pointer arrives.
			if (IsHighlighted())
			{
				background = TBD_UITheme.ROW_HOVER;
				border     = TBD_UITheme.GLASS_BORDER_STRONG;
				ink        = TBD_UITheme.PRIMARY_FIXED;
			}
			else
			{
				background = TBD_UITheme.TRANSPARENT;
				border     = TBD_UITheme.GLASS_BORDER;
				ink        = TBD_UITheme.PRIMARY;
			}
		}

		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.Ground();

		TBD_UITheme.PaintOver(m_wBackground, background, ground);
		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.Paint(m_wLabel, ink);
	}

	//! Set and write the label.
	void SetLabel(string label)
	{
		m_sLabel = label;
		TBD_UITheme.Write(m_wLabel, label);
	}

	//! Switch the primary look on or off and repaint.
	void SetPrimary(bool primary)
	{
		m_bPrimary = primary;
		Repaint();
	}

	//! SUCCESS / WARNING recolour a primary or quiet button for a toggled state; PRIMARY restores.
	void SetTint(TBD_EUITint tint)
	{
		m_eTint = tint;
		Repaint();
	}

	//! (TBD_UIButton) -- created lazily so a button nobody listens to costs nothing.
	ScriptInvoker GetOnActivate()
	{
		if (!m_OnActivate)
			m_OnActivate = new ScriptInvoker();

		return m_OnActivate;
	}

	//! Give this button gamepad focus; does nothing without a root or a workspace.
	void Focus()
	{
		if (!m_wRoot)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (workspace)
			workspace.SetFocusedWidget(m_wRoot);
	}
}
