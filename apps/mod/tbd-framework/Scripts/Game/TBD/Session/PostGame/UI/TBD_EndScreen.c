//! T-941.3 - END stage banner. Winning faction + reason, as a workspace overlay.
//!
//! Not a Chimera menu: this slice does not own `chimeraMenus.conf`, and a menu that can swallow
//! Esc must never be able to refuse a stage change. `TBD_FrameworkManager.ApplyEndScreens` opens
//! this on END and closes it on every other stage; `Open`/`Close` cannot feed back into SetStage.
class TBD_EndScreen : ScriptedWidgetComponent
{
	protected static Widget s_wRoot;
	protected static TBD_EndScreen s_Instance;

	protected Widget m_wRoot;
	protected TextWidget m_wTitle;
	protected TextWidget m_wSubtitle;
	protected TextWidget m_wWinner;
	protected TextWidget m_wReason;
	protected TextWidget m_wStatus;
	protected TBD_UIButton m_BackAction;

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	static void Close()
	{
		if (!s_wRoot)
			return;

		s_wRoot.RemoveFromHierarchy();
		s_wRoot = null;
		s_Instance = null;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsOpen()
	{
		return s_wRoot != null;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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
	protected ScriptedWidgetComponent FindHandlerOn(string name, typename handler)
	{
		Widget w = Find(name);
		if (!w)
			return null;

		return ScriptedWidgetComponent.Cast(w.FindHandler(handler));
	}

	//------------------------------------------------------------------------------------------------
	protected void OnBackClicked(TBD_UIButton button)
	{
		Close();
	}
}
