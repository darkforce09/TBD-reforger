/**
 * @file TBD_PlayerLane.c
 * @brief One section of the PLAYERS panel: a tinted header, a column header and scrolling player rows.
 *
 * Role: builds TBD_UILayouts.PLAYERS_LANE into a panel dock with the section name, an optional role
 * chip, a count chip, the column header and one row (index, icon, name, tag chip, ping) per player.
 * Position: TBD_PlayersPanel creates and destroys it; rows read TBD_PlayerInfo.
 * State: the section root widget and its scroll list, per instance, client.
 * Invariants: faction sections take the side tint, the others stay neutral; ping is green under 40 ms,
 * amber under 80 ms and red from 80 ms.
 */

//! One section of the panel: tinted header (name - role chip - count), column header, scrolling rows.
class TBD_PlayerLane : Managed
{
	protected Widget m_wRoot; //!< the section root inside its dock
	protected ref TBD_ScrollList m_List; //!< the scrolling row list; null when it did not mount

	//! Build the section into a dock and fill its rows.
	//! @param dock the section dock widget
	//! @param name the section title
	//! @param role the role chip text; empty for no chip
	//! @param tint the section tint
	//! @param rows the players of the section
	//! @param countLabel the label before the count chip
	//! @param countText the count chip text
	//! @return false when the dock is null or the layout will not load; true otherwise, even
	//! without a row list
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

		Print(string.Format("[TBD][players] lane %1: %2 rows asked, %3 created", name, rows.Count(), created));
		return true;
	}

	//! Add one player row: two-digit index, icon, name, optional tag chip and a tinted ping.
	//! @param content the scroll list content widget
	//! @param index the 1-based row number
	//! @param player the player
	//! @param ground the colour the row is painted over
	//! @return false when the row layout will not load
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
		// The rule's 1 px is the RowRuleSize SizeLayout in the layout: an untextured image in a
		// bottom-aligned overlay slot has no height of its own and would fill the whole row.
		TBD_UITheme.PaintOver(row.FindAnyWidget("RowRule"), TBD_UITheme.SLOT_RULE, ground);
		return true;
	}

	//! Destroy the row list and forget the widgets.
	void Destroy()
	{
		if (m_List)
			m_List.Destroy();

		m_List = null;
		m_wRoot = null;
	}
}
