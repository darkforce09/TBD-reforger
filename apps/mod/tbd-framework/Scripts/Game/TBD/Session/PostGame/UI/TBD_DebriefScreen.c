//! T-941.3 - one scoreboard row. Packed across replication as kills/deaths/faction/role/name.
class TBD_DebriefRow
{
	string m_sName;
	string m_sFaction;
	string m_sRole;
	int m_iKills;
	int m_iDeaths;
}

//! T-941.3 - DEBRIEF scoreboard overlay. Rows come from TBD_ResultsReporter.FillScoreboard
//! (deaths = the reporter's ONE LIFE counter; kills counted on TBD_FrameworkManager because
//! the reporter still omits kill tracking - T-181.13.1). Sorted by kills, then deaths, then name.
//!
//! Same overlay rules as TBD_EndScreen: never a Chimera menu, never able to refuse SetStage.
class TBD_DebriefScreen : ScriptedWidgetComponent
{
	protected static const string LINE_SEP = "\n";
	protected static const string FIELD_SEP = "\t";

	protected static Widget s_wRoot;
	protected static TBD_DebriefScreen s_Instance;

	protected Widget m_wRoot;
	protected TextWidget m_wTitle;
	protected TextWidget m_wSubtitle;
	protected TextWidget m_wStatus;
	protected TBD_ListBox m_List;
	protected TBD_UIButton m_BackAction;
	protected TBD_UIButton m_PrimaryAction;
	protected bool m_bKillsDescending = true;

	//------------------------------------------------------------------------------------------------
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
	static string PackRows(notnull array<ref TBD_DebriefRow> rows)
	{
		string packed;
		foreach (TBD_DebriefRow row : rows)
		{
			if (!row)
				continue;

			if (!packed.IsEmpty())
				packed += LINE_SEP;

			packed += string.Format("%1%2%3%2%4%2%5%2%6",
				row.m_iKills, FIELD_SEP, row.m_iDeaths, SanitizeField(row.m_sFaction),
				SanitizeField(row.m_sRole), SanitizeField(row.m_sName));
		}

		return packed;
	}

	//------------------------------------------------------------------------------------------------
	static void UnpackRows(string packed, notnull array<ref TBD_DebriefRow> outRows)
	{
		outRows.Clear();
		if (packed.IsEmpty())
			return;

		array<string> lines = {};
		packed.Split(LINE_SEP, lines, false);
		foreach (string line : lines)
		{
			if (line.IsEmpty())
				continue;

			array<string> cols = {};
			line.Split(FIELD_SEP, cols, false);
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

	//------------------------------------------------------------------------------------------------
	protected static string SanitizeField(string value)
	{
		string cleaned = string.Format("%1", value);
		cleaned.Replace("\t", " ");
		cleaned.Replace("\n", " ");
		return cleaned;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected void OnSortClicked(TBD_UIButton button)
	{
		m_bKillsDescending = !m_bKillsDescending;
		Populate();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnRowClicked(TBD_ListBox list, int tag)
	{
		if (tag == -2)
		{
			m_bKillsDescending = !m_bKillsDescending;
			Populate();
		}
	}
}

//! T-941.3 - public scoreboard snapshot. Lives as a modded method so T-940.4 can keep owning
//! TBD_ResultsReporter.c. Deaths are the reporter's ONE LIFE counter (SpawnManager.IsPlayerDead).
//! Kills are the live map TBD_FrameworkManager owns; the reporter still does not track them.
modded class TBD_ResultsReporter
{
	//------------------------------------------------------------------------------------------------
	static void FillScoreboard(notnull array<ref TBD_DebriefRow> outRows)
	{
		outRows.Clear();

		if (RplSession.Mode() == RplMode.Client)
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!players)
			return;

		array<int> ids = {};
		int count = players.GetPlayers(ids);
		for (int i = 0; i < count; i++)
		{
			int playerId = ids[i];
			TBD_DebriefRow row = new TBD_DebriefRow();
			row.m_sName = players.GetPlayerName(playerId);
			if (row.m_sName.IsEmpty())
				row.m_sName = string.Format("Player %1", playerId);

			TBD_MissionSlotStruct slot;
			if (sm)
				slot = sm.GetAssignedSlot(playerId);
			if (slot)
			{
				row.m_sFaction = slot.faction;
				row.m_sRole = slot.role;
			}

			row.m_iDeaths = 0;
			if (sm && sm.IsPlayerDead(playerId))
				row.m_iDeaths = 1;

			row.m_iKills = 0;
			if (fm)
				row.m_iKills = fm.GetKills(playerId);

			outRows.Insert(row);
		}
	}
}
