//! T-181.7 — the TBD screen stack and focus manager.
//!
//! Vanilla's `MenuManager` can open, close and tell you the top *engine* menu, but it has no
//! concept of "which of MY screens is in front", it cannot tell you which preset a menu came from
//! (`MenuBase.GetPresetID()` does not exist — probed), and it does nothing about input or focus
//! when a screen goes away. The lobby, briefing and spectator screens all need those answers, so
//! they live here, once.
//!
//! ── Invariants ──────────────────────────────────────────────────────────────────────────────
//!  1. **A screen appears at most once.** `Open()` on an already-open preset returns the existing
//!     screen instead of stacking a duplicate.
//!  2. **The stack is the truth even when the engine closes a screen behind our back.** Esc, a
//!     mission restart, or a raw `MenuManager.CloseMenu` all route through
//!     `TBD_MenuBase.OnMenuClose` -> `NotifyClosed`, so nothing can leave a ghost entry.
//!  3. **Exactly one screen owns input.** Screens re-arm their input context per frame only while
//!     `IsTop(this)`; when the stack empties nobody re-arms anything and gameplay input returns.
//!  4. **Focus follows the top.** After every push and pop the new top screen re-seeds focus via
//!     `FocusDefault()` — never a remembered widget pointer, which can dangle.
//!
//! ── What this class deliberately does NOT do ─────────────────────────────────────────────────
//! It does not show or hide the mouse cursor. There is no script API for it:
//! `WorkspaceWidget.SetCursorVisible` does not exist (probed). Cursor behaviour is a property of
//! the preset's `ActionContext` in `Configs/Systems/Menus/chimeraMenus.conf`, i.e. authored data,
//! not runtime code. What the stack owns is the input context and the focus, which is what
//! actually strands a player when it goes wrong.
class TBD_MenuStack
{
	//! Front-to-back order; last element is the top. Elements are weak — the engine's MenuManager
	//! owns menu lifetime, exactly like vanilla's `array<SCR_ListBoxElementComponent>` holds
	//! widget-owned handlers.
	protected static ref array<TBD_MenuBase> m_aStack;

	//! (TBD_MenuBase top) — top may be null when the stack empties. HUDs listen to this to know
	//! whether a full-screen TBD screen is covering the world.
	protected static ref ScriptInvoker m_OnStackChanged;

	//! Set for the duration of one Open() so the screen can be stamped from inside its own
	//! OnMenuOpen, which the engine fires synchronously before OpenMenu() returns.
	protected static int m_iPendingPreset = -1;

	//------------------------------------------------------------------------------------------------
	//! Push a screen. Returns the live screen for the preset — the existing one if it was already
	//! open, so callers may treat this as "make sure this screen is up".
	//! Returns null if the preset is not a TBD screen or the menu could not be created.
	static TBD_MenuBase Open(ChimeraMenuPreset preset)
	{
		EnsureStack();

		TBD_MenuBase existing = FindByPreset(preset);
		if (existing)
			return existing;

		MenuManager menuManager = GetGame().GetMenuManager();
		if (!menuManager)
			return null;

		m_iPendingPreset = preset;
		MenuBase opened = menuManager.OpenMenu(preset);
		m_iPendingPreset = -1;

		if (!opened)
		{
			// MEASURED: the engine logs `GUI (E): Menu preset '<name>' not found!` at startup for
			// every ChimeraMenuPreset value with no MenuPreset block the resource system can see.
			// A hand-authored Configs/System/chimeraMenus.conf is NOT enough on its own — the file
			// must also be in the addon's resourceDatabase.rdb, which only Workbench regenerates.
			// See the operator note in TBD_UILayouts.
			Print(string.Format("[TBD][ui] preset %1 did not open — is it registered in Configs/System/chimeraMenus.conf AND in resourceDatabase.rdb?", preset), LogLevel.ERROR);
			return null;
		}

		TBD_MenuBase screen = TBD_MenuBase.Cast(opened);
		if (!screen)
		{
			// Not one of ours: close it again rather than leaving a screen the stack cannot manage.
			menuManager.CloseMenu(opened);
			Print(string.Format("[TBD][ui] preset %1 is not a TBD_MenuBase — refusing to stack it.", preset), LogLevel.WARNING);
			return null;
		}

		// RegisterOpening already pushed it from OnMenuOpen; this is the belt-and-braces path for
		// an engine build that ever defers OnMenuOpen.
		if (m_aStack.Find(screen) < 0)
		{
			screen.SetPreset(preset);
			m_aStack.Insert(screen);
			OnTopChanged();
		}

		return screen;
	}

	//------------------------------------------------------------------------------------------------
	//! Swap the top screen for another — LOBBY -> BRIEFING, BRIEFING -> SAFESTART. Distinct from
	//! Open() because a phase transition must not leave the previous phase's screen underneath.
	static TBD_MenuBase Replace(ChimeraMenuPreset preset)
	{
		CloseTop();
		return Open(preset);
	}

	//------------------------------------------------------------------------------------------------
	//! Close a specific screen by preset. Returns false when it was not open.
	static bool Close(ChimeraMenuPreset preset)
	{
		TBD_MenuBase screen = FindByPreset(preset);
		if (!screen)
			return false;

		return CloseScreen(screen);
	}

	//------------------------------------------------------------------------------------------------
	//! Close a screen instance. The pop itself happens in NotifyClosed, driven by the engine's
	//! close callback, so this stays correct no matter who initiates the close.
	static bool CloseScreen(TBD_MenuBase screen)
	{
		if (!screen)
			return false;

		MenuManager menuManager = GetGame().GetMenuManager();
		if (!menuManager)
			return false;

		menuManager.CloseMenu(screen);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	static bool CloseTop()
	{
		return CloseScreen(Top());
	}

	//------------------------------------------------------------------------------------------------
	//! Tear every TBD screen down, top first. Used on game-stage changes and on mission teardown.
	static void CloseAll()
	{
		EnsureStack();

		// Snapshot: every close re-enters NotifyClosed and mutates m_aStack.
		array<TBD_MenuBase> pending = {};
		foreach (TBD_MenuBase screen : m_aStack)
		{
			pending.Insert(screen);
		}

		for (int i = pending.Count() - 1; i >= 0; i--)
		{
			CloseScreen(pending[i]);
		}

		// Anything the engine failed to call back for must not linger.
		if (!m_aStack.IsEmpty())
		{
			m_aStack.Clear();
			OnTopChanged();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! The screen currently in front, or null.
	static TBD_MenuBase Top()
	{
		EnsureStack();

		if (m_aStack.IsEmpty())
			return null;

		return m_aStack[m_aStack.Count() - 1];
	}

	//------------------------------------------------------------------------------------------------
	//! Preset of the front screen, or -1 when no TBD screen is up.
	static int TopPreset()
	{
		TBD_MenuBase top = Top();
		if (!top)
			return -1;

		return top.GetPreset();
	}

	//------------------------------------------------------------------------------------------------
	static bool IsTop(TBD_MenuBase screen)
	{
		return screen && Top() == screen;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsOpen(ChimeraMenuPreset preset)
	{
		return FindByPreset(preset) != null;
	}

	//------------------------------------------------------------------------------------------------
	static int Depth()
	{
		EnsureStack();
		return m_aStack.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! True while any TBD screen is covering the world — the question a HUD or an input handler
	//! actually wants answered.
	static bool IsAnyScreenOpen()
	{
		return Depth() > 0;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_MenuBase FindByPreset(ChimeraMenuPreset preset)
	{
		EnsureStack();

		foreach (TBD_MenuBase screen : m_aStack)
		{
			if (screen && screen.GetPreset() == preset)
				return screen;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_MenuBase top) — lazily created.
	static ScriptInvoker GetOnStackChanged()
	{
		if (!m_OnStackChanged)
			m_OnStackChanged = new ScriptInvoker();

		return m_OnStackChanged;
	}

	// ── Called by TBD_MenuBase. Not part of the screen-facing API. ──────────────────────────

	//------------------------------------------------------------------------------------------------
	//! A TBD screen is opening. Stamps the preset Open() is waiting on and pushes it, so the stack
	//! is already correct by the time the screen's own OnScreenOpen runs.
	static void RegisterOpening(TBD_MenuBase screen)
	{
		if (!screen)
			return;

		EnsureStack();

		if (m_iPendingPreset >= 0)
			screen.SetPreset(m_iPendingPreset);

		if (m_aStack.Find(screen) >= 0)
			return;

		m_aStack.Insert(screen);
		OnTopChanged();
	}

	//------------------------------------------------------------------------------------------------
	//! A TBD screen has closed — by our hand or the engine's. Pops it from wherever it sits (a
	//! screen underneath can be closed out of order) and hands focus to the new top.
	static void NotifyClosed(TBD_MenuBase screen)
	{
		if (!screen)
			return;

		EnsureStack();

		int index = m_aStack.Find(screen);
		if (index < 0)
			return;

		// Enfusion arrays remove BY INDEX (docs/mod/TBD_MOD_DESIGN.md §5) — never by value.
		m_aStack.Remove(index);
		OnTopChanged();
	}

	//------------------------------------------------------------------------------------------------
	//! Drop all bookkeeping without touching the engine. For mission teardown, where the menus are
	//! already gone.
	//!
	//! ── T-181.49: why this exists and who calls it ──────────────────────────────────────────
	//! Until T-181.49 this had ZERO callers, which made invariant 2 above conditional on the
	//! engine always firing `OnMenuClose`. It does not on world teardown. `m_aStack` holds
	//! deliberately weak elements popped only via `NotifyClosed`, so a menu the engine destroyed
	//! behind our back leaves an entry in a STATIC array that outlives the world — and the next
	//! world's `IsOpen(preset)` then answers true forever, for a screen that no longer exists.
	//! `TBD_LobbyStage.Start` is the caller: a new world arming its watcher is the unambiguous
	//! moment at which nothing from the previous world may still be believed.
	//!
	//! `CloseAll()` still has no caller. That is correct — it drives the ENGINE, so it is only
	//! valid while the menus are alive, and nothing in the addon currently needs a bulk close.
	//! Do not wire it to teardown to "match" this: teardown is exactly when those menus are gone.
	static void Reset()
	{
		EnsureStack();

		if (m_aStack.IsEmpty())
			return;

		m_aStack.Clear();
		OnTopChanged();
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected static void EnsureStack()
	{
		if (!m_aStack)
			m_aStack = {};
	}

	//------------------------------------------------------------------------------------------------
	//! One place decides what "the top changed" means: re-seed focus, or hand input back.
	protected static void OnTopChanged()
	{
		TBD_MenuBase top = Top();

		if (top)
			top.FocusDefault();
		else
			ReleaseFocus();

		if (m_OnStackChanged)
			m_OnStackChanged.Invoke(top);
	}

	//------------------------------------------------------------------------------------------------
	//! Stack empty: drop widget focus so keyboard input stops being eaten by a menu widget. The
	//! input context needs no explicit release — contexts are per-frame and nothing re-arms them
	//! once the last screen is gone.
	protected static void ReleaseFocus()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (workspace)
			workspace.SetFocusedWidget(null);
	}
}
