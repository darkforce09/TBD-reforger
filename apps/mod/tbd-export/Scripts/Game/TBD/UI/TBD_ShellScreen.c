//! T-181.7 - the screen shell: the chrome every TBD screen sits in, and the end-to-end proof
//! that the stack, the theme and the list work together.
//!
//! `TBD_ScreenShell.layout` is deliberately one shape, reused by every screen in the program:
//!
//! ```
//!   +----------------------------------------------------------+
//!   |  TITLE                                        [ Back ]   |   header: what am I looking at
//!   |  subtitle                                                |
//!   +----------------------------------------------------------+
//!   |                                                          |
//!   |  scrollable TBD_ListBox                                  |   content: one list, one job
//!   |                                                          |
//!   +----------------------------------------------------------+
//!   |  status line                              [ DEPLOY ]     |   footer: ONE primary action
//!   +----------------------------------------------------------+
//! ```
//!
//! Why a shell rather than a screen-per-layout: the lobby, the briefing and the spectator list all
//! want the same frame, and design law says one obvious primary action per screen. Making the
//! frame a base class means a screen physically cannot grow a second loud button by accident.
//!
//! **Subclass it** (`class TBD_LobbyScreen : TBD_ShellScreen`), register the subclass in
//! `Configs/Systems/Menus/chimeraMenus.conf` against your own preset, and override
//! `OnScreenOpen()` - call `super.OnScreenOpen()` first, then populate `GetList()`.
//!
//! It is also directly usable as-is: preset `TBD_UIShell` opens this class and renders an empty
//! shell, which is what proves the menu stack end to end without a game mode attached.
class TBD_ShellScreen : TBD_MenuBase
{
	protected TextWidget m_wTitle;
	protected TextWidget m_wSubtitle;
	protected TextWidget m_wStatus;

	protected TBD_ListBox m_List;
	protected TBD_UIButton m_PrimaryAction;
	protected TBD_UIButton m_BackAction;

	//! (TBD_ShellScreen screen) - the one primary action was triggered.
	protected ref ScriptInvoker m_OnPrimaryAction;

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenOpen()
	{
		m_wTitle = FindText("Title");
		m_wSubtitle = FindText("Subtitle");
		m_wStatus = FindText("Status");

		m_List = TBD_ListBox.Cast(FindHandlerOn("List", TBD_ListBox));
		m_PrimaryAction = TBD_UIButton.Cast(FindHandlerOn("PrimaryAction", TBD_UIButton));
		m_BackAction = TBD_UIButton.Cast(FindHandlerOn("BackAction", TBD_UIButton));

		if (m_PrimaryAction)
			m_PrimaryAction.GetOnActivate().Insert(OnPrimaryActionClicked);

		if (m_BackAction)
			m_BackAction.GetOnActivate().Insert(OnBackActionClicked);

		// Backdrop and panel are painted from code, not baked into the layout, so a palette change
		// is a one-line edit in TBD_UITheme rather than a sweep through every .layout.
		TBD_UITheme.Paint(Find("Backdrop"), TBD_UITheme.SCRIM);
		TBD_UITheme.Paint(Find("Panel"), TBD_UITheme.SURFACE);
		TBD_UITheme.Paint(Find("HeaderRule"), TBD_UITheme.OUTLINE_VARIANT);
		TBD_UITheme.Paint(Find("FooterRule"), TBD_UITheme.OUTLINE_VARIANT);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wSubtitle, TBD_UITheme.ON_SURFACE_VARIANT);
		TBD_UITheme.Paint(m_wStatus, TBD_UITheme.ON_SURFACE_VARIANT);

		SetTitle(GetScreenTitle());
		SetSubtitle(GetScreenSubtitle());
		SetStatus(string.Empty);

		// No primary action until a screen declares one - an empty shell shows no loud button.
		SetPrimaryAction(string.Empty, false);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		if (m_PrimaryAction)
			m_PrimaryAction.GetOnActivate().Remove(OnPrimaryActionClicked);

		if (m_BackAction)
			m_BackAction.GetOnActivate().Remove(OnBackActionClicked);
	}

	//------------------------------------------------------------------------------------------------
	//! Land focus where the user's next move is: the first pickable row, else the primary action.
	override void FocusDefault()
	{
		if (m_List && m_List.FocusFirst())
			return;

		if (m_PrimaryAction && m_PrimaryAction.IsInteractive())
		{
			m_PrimaryAction.Focus();
			return;
		}

		super.FocusDefault();
	}

	// -- Subclass hooks ----------------------------------------------------------------------

	//! Header title. Override in a screen; do not write to the widget directly.
	protected string GetScreenTitle()
	{
		return "TBD FRAMEWORK";
	}

	//! One line of context under the title.
	protected string GetScreenSubtitle()
	{
		return "UI framework online";
	}

	// -- Public surface ----------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	void SetTitle(string title)
	{
		TBD_UITheme.Write(m_wTitle, title);
	}

	//------------------------------------------------------------------------------------------------
	void SetSubtitle(string subtitle)
	{
		TBD_UITheme.Write(m_wSubtitle, subtitle);
		TBD_UITheme.Show(m_wSubtitle, !subtitle.IsEmpty());
	}

	//------------------------------------------------------------------------------------------------
	//! Non-blocking feedback line in the footer. Use it instead of a modal - design law: nothing
	//! blocking.
	void SetStatus(string status)
	{
		TBD_UITheme.Write(m_wStatus, status);
		TBD_UITheme.Show(m_wStatus, !status.IsEmpty());
	}

	//------------------------------------------------------------------------------------------------
	//! The ONE primary action. An empty label hides it - a screen with nothing to commit shows no
	//! loud button at all.
	void SetPrimaryAction(string label, bool enabled)
	{
		if (!m_PrimaryAction)
			return;

		bool shown = !label.IsEmpty();

		Widget actionWidget = m_PrimaryAction.GetRootWidget();
		TBD_UITheme.Show(actionWidget, shown);

		if (!shown)
			return;

		m_PrimaryAction.SetLabel(label);
		m_PrimaryAction.SetInteractive(enabled);
	}

	//------------------------------------------------------------------------------------------------
	//! The shell's list. Null only if the layout has no `List` widget.
	TBD_ListBox GetList()
	{
		return m_List;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_ShellScreen)
	ScriptInvoker GetOnPrimaryAction()
	{
		if (!m_OnPrimaryAction)
			m_OnPrimaryAction = new ScriptInvoker();

		return m_OnPrimaryAction;
	}

	// -- Internals ---------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	protected void OnPrimaryActionClicked(TBD_UIButton button)
	{
		if (m_OnPrimaryAction)
			m_OnPrimaryAction.Invoke(this);
	}

	//------------------------------------------------------------------------------------------------
	protected void OnBackActionClicked(TBD_UIButton button)
	{
		CloseScreen();
	}
}

//! Presets TBD owns. Each entry needs a matching `MenuPreset` block in
//! `Configs/System/chimeraMenus.conf` binding it to a layout and a class. Screens in later slices
//! add their own `modded enum` block - several across files are fine.
//!
//! -- MEASURED: registration needs one Workbench pass -----------------------------------------
//! Adding the enum value and the `.conf` is necessary but NOT sufficient. Until the addon's
//! `resourceDatabase.rdb` lists `Configs/System/chimeraMenus.conf`, the engine cannot see it and
//! logs, at every startup:
//!
//!     GUI       (E): Menu preset 'TBD_UIShell' not found!
//!
//! Only Workbench regenerates that index; the headless compile lane cannot. Proven by
//! elimination, not assumed: the file was tried at both the vanilla path
//! (`Configs/System/`) and at a custom path, with and without a `.meta`, and its content was even
//! moved onto an already-indexed `.conf` path - the error persisted in every case. There is also
//! no script-side escape hatch: `MenuManager.RegisterPreset`, `OpenMenuByLayout`, `GetMenuPresets`
//! and `FindPreset` all fail to compile, i.e. they do not exist.
//!
//! That startup line is therefore the exact green light: it disappears the moment the resource is
//! registered, and it is cheap to check from the headless lane -
//! `grep "Menu preset" <profile>/logs/logs_*/error.log`.
modded enum ChimeraMenuPreset
{
	//! The bare shell. Opens TBD_ShellScreen with no content - the end-to-end proof of the stack.
	TBD_UIShell
}
