/**
 * @file TBD_MissionInspectorCards.c
 * @brief The four cards of the mission inspector: modset and mods, summary, ORBAT and objectives.
 *
 * Role: mounts the four card panels once and rebuilds their bodies for each mission from Common
 * primitives (TBD_Columns2, TBD_ModGridItem, TBD_InsetText, tinted TBD_Panel with
 * TBD_FactionColumn, TBD_KeyValueRow).  Position: TBD_MissionInspectorPanel creates it on the
 * inspector's `CardsContent` widget and calls Fill when the selection changes; data comes from
 * TBD_MissionSummary.
 * State: the card panels and badge chips, and the ground of the faction column built last; owned
 * by the inspector panel on the client.  Invariants: a card whose panel failed to mount is skipped;
 * bodies are cleared before each rebuild, because selection is user-paced and rebuild-not-pool is
 * the cost model here.
 */

//! Builds and fills the inspector's four cards.
class TBD_MissionInspectorCards : Managed
{
	static const int CARD_GAP = 12; //!< px below each card
	static const int CARD_INSET = 14; //!< px inset of a card body's content

	protected Widget m_wCardsContent; //!< the inspector's `CardsContent` widget; cards mount into it
	protected int m_iColumnGround; //!< ground of the faction column MountFactionColumn built last, for its rows
	protected TBD_PanelComponent m_ModsPanel; //!< REQUIRED MODSET & MODS card; null when it failed to mount
	protected TBD_ChipComponent m_ModsetChip; //!< modset name badge
	protected TBD_ChipComponent m_SyncedChip; //!< `N SYNCED & ACTIVE` badge
	protected TBD_PanelComponent m_SummaryPanel; //!< MISSION SUMMARY card
	protected TBD_PanelComponent m_OrbatPanel; //!< ORBAT OVERVIEW card
	protected TBD_ChipComponent m_OrbatChip; //!< `N SLOTS` badge
	protected TBD_PanelComponent m_ObjectivesPanel; //!< OBJECTIVES card
	protected TBD_ChipComponent m_ObjectivesChip; //!< `N ACTIVE` badge

	//! Mount the four cards into `cardsContent`.
	//! @param cardsContent the inspector's `CardsContent` widget; null mounts nothing
	void TBD_MissionInspectorCards(Widget cardsContent)
	{
		m_wCardsContent = cardsContent;
		BuildCards();
	}

	//! Rebuild every card body for `mission`, in card order.
	//! @param mission the mission to show; not null
	void Fill(notnull TBD_MissionSummary mission)
	{
		FillMods(mission);
		FillSummary(mission);
		FillOrbat(mission);
		FillObjectives(mission);
	}

	//! Mount the four card panels and their badge chips.
	protected void BuildCards()
	{
		m_ModsPanel = MountCard("Required Modset & Mods", "extension");
		if (m_ModsPanel)
		{
			m_ModsetChip = TBD_ChipComponent.Mount(m_ModsPanel.GetBadgeDock(), "", TBD_EUITint.PRIMARY, m_ModsPanel.GetGround());
			m_SyncedChip = TBD_ChipComponent.Mount(m_ModsPanel.GetBadgeDock(), "", TBD_EUITint.SUCCESS, m_ModsPanel.GetGround());
			if (m_SyncedChip)
			{
				m_SyncedChip.SetDotVisible(true);
				AlignableSlot.SetPadding(m_SyncedChip.GetRootWidget(), 8, 0, 0, 0);
			}
		}

		m_SummaryPanel = MountCard("Mission Summary", "notes");
		if (m_SummaryPanel)
			TBD_ChipComponent.Mount(m_SummaryPanel.GetBadgeDock(), "SITREP", TBD_EUITint.NEUTRAL, m_SummaryPanel.GetGround());

		m_OrbatPanel = MountCard("ORBAT Overview", "groups");
		if (m_OrbatPanel)
			m_OrbatChip = TBD_ChipComponent.Mount(m_OrbatPanel.GetBadgeDock(), "", TBD_EUITint.NEUTRAL, m_OrbatPanel.GetGround());

		m_ObjectivesPanel = MountCard("Objectives", "description");
		if (m_ObjectivesPanel)
			m_ObjectivesChip = TBD_ChipComponent.Mount(m_ObjectivesPanel.GetBadgeDock(), "", TBD_EUITint.NEUTRAL, m_ObjectivesPanel.GetGround());
	}

	//! Mount one card panel on the inspector's glass.
	//! @param title the card title
	//! @param icon the TBD_UIIcons key of its header icon
	//! @return the panel, or null when there is no content widget or the layout fails
	protected TBD_PanelComponent MountCard(string title, string icon)
	{
		if (!m_wCardsContent)
			return null;

		TBD_PanelComponent panel = TBD_PanelComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.PANEL, m_wCardsContent, TBD_PanelComponent));
		if (!panel)
			return null;

		Widget root = panel.GetRootWidget();
		AlignableSlot.SetHorizontalAlign(root, LayoutHorizontalAlign.Stretch);
		AlignableSlot.SetPadding(root, 0, 0, 0, CARD_GAP);

		// A card sits on the inspector's glass, not on the backdrop.
		panel.SetGround(TBD_UITheme.PanelGround());
		panel.SetTitle(title);
		panel.SetIcon(icon);
		return panel;
	}

	//! Rebuild the modset card: badges, then one grid item per required mod in two columns, marked
	//! synced or missing.
	protected void FillMods(TBD_MissionSummary mission)
	{
		if (!m_ModsPanel)
			return;

		if (m_ModsetChip)
		{
			m_ModsetChip.SetText(mission.m_sModsetName);
			m_ModsetChip.SetChipVisible(!mission.m_sModsetName.IsEmpty());
		}

		if (m_SyncedChip)
			m_SyncedChip.SetText(string.Format("%1 SYNCED & ACTIVE", mission.GetSyncedModCount()));

		Widget body = m_ModsPanel.GetBodyDock();
		TBD_UILayouts.Clear(body);

		Widget columns = MountColumns(body);
		if (!columns)
			return;

		Widget columnA = columns.FindAnyWidget("ColumnA");
		Widget columnB = columns.FindAnyWidget("ColumnB");

		for (int i = 0; i < mission.m_aMods.Count(); i++)
		{
			TBD_MissionMod mod = mission.m_aMods[i];

			Widget column = columnA;
			if (i % 2 == 1)
				column = columnB;

			Widget item = TBD_UILayouts.CreateStretched(TBD_UILayouts.MISSION_SELECTOR_MOD_ITEM, column);
			if (!item)
				continue;

			int cardGround = m_ModsPanel.GetGround();
			int itemGround = TBD_UITheme.Over(TBD_UITheme.INSET_FILL, cardGround);
			Widget itemBorder = item.FindAnyWidget("Border");
			Widget itemBG = item.FindAnyWidget("Background");
			TBD_UILayouts.MountRounded(itemBorder, TBD_UITheme.RADIUS_ROW);
			TBD_UILayouts.MountRounded(itemBG, TBD_UITheme.RADIUS_ROW - 1);
			TBD_UITheme.PaintOver(itemBorder, TBD_UITheme.GLASS_BORDER, cardGround);
			TBD_UITheme.PaintOver(itemBG, TBD_UITheme.INSET_FILL, cardGround);
			TBD_UITheme.Write(TextWidget.Cast(item.FindAnyWidget("ModName")), mod.m_sName);
			TBD_UITheme.Paint(item.FindAnyWidget("ModName"), TBD_UITheme.ON_SURFACE);

			int ink = TBD_UITheme.SUCCESS;
			string icon = "check_circle";
			string glyph = "+";
			if (!mod.m_bSynced)
			{
				ink = TBD_UITheme.WARNING;
				icon = "cancel";
				glyph = "!";
			}

			ImageWidget check = ImageWidget.Cast(item.FindAnyWidget("CheckIcon"));
			bool iconShown = TBD_UIIcons.Load(check, icon);
			TBD_UITheme.Paint(check, ink);
			TextWidget checkGlyph = TextWidget.Cast(item.FindAnyWidget("CheckGlyph"));
			TBD_UITheme.Show(checkGlyph, !iconShown);
			TBD_UITheme.Write(checkGlyph, glyph);
			TBD_UITheme.Paint(checkGlyph, ink);

			TBD_ChipComponent.Mount(item.FindAnyWidget("VersionChipDock"), mod.m_sVersion, TBD_EUITint.NEUTRAL, itemGround);
		}
	}

	//! Rebuild the summary card: the SITREP text in an inset.
	protected void FillSummary(TBD_MissionSummary mission)
	{
		if (!m_SummaryPanel)
			return;

		Widget body = m_SummaryPanel.GetBodyDock();
		TBD_UILayouts.Clear(body);

		Widget inset = TBD_UILayouts.CreateStretched(TBD_UILayouts.INSET_TEXT, body);
		if (!inset)
			return;

		AlignableSlot.SetPadding(inset, CARD_INSET, CARD_INSET, CARD_INSET, CARD_INSET);
		Widget insetBorder = inset.FindAnyWidget("InsetBorder");
		Widget insetBG = inset.FindAnyWidget("InsetBG");
		TBD_UILayouts.MountRounded(insetBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(insetBG, TBD_UITheme.RADIUS_ROW - 1);
		TBD_UITheme.PaintOver(insetBorder, TBD_UITheme.GLASS_BORDER, m_SummaryPanel.GetGround());
		TBD_UITheme.PaintOver(insetBG, TBD_UITheme.INSET_FILL, m_SummaryPanel.GetGround());

		TextWidget text = TextWidget.Cast(inset.FindAnyWidget("Body"));
		TBD_UITheme.Write(text, mission.m_sSummary);
		TBD_UITheme.Paint(text, TBD_UITheme.ChipInk(TBD_EUITint.NEUTRAL));
	}

	//! Rebuild the ORBAT card: the slot badge and one faction column of counted vehicles per faction.
	protected void FillOrbat(TBD_MissionSummary mission)
	{
		if (!m_OrbatPanel)
			return;

		if (m_OrbatChip)
			m_OrbatChip.SetText(string.Format("%1 SLOTS", mission.GetSlotTotal()));

		Widget body = m_OrbatPanel.GetBodyDock();
		TBD_UILayouts.Clear(body);

		Widget columns = MountColumns(body);
		if (!columns)
			return;

		for (int i = 0; i < mission.m_aFactions.Count(); i++)
		{
			TBD_MissionFactionSummary faction = mission.m_aFactions[i];
			Widget rows = MountFactionColumn(columns, i, faction, faction.m_iSlots, "SLOTS", m_OrbatPanel.GetGround());
			if (!rows)
				continue;

			foreach (TBD_MissionAsset asset : faction.m_aVehicles)
			{
				TBD_KeyValueRowComponent row = TBD_KeyValueRowComponent.Mount(rows);
				if (!row)
					continue;

				row.SetGround(m_iColumnGround);
				row.Set(asset.m_sName, string.Empty);
				row.SetKeyChip(string.Format("%1x", asset.m_iCount), faction.m_eTint);
			}
		}
	}

	//! Rebuild the objectives card: the active badge and one faction column of objectives per faction.
	protected void FillObjectives(TBD_MissionSummary mission)
	{
		if (!m_ObjectivesPanel)
			return;

		if (m_ObjectivesChip)
			m_ObjectivesChip.SetText(string.Format("%1 ACTIVE", mission.GetObjectiveTotal()));

		Widget body = m_ObjectivesPanel.GetBodyDock();
		TBD_UILayouts.Clear(body);

		Widget columns = MountColumns(body);
		if (!columns)
			return;

		for (int i = 0; i < mission.m_aFactions.Count(); i++)
		{
			TBD_MissionFactionSummary faction = mission.m_aFactions[i];
			Widget rows = MountFactionColumn(columns, i, faction, faction.m_aObjectives.Count(), "OBJECTIVES", m_ObjectivesPanel.GetGround());
			if (!rows)
				continue;

			foreach (TBD_MissionObjective objective : faction.m_aObjectives)
			{
				TBD_KeyValueRowComponent row = TBD_KeyValueRowComponent.Mount(rows);
				if (!row)
					continue;

				row.SetGround(m_iColumnGround);
				row.Set(objective.m_sTitle, string.Empty);
				row.SetIcon(objective.m_sIcon, TBD_UITheme.ChipInk(faction.m_eTint));
			}
		}
	}


	//! Mount a two-column grid inset inside a card body.
	//! @return the grid, or null when the layout fails
	protected Widget MountColumns(Widget body)
	{
		Widget columns = TBD_UILayouts.CreateStretched(TBD_UILayouts.COLUMNS_2, body);
		if (columns)
			AlignableSlot.SetPadding(columns, CARD_INSET, CARD_INSET, CARD_INSET, CARD_INSET - 8);

		return columns;
	}

	//! A faction-tinted panel with a `TBD_FactionColumn` body in column `index % 2`. Returns the
	//! `Rows` container to fill, or null. `ground` is the owning card's fill; the column's own
	//! composited fill is left in `m_iColumnGround` for the rows the caller adds.
	protected Widget MountFactionColumn(Widget columns, int index, TBD_MissionFactionSummary faction, int count, string countLabel, int ground)
	{
		string columnName = "ColumnA";
		if (index % 2 == 1)
			columnName = "ColumnB";

		Widget column = columns.FindAnyWidget(columnName);
		if (!column)
			return null;

		TBD_PanelComponent panel = TBD_PanelComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.PANEL, column, TBD_PanelComponent));
		if (!panel)
			return null;

		AlignableSlot.SetHorizontalAlign(panel.GetRootWidget(), LayoutHorizontalAlign.Stretch);
		panel.ShowHeader(false);
		panel.SetGround(ground);
		panel.SetTint(faction.m_eTint);
		m_iColumnGround = panel.GetGround();

		Widget body = TBD_UILayouts.CreateStretched(TBD_UILayouts.MISSION_SELECTOR_FACTION_COL, panel.GetBodyDock());
		if (!body)
			return null;

		TBD_ChipComponent.Mount(body.FindAnyWidget("FactionChipDock"), faction.m_sKey, faction.m_eTint, m_iColumnGround);
		TBD_UITheme.Write(TextWidget.Cast(body.FindAnyWidget("RoleText")), faction.m_sRole);
		TBD_UITheme.Paint(body.FindAnyWidget("RoleText"), TBD_UITheme.ChipInk(TBD_EUITint.NEUTRAL));
		TBD_UITheme.Write(TextWidget.Cast(body.FindAnyWidget("CountText")), count.ToString());
		TBD_UITheme.Paint(body.FindAnyWidget("CountText"), TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.Write(TextWidget.Cast(body.FindAnyWidget("CountLabel")), countLabel);
		TBD_UITheme.Paint(body.FindAnyWidget("CountLabel"), TBD_UITheme.MUTED_INK);
		TBD_UITheme.PaintOver(body.FindAnyWidget("HeaderRule"), TBD_UITheme.PanelBorder(faction.m_eTint), m_iColumnGround);

		return body.FindAnyWidget("Rows");
	}
}
