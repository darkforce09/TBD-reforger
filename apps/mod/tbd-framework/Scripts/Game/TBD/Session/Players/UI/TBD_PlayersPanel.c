//! Briefing rebuild (2026-09-14) — the PLAYERS panel (`players_panel` mockup) as a briefing MODE:
//! press Players in the primary nav and it pops out directly right of it, the way Briefing and
//! Markers do (operator word) — no scrim, no window. Layout `Session/Shared/TBD_PlayersPanel.layout`
//! fills its dock: header (title · TOTAL) and four lanes — BLUFOR and OPFOR on top, Spectators and
//! Unslotted below — each a `TBD_PlayerLane`. Build into the host's `WideDock`; `Destroy` removes it.
class TBD_PlayersPanel : Managed
{
	protected Widget m_wRoot;
	protected ref array<ref TBD_PlayerLane> m_aLanes;

	//------------------------------------------------------------------------------------------------
	bool Build(Widget dock)
	{
		m_aLanes = {};
		if (!dock)
			return false;

		m_wRoot = TBD_UILayouts.Create(TBD_UILayouts.PLAYERS_PANEL, dock);
		if (!m_wRoot)
			return false;

		TBD_PlayersCatalog players = TBD_PlayersCatalog.Get();
		TBD_LobbyCatalog lobby = TBD_LobbyCatalog.Get();

		Widget border = m_wRoot.FindAnyWidget("WindowBorder");
		Widget background = m_wRoot.FindAnyWidget("WindowBG");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.PaintOver(border, TBD_UITheme.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(background, TBD_UITheme.PANEL_FILL, TBD_UITheme.Ground());
		int ground = TBD_UITheme.PanelGround();

		TBD_UITheme.Paint(m_wRoot.FindAnyWidget("Title"), TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.PaintOver(m_wRoot.FindAnyWidget("HeaderRule"), TBD_UITheme.STRIP_BORDER, ground);
		TextWidget total = TextWidget.Cast(m_wRoot.FindAnyWidget("TotalText"));
		TBD_UITheme.Write(total, string.Format("TOTAL: %1 PLAYERS", players.Total()));
		TBD_UITheme.Paint(total, TBD_UITheme.MUTED_INK);

		AddLane("BluforDock", "BLUFOR", RoleOf(lobby, "BLUFOR"), TBD_EUITint.BLUFOR, players.GetSlotted("BLUFOR"), "Slotted:", CountText(players.CountSlotted("BLUFOR"), players.Capacity("BLUFOR")));
		AddLane("OpforDock", "OPFOR", RoleOf(lobby, "OPFOR"), TBD_EUITint.OPFOR, players.GetSlotted("OPFOR"), "Slotted:", CountText(players.CountSlotted("OPFOR"), players.Capacity("OPFOR")));
		array<TBD_PlayerInfo> spectators = players.GetByState(TBD_EPlayerState.SPECTATOR);
		AddLane("SpectatorDock", "Spectators", "", TBD_EUITint.NEUTRAL, spectators, "Count:", spectators.Count().ToString());
		array<TBD_PlayerInfo> unslotted = players.GetByState(TBD_EPlayerState.UNSLOTTED);
		AddLane("UnslottedDock", "Unslotted", "", TBD_EUITint.NEUTRAL, unslotted, "Pending:", unslotted.Count().ToString());

		Print("[TBD][players] panel opened.");
		return true;
	}

	//------------------------------------------------------------------------------------------------
	void Destroy()
	{
		if (m_aLanes)
		{
			foreach (TBD_PlayerLane lane : m_aLanes)
			{
				if (lane)
					lane.Destroy();
			}
			m_aLanes.Clear();
		}

		if (m_wRoot)
			m_wRoot.RemoveFromHierarchy();

		m_wRoot = null;
		Print("[TBD][players] panel closed.");
	}

	//------------------------------------------------------------------------------------------------
	protected void AddLane(string dockName, string name, string role, TBD_EUITint tint, array<TBD_PlayerInfo> rows, string countLabel, string countText)
	{
		TBD_PlayerLane lane = new TBD_PlayerLane();
		if (lane.Build(m_wRoot.FindAnyWidget(dockName), name, role, tint, rows, countLabel, countText))
			m_aLanes.Insert(lane);
	}

	//------------------------------------------------------------------------------------------------
	protected static string RoleOf(TBD_LobbyCatalog lobby, string factionKey)
	{
		if (!lobby)
			return string.Empty;

		TBD_LobbyFactionInfo faction = lobby.GetFaction(factionKey);
		if (!faction)
			return string.Empty;

		string role = faction.m_sRoleLabel;
		role.ToLower();
		if (role.IsEmpty())
			return string.Empty;

		string first = role.Substring(0, 1);
		first.ToUpper();
		return first + role.Substring(1, role.Length() - 1);
	}

	//------------------------------------------------------------------------------------------------
	protected static string CountText(int slotted, int capacity)
	{
		if (capacity > 0)
			return string.Format("%1 / %2", slotted, capacity);

		return slotted.ToString();
	}
}

//! One lane of the panel: tinted header (name · role chip · count), column header, scrolling rows.
class TBD_PlayerLane : Managed
{
	protected Widget m_wRoot;
	protected ref TBD_ScrollList m_List;
	protected string m_sName;

	//------------------------------------------------------------------------------------------------
	bool Build(Widget dock, string name, string role, TBD_EUITint tint, array<TBD_PlayerInfo> rows, string countLabel, string countText)
	{
		if (!dock)
			return false;

		m_wRoot = TBD_UILayouts.Create(TBD_UILayouts.PLAYERS_LANE, dock);
		if (!m_wRoot)
			return false;

		Widget border = m_wRoot.FindAnyWidget("Border");
		Widget background = m_wRoot.FindAnyWidget("Background");
		Widget headerBG = m_wRoot.FindAnyWidget("HeaderBG");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UILayouts.MountRounded(headerBG, TBD_UITheme.RADIUS_PANEL - 1); // HeaderClip squares its bottom

		int panelGround = TBD_UITheme.PanelGround();
		int borderTone = TBD_UITheme.STRIP_BORDER;
		int laneFill = TBD_UITheme.SQUAD_CARD_FILL;
		int headerFill = TBD_UITheme.SQUAD_HEADER_FILL;
		int nameInk = TBD_UITheme.BRIGHT_INK;
		if (tint == TBD_EUITint.BLUFOR || tint == TBD_EUITint.OPFOR)
		{
			borderTone = TBD_UITheme.FactionRowBorder(tint);
			laneFill = TBD_UITheme.PanelFill(tint);
			headerFill = TBD_UITheme.FactionRowFill(tint);
			nameInk = TBD_UITheme.FactionRowInk(tint);
		}

		TBD_UITheme.PaintOver(border, borderTone, panelGround);
		TBD_UITheme.PaintOver(background, laneFill, panelGround);
		int laneGround = TBD_UITheme.Over(laneFill, panelGround);
		TBD_UITheme.PaintOver(headerBG, headerFill, laneGround);
		int headerGround = TBD_UITheme.Over(headerFill, laneGround);

		TextWidget nameText = TextWidget.Cast(m_wRoot.FindAnyWidget("NameText"));
		TBD_UITheme.Write(nameText, name);
		TBD_UITheme.Paint(nameText, nameInk);

		if (!role.IsEmpty())
			TBD_ChipComponent.Mount(m_wRoot.FindAnyWidget("RoleChipDock"), role, tint, headerGround);

		TBD_UITheme.Paint(m_wRoot.FindAnyWidget("CountLabel"), TBD_UITheme.MUTED_INK);
		TBD_UITheme.Write(TextWidget.Cast(m_wRoot.FindAnyWidget("CountLabel")), countLabel);
		TBD_ChipComponent count = TBD_ChipComponent.Mount(m_wRoot.FindAnyWidget("CountChipDock"), countText, tint, headerGround);
		if (count)
			count.SetUppercase(false);

		TBD_UITheme.Paint(m_wRoot.FindAnyWidget("ColIndex"), TBD_UITheme.DIM_INK);
		TBD_UITheme.Paint(m_wRoot.FindAnyWidget("ColPlayer"), TBD_UITheme.DIM_INK);
		TBD_UITheme.Paint(m_wRoot.FindAnyWidget("ColPing"), TBD_UITheme.DIM_INK);
		TBD_UITheme.PaintOver(m_wRoot.FindAnyWidget("ColumnRule"), TBD_UITheme.SLOT_RULE, laneGround);

		m_List = TBD_ScrollList.Mount(m_wRoot.FindAnyWidget("ListDock"), laneGround, 0);
		if (!m_List)
			return true;

		Widget content = m_List.GetContent();
		int created;
		foreach (int i, TBD_PlayerInfo player : rows)
		{
			if (AddRow(content, i + 1, player, laneGround))
				created++;
		}

		m_List.ResetScroll();

		// Diagnostic (MEASURED run 6: rows created, list 0x0 when measured at once) — measure after a
		// layout pass, every level from the dock down.
		m_sName = name;
		Print(string.Format("[TBD][players] lane %1: %2 rows asked, %3 created", name, rows.Count(), created));
		GetGame().GetCallqueue().CallLater(Measure, 250, false);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void Measure()
	{
		if (!m_wRoot || !m_List)
			return;

		Widget content = m_List.GetContent();
		Widget first;
		if (content)
			first = content.GetChildren();

		Print(string.Format("[TBD][players] %1 rects: lane %2 · dock %3 · list %4 · frame %5 · scroll %6 · content %7 · row %8",
			m_sName, Rect(m_wRoot), Rect(m_wRoot.FindAnyWidget("ListDock")), Rect(m_List.GetRoot()),
			Rect(m_List.GetRoot().FindAnyWidget("ListFrame")), Rect(m_List.GetRoot().FindAnyWidget("Scroll")), Rect(content), Rect(first)));
	}

	//------------------------------------------------------------------------------------------------
	protected static string Rect(Widget w)
	{
		if (!w)
			return "null";

		float x, y, sx, sy;
		w.GetScreenPos(x, y);
		w.GetScreenSize(sx, sy);
		return string.Format("%1,%2 %3x%4 vis=%5", Math.Round(x), Math.Round(y), Math.Round(sx), Math.Round(sy), w.IsVisible());
	}

	//------------------------------------------------------------------------------------------------
	protected bool AddRow(Widget content, int index, TBD_PlayerInfo player, int ground)
	{
		Widget row = TBD_UILayouts.CreateStretched(TBD_UILayouts.PLAYERS_ROW, content);
		if (!row)
			return false;

		string number = index.ToString();
		if (index < 10)
			number = "0" + number;

		TextWidget indexText = TextWidget.Cast(row.FindAnyWidget("Index"));
		TBD_UITheme.Write(indexText, number);
		TBD_UITheme.Paint(indexText, TBD_UITheme.DIM_INK);

		ImageWidget icon = ImageWidget.Cast(row.FindAnyWidget("Icon"));
		if (TBD_UIIcons.Load(icon, "person"))
			TBD_UITheme.Paint(icon, TBD_UITheme.MUTED_INK);

		TextWidget name = TextWidget.Cast(row.FindAnyWidget("Name"));
		TBD_UITheme.Write(name, player.m_sName);
		TBD_UITheme.Paint(name, TBD_UITheme.ON_SURFACE);

		if (!player.m_sTag.IsEmpty())
			TBD_ChipComponent.Mount(row.FindAnyWidget("TagChipDock"), player.m_sTag, TBD_EUITint.WARNING, ground);

		TextWidget ping = TextWidget.Cast(row.FindAnyWidget("Ping"));
		TBD_UITheme.Write(ping, string.Format("%1ms", player.m_iPing));
		TBD_EUITint pingTint = TBD_EUITint.SUCCESS;
		if (player.m_iPing >= 80)
			pingTint = TBD_EUITint.DANGER;
		else if (player.m_iPing >= 40)
			pingTint = TBD_EUITint.WARNING;
		TBD_UITheme.Paint(ping, TBD_UITheme.ChipInk(pingTint));

		TBD_UITheme.PaintOver(row.FindAnyWidget("Background"), TBD_UITheme.TRANSPARENT, ground);
		TBD_UITheme.PaintOver(row.FindAnyWidget("RowRule"), TBD_UITheme.SLOT_RULE, ground);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	void Destroy()
	{
		GetGame().GetCallqueue().Remove(Measure);
		if (m_List)
			m_List.Destroy();

		m_List = null;
		m_wRoot = null;
	}
}
