/**
 * @file TBD_ObjectiveHud.c
 * @brief The live-round objective list and capture bar.
 *
 * Role: handler of `TBD_UILayouts.OBJECTIVE_HUD`; paints one objective snapshot (icon, title and
 * detail per row, a capture bar) and closes when the snapshot says the stage left LIVE.
 * Position: fed by `TBD_ObjectiveHudPublisher` through `SCR_PlayerController.TBD_PushObjectiveHud`
 * (this folder's `SCR_PlayerController.c`), one snapshot per client over an Owner RPC.
 * State: the static root, instance and last snapshot with its signature, on the client.
 * Invariants: an unchanged snapshot does not repaint (signature check); the bar percent is clamped
 * to 0..100; a HUD never opens without a workspace.
 */

//! Objective HUD handler and its static snapshot.
class TBD_ObjectiveHud : ScriptedWidgetComponent
{
	static const ResourceName LAYOUT = TBD_UILayouts.OBJECTIVE_HUD; //!< the HUD layout resource

	static const float BAR_WIDTH = 328.0; //!< capture fill width at 100 %, reference px; matches CaptureFill SizeX in the layout
	static const float BAR_HEIGHT = 10.0; //!< capture fill height, reference px

	protected static Widget s_wRoot; //!< the open HUD root; null while closed
	protected static TBD_ObjectiveHud s_Instance; //!< the handler on the open root

	protected static ref array<string> s_aIcons; //!< per-row status glyph of the last snapshot (`!`, `v`, `+`, `-`)
	protected static ref array<string> s_aTitles; //!< per-row objective title of the last snapshot
	protected static ref array<string> s_aDetails; //!< per-row detail text of the last snapshot
	protected static string s_sBarLabel; //!< capture bar label of the last snapshot
	protected static int s_iBarPercent; //!< capture progress of the last snapshot, percent
	protected static int s_iBarVisible; //!< 1 shows the capture bar, 0 hides it
	protected static string s_sLastSignature; //!< signature of the last painted snapshot; empty forces a repaint

	protected Widget m_wRoot; //!< the widget this handler sits on
	protected Widget m_wPanel; //!< `Panel`, painted with real alpha over the world
	protected Widget m_wCaptureBar; //!< `CaptureBar`
	protected Widget m_wCaptureFill; //!< `CaptureFill`, sized by percent
	protected TextWidget m_wTitle; //!< `Title`
	protected TextWidget m_wCaptureLabel; //!< `CaptureLabel`
	protected TBD_ListBox m_List; //!< the `List` of objectives

	//! Create the HUD layout under the workspace root; does nothing when open or without a workspace.
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

	//! Remove the HUD and forget the painted snapshot.
	static void Close()
	{
		if (!s_wRoot)
			return;

		s_wRoot.RemoveFromHierarchy();
		s_wRoot = null;
		s_Instance = null;
		s_sLastSignature = string.Empty;
	}

	//! @return true while the HUD is open
	static bool IsOpen()
	{
		return s_wRoot != null;
	}

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

	//! Bind the widgets, paint the chrome, title the panel and paint the pending snapshot.
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

	//! Forget this instance and root when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		if (s_Instance == this)
			s_Instance = null;
		if (s_wRoot == w)
			s_wRoot = null;

		super.HandlerDeattached(w);
	}

	//! Repaint list and bar when the snapshot signature changed (always on the first paint).
	protected void PaintPending()
	{
		string signature = SnapshotSignature();
		if (signature == s_sLastSignature && m_List)
			return;

		s_sLastSignature = signature;

		PaintList();
		PaintBar();
	}

	//! Rebuild the objective list, one `[glyph] title` row per objective, rows not clickable.
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

	//! Show or hide the capture bar; write `label  N%` and size the fill (at least 1 px).
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

	//! @return the row state for a status glyph: `!` danger, `v` or `+` active, `-` taken, else normal
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

	//! @return a string that changes whenever any field of the snapshot changes
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

	//! @return the widget `name` under the root, or null
	protected Widget Find(string name)
	{
		if (!m_wRoot)
			return null;

		return m_wRoot.FindAnyWidget(name);
	}

	//! @return the text widget `name` under the root, or null
	protected TextWidget FindText(string name)
	{
		return TextWidget.Cast(Find(name));
	}

	//! @return the handler of class `handler` on the widget `name`, or null
	protected ScriptedWidgetComponent FindHandlerOn(string name, typename handler)
	{
		Widget w = Find(name);
		if (!w)
			return null;

		return ScriptedWidgetComponent.Cast(w.FindHandler(handler));
	}
}
