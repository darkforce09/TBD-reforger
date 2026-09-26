/**
 * @file TBD_EndScreen.c
 * @brief The END stage banner: the winning faction and the reason, as a workspace overlay.
 *
 * Role: shows "MISSION ENDED", the winner (or "No winner") and a readable end reason.
 * Position: TBD_EndBanner.ApplyEndScreens opens it in the END stage and closes it on any other;
 * the winner and reason come from TBD_FrameworkManager.GetEndWinner and GetEndReason.
 * State: the open root and live instance (static), and per instance the widgets; client.
 * Invariants: an overlay widget, never a Chimera menu, so Esc cannot refuse a stage change; Open
 * and Close never feed back into the stage machine.
 */

//! The END stage banner, handler of the root widget of TBD_UILayouts.END_SCREEN.
class TBD_EndScreen : ScriptedWidgetComponent
{
	protected static Widget s_wRoot; //!< the open overlay root; null when closed
	protected static TBD_EndScreen s_Instance; //!< the attached handler; null when closed

	protected Widget m_wRoot; //!< this handler's root widget
	protected TextWidget m_wTitle; //!< "MISSION ENDED"
	protected TextWidget m_wSubtitle; //!< "The round is over."
	protected TextWidget m_wWinner; //!< winning faction, or "No winner"
	protected TextWidget m_wReason; //!< readable end reason
	protected TextWidget m_wStatus; //!< "The next stage closes this screen."
	protected TBD_UIButton m_BackAction; //!< closes the overlay

	//! Open the overlay on this machine; no-op when open or without a workspace. Logs an ERROR when
	//! the layout will not load.
	//! @authority client
	static void Open()
	{
		if (s_wRoot)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return;

		Widget root = TBD_UILayouts.Create(TBD_UILayouts.END_SCREEN, null);
		if (!root)
		{
			Print("[TBD][ui] END_SCREEN layout did not instantiate", LogLevel.ERROR);
			return;
		}

		s_wRoot = root;
	}

	//! Remove the overlay; no-op when closed.
	static void Close()
	{
		if (!s_wRoot)
			return;

		s_wRoot.RemoveFromHierarchy();
		s_wRoot = null;
		s_Instance = null;
	}

	//! @return true while the overlay is open
	static bool IsOpen()
	{
		return s_wRoot != null;
	}

	//! Bind the widgets and the back action, paint the overlay and populate it.
	//! @param w the overlay root widget
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);

		m_wRoot = w;
		s_Instance = this;
		s_wRoot = w;

		m_wTitle = FindText("Title");
		m_wSubtitle = FindText("Subtitle");
		m_wWinner = FindText("Winner");
		m_wReason = FindText("Reason");
		m_wStatus = FindText("Status");
		m_BackAction = TBD_UIButton.Cast(FindHandlerOn("BackAction", TBD_UIButton));

		TBD_UITheme.PaintAlpha(Find("Backdrop"), TBD_UITheme.SCRIM);
		TBD_UITheme.Paint(Find("Panel"), TBD_UITheme.SURFACE);
		TBD_UITheme.Paint(Find("HeaderRule"), TBD_UITheme.OUTLINE_VARIANT);
		TBD_UITheme.Paint(Find("FooterRule"), TBD_UITheme.OUTLINE_VARIANT);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wSubtitle, TBD_UITheme.ON_SURFACE_VARIANT);
		TBD_UITheme.Paint(m_wWinner, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wReason, TBD_UITheme.ON_SURFACE_VARIANT);
		TBD_UITheme.Paint(m_wStatus, TBD_UITheme.ON_SURFACE_VARIANT);

		TBD_UITheme.Show(Find("List"), false);
		TBD_UITheme.Show(Find("PrimaryAction"), false);

		if (m_BackAction)
			m_BackAction.GetOnActivate().Insert(OnBackClicked);

		Populate();
	}

	//! Unbind the back action and clear the statics when they point here.
	//! @param w the overlay root widget
	override void HandlerDeattached(Widget w)
	{
		if (m_BackAction)
			m_BackAction.GetOnActivate().Remove(OnBackClicked);

		if (s_Instance == this)
			s_Instance = null;
		if (s_wRoot == w)
			s_wRoot = null;

		super.HandlerDeattached(w);
	}

	//! Fill the title, winner, reason, subtitle and status from TBD_FrameworkManager.
	protected void Populate()
	{
		TBD_UITheme.Write(m_wTitle, "MISSION ENDED");

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		string winner;
		string reason;
		if (fm)
		{
			winner = fm.GetEndWinner();
			reason = fm.GetEndReason();
		}

		if (winner.IsEmpty())
			winner = "No winner";

		TBD_UITheme.Write(m_wWinner, winner);
		TBD_UITheme.Write(m_wReason, DescribeReason(reason));
		TBD_UITheme.Write(m_wSubtitle, "The round is over.");
		TBD_UITheme.Show(m_wSubtitle, true);
		TBD_UITheme.Write(m_wStatus, "The next stage closes this screen.");
		TBD_UITheme.Show(m_wStatus, true);
	}

	//! @param reason the end reason key: faction_eliminated, time_limit, admin, or any other text
	//! @return a readable sentence; an unknown key is returned as is, an empty one as "The round ended."
	protected string DescribeReason(string reason)
	{
		if (reason.IsEmpty())
			return "The round ended.";
		if (reason == "faction_eliminated")
			return "Faction eliminated.";
		if (reason == "time_limit")
			return "Time limit expired.";
		if (reason == "admin")
			return "An admin ended the round.";
		return reason;
	}

	//! @param name a widget name under the root
	//! @return the widget, or null
	protected Widget Find(string name)
	{
		if (!m_wRoot)
			return null;

		return m_wRoot.FindAnyWidget(name);
	}

	//! @param name a text widget name under the root
	//! @return the text widget, or null
	protected TextWidget FindText(string name)
	{
		return TextWidget.Cast(Find(name));
	}

	//! @param name a widget name under the root
	//! @param handler the handler type to find on it
	//! @return the handler, or null
	protected ScriptedWidgetComponent FindHandlerOn(string name, typename handler)
	{
		Widget w = Find(name);
		if (!w)
			return null;

		return ScriptedWidgetComponent.Cast(w.FindHandler(handler));
	}

	//! Back: close the overlay.
	//! @param button the back button
	protected void OnBackClicked(TBD_UIButton button)
	{
		Close();
	}
}
