//! T-181.7 — the base class every TBD screen derives from.
//!
//! Derived from vanilla `ChimeraMenuBase` (read from real source at
//! `apps/mod/vanilla_reference/Source/ChimeraMenuBase.c`, not from memory). ChimeraMenuBase gives
//! us the lifecycle (`OnMenuOpen/Opened/Close/Show/Hide/FocusGained/FocusLost/Update`) and
//! forwards each one to `SCR_MenuHelper`'s global invokers. It gives us nothing else: no stack,
//! no input ownership, no focus policy. That is what this class and TBD_MenuStack add.
//!
//! ── Two facts this design is built on (both probed against the engine, not assumed) ──────────
//!  1. `MenuBase.GetPresetID()` DOES NOT EXIST. A menu cannot tell you which preset opened it, so
//!     TBD_MenuStack stamps the preset onto the screen as it opens (`SetPreset`).
//!  2. Input contexts decay: `InputManager.ActivateContext()` must be re-called every frame while
//!     the context should be live. `OnMenuUpdate` does that — but ONLY for the top screen, which
//!     is how a stacked screen is prevented from stealing input from the one above it.
//!
//! Subclasses override the `OnScreen*` hooks, never the `OnMenu*` ones, so the stack bookkeeping
//! can never be skipped by forgetting a `super` call.
class TBD_MenuBase : ChimeraMenuBase
{
	protected Widget m_wRoot;

	//! ChimeraMenuPreset this screen was opened with, stamped by TBD_MenuStack. -1 = opened
	//! outside the stack (e.g. a raw MenuManager.OpenMenu somewhere) — still tracked, just not
	//! closable by preset.
	protected int m_iPreset = -1;

	protected bool m_bScreenOpen;

	//! (TBD_MenuBase screen) — fired once, after the screen has closed and left the stack.
	protected ref ScriptInvoker m_OnScreenClosed;

	//------------------------------------------------------------------------------------------------
	override void OnMenuOpen()
	{
		super.OnMenuOpen();

		m_wRoot = GetRootWidget();
		m_bScreenOpen = true;

		// Register BEFORE the subclass binds, so a screen may consult the stack from OnScreenOpen.
		TBD_MenuStack.RegisterOpening(this);

		OnScreenOpen();
	}

	//------------------------------------------------------------------------------------------------
	override void OnMenuOpened()
	{
		super.OnMenuOpened();
		FocusDefault();
	}

	//------------------------------------------------------------------------------------------------
	override void OnMenuUpdate(float tDelta)
	{
		super.OnMenuUpdate(tDelta);

		// Only the top of the TBD stack owns input. Contexts are per-frame, so simply not
		// re-arming is how a covered screen releases them.
		if (!TBD_MenuStack.IsTop(this))
			return;

		string context = GetInputContext();
		if (!context.IsEmpty())
		{
			InputManager inputManager = GetGame().GetInputManager();
			if (inputManager)
				inputManager.ActivateContext(context);
		}

		OnScreenUpdate(tDelta);
	}

	//------------------------------------------------------------------------------------------------
	//! Runs for every close path — ours, and the engine's (Esc, menu manager teardown, mission
	//! restart). That is why the stack is popped from here and not from TBD_MenuStack.Close().
	override void OnMenuClose()
	{
		super.OnMenuClose();

		if (!m_bScreenOpen)
			return;

		m_bScreenOpen = false;
		OnScreenClose();

		TBD_MenuStack.NotifyClosed(this);

		if (m_OnScreenClosed)
			m_OnScreenClosed.Invoke(this);
	}

	// ── Subclass hooks ──────────────────────────────────────────────────────────────────────

	//! Bind widgets here. The root widget is already available via GetRoot().
	protected void OnScreenOpen() {}

	//! Per-frame work for the TOP screen only. Covered screens do not tick.
	protected void OnScreenUpdate(float tDelta) {}

	//! Release listeners here. Runs before the screen leaves the stack.
	protected void OnScreenClose() {}

	//! Input context this screen arms while it is on top. Empty string = arm nothing.
	//! "MenuContext" is the vanilla menu context (CRF uses the same name for its menus).
	protected string GetInputContext()
	{
		return "MenuContext";
	}

	//------------------------------------------------------------------------------------------------
	//! Where focus lands when this screen becomes the top one. The stack calls this after every
	//! push and every pop, so a keyboard/gamepad user is never left with focus on a dead widget.
	//!
	//! Deliberately NOT "restore the widget that was focused before we opened": that pointer can
	//! outlive its widget when a screen underneath rebuilds its list, and a dangling focus target
	//! is exactly the bug that leaves a player unable to click anything. Re-seeding from the live
	//! top screen is always safe.
	void FocusDefault()
	{
		if (!m_wRoot)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return;

		// A screen may name one widget "FocusAnchor" to claim initial focus.
		Widget anchor = m_wRoot.FindAnyWidget("FocusAnchor");
		if (anchor)
		{
			workspace.SetFocusedWidget(anchor);
			return;
		}

		workspace.SetFocusedWidget(m_wRoot);
	}

	// ── Public surface ──────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	Widget GetRoot()
	{
		return m_wRoot;
	}

	//------------------------------------------------------------------------------------------------
	//! The ChimeraMenuPreset this screen was opened with, or -1 when it was opened outside the
	//! stack. The engine cannot answer this — see the class header.
	int GetPreset()
	{
		return m_iPreset;
	}

	//------------------------------------------------------------------------------------------------
	//! TBD_MenuStack only. Public because Enfusion has no friend classes.
	void SetPreset(int preset)
	{
		m_iPreset = preset;
	}

	//------------------------------------------------------------------------------------------------
	bool IsScreenOpen()
	{
		return m_bScreenOpen;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_MenuBase) — lazily created.
	ScriptInvoker GetOnScreenClosed()
	{
		if (!m_OnScreenClosed)
			m_OnScreenClosed = new ScriptInvoker();

		return m_OnScreenClosed;
	}

	//------------------------------------------------------------------------------------------------
	//! Close this screen through the stack so input and focus are handed back correctly.
	void CloseScreen()
	{
		TBD_MenuStack.CloseScreen(this);
	}

	// ── Widget helpers. Every screen needs these; none should hand-roll a null check. ───────

	//------------------------------------------------------------------------------------------------
	protected Widget Find(string name)
	{
		if (!m_wRoot)
			return null;

		return m_wRoot.FindAnyWidget(name);
	}

	//------------------------------------------------------------------------------------------------
	protected TextWidget FindText(string name)
	{
		return TextWidget.Cast(Find(name));
	}

	//------------------------------------------------------------------------------------------------
	//! Find a widget by name and pull one of our handlers off it in a single step.
	protected ScriptedWidgetComponent FindHandlerOn(string name, typename handler)
	{
		Widget w = Find(name);
		if (!w)
			return null;

		return ScriptedWidgetComponent.Cast(w.FindHandler(handler));
	}

	//------------------------------------------------------------------------------------------------
	protected void SetTextOn(string name, string text)
	{
		TBD_UITheme.Write(FindText(name), text);
	}
}
