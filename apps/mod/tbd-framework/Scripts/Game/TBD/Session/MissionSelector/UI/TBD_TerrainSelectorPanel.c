/**
 * @file TBD_TerrainSelectorPanel.c
 * @brief The TERRAINS column of the Mission Selector: search box, pooled terrain rows, selection.
 *
 * Role: fills the TERRAINS panel from the catalog through the search query and owns the terrain
 * selection.  Position: TBD_MissionSelectorScreen builds it on LeftDock; rows are
 * TBD_TerrainRowComponent; GetOnSelected (panel, terrainKey) is its only output.
 * State: the row pool, the live row keys, the selected key and the query, owned by the screen on
 * the client.  Invariants: rows are created once and rebound, surplus rows hidden, so typing in the
 * search never churns widgets.
 *
 *   | TERRAINS                    |
 *   | [ Search Maps...          ] |
 *   | ==  Everon           3  >   |  <- selected: glow bar, chevron, blue border
 *   |     Arland           3      |
 */

//! Controller of the TERRAINS column. Not a widget handler: the screen mounts a TBD_Panel into
//! LeftDock and hands its root here.
class TBD_TerrainSelectorPanel
{
	protected TBD_PanelComponent m_Panel; //!< the mounted TERRAINS panel
	protected Widget m_wBody; //!< the terrains body layout inside the panel
	protected TBD_SearchBoxComponent m_Search; //!< `Search Maps...` box
	protected Widget m_wContent; //!< `Content`: the row container
	protected Widget m_wEmptyState; //!< `EmptyState`, shown when no row matches
	protected ref TBD_UIScrollBar m_ScrollBar; //!< scroll bar of the row list

	protected TBD_MissionCatalog m_Catalog; //!< the catalog in force
	protected ref array<TBD_TerrainRowComponent> m_aRows; //!< row pool; handlers owned by their widgets
	protected ref array<string> m_aRowKeys; //!< terrain key per live row
	protected int m_iLiveRows; //!< rows bound by the last Refresh
	protected string m_sSelectedKey; //!< selected terrain key, empty when none
	protected string m_sQuery; //!< current search text

	protected ref ScriptInvoker m_OnSelected; //!< (TBD_TerrainSelectorPanel panel, string terrainKey)

	//! Title the panel, mount the search box and scroll bar, and fill the rows.
	//! @param panelRoot a mounted `TBD_Panel.layout`
	//! @param catalog the catalog in force
	//! @return false when the layout tree is missing pieces; the column then stays empty
	bool Build(Widget panelRoot, TBD_MissionCatalog catalog)
	{
		m_Catalog = catalog;
		m_aRows = {};
		m_aRowKeys = {};

		if (!panelRoot)
			return false;

		m_Panel = TBD_PanelComponent.Cast(panelRoot.FindHandler(TBD_PanelComponent));
		if (!m_Panel)
			return false;

		m_Panel.SetTitle("Terrains");

		m_wBody = TBD_UILayouts.Create(TBD_UILayouts.MISSION_SELECTOR_TERRAINS, m_Panel.GetBodyDock());
		if (!m_wBody)
			return false;

		m_Search = TBD_SearchBoxComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.SEARCH_BOX, m_wBody.FindAnyWidget("SearchDock"), TBD_SearchBoxComponent));
		if (m_Search)
		{
			m_Search.SetPlaceholder("Search Maps...");
			m_Search.SetGround(m_Panel.GetGround());
			m_Search.GetOnChanged().Insert(OnSearchChanged);
		}

		m_wContent = m_wBody.FindAnyWidget("Content");
		m_wEmptyState = m_wBody.FindAnyWidget("EmptyState");
		m_ScrollBar = TBD_UIScrollBar.Mount(m_wBody.FindAnyWidget("ScrollBarDock"), ScrollLayoutWidget.Cast(m_wBody.FindAnyWidget("Scroll")), m_wContent, m_Panel.GetGround());
		TBD_UITheme.PaintOver(m_wBody.FindAnyWidget("SearchRule"), TBD_UITheme.GLASS_BORDER, m_Panel.GetGround());
		TBD_UITheme.PaintOver(m_wBody.FindAnyWidget("SearchStrip"), TBD_UITheme.SEARCH_STRIP_FILL, m_Panel.GetGround());
		TBD_UITheme.Paint(m_wEmptyState, TBD_UITheme.MUTED_INK);

		Refresh();
		return true;
	}

	//! Release the scroll bar and the search listener before the screen closes.
	void Destroy()
	{
		if (m_ScrollBar)
			m_ScrollBar.Destroy();

		m_ScrollBar = null;
		if (m_Search)
			m_Search.GetOnChanged().Remove(OnSearchChanged);

		m_Search = null;
		m_Panel = null;
		m_wBody = null;
		m_wContent = null;
		if (m_aRows)
			m_aRows.Clear();
	}

	//! Rebind every visible row from the catalog through the current search query.
	void Refresh()
	{
		if (!m_wContent || !m_Catalog)
			return;

		m_aRowKeys.Clear();
		int cursor;

		foreach (TBD_TerrainInfo terrain : m_Catalog.GetTerrains())
		{
			if (!TBD_SearchBoxComponent.Matches(m_sQuery, terrain.m_sName))
				continue;

			TBD_TerrainRowComponent row = AcquireRow(cursor);
			if (!row)
				continue;

			row.Bind(this, cursor, terrain, m_Catalog.CountMissions(terrain.m_sKey));
			row.SetSelected(terrain.m_sKey == m_sSelectedKey);
			row.SetRowVisible(true);
			m_aRowKeys.Insert(terrain.m_sKey);
			cursor++;
		}

		m_iLiveRows = cursor;
		for (int i = cursor; i < m_aRows.Count(); i++)
		{
			m_aRows[i].SetRowVisible(false);
		}

		TBD_UITheme.Show(m_wEmptyState, m_iLiveRows == 0);
	}

	//! Select a terrain.
	//! @param terrainKey the terrain
	//! @param notify false updates the rows only, without raising GetOnSelected
	void Select(string terrainKey, bool notify)
	{
		m_sSelectedKey = terrainKey;

		for (int i = 0; i < m_iLiveRows; i++)
		{
			m_aRows[i].SetSelected(m_aRowKeys[i] == terrainKey);
		}

		if (notify && m_OnSelected)
			m_OnSelected.Invoke(this, terrainKey);
	}

	//! @return the selected terrain key, empty when none
	string GetSelectedKey()
	{
		return m_sSelectedKey;
	}

	//! @return the key of the first terrain in catalog order, or empty
	string GetFirstKey()
	{
		array<ref TBD_TerrainInfo> terrains = m_Catalog.GetTerrains();
		if (terrains.IsEmpty())
			return string.Empty;

		return terrains[0].m_sKey;
	}

	//! Put focus on the selected row, else the first.
	//! @return false when there is no row to focus
	bool FocusSelected()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace || m_iLiveRows == 0)
			return false;

		int index = m_aRowKeys.Find(m_sSelectedKey);
		if (index < 0)
			index = 0;

		Widget target = m_aRows[index].GetRootWidget();
		if (!target)
			return false;

		workspace.SetFocusedWidget(target);
		return true;
	}

	//! @return the invoker raised with (TBD_TerrainSelectorPanel panel, string terrainKey) on a selection
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	//! Select the terrain of an activated row.
	//! @param index the row's live index; out of range is ignored
	void OnRowActivated(int index)
	{
		if (index < 0 || index >= m_iLiveRows)
			return;

		Select(m_aRowKeys[index], true);
	}

	//! Refilter the rows by the new search text.
	protected void OnSearchChanged(TBD_SearchBoxComponent box, string query)
	{
		m_sQuery = query;
		Refresh();
	}

	//! The pooled row at `index`, created when the pool is shorter.
	//! @return the row, or null when the layout fails
	protected TBD_TerrainRowComponent AcquireRow(int index)
	{
		if (index < m_aRows.Count())
			return m_aRows[index];

		TBD_TerrainRowComponent row = TBD_TerrainRowComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.MISSION_SELECTOR_TERRAIN_ROW, m_wContent, TBD_TerrainRowComponent));
		if (!row)
			return null;

		AlignableSlot.SetHorizontalAlign(row.GetRootWidget(), LayoutHorizontalAlign.Stretch);
		m_aRows.Insert(row);
		return row;
	}
}
