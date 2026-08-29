//! T-181.7 - shared behaviour for every clickable TBD surface (buttons, list rows).
//!
//! Enfusion gives a widget handler seven separate hooks and no notion of "interaction state".
//! This collapses them into one: pointer hover and input focus both mean *highlighted*, so a
//! mouse user and a gamepad user see the same affordance, and every subclass repaints through a
//! single `Repaint()` that reads TBD_UITheme.
//!
//! Design law it enforces (docs/mod/TBD_MOD_DESIGN.md S2 - macOS methodology):
//!   * **Direct manipulation.** A click is the action. There is no "select, then confirm" -
//!     `OnActivated()` fires on the click itself.
//!   * **Immediate feedback.** Every state change repaints in the same frame.
//!   * **Progressive disclosure.** `OnHighlighted()` is the hook a screen uses to reveal the next
//!     level (hover a group -> its slots appear) without committing to anything.
//!
//! Attach it to a `ButtonWidgetClass` root: plain frames/overlays do not receive click or focus.
class TBD_UIInteractive : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected bool m_bHovered;
	protected bool m_bFocused;
	protected bool m_bInteractive = true;

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		OnBind(w);
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		m_bHovered = false;
		m_bFocused = false;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	override bool OnMouseEnter(Widget w, int x, int y)
	{
		m_bHovered = true;
		Repaint();
		OnHighlighted();
		return super.OnMouseEnter(w, x, y);
	}

	//------------------------------------------------------------------------------------------------
	override bool OnMouseLeave(Widget w, Widget enterW, int x, int y)
	{
		m_bHovered = false;
		Repaint();
		return super.OnMouseLeave(w, enterW, x, y);
	}

	//------------------------------------------------------------------------------------------------
	override bool OnFocus(Widget w, int x, int y)
	{
		m_bFocused = true;
		Repaint();
		OnHighlighted();
		return super.OnFocus(w, x, y);
	}

	//------------------------------------------------------------------------------------------------
	override bool OnFocusLost(Widget w, int x, int y)
	{
		m_bFocused = false;
		Repaint();
		return super.OnFocusLost(w, x, y);
	}

	//------------------------------------------------------------------------------------------------
	//! One click = the action. Returns true (consumed) only when we actually acted, so a disabled
	//! surface still lets the event reach whatever is behind it.
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (!m_bInteractive)
			return super.OnClick(w, x, y, button);

		OnActivated();
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Subclass hook: cache child widgets. Called once, on attach.
	protected void OnBind(Widget w) {}

	//------------------------------------------------------------------------------------------------
	//! Subclass hook: the user committed. Fires on the click - never on hover, never on focus.
	protected void OnActivated() {}

	//------------------------------------------------------------------------------------------------
	//! Subclass hook: the user is looking at this without committing. Safe to preview.
	protected void OnHighlighted() {}

	//------------------------------------------------------------------------------------------------
	//! Subclass hook: repaint from TBD_UITheme using the current state. Must be idempotent -
	//! it is called on every state change and on rebind.
	void Repaint() {}

	//------------------------------------------------------------------------------------------------
	//! Hovered OR focused. One concept, so mouse and gamepad render identically.
	bool IsHighlighted()
	{
		return m_bHovered || m_bFocused;
	}

	//------------------------------------------------------------------------------------------------
	//! Disabled surfaces stay visible and readable (progressive disclosure beats hiding things),
	//! they just stop responding.
	void SetInteractive(bool interactive)
	{
		if (m_bInteractive == interactive)
			return;

		m_bInteractive = interactive;

		if (m_wRoot)
			m_wRoot.SetEnabled(interactive);

		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	bool IsInteractive()
	{
		return m_bInteractive;
	}

	//------------------------------------------------------------------------------------------------
	Widget GetRootWidget()
	{
		return m_wRoot;
	}
}
