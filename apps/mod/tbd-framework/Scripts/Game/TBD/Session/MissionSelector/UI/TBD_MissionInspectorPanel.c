//! Pre-game rebuild (2026-09-12) — the right column of the Mission Selector.
//!
//! ```
//!   ┌─────────────────────────────────────────────────────────────────┐
//!   │  PVP Test 1  [PVP]                          [v2.14.99 LATEST v] │  hero
//!   │  👤 by Bohemia Interactive [AUTHOR]                              │
//!   ├─────────────────────────────────────────────────────────────────┤
//!   │ ┌ REQUIRED MODSET & MODS [TBD CORE COMPETITIVE V1.8] [● 6 SYNCED] ┐
//!   │ │ ✓ @CRF_Framework v2.4.0   │ ✓ @RHSAFRF v0.6.1                │
//!   │ └─────────────────────────────────────────────────────────────┘ │
//!   │ ┌ MISSION SUMMARY                                     [SITREP] ┐
//!   │ ┌ ORBAT OVERVIEW                                    [48 SLOTS] ┐
//!   │ │ ┌ BLUFOR Defending  24 SLOTS ┐ ┌ OPFOR Attacking 24 SLOTS ┐ │
//!   │ ┌ OBJECTIVES                                         [4 ACTIVE] ┐
//!   └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Controller for `TBD_MissionInspector.layout` (hero + `CardsContent`). The four cards are
//! `TBD_Panel`s mounted once; their bodies are rebuilt per mission from Common primitives
//! (`TBD_Columns2`, `TBD_ModGridItem`, `TBD_InsetText`, tinted `TBD_Panel` + `TBD_FactionColumn`,
//! `TBD_KeyValueRow`). Selection is user-paced, so rebuild-not-pool is the right cost model here.
class TBD_MissionInspectorPanel
{
	protected Widget m_wRoot;
	protected TextWidget m_wHeroTitle;
	protected Widget m_wHeroTagDock;
	//! Composited hero band colour — ground for the tag chip, author chip and version pill.
	protected int m_iHeroGround;
	//! Ground of the faction column most recently built by MountFactionColumn (for its rows).
	protected int m_iColumnGround;
	protected ImageWidget m_wAuthorIcon;
	protected TextWidget m_wAuthorBy;
	protected TextWidget m_wAuthorName;
	protected Widget m_wAuthorChipDock;
	protected Widget m_wVersionDock;
	protected Widget m_wHero;
	protected ImageWidget m_wHeroImage;
	protected ref TBD_UIScrollBar m_ScrollBar;
	protected Widget m_wCardsContent;
	protected Widget m_wEmptyState;

	protected TBD_ChipComponent m_TagChip;
	protected TBD_ChipComponent m_AuthorChip;
	protected TBD_DropdownComponent m_Version;

	protected TBD_PanelComponent m_ModsPanel;
	protected TBD_ChipComponent m_ModsetChip;
	protected TBD_ChipComponent m_SyncedChip;
	protected TBD_PanelComponent m_SummaryPanel;
	protected TBD_PanelComponent m_OrbatPanel;
	protected TBD_ChipComponent m_OrbatChip;
	protected TBD_PanelComponent m_ObjectivesPanel;
	protected TBD_ChipComponent m_ObjectivesChip;

	protected TBD_MissionCatalog m_Catalog;
	protected TBD_MissionSummary m_Mission;

	//! (TBD_MissionInspectorPanel panel, int versionIndex)
	protected ref ScriptInvoker m_OnVersionChanged;

	static const int CARD_GAP = 12;
	static const int CARD_INSET = 14;

	//------------------------------------------------------------------------------------------------
	//! `root` is a mounted `TBD_MissionInspector.layout`.
	bool Build(Widget root, Widget overlayHost, TBD_MissionCatalog catalog)
	{
		m_Catalog = catalog;
		m_wRoot = root;
		if (!root)
			return false;

		m_wHero = root.FindAnyWidget("Hero");
		m_wHeroTitle = TextWidget.Cast(root.FindAnyWidget("HeroTitle"));
		m_wHeroTagDock = root.FindAnyWidget("HeroTagDock");
		m_wAuthorIcon = ImageWidget.Cast(root.FindAnyWidget("AuthorIcon"));
		m_wAuthorBy = TextWidget.Cast(root.FindAnyWidget("AuthorBy"));
		m_wAuthorName = TextWidget.Cast(root.FindAnyWidget("AuthorName"));
		m_wAuthorChipDock = root.FindAnyWidget("AuthorChipDock");
		m_wVersionDock = root.FindAnyWidget("VersionDock");
		m_wCardsContent = root.FindAnyWidget("CardsContent");
		m_wEmptyState = root.FindAnyWidget("EmptyState");

		// The inspector is a rounded glass panel on the backdrop. The hero is a PHOTO (terrain
		// satellite, or the topo art) under a dim and a bottom fade. A photo cannot be clipped to
		// a round corner, so two layers of inverse-disc masks fake it: inside the hero, r11 quarters
		// painted in the BORDER colour (the ring), and on the root, r12 quarters painted in the
		// BACKDROP colour (outside the panel). Photo inside r11, border r11-r12, backdrop beyond.
		Widget panelBorder = root.FindAnyWidget("PanelBorder");
		Widget panelBG = root.FindAnyWidget("PanelBG");
		TBD_UILayouts.MountRounded(panelBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(panelBG, TBD_UITheme.RADIUS_PANEL - 1);

		int ground = TBD_UITheme.PanelGround();
		int borderColour = TBD_UITheme.Over(TBD_UITheme.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(panelBorder, TBD_UITheme.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(panelBG, TBD_UITheme.PANEL_FILL, TBD_UITheme.Ground());

		m_wHeroImage = ImageWidget.Cast(root.FindAnyWidget("HeroImage"));
		TBD_UITheme.PaintAlpha(root.FindAnyWidget("HeroDim"), TBD_UITheme.HERO_DIM);
		ImageWidget fade = ImageWidget.Cast(root.FindAnyWidget("HeroFade"));
		if (TBD_UILayouts.LoadTexture(fade, TBD_UILayouts.FADE_DOWN))
			TBD_UITheme.Paint(fade, TBD_UITheme.HERO_FADE);

		MountCornerMask(root, "HeroMaskTLImg", borderColour);
		MountCornerMask(root, "HeroMaskTRImg", borderColour);
		MountCornerMask(root, "PanelMaskTLImg", TBD_UITheme.Ground());
		MountCornerMask(root, "PanelMaskTRImg", TBD_UITheme.Ground());

		// Chips and the version pill sit at the bottom of the hero, where the fade is solid.
		m_iHeroGround = TBD_UITheme.HERO_FADE;

		m_ScrollBar = TBD_UIScrollBar.Mount(root.FindAnyWidget("ScrollBarDock"), ScrollLayoutWidget.Cast(root.FindAnyWidget("Scroll")), m_wCardsContent, ground);
		TBD_UITheme.Paint(m_wHeroTitle, TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.Paint(m_wAuthorBy, TBD_UITheme.MUTED_INK);
		TBD_UITheme.Paint(m_wAuthorName, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wEmptyState, TBD_UITheme.MUTED_INK);

		if (m_wAuthorIcon)
		{
			TBD_UIIcons.Load(m_wAuthorIcon, "person");
			TBD_UITheme.Paint(m_wAuthorIcon, TBD_UITheme.MUTED_INK);
		}

		m_AuthorChip = TBD_ChipComponent.Mount(m_wAuthorChipDock, "AUTHOR", TBD_EUITint.NEUTRAL, m_iHeroGround);

		m_Version = TBD_DropdownComponent.Mount(m_wVersionDock, overlayHost, "Version", false);
		if (m_Version)
		{
			m_Version.SetMenuTitle("Select mission version");
			m_Version.SetGround(m_iHeroGround);
			m_Version.GetOnChanged().Insert(OnVersionChanged);
			AlignableSlot.SetHorizontalAlign(m_Version.GetRootWidget(), LayoutHorizontalAlign.Right);
		}

		BuildCards();
		Show(null);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! One quarter of the inverse disc, painted in the colour that lies BEHIND the hero there.
	protected void MountCornerMask(Widget root, string name, int opaqueArgb)
	{
		ImageWidget mask = ImageWidget.Cast(root.FindAnyWidget(name));
		if (TBD_UILayouts.LoadTexture(mask, TBD_UILayouts.CORNER_DISC_INV))
			TBD_UITheme.Paint(mask, opaqueArgb);
	}

	//------------------------------------------------------------------------------------------------
	//! Satellite band for terrains that have one, the topo art (at 25 %) for the rest.
	protected void ShowHeroImage(string terrainKey)
	{
		ResourceName texture = TBD_UILayouts.HERO_TOPO;
		bool art = true;

		TBD_TerrainInfo terrain = m_Catalog.GetTerrain(terrainKey);
		if (terrain && !terrain.m_sHeroImage.IsEmpty())
		{
			texture = terrain.m_sHeroImage;
			art = texture == TBD_UILayouts.HERO_TOPO;
		}

		if (!TBD_UILayouts.LoadTexture(m_wHeroImage, texture))
			return;

		if (art)
			TBD_UITheme.PaintAlpha(m_wHeroImage, TBD_UITheme.HERO_ART_INK);
		else
			TBD_UITheme.Paint(m_wHeroImage, TBD_UITheme.BRIGHT_INK);
	}

	//------------------------------------------------------------------------------------------------
	void Destroy()
	{
		if (m_ScrollBar)
			m_ScrollBar.Destroy();

		m_ScrollBar = null;
		if (m_Version)
		{
			m_Version.GetOnChanged().Remove(OnVersionChanged);
			m_Version.Close();
		}

		m_Version = null;
		m_wRoot = null;
		m_wCardsContent = null;
		m_Mission = null;
	}

	//------------------------------------------------------------------------------------------------
	//! Rebind everything to `mission`. Null shows the empty state.
	void Show(TBD_MissionSummary mission)
	{
		m_Mission = mission;

		bool has = mission != null;
		TBD_UITheme.Show(m_wEmptyState, !has);
		TBD_UITheme.Show(m_wCardsContent, has);
		TBD_UITheme.Show(m_wHeroTitle, has);
		TBD_UITheme.Show(m_wHeroTagDock, has);
		TBD_UITheme.Show(m_wAuthorBy, has);
		TBD_UITheme.Show(m_wAuthorName, has);
		TBD_UITheme.Show(m_wAuthorChipDock, has);
		TBD_UITheme.Show(m_wVersionDock, has);
		TBD_UITheme.Show(m_wAuthorIcon, has);
		TBD_UITheme.Show(m_wHeroImage, has);

		if (!has)
			return;

		TBD_UITheme.Write(m_wHeroTitle, mission.m_sTitle);
		ShowHeroImage(mission.m_sTerrainKey);
		TBD_UITheme.Write(m_wAuthorName, mission.m_sAuthor);

		if (!m_TagChip)
			m_TagChip = TBD_ChipComponent.Mount(m_wHeroTagDock, m_Catalog.ModeLabel(mission.m_sTag), m_Catalog.ModeTint(mission.m_sTag), m_iHeroGround);
		else
			m_TagChip.Set(m_Catalog.ModeLabel(mission.m_sTag), m_Catalog.ModeTint(mission.m_sTag));

		FillVersions(mission);
		FillMods(mission);
		FillSummary(mission);
		FillOrbat(mission);
		FillObjectives(mission);
	}

	//------------------------------------------------------------------------------------------------
	//! Index into the mission's version list, -1 when none.
	int GetSelectedVersionIndex()
	{
		if (!m_Version)
			return -1;

		return m_Version.GetSelectedTag();
	}

	//------------------------------------------------------------------------------------------------
	//! Label of the chosen version ("v2.14.99"), empty when none.
	string GetSelectedVersionLabel()
	{
		if (!m_Mission)
			return string.Empty;

		int index = GetSelectedVersionIndex();
		if (index < 0 || index >= m_Mission.m_aVersions.Count())
			return string.Empty;

		return m_Mission.m_aVersions[index].m_sLabel;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_MissionInspectorPanel panel, int versionIndex)
	ScriptInvoker GetOnVersionChanged()
	{
		if (!m_OnVersionChanged)
			m_OnVersionChanged = new ScriptInvoker();

		return m_OnVersionChanged;
	}

	// ── Cards ───────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected void FillVersions(TBD_MissionSummary mission)
	{
		if (!m_Version)
			return;

		array<ref TBD_DropdownItem> items = {};
		for (int i = 0; i < mission.m_aVersions.Count(); i++)
		{
			TBD_MissionVersion version = mission.m_aVersions[i];
			items.Insert(new TBD_DropdownItem(version.m_sLabel, i, version.m_sBadge));
		}

		m_Version.SetSelectedTag(-1);
		m_Version.SetItems(items);
		if (!items.IsEmpty())
			m_Version.SetSelectedTag(0);
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	// ── Builders ────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Two-column grid inset inside a card body.
	protected Widget MountColumns(Widget body)
	{
		Widget columns = TBD_UILayouts.CreateStretched(TBD_UILayouts.COLUMNS_2, body);
		if (columns)
			AlignableSlot.SetPadding(columns, CARD_INSET, CARD_INSET, CARD_INSET, CARD_INSET - 8);

		return columns;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected void OnVersionChanged(TBD_DropdownComponent dropdown, int tag)
	{
		if (m_OnVersionChanged)
			m_OnVersionChanged.Invoke(this, tag);
	}
}
