//! T-941.4 — objective list + capture bar. Replaces the per-tick chat pump.
//!
//! Layout is `TBD_UILayouts.OBJECTIVE_HUD` (UI reorg 2026-09-12; used to be pinned here). Transport hangs off SCR_PlayerController so Owner RPC delivers
//! one snapshot per client, the same pattern TBD_TaskHud uses.

class TBD_ObjectiveHud : ScriptedWidgetComponent
{
	static const ResourceName LAYOUT = TBD_UILayouts.OBJECTIVE_HUD;

	//! Matches CaptureFill SizeX in TBD_ObjectiveHud.layout (C3: 328).
	static const float BAR_WIDTH = 328.0;
	static const float BAR_HEIGHT = 10.0;

	protected static Widget s_wRoot;
	protected static TBD_ObjectiveHud s_Instance;

	protected static ref array<string> s_aIcons;
	protected static ref array<string> s_aTitles;
	protected static ref array<string> s_aDetails;
	protected static string s_sBarLabel;
	protected static int s_iBarPercent;
	protected static int s_iBarVisible;
	protected static string s_sLastSignature;

	protected Widget m_wRoot;
	protected Widget m_wPanel;
	protected Widget m_wCaptureBar;
	protected Widget m_wCaptureFill;
	protected TextWidget m_wTitle;
	protected TextWidget m_wCaptureLabel;
	protected TBD_ListBox m_List;

	//------------------------------------------------------------------------------------------------
	static void Open()
	{
		if (s_wRoot)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return;

		Widget root = TBD_UILayouts.Create(LAYOUT, null);
		if (!root)
		{
			Print("[TBD][ui] OBJECTIVE_HUD layout did not instantiate", LogLevel.ERROR);
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
		s_sLastSignature = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsOpen()
	{
		return s_wRoot != null;
	}

	//------------------------------------------------------------------------------------------------
	//! Apply one replicated snapshot. `show == 0` closes the HUD (stage left LIVE).
	static void Accept(array<string> icons, array<string> titles, array<string> details,
		string barLabel, int barPercent, int barVisible, int show)
	{
		if (show == 0)
		{
			Close();
			return;
		}

		s_aIcons = icons;
		s_aTitles = titles;
		s_aDetails = details;
		s_sBarLabel = barLabel;
		s_iBarPercent = barPercent;
		s_iBarVisible = barVisible;

		if (!s_wRoot)
			Open();

		if (s_Instance)
			s_Instance.PaintPending();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);

		m_wRoot = w;
		s_Instance = this;
		s_wRoot = w;

		m_wPanel = Find("Panel");
		m_wTitle = FindText("Title");
		m_wCaptureLabel = FindText("CaptureLabel");
		m_wCaptureBar = Find("CaptureBar");
		m_wCaptureFill = Find("CaptureFill");
		m_List = TBD_ListBox.Cast(FindHandlerOn("List", TBD_ListBox));

		TBD_UITheme.PaintAlpha(m_wPanel, TBD_UITheme.SURFACE_GLASS);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wCaptureLabel, TBD_UITheme.ON_SURFACE_VARIANT);
		TBD_UITheme.Paint(Find("CaptureTrack"), TBD_UITheme.SURFACE_CONTAINER_HIGH);
		TBD_UITheme.Paint(m_wCaptureFill, TBD_UITheme.ACTION);
		TBD_UITheme.Write(m_wTitle, "OBJECTIVES");

		PaintPending();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		if (s_Instance == this)
			s_Instance = null;
		if (s_wRoot == w)
			s_wRoot = null;

		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	protected void PaintPending()
	{
		string signature = SnapshotSignature();
		if (signature == s_sLastSignature && m_List)
			return;

		s_sLastSignature = signature;

		PaintList();
		PaintBar();
	}

	//------------------------------------------------------------------------------------------------
	protected void PaintList()
	{
		if (!m_List)
			return;

		m_List.BeginUpdate();

		if (s_aTitles)
		{
			int count = s_aTitles.Count();
			for (int i = 0; i < count; i++)
			{
				string title = s_aTitles[i];
				string detail;
				if (s_aDetails && s_aDetails.IsIndexValid(i))
					detail = s_aDetails[i];

				string icon;
				if (s_aIcons && s_aIcons.IsIndexValid(i))
					icon = s_aIcons[i];

				string labelled = "[";
				labelled += icon;
				labelled += "] ";
				labelled += title;

				m_List.AddItem(labelled, detail, i, StateForIcon(icon), false);
			}
		}

		m_List.EndUpdate();
	}

	//------------------------------------------------------------------------------------------------
	protected void PaintBar()
	{
		bool show = s_iBarVisible != 0;
		TBD_UITheme.Show(m_wCaptureBar, show);
		TBD_UITheme.Show(m_wCaptureFill, show);

		if (!show)
		{
			TBD_UITheme.Write(m_wCaptureLabel, "");
			return;
		}

		int percent = s_iBarPercent;
		if (percent < 0)
			percent = 0;
		if (percent > 100)
			percent = 100;

		string label = s_sBarLabel;
		label += "  ";
		label += percent.ToString();
		label += "%";
		TBD_UITheme.Write(m_wCaptureLabel, label);

		if (!m_wCaptureFill)
			return;

		float width = BAR_WIDTH * (percent / 100.0);
		if (width < 1.0)
			width = 1.0;

		FrameSlot.SetSize(m_wCaptureFill, width, BAR_HEIGHT);
	}

	//------------------------------------------------------------------------------------------------
	protected TBD_EUIState StateForIcon(string icon)
	{
		if (icon == "!")
			return TBD_EUIState.DANGER;
		if (icon == "v" || icon == "+")
			return TBD_EUIState.ACTIVE;
		if (icon == "-")
			return TBD_EUIState.TAKEN;

		return TBD_EUIState.NORMAL;
	}

	//------------------------------------------------------------------------------------------------
	protected string SnapshotSignature()
	{
		string sig = s_sBarLabel;
		sig += ":";
		sig += s_iBarPercent.ToString();
		sig += ":";
		sig += s_iBarVisible.ToString();

		if (!s_aTitles)
			return sig;

		int count = s_aTitles.Count();
		for (int i = 0; i < count; i++)
		{
			sig += ";";
			if (s_aIcons && s_aIcons.IsIndexValid(i))
				sig += s_aIcons[i];
			sig += s_aTitles[i];
			if (s_aDetails && s_aDetails.IsIndexValid(i))
				sig += s_aDetails[i];
		}

		return sig;
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
}

//------------------------------------------------------------------------------------------------
modded class SCR_PlayerController
{
	//------------------------------------------------------------------------------------------------
	void TBD_RequestObjectiveHud()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_ObjectiveHud);
			return;
		}

		TBD_ObjectivesComponent runner = TBD_ObjectivesComponent.GetInstance();
		if (runner)
			runner.PushHudTo(GetPlayerId());
	}

	//------------------------------------------------------------------------------------------------
	void TBD_PushObjectiveHud(array<string> icons, array<string> titles, array<string> details,
		string barLabel, int barPercent, int barVisible, int show)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (GetGame().GetPlayerController() == this)
		{
			TBD_ObjectiveHud.Accept(icons, titles, details, barLabel, barPercent, barVisible, show);
			return;
		}

		Rpc(TBD_RpcDo_ObjectiveHud, icons, titles, details, barLabel, barPercent, barVisible, show);
	}

	//------------------------------------------------------------------------------------------------
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ObjectiveHud()
	{
		TBD_ObjectivesComponent runner = TBD_ObjectivesComponent.GetInstance();
		if (runner)
			runner.PushHudTo(GetPlayerId());
	}

	//------------------------------------------------------------------------------------------------
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_ObjectiveHud(array<string> icons, array<string> titles,
		array<string> details, string barLabel, int barPercent, int barVisible, int show)
	{
		TBD_ObjectiveHud.Accept(icons, titles, details, barLabel, barPercent, barVisible, show);
	}
}
