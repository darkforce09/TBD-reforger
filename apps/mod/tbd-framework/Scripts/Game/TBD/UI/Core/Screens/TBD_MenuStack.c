/**
 * @file TBD_MenuStack.c
 * @brief The TBD screen stack and focus manager: one screen per preset, input and focus on top.
 *
 * Role: opens, replaces and closes TBD screens by `ChimeraMenuPreset`, keeps the front-to-back
 * order, stamps each screen's preset, and hands focus to the new top after every push and pop.
 * Position: called by the screens under Session (open, replace, close), by `TBD_MenuBase` (register
 * and notify closed) and by `TBD_LobbyStage.Start` (reset); HUDs listen to `GetOnStackChanged()`.
 * State: the static stack, the pending preset of an `Open` in flight and the change event, for the
 * life of the script VM on the client.
 * Invariants: a preset appears at most once (`Open` on an open preset returns it); every close
 * path pops through `TBD_MenuBase.OnMenuClose` -> `NotifyClosed`, so no ghost entry stays; only
 * the top screen re-arms its input context, and when the stack empties gameplay input returns.
 */

//! Static screen stack. Vanilla `MenuManager` knows the top engine menu but not which TBD screen is
//! in front or which preset opened a menu (`MenuBase.GetPresetID()` does not exist), and does
//! nothing about input or focus when a screen goes away; this class answers those once. Focus
//! follows the top through `FocusDefault()`, never a remembered widget pointer, which can dangle.
//! It does not show or hide the mouse cursor: `WorkspaceWidget.SetCursorVisible` does not exist,
//! and cursor behaviour is the preset's `ActionContext` in `Configs/System/chimeraMenus.conf`.
class TBD_MenuStack
{
	//! Front-to-back order; last element is the top. Elements are weak -- the engine's MenuManager
	//! owns menu lifetime, exactly like vanilla's `array<SCR_ListBoxElementComponent>` holds
	//! widget-owned handlers.
	protected static ref array<TBD_MenuBase> m_aStack; //!< front to back; the last element is the top; weak, the MenuManager owns menu lifetime

	//! (TBD_MenuBase top) -- top may be null when the stack empties. HUDs listen to this to know
	//! whether a full-screen TBD screen is covering the world.
	protected static ref ScriptInvoker m_OnStackChanged; //!< created on first GetOnStackChanged

	//! Set for the duration of one Open() so the screen can be stamped from inside its own
	//! OnMenuOpen, which the engine fires synchronously before OpenMenu() returns.
	protected static int m_iPendingPreset = -1; //!< -1 = no Open in flight

	//! Push a screen. Returns the live screen for the preset -- the existing one if it was already
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
			// The engine logs `GUI (E): Menu preset '<name>' not found!` at startup for every
			// ChimeraMenuPreset value with no MenuPreset block the resource system can see: a hand-authored
			// Configs/System/chimeraMenus.conf also has to be in the addon's resourceDatabase.rdb, which only
			// Workbench regenerates.
			Print(string.Format("[TBD][ui] preset %1 did not open -- is it registered in Configs/System/chimeraMenus.conf AND in resourceDatabase.rdb?", preset), LogLevel.ERROR);
			return null;
		}

		TBD_MenuBase screen = TBD_MenuBase.Cast(opened);
		if (!screen)
		{
			// Not one of ours: close it again rather than leaving a screen the stack cannot manage.
			menuManager.CloseMenu(opened);
			Print(string.Format("[TBD][ui] preset %1 is not a TBD_MenuBase -- refusing to stack it.", preset), LogLevel.WARNING);
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

	//! Swap the top screen for another -- LOBBY -> BRIEFING, BRIEFING -> SAFESTART. Distinct from
	//! Open() because a phase transition must not leave the previous phase's screen underneath.
	static TBD_MenuBase Replace(ChimeraMenuPreset preset)
	{
		CloseTop();
		return Open(preset);
	}

	//! Close a specific screen by preset. Returns false when it was not open.
	static bool Close(ChimeraMenuPreset preset)
	{
		TBD_MenuBase screen = FindByPreset(preset);
		if (!screen)
			return false;

		return CloseScreen(screen);
	}

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

	//! Close the front screen. @return false when no screen is up or there is no menu manager
	static bool CloseTop()
	{
		return CloseScreen(Top());
	}

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

	//! The screen currently in front, or null.
	static TBD_MenuBase Top()
	{
		EnsureStack();

		if (m_aStack.IsEmpty())
			return null;

		return m_aStack[m_aStack.Count() - 1];
	}

	//! Preset of the front screen, or -1 when no TBD screen is up.
	static int TopPreset()
	{
		TBD_MenuBase top = Top();
		if (!top)
			return -1;

		return top.GetPreset();
	}

	//! @return true when `screen` is the front screen
	static bool IsTop(TBD_MenuBase screen)
	{
		return screen && Top() == screen;
	}

	//! @return true when a screen for `preset` is on the stack
	static bool IsOpen(ChimeraMenuPreset preset)
	{
		return FindByPreset(preset) != null;
	}

	//! @return how many TBD screens are on the stack
	static int Depth()
	{
		EnsureStack();
		return m_aStack.Count();
	}

	//! True while any TBD screen is covering the world -- the question a HUD or an input handler
	//! actually wants answered.
	static bool IsAnyScreenOpen()
	{
		return Depth() > 0;
	}

	//! @return the stacked screen opened with `preset`, or null
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

	//! (TBD_MenuBase top) -- lazily created.
	static ScriptInvoker GetOnStackChanged()
	{
		if (!m_OnStackChanged)
			m_OnStackChanged = new ScriptInvoker();

		return m_OnStackChanged;
	}

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

	//! A TBD screen has closed -- by our hand or the engine's. Pops it from wherever it sits (a
	//! screen underneath can be closed out of order) and hands focus to the new top.
	static void NotifyClosed(TBD_MenuBase screen)
	{
		if (!screen)
			return;

		EnsureStack();

		int index = m_aStack.Find(screen);
		if (index < 0)
			return;

		// Enfusion arrays remove BY INDEX (documentation/apps/mod/tbd-framework/mod_design.md
		// section 5), never by value.
		m_aStack.Remove(index);
		OnTopChanged();
	}

	//! Drop all bookkeeping without touching the engine. For mission teardown, where the menus are
	//! already gone.
	//!
	//! The engine does not fire `OnMenuClose` on world teardown, so a destroyed menu would otherwise
	//! leave an entry in this static stack that outlives the world, and the next world's
	//! `IsOpen(preset)` would answer true for a screen that is gone. `TBD_LobbyStage.Start`
	//! calls this when a new world arms its watcher. `CloseAll()` drives the engine and is valid only
	//! while the menus are alive, so teardown uses this instead.
	static void Reset()
	{
		EnsureStack();

		if (m_aStack.IsEmpty())
			return;

		m_aStack.Clear();
		OnTopChanged();
	}

	//! Create the stack array on first use.
	protected static void EnsureStack()
	{
		if (!m_aStack)
			m_aStack = {};
	}

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

	//! Stack empty: drop widget focus so keyboard input stops being eaten by a menu widget. The
	//! input context needs no explicit release -- contexts are per-frame and nothing re-arms them
	//! once the last screen is gone.
	protected static void ReleaseFocus()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (workspace)
			workspace.SetFocusedWidget(null);
	}
}
