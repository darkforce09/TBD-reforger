/**
 * @file TBD_UIInteractive.c
 * @brief Shared behaviour of every clickable TBD surface: one highlight state, one click action.
 *
 * Role: base handler that folds the engine's hover, focus and click hooks into a highlighted state
 * and an activation, and repaints through one `Repaint()`.
 * Position: attached through its subclasses (`TBD_UIButton`, `TBD_ListBoxRow`,
 * `TBD_NavItemComponent`, `TBD_DropdownComponent` and the briefing navigation items) to a
 * `ButtonWidgetClass` root, since plain frames and overlays receive no click or focus.
 * State: the root, the hover, focus and interactive flags, on the client.
 * Invariants: hover and focus both mean highlighted, so mouse and gamepad render identically;
 * `OnActivated` fires on the click itself; a disabled surface lets the click through.
 */

//! Base of the clickable TBD surfaces.
//!
//! Enfusion gives a widget handler seven separate hooks and no notion of "interaction state".
//! This collapses them into one: pointer hover and input focus both mean *highlighted*, so a
//! mouse user and a gamepad user see the same affordance, and every subclass repaints through a
//! single `Repaint()` that reads TBD_UITheme.
//!
//! Design law it enforces (the macOS methodology of
//! documentation_v2/mod/tbd-framework/mod_design.md section 2):
//!   * **Direct manipulation.** A click is the action. There is no "select, then confirm" --
//!     `OnActivated()` fires on the click itself.
//!   * **Immediate feedback.** Every state change repaints in the same frame.
//!   * **Progressive disclosure.** `OnHighlighted()` is the hook a screen uses to reveal the next
//!     level (hover a group -> its slots appear) without committing to anything.
//!
//! Attach it to a `ButtonWidgetClass` root: plain frames/overlays do not receive click or focus.
class TBD_UIInteractive : ScriptedWidgetComponent
{
	protected Widget m_wRoot; //!< the widget this handler sits on; null after detach
	protected bool m_bHovered; //!< true while the pointer is over the root
	protected bool m_bFocused; //!< true while the root has input focus
	protected bool m_bInteractive = true; //!< false disables clicks and dims the surface; default true

	//! Remember the root, let the subclass bind its children, then repaint.
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		OnBind(w);
		Repaint();
	}

	//! Clear the root and the highlight when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		m_bHovered = false;
		m_bFocused = false;
		super.HandlerDeattached(w);
	}

	//! Pointer entered: highlight, repaint and offer the preview hook.
	override bool OnMouseEnter(Widget w, int x, int y)
	{
		m_bHovered = true;
		Repaint();
		OnHighlighted();
		return super.OnMouseEnter(w, x, y);
	}

	//! Pointer left: repaint without the hover.
	override bool OnMouseLeave(Widget w, Widget enterW, int x, int y)
	{
		m_bHovered = false;
		Repaint();
		return super.OnMouseLeave(w, enterW, x, y);
	}

	//! Focus arrived: highlight, repaint and offer the preview hook.
	override bool OnFocus(Widget w, int x, int y)
	{
		m_bFocused = true;
		Repaint();
		OnHighlighted();
		return super.OnFocus(w, x, y);
	}

	//! Focus left: repaint without the focus.
	override bool OnFocusLost(Widget w, int x, int y)
	{
		m_bFocused = false;
		Repaint();
		return super.OnFocusLost(w, x, y);
	}

	//! One click = the action. Returns true (consumed) only when the surface acted, so a disabled
	//! surface still lets the event reach whatever is behind it.
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (!m_bInteractive)
			return super.OnClick(w, x, y, button);

		OnActivated();
		return true;
	}

	//! Subclass hook: cache child widgets. Called once, on attach.
	protected void OnBind(Widget w) {}

	//! Subclass hook: the user committed. Fires on the click -- never on hover, never on focus.
	protected void OnActivated() {}

	//! Subclass hook: the user is looking at this without committing. Safe to preview.
	protected void OnHighlighted() {}

	//! Subclass hook: repaint from TBD_UITheme using the current state. Must be idempotent --
	//! it is called on every state change and on rebind.
	void Repaint() {}

	//! Hovered OR focused. One concept, so mouse and gamepad render identically.
	bool IsHighlighted()
	{
		return m_bHovered || m_bFocused;
	}

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

	//! @return false while the surface is disabled
	bool IsInteractive()
	{
		return m_bInteractive;
	}

	//! @return the widget this handler sits on; null after detach
	Widget GetRootWidget()
	{
		return m_wRoot;
	}
}
