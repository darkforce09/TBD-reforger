//! T-181.7 — the TBD button.
//!
//! Two variants only, because design law allows exactly ONE obvious primary action per screen:
//!   * `m_bPrimary = true`  — the single loud trigger (Deploy, Confirm). Filled ACTION blue.
//!   * `m_bPrimary = false` — everything else. Quiet: no fill until you touch it.
//!
//! Every colour comes from TBD_UITheme; the hover state of the primary button reuses the existing
//! PRIMARY / ON_PRIMARY pair rather than inventing a lighter blue, so the palette stays closed.
//!
//! Attach to a `ButtonWidgetClass` containing an image named `Background` and a text named `Label`.
class TBD_UIButton : TBD_UIInteractive
{
	[Attribute("0", UIWidgets.CheckBox, "Primary action styling — at most one per screen")]
	protected bool m_bPrimary;

	[Attribute("", UIWidgets.EditBox, "Label text, if the layout does not already carry it")]
	protected string m_sLabel;

	protected Widget m_wBackground;
	protected TextWidget m_wLabel;

	//! (TBD_UIButton button) — fires on the click itself. There is no separate confirm step.
	protected ref ScriptInvoker m_OnActivate;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wBackground = w.FindAnyWidget("Background");
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("Label"));

		if (!m_sLabel.IsEmpty())
			TBD_UITheme.Write(m_wLabel, m_sLabel);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		if (m_OnActivate)
			m_OnActivate.Invoke(this);
	}

	//------------------------------------------------------------------------------------------------
	//! Highlight is not a commitment, so a button has nothing to preview.
	override protected void OnHighlighted() {}

	//------------------------------------------------------------------------------------------------
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int background;
		int ink;

		if (!m_bInteractive)
		{
			background = TBD_UITheme.SURFACE_CONTAINER;
			ink        = TBD_UITheme.ROW_DISABLED_TEXT;
		}
		else if (m_bPrimary)
		{
			// Touched -> the lighter Aegis primary pair. Untouched -> the one action blue.
			if (IsHighlighted())
			{
				background = TBD_UITheme.PRIMARY;
				ink        = TBD_UITheme.ON_PRIMARY;
			}
			else
			{
				background = TBD_UITheme.ACTION;
				ink        = TBD_UITheme.ON_ACTION;
			}
		}
		else
		{
			// Quiet by default: nothing but ink until the pointer arrives.
			if (IsHighlighted())
			{
				background = TBD_UITheme.ROW_HOVER;
				ink        = TBD_UITheme.PRIMARY_FIXED;
			}
			else
			{
				background = TBD_UITheme.TRANSPARENT;
				ink        = TBD_UITheme.PRIMARY;
			}
		}

		TBD_UITheme.Paint(m_wBackground, background);
		TBD_UITheme.Paint(m_wLabel, ink);
	}

	//------------------------------------------------------------------------------------------------
	void SetLabel(string label)
	{
		m_sLabel = label;
		TBD_UITheme.Write(m_wLabel, label);
	}

	//------------------------------------------------------------------------------------------------
	void SetPrimary(bool primary)
	{
		m_bPrimary = primary;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_UIButton) — created lazily so a button nobody listens to costs nothing.
	ScriptInvoker GetOnActivate()
	{
		if (!m_OnActivate)
			m_OnActivate = new ScriptInvoker();

		return m_OnActivate;
	}

	//------------------------------------------------------------------------------------------------
	void Focus()
	{
		if (!m_wRoot)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (workspace)
			workspace.SetFocusedWidget(m_wRoot);
	}
}
