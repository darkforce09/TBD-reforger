/**
 * @file TBD_ScenarioBrowserPanel.c
 * @brief The missions column of the Mission Selector: search, Modes filter, pooled mission cards.
 *
 * Role: filters the catalog's missions by terrain, ticked modes and search text, and owns the
 * mission selection.  Position: TBD_MissionSelectorScreen builds it on CenterDock and calls
 * SetTerrain on a terrain pick; cards are TBD_MissionCardComponent; GetOnSelected (panel,
 * missionId) is its only output, with an empty id when nothing is visible.
 * State: the card pool, the live card ids, the terrain, the selection, the query and the hidden
 * modes, owned by the screen on the client.  Invariants: every count (`N AVAILABLE`, the Modes
 * badge, per-mode counts) is computed from the catalog; a selection the filter hides moves to the
 * first visible card, or clears.
 *
 *   |  EVERON MISSIONS                      [3 AVAILABLE] |
 *   | [ Search Scenario...       ]  [Modes (5) v]          |
 *   | [PVP]  Everon                      48 SLOTS *        |  <- selected card
 *   |  PVP Test 1                                  (ok)    |
 *   | [COOP] Everon                       4 SLOTS          |
 *   |  Co-op Test 1                                  >     |
 */

//! Controller of the missions column. Not a widget handler: the screen mounts a TBD_Panel into
//! CenterDock and hands its root here.
class TBD_ScenarioBrowserPanel
{
	protected TBD_PanelComponent m_Panel; //!< the mounted missions panel
	protected TBD_ChipComponent m_CountChip; //!< `N AVAILABLE` badge
	protected Widget m_wBody; //!< the browser body layout inside the panel
	protected TBD_SearchBoxComponent m_Search; //!< `Search Scenario...` box
	protected TBD_DropdownComponent m_Modes; //!< Modes checklist dropdown
	protected Widget m_wContent; //!< `Content`: the card container
	protected Widget m_wEmptyState; //!< `EmptyState`, shown when no card matches
	protected ref TBD_UIScrollBar m_ScrollBar; //!< scroll bar of the card list

	protected TBD_MissionCatalog m_Catalog; //!< the catalog in force
	protected ref array<TBD_MissionCardComponent> m_aCards; //!< card pool; handlers owned by their widgets
	protected ref array<string> m_aCardIds; //!< mission id per live card
	protected int m_iLiveCards; //!< cards bound by the last Refresh

	protected string m_sTerrainKey; //!< the terrain shown
	protected string m_sSelectedId; //!< selected mission id, empty when none
	protected string m_sQuery; //!< current search text
	protected ref set<string> m_sHiddenModes; //!< mode keys the user unticked; kept across terrain changes

	protected ref ScriptInvoker m_OnSelected; //!< (TBD_ScenarioBrowserPanel panel, string missionId)

	//! Title the panel, mount the count badge, search box, Modes dropdown and scroll bar.
	//! @param panelRoot a mounted `TBD_Panel.layout`
	//! @param overlayHost where the Modes dropdown opens its menu
	//! @param catalog the catalog in force
	//! @return false when the layout tree is missing pieces
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

	//! Release the scroll bar, the search listener and the Modes dropdown before the screen closes.
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

	//! Point the browser at a terrain: retitle, recount the mode checklist, refilter the cards.
	//! @param terrainKey the terrain
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

	//! Select a mission.
	//! @param missionId the mission; empty selects nothing
	//! @param notify false updates the cards only, without raising GetOnSelected
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

	//! @return the selected mission id, empty when none
	string GetSelectedId()
	{
		return m_sSelectedId;
	}

	//! @return the invoker raised with (TBD_ScenarioBrowserPanel panel, string missionId) on a selection
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	//! Select the mission of an activated card.
	//! @param index the card's live index; out of range is ignored
	void OnCardActivated(int index)
	{
		if (index < 0 || index >= m_iLiveCards)
			return;

		Select(m_aCardIds[index], true);
	}

	//! Refilter the cards by the new search text.
	protected void OnSearchChanged(TBD_SearchBoxComponent box, string query)
	{
		m_sQuery = query;
		Refresh();
	}

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

	//! The pooled card at `index`, created when the pool is shorter.
	//! @return the card, or null when the layout fails
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
