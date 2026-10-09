/**
 * @file TBD_ShellScreen.c
 * @brief The list shell: title, subtitle, one list, a status line and one primary action.
 *
 * Role: base screen over `Common/TBD_ScreenShell.layout`; binds the header, the `TBD_ListBox`, the
 * status line and the primary and back buttons, and paints the chrome from `TBD_UITheme`.
 * Position: base of `TBD_AdminScreen` and `TBD_SpectatorScreen`; the bare `TBD_UIShell` preset in
 * `Configs/System/chimeraMenus.conf` opens this class as an empty shell.
 * State: the bound widgets and handlers and the primary-action event, on the client.
 * Invariants: one obvious primary action per screen, hidden while its label is empty, so a screen
 * cannot grow a second loud button; the backdrop and panel are painted from code, so a palette
 * change is one edit in `TBD_UITheme`.
 */

//! List shell screen. The layout, top to bottom: header (TITLE, subtitle, Back), a scrollable
//! `TBD_ListBox` (one list, one job), and a footer (status line and the ONE primary action).
//! A subclass registers against its own preset in `Configs/System/chimeraMenus.conf` and overrides
//! `OnScreenOpen()`, calling `super.OnScreenOpen()` first, then populating `GetList()`.
class TBD_ShellScreen : TBD_MenuBase
{
	protected TextWidget m_wTitle; //!< `Title`
	protected TextWidget m_wSubtitle; //!< `Subtitle`, hidden while empty
	protected TextWidget m_wStatus; //!< `Status`, hidden while empty

	protected TBD_ListBox m_List; //!< the `List` handler; null when the layout has none
	protected TBD_UIButton m_PrimaryAction; //!< the `PrimaryAction` button
	protected TBD_UIButton m_BackAction; //!< the `BackAction` button, which closes the screen

	//! (TBD_ShellScreen screen) -- the one primary action was triggered.
	protected ref ScriptInvoker m_OnPrimaryAction; //!< created on first GetOnPrimaryAction

	//! Bind the shell widgets, subscribe to the buttons, paint the chrome, write title and subtitle
	//! and hide the primary action.
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
		TBD_UITheme.PaintAlpha(Find("Backdrop"), TBD_UITheme.SCRIM);
		TBD_UITheme.Paint(Find("Panel"), TBD_UITheme.SURFACE);
		TBD_UITheme.Paint(Find("HeaderRule"), TBD_UITheme.OUTLINE_VARIANT);
		TBD_UITheme.Paint(Find("FooterRule"), TBD_UITheme.OUTLINE_VARIANT);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wSubtitle, TBD_UITheme.ON_SURFACE_VARIANT);
		TBD_UITheme.Paint(m_wStatus, TBD_UITheme.ON_SURFACE_VARIANT);

		SetTitle(GetScreenTitle());
		SetSubtitle(GetScreenSubtitle());
		SetStatus(string.Empty);

		// No primary action until a screen declares one -- an empty shell shows no loud button.
		SetPrimaryAction(string.Empty, false);
	}

	//! Unsubscribe from the buttons.
	override protected void OnScreenClose()
	{
		if (m_PrimaryAction)
			m_PrimaryAction.GetOnActivate().Remove(OnPrimaryActionClicked);

		if (m_BackAction)
			m_BackAction.GetOnActivate().Remove(OnBackActionClicked);
	}

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

	//! Write the header title.
	void SetTitle(string title)
	{
		TBD_UITheme.Write(m_wTitle, title);
	}

	//! Write the subtitle; empty hides it.
	void SetSubtitle(string subtitle)
	{
		TBD_UITheme.Write(m_wSubtitle, subtitle);
		TBD_UITheme.Show(m_wSubtitle, !subtitle.IsEmpty());
	}

	//! Non-blocking feedback line in the footer. Use it instead of a modal -- design law: nothing
	//! blocking.
	void SetStatus(string status)
	{
		TBD_UITheme.Write(m_wStatus, status);
		TBD_UITheme.Show(m_wStatus, !status.IsEmpty());
	}

	//! The ONE primary action. An empty label hides it -- a screen with nothing to commit shows no
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

	//! The shell's list. Null only if the layout has no `List` widget.
	TBD_ListBox GetList()
	{
		return m_List;
	}

	//! (TBD_ShellScreen)
	ScriptInvoker GetOnPrimaryAction()
	{
		if (!m_OnPrimaryAction)
			m_OnPrimaryAction = new ScriptInvoker();

		return m_OnPrimaryAction;
	}

	//! Fire OnPrimaryAction with this screen.
	protected void OnPrimaryActionClicked(TBD_UIButton button)
	{
		if (m_OnPrimaryAction)
			m_OnPrimaryAction.Invoke(this);
	}

	//! The back button closes this screen through the stack.
	protected void OnBackActionClicked(TBD_UIButton button)
	{
		CloseScreen();
	}
}
