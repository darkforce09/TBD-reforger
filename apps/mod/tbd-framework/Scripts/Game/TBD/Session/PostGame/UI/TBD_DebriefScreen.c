/**
 * @file TBD_DebriefScreen.c
 * @brief The DEBRIEF scoreboard overlay and the packed row format it reads.
 *
 * Role: packs scoreboard rows into one string for replication and unpacks them; shows the winner
 * and one row per player (name, faction, role, kills / deaths) sorted by kills (toggled high or
 * low first), then fewer deaths, then name.
 * Position: TBD_EndBanner.ApplyEndScreens opens it in the DEBRIEF stage and closes it on any other;
 * TBD_EndBanner.PackDebriefBoard packs rows from TBD_DebriefScoreboard.Fill on the server; the
 * packed string arrives through TBD_FrameworkManager.GetDebriefBoard, with the winner and reason.
 * State: the open root and live instance (static), and per instance the widgets and the sort
 * direction; client.
 * Invariants: an overlay widget, never a Chimera menu, so Esc cannot refuse a stage change; the
 * packed format is one row per TBD_WireCodec.LINE_SEP with kills, deaths, faction, role and name
 * separated by TBD_WireCodec.FIELD_SEP; field text never carries a separator; a line with fewer than
 * five fields is skipped.
 */

//! One scoreboard row, packed across replication as kills, deaths, faction, role, name.
class TBD_DebriefRow
{
	string m_sName; //!< player name
	string m_sFaction; //!< slot faction key; empty when unslotted
	string m_sRole; //!< slot role; empty when unslotted
	int m_iKills; //!< kills this round
	int m_iDeaths; //!< deaths this round; 0 or 1 under one life
}

//! The DEBRIEF scoreboard overlay, handler of the root widget of TBD_UILayouts.DEBRIEF_SCREEN.
class TBD_DebriefScreen : ScriptedWidgetComponent
{
	protected static Widget s_wRoot; //!< the open overlay root; null when closed
	protected static TBD_DebriefScreen s_Instance; //!< the attached handler; null when closed

	protected Widget m_wRoot; //!< this handler's root widget
	protected TextWidget m_wTitle; //!< "DEBRIEF"
	protected TextWidget m_wSubtitle; //!< winner line, or "Scoreboard"
	protected TextWidget m_wStatus; //!< sort direction line
	protected TBD_ListBox m_List; //!< the scoreboard rows
	protected TBD_UIButton m_BackAction; //!< closes the overlay
	protected TBD_UIButton m_PrimaryAction; //!< toggles the sort direction
	protected bool m_bKillsDescending = true; //!< most kills first; default true

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

		Widget root = TBD_UILayouts.Create(TBD_UILayouts.DEBRIEF_SCREEN, null);
		if (!root)
		{
			Print("[TBD][ui] DEBRIEF_SCREEN layout did not instantiate", LogLevel.ERROR);
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

	//! Pack rows into the replicated string; null rows are skipped.
	//! @param rows the scoreboard rows
	//! @return one line per row: kills, deaths, faction, role, name, separated by TBD_WireCodec.FIELD_SEP
	static string PackRows(notnull array<ref TBD_DebriefRow> rows)
	{
		string packed;
		foreach (TBD_DebriefRow row : rows)
		{
			if (!row)
				continue;

			if (!packed.IsEmpty())
				packed += TBD_WireCodec.LINE_SEP;

			packed += string.Format("%1%2%3%2%4%2%5%2%6",
				row.m_iKills, TBD_WireCodec.FIELD_SEP, row.m_iDeaths, SanitizeField(row.m_sFaction),
				SanitizeField(row.m_sRole), SanitizeField(row.m_sName));
		}

		return packed;
	}

	//! Unpack a string written by PackRows. Lines with fewer than five fields are skipped; fields
	//! past the fifth are joined back into the name with spaces.
	//! @param packed the packed string; empty yields no rows
	//! @param outRows cleared, then filled
	static void UnpackRows(string packed, notnull array<ref TBD_DebriefRow> outRows)
	{
		outRows.Clear();
		if (packed.IsEmpty())
			return;

		array<string> lines = {};
		packed.Split(TBD_WireCodec.LINE_SEP, lines, false);
		foreach (string line : lines)
		{
			if (line.IsEmpty())
				continue;

			array<string> cols = {};
			line.Split(TBD_WireCodec.FIELD_SEP, cols, false);
			if (cols.Count() < 5)
				continue;

			TBD_DebriefRow row = new TBD_DebriefRow();
			row.m_iKills = cols[0].ToInt();
			row.m_iDeaths = cols[1].ToInt();
			row.m_sFaction = cols[2];
			row.m_sRole = cols[3];
			row.m_sName = cols[4];
			for (int i = 5; i < cols.Count(); i++)
			{
				row.m_sName += " ";
				row.m_sName += cols[i];
			}

			outRows.Insert(row);
		}
	}

	//! Replace tabs and newlines with spaces, so a field never splits a record.
	//! @param value the field text
	//! @return the cleaned text
	protected static string SanitizeField(string value)
	{
		string cleaned = string.Format("%1", value);
		cleaned.Replace("\t", " ");
		cleaned.Replace("\n", " ");
		return cleaned;
	}

	//! Bind the widgets and actions, paint the overlay and populate it.
	//! @param w the overlay root widget
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);

		m_wRoot = w;
		s_Instance = this;
		s_wRoot = w;
		m_bKillsDescending = true;

		m_wTitle = FindText("Title");
		m_wSubtitle = FindText("Subtitle");
		m_wStatus = FindText("Status");
		m_List = TBD_ListBox.Cast(FindHandlerOn("List", TBD_ListBox));
		m_BackAction = TBD_UIButton.Cast(FindHandlerOn("BackAction", TBD_UIButton));
		m_PrimaryAction = TBD_UIButton.Cast(FindHandlerOn("PrimaryAction", TBD_UIButton));

		TBD_UITheme.PaintAlpha(Find("Backdrop"), TBD_UITheme.SCRIM);
		TBD_UITheme.Paint(Find("Panel"), TBD_UITheme.SURFACE);
		TBD_UITheme.Paint(Find("HeaderRule"), TBD_UITheme.OUTLINE_VARIANT);
		TBD_UITheme.Paint(Find("FooterRule"), TBD_UITheme.OUTLINE_VARIANT);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wSubtitle, TBD_UITheme.ON_SURFACE_VARIANT);
		TBD_UITheme.Paint(m_wStatus, TBD_UITheme.ON_SURFACE_VARIANT);

		if (m_BackAction)
			m_BackAction.GetOnActivate().Insert(OnBackClicked);

		if (m_PrimaryAction)
			m_PrimaryAction.GetOnActivate().Insert(OnSortClicked);

		if (m_List)
			m_List.GetOnActivate().Insert(OnRowClicked);

		Populate();
	}

	//! Unbind the actions and clear the statics when they point here.
	//! @param w the overlay root widget
	override void HandlerDeattached(Widget w)
	{
		if (m_BackAction)
			m_BackAction.GetOnActivate().Remove(OnBackClicked);

		if (m_PrimaryAction)
			m_PrimaryAction.GetOnActivate().Remove(OnSortClicked);

		if (m_List)
			m_List.GetOnActivate().Remove(OnRowClicked);

		if (s_Instance == this)
			s_Instance = null;
		if (s_wRoot == w)
			s_wRoot = null;

		super.HandlerDeattached(w);
	}

	//! Fill the title, the winner subtitle, the sorted rows, the sort action and the status line
	//! from TBD_FrameworkManager's end winner and packed board.
	protected void Populate()
	{
		TBD_UITheme.Write(m_wTitle, "DEBRIEF");

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		string winner;
		string reason;
		string packed;
		if (fm)
		{
			winner = fm.GetEndWinner();
			reason = fm.GetEndReason();
			packed = fm.GetDebriefBoard();
		}

		string subtitle = "Scoreboard";
		if (!winner.IsEmpty())
			subtitle = string.Format("Winner: %1", winner);
		TBD_UITheme.Write(m_wSubtitle, subtitle);
		TBD_UITheme.Show(m_wSubtitle, true);

		array<ref TBD_DebriefRow> rows = {};
		UnpackRows(packed, rows);
		SortByKills(rows);

		if (!m_List)
			return;

		array<ref TBD_ListRowData> listRows = {};
		listRows.Insert(new TBD_ListRowData("PLAYER", "KILLS / DEATHS", -2, TBD_EUIState.NORMAL, true, true));

		foreach (TBD_DebriefRow row : rows)
		{
			if (!row)
				continue;

			string title = row.m_sName;
			if (!row.m_sFaction.IsEmpty())
				title = string.Format("%1  [%2]", row.m_sName, row.m_sFaction);
			if (!row.m_sRole.IsEmpty())
				title = string.Format("%1  %2", title, row.m_sRole);

			string detail = string.Format("%1 / %2", row.m_iKills, row.m_iDeaths);
			listRows.Insert(new TBD_ListRowData(title, detail, 0, TBD_EUIState.NORMAL, true, false));
		}

		m_List.SetRows(listRows);

		if (m_PrimaryAction)
		{
			m_PrimaryAction.SetLabel("SORT BY KILLS");
			m_PrimaryAction.SetInteractive(true);
			Widget actionWidget = m_PrimaryAction.GetRootWidget();
			TBD_UITheme.Show(actionWidget, true);
		}

		string status = "Sorted by kills (high first). Next stage closes this screen.";
		if (!m_bKillsDescending)
			status = "Sorted by kills (low first). Next stage closes this screen.";
		TBD_UITheme.Write(m_wStatus, status);
		TBD_UITheme.Show(m_wStatus, true);
	}

	//! Sort rows in place with KillsSortBefore, in the current direction.
	//! @param rows the rows to sort
	protected void SortByKills(notnull array<ref TBD_DebriefRow> rows)
	{
		int n = rows.Count();
		for (int i = 0; i < n; i++)
		{
			for (int j = i + 1; j < n; j++)
			{
				if (KillsSortBefore(rows[j], rows[i], m_bKillsDescending))
				{
					TBD_DebriefRow tmp = rows[i];
					rows[i] = rows[j];
					rows[j] = tmp;
				}
			}
		}
	}

	//! Order two rows: kills in the given direction, then fewer deaths, then name. A null row sorts last.
	//! @param a the first row
	//! @param b the second row
	//! @param descending most kills first when true
	//! @return true when a sorts before b
	protected bool KillsSortBefore(TBD_DebriefRow a, TBD_DebriefRow b, bool descending)
	{
		if (!a)
			return false;
		if (!b)
			return true;

		if (a.m_iKills != b.m_iKills)
		{
			if (descending)
				return a.m_iKills > b.m_iKills;
			return a.m_iKills < b.m_iKills;
		}

		if (a.m_iDeaths != b.m_iDeaths)
			return a.m_iDeaths < b.m_iDeaths;

		return a.m_sName < b.m_sName;
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

	//! Primary action: flip the sort direction and repopulate.
	//! @param button the primary button
	protected void OnSortClicked(TBD_UIButton button)
	{
		m_bKillsDescending = !m_bKillsDescending;
		Populate();
	}

	//! A click on the header row (tag -2) flips the sort direction; player rows do nothing.
	//! @param list the scoreboard list
	//! @param tag the clicked row's tag
	protected void OnRowClicked(TBD_ListBox list, int tag)
	{
		if (tag == -2)
		{
			m_bKillsDescending = !m_bKillsDescending;
			Populate();
		}
	}
}
