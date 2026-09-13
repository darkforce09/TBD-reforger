//! Pre-game rebuild (2026-09-12) — the <TERRAIN> MISSIONS column of the Mission Selector.
//!
//! ```
//!   ┌ ▦ EVERON MISSIONS                      [3 AVAILABLE] ┐
//!   │ [🔍 Search Scenario...        ] [Modes (5) v]        │
//!   ├──────────────────────────────────────────────────────┤
//!   │ ┌ PVP  Everon                     48 SLOTS ● ┐       │  <- selected card
//!   │ │ PVP Test 1                              ✓ │       │
//!   │ └───────────────────────────────────────────┘       │
//!   │ ┌ COOP Everon                      4 SLOTS ┐         │
//!   │ │ Co-op Test 1                            > │         │
//!   └──────────────────────────────────────────────────────┘
//! ```
//!
//! Two classes, one file:
//!   * `TBD_MissionCardComponent` — one pooled card (`TBD_MissionCard.layout`). Widget contract:
//!     `Border`, `Background`, `TagChipDock`, `TerrainText`, `SlotCount`, `PulseDot`, `Title`,
//!     `Indicator` (image), `IndicatorGlyph` (text fallback).
//!   * `TBD_ScenarioBrowserPanel` — the controller. Filters = terrain ∩ checked modes ∩ query; all
//!     counts (`N AVAILABLE`, the Modes badge, per-mode counts) are computed from the catalog.
//!     Output: `GetOnSelected()(panel, missionId)` — empty id when nothing is visible.

class TBD_MissionCardComponent : TBD_UIInteractive
{
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wTagDock;
	protected TextWidget m_wTerrain;
	protected TextWidget m_wSlots;
	protected Widget m_wPulse;
	protected TextWidget m_wTitle;
	protected ImageWidget m_wIndicator;
	protected TextWidget m_wIndicatorGlyph;
	protected TBD_ChipComponent m_TagChip;

	protected TBD_ScenarioBrowserPanel m_Owner;
	protected int m_iIndex = -1;
	protected bool m_bSelected;
	protected TBD_EUITint m_eTagTint = TBD_EUITint.NEUTRAL;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wTagDock = w.FindAnyWidget("TagChipDock");
		m_wTerrain = TextWidget.Cast(w.FindAnyWidget("TerrainText"));
		m_wSlots = TextWidget.Cast(w.FindAnyWidget("SlotCount"));
		m_wPulse = w.FindAnyWidget("PulseDot");
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wIndicator = ImageWidget.Cast(w.FindAnyWidget("Indicator"));
		m_wIndicatorGlyph = TextWidget.Cast(w.FindAnyWidget("IndicatorGlyph"));

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
	}

	//------------------------------------------------------------------------------------------------
	void Bind(TBD_ScenarioBrowserPanel owner, int index, TBD_MissionSummary mission, string terrainName, string tagLabel, TBD_EUITint tagTint)
	{
		m_Owner = owner;
		m_iIndex = index;
		m_eTagTint = tagTint;

		if (!m_TagChip)
			m_TagChip = TBD_ChipComponent.Mount(m_wTagDock, tagLabel, tagTint);
		else
			m_TagChip.Set(tagLabel, tagTint);

		TBD_UITheme.Write(m_wTerrain, terrainName);
		TBD_UITheme.Write(m_wSlots, string.Format("%1 SLOTS", mission.GetSlotTotal()));
		TBD_UITheme.Write(m_wTitle, mission.m_sTitle);

		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	void SetSelected(bool selected)
	{
		if (m_bSelected == selected)
			return;

		m_bSelected = selected;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int fill;
		int border;
		int titleInk;
		int slotInk;
		int terrainInk;
		int indicatorInk;
		string indicatorIcon;
		string glyph;

		if (m_bSelected)
		{
			fill         = TBD_UITheme.CARD_SELECTED_FILL;
			border       = TBD_UITheme.CARD_SELECTED_BORDER;
			titleInk     = TBD_UITheme.BRIGHT_INK;
			slotInk      = TBD_UITheme.CARD_SELECTED_INK;
			terrainInk   = TBD_UITheme.PRIMARY_FIXED;
			indicatorInk = TBD_UITheme.CARD_SELECTED_INK;
			indicatorIcon = "check_circle";
			glyph = "*";
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill         = TBD_UITheme.CARD_HOVER_FILL;
			border       = TBD_UITheme.CARD_HOVER_BORDER;
			titleInk     = TBD_UITheme.BRIGHT_INK;
			slotInk      = TBD_UITheme.ON_SURFACE;
			terrainInk   = TBD_UITheme.MUTED_INK;
			indicatorInk = TBD_UITheme.ON_SURFACE;
			indicatorIcon = "chevron_right";
			glyph = ">";
		}
		else
		{
			fill         = TBD_UITheme.CARD_IDLE_FILL;
			border       = TBD_UITheme.CARD_IDLE_BORDER;
			titleInk     = TBD_UITheme.ON_SURFACE;
			slotInk      = TBD_UITheme.ChipInk(TBD_EUITint.NEUTRAL);
			terrainInk   = TBD_UITheme.MUTED_INK;
			indicatorInk = TBD_UITheme.DIM_INK;
			indicatorIcon = "chevron_right";
			glyph = ">";
		}

		// Cards sit on the MISSIONS panel; the tag chip sits on the card's own fill.
		int ground = TBD_UITheme.PanelGround();
		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.Paint(m_wTitle, titleInk);
		TBD_UITheme.Paint(m_wSlots, slotInk);
		TBD_UITheme.Paint(m_wTerrain, terrainInk);
		TBD_UITheme.Show(m_wPulse, m_bSelected);
		TBD_UITheme.Paint(m_wPulse, TBD_UITheme.CARD_BORDER);

		if (m_TagChip)
		{
			m_TagChip.SetGround(TBD_UITheme.Over(fill, ground));
			if (m_bSelected)
				m_TagChip.SetTint(TBD_EUITint.SOLID);
			else
				m_TagChip.SetTint(m_eTagTint);
		}

		// Icon when the imageset has one, glyph otherwise — never both, never neither.
		bool iconShown = TBD_UIIcons.Load(m_wIndicator, indicatorIcon);
		TBD_UITheme.Paint(m_wIndicator, indicatorInk);
		TBD_UITheme.Show(m_wIndicatorGlyph, !iconShown);
		TBD_UITheme.Write(m_wIndicatorGlyph, glyph);
		TBD_UITheme.Paint(m_wIndicatorGlyph, indicatorInk);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnCardActivated(m_iIndex);
	}

	//------------------------------------------------------------------------------------------------
	void SetCardVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
	}
}

class TBD_ScenarioBrowserPanel
{
	protected TBD_PanelComponent m_Panel;
	protected TBD_ChipComponent m_CountChip;
	protected Widget m_wBody;
	protected TBD_SearchBoxComponent m_Search;
	protected TBD_DropdownComponent m_Modes;
	protected Widget m_wContent;
	protected Widget m_wEmptyState;
	protected ref TBD_UIScrollBar m_ScrollBar;

	protected TBD_MissionCatalog m_Catalog;
	protected ref array<TBD_MissionCardComponent> m_aCards;
	protected ref array<string> m_aCardIds;
	protected int m_iLiveCards;

	protected string m_sTerrainKey;
	protected string m_sSelectedId;
	protected string m_sQuery;
	//! Modes the user unticked. Kept across terrain changes so a filter survives browsing.
	protected ref set<string> m_sHiddenModes;

	//! (TBD_ScenarioBrowserPanel panel, string missionId)
	protected ref ScriptInvoker m_OnSelected;

	//------------------------------------------------------------------------------------------------
	bool Build(Widget panelRoot, Widget overlayHost, TBD_MissionCatalog catalog)
	{
		m_Catalog = catalog;
		m_aCards = {};
		m_aCardIds = {};
		m_sHiddenModes = new set<string>();

		if (!panelRoot)
			return false;

		m_Panel = TBD_PanelComponent.Cast(panelRoot.FindHandler(TBD_PanelComponent));
		if (!m_Panel)
			return false;

		m_Panel.SetTitle("Missions");
		m_Panel.SetIcon("grid_view");
		int ground = m_Panel.GetGround();
		m_CountChip = TBD_ChipComponent.Mount(m_Panel.GetBadgeDock(), "0 AVAILABLE", TBD_EUITint.PRIMARY, ground);
		if (m_CountChip)
			m_CountChip.SetPill(true);

		m_wBody = TBD_UILayouts.Create(TBD_UILayouts.MISSION_SELECTOR_BROWSER, m_Panel.GetBodyDock());
		if (!m_wBody)
			return false;

		m_Search = TBD_SearchBoxComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.SEARCH_BOX, m_wBody.FindAnyWidget("SearchDock"), TBD_SearchBoxComponent));
		if (m_Search)
		{
			m_Search.SetPlaceholder("Search Scenario...");
			m_Search.SetGround(ground);
			m_Search.GetOnChanged().Insert(OnSearchChanged);
		}

		m_Modes = TBD_DropdownComponent.Mount(m_wBody.FindAnyWidget("ModesDock"), overlayHost, "Modes", true);
		if (m_Modes)
		{
			m_Modes.SetMenuTitle("Filter modes");
			m_Modes.SetGround(ground);
			m_Modes.GetOnChanged().Insert(OnModesChanged);
		}

		m_wContent = m_wBody.FindAnyWidget("Content");
		m_wEmptyState = m_wBody.FindAnyWidget("EmptyState");
		m_ScrollBar = TBD_UIScrollBar.Mount(m_wBody.FindAnyWidget("ScrollBarDock"), ScrollLayoutWidget.Cast(m_wBody.FindAnyWidget("Scroll")), m_wContent, m_Panel.GetGround());
		TBD_UITheme.PaintOver(m_wBody.FindAnyWidget("ToolRule"), TBD_UITheme.GLASS_BORDER, ground);
		TBD_UITheme.Paint(m_wEmptyState, TBD_UITheme.MUTED_INK);

		return true;
	}

	//------------------------------------------------------------------------------------------------
	void Destroy()
	{
		if (m_ScrollBar)
			m_ScrollBar.Destroy();

		m_ScrollBar = null;
		if (m_Search)
			m_Search.GetOnChanged().Remove(OnSearchChanged);

		if (m_Modes)
		{
			m_Modes.GetOnChanged().Remove(OnModesChanged);
			m_Modes.Close();
		}

		m_Search = null;
		m_Modes = null;
		m_Panel = null;
		m_wBody = null;
		m_wContent = null;
		if (m_aCards)
			m_aCards.Clear();
	}

	//------------------------------------------------------------------------------------------------
	//! Point the browser at a terrain: retitles, recounts the mode checklist, refilters the cards.
	void SetTerrain(string terrainKey)
	{
		m_sTerrainKey = terrainKey;

		TBD_TerrainInfo terrain = m_Catalog.FindTerrain(terrainKey);
		if (m_Panel)
		{
			if (terrain)
				m_Panel.SetTitle(terrain.m_sName + " Missions");
			else
				m_Panel.SetTitle("Missions");
		}

		RebuildModes();
		Refresh();
	}

	//------------------------------------------------------------------------------------------------
	//! Rebind the visible cards from the catalog through terrain, modes and query. Keeps the
	//! current selection when it survives the filter, else picks the first card, else clears.
	void Refresh()
	{
		if (!m_wContent || !m_Catalog)
			return;

		string terrainName;
		TBD_TerrainInfo terrain = m_Catalog.FindTerrain(m_sTerrainKey);
		if (terrain)
			terrainName = terrain.m_sName;

		array<TBD_MissionSummary> missions = {};
		m_Catalog.GetMissions(m_sTerrainKey, missions);

		m_aCardIds.Clear();
		int cursor;

		foreach (TBD_MissionSummary mission : missions)
		{
			if (m_sHiddenModes.Contains(mission.m_sTag))
				continue;

			if (!TBD_SearchBoxComponent.Matches(m_sQuery, mission.m_sTitle))
				continue;

			TBD_MissionCardComponent card = AcquireCard(cursor);
			if (!card)
				continue;

			card.Bind(this, cursor, mission, terrainName, m_Catalog.ModeLabel(mission.m_sTag), m_Catalog.ModeTint(mission.m_sTag));
			card.SetSelected(mission.m_sId == m_sSelectedId);
			card.SetCardVisible(true);
			m_aCardIds.Insert(mission.m_sId);
			cursor++;
		}

		m_iLiveCards = cursor;
		for (int i = cursor; i < m_aCards.Count(); i++)
		{
			m_aCards[i].SetCardVisible(false);
		}

		TBD_UITheme.Show(m_wEmptyState, m_iLiveCards == 0);
		if (m_CountChip)
			m_CountChip.SetText(string.Format("%1 AVAILABLE", m_iLiveCards));

		// Selection follows the filter: an invisible selection is a lie.
		if (m_aCardIds.Find(m_sSelectedId) < 0)
		{
			if (m_iLiveCards > 0)
				Select(m_aCardIds[0], true);
			else
				Select(string.Empty, true);
		}
	}

	//------------------------------------------------------------------------------------------------
	void Select(string missionId, bool notify)
	{
		m_sSelectedId = missionId;

		for (int i = 0; i < m_iLiveCards; i++)
		{
			m_aCards[i].SetSelected(m_aCardIds[i] == missionId);
		}

		if (notify && m_OnSelected)
			m_OnSelected.Invoke(this, missionId);
	}

	//------------------------------------------------------------------------------------------------
	string GetSelectedId()
	{
		return m_sSelectedId;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_ScenarioBrowserPanel panel, string missionId)
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	// ── Called by TBD_MissionCardComponent ──────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	void OnCardActivated(int index)
	{
		if (index < 0 || index >= m_iLiveCards)
			return;

		Select(m_aCardIds[index], true);
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnSearchChanged(TBD_SearchBoxComponent box, string query)
	{
		m_sQuery = query;
		Refresh();
	}

	//------------------------------------------------------------------------------------------------
	//! Mirror the checklist into the hidden-modes set, then refilter.
	protected void OnModesChanged(TBD_DropdownComponent dropdown, int tag)
	{
		m_sHiddenModes.Clear();

		array<ref TBD_MissionMode> modes = m_Catalog.GetModes();
		for (int i = 0; i < modes.Count(); i++)
		{
			if (!dropdown.IsChecked(i))
				m_sHiddenModes.Insert(modes[i].m_sKey);
		}

		Refresh();
	}

	//------------------------------------------------------------------------------------------------
	//! One checklist row per mode, count = missions on this terrain with that mode. Tag = mode
	//! index so OnModesChanged can map back without string compares.
	protected void RebuildModes()
	{
		if (!m_Modes)
			return;

		array<ref TBD_DropdownItem> items = {};
		array<ref TBD_MissionMode> modes = m_Catalog.GetModes();
		for (int i = 0; i < modes.Count(); i++)
		{
			TBD_MissionMode mode = modes[i];
			int count = m_Catalog.CountMode(m_sTerrainKey, mode.m_sKey);
			bool checked = !m_sHiddenModes.Contains(mode.m_sKey);
			items.Insert(new TBD_DropdownItem(mode.m_sLabel, i, count.ToString(), checked));
		}

		m_Modes.SetItems(items);
	}

	//------------------------------------------------------------------------------------------------
	protected TBD_MissionCardComponent AcquireCard(int index)
	{
		if (index < m_aCards.Count())
			return m_aCards[index];

		TBD_MissionCardComponent card = TBD_MissionCardComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.MISSION_SELECTOR_MISSION_CARD, m_wContent, TBD_MissionCardComponent));
		if (!card)
			return null;

		AlignableSlot.SetHorizontalAlign(card.GetRootWidget(), LayoutHorizontalAlign.Stretch);
		m_aCards.Insert(card);
		return card;
	}
}
