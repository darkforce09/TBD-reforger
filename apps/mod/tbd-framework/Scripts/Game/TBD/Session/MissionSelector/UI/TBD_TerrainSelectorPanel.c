//! Pre-game rebuild (2026-09-12) — the TERRAINS column of the Mission Selector.
//!
//! ```
//!   ┌ TERRAINS ─────────────────────┐
//!   │ [🔍 Search Maps...          ] │
//!   ├───────────────────────────────┤
//!   │ ▔▔  Everon              3  >  │  <- selected: glow bar, chevron, blue border
//!   │     Arland              3     │
//!   │     Kolguyev            3     │
//!   └───────────────────────────────┘
//! ```
//!
//! Two classes, one file:
//!   * `TBD_TerrainRowComponent` — one pooled row (`TBD_TerrainRow.layout`). Widget contract:
//!     `Border`, `Background`, `Accent`, `Icon`, `Title`, `CountBadgeDock`, `Chevron`.
//!   * `TBD_TerrainSelectorPanel` — the controller. Not a widget handler: the screen mounts a
//!     `TBD_Panel` into LeftDock and hands its root here; this class fills it (search box, rows)
//!     and owns the selection. `GetOnSelected()(panel, terrainKey)` is its only output.
//!
//! Rows are pooled the TBD_ListBox way (create once, rebind, hide surplus) so search-as-you-type
//! never churns widgets.

class TBD_TerrainRowComponent : TBD_UIInteractive
{
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wAccent;
	protected ImageWidget m_wIcon;
	protected TextWidget m_wTitle;
	protected Widget m_wCountDock;
	protected TextWidget m_wChevron;
	protected TBD_ChipComponent m_CountChip;

	protected TBD_TerrainSelectorPanel m_Owner; //!< weak — the panel outlives its rows only by a frame
	protected int m_iIndex = -1;
	protected bool m_bSelected;
	protected bool m_bIconShown;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wAccent = w.FindAnyWidget("Accent");
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("Icon"));
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wCountDock = w.FindAnyWidget("CountBadgeDock");
		m_wChevron = TextWidget.Cast(w.FindAnyWidget("Chevron"));

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
	}

	//------------------------------------------------------------------------------------------------
	void Bind(TBD_TerrainSelectorPanel owner, int index, TBD_TerrainInfo terrain, int missionCount)
	{
		m_Owner = owner;
		m_iIndex = index;

		TBD_UITheme.Write(m_wTitle, terrain.m_sName);
		m_bIconShown = TBD_UIIcons.Load(m_wIcon, terrain.m_sIcon);

		if (!m_CountChip)
			m_CountChip = TBD_ChipComponent.Mount(m_wCountDock, missionCount.ToString(), TBD_EUITint.NEUTRAL);
		else
			m_CountChip.SetText(missionCount.ToString());

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
		int accent;
		int ink;
		int iconInk;

		if (m_bSelected)
		{
			fill    = TBD_UITheme.CARD_SELECTED_FILL;
			border  = TBD_UITheme.ROW_SELECTED_BORDER;
			accent  = TBD_UITheme.ROW_ACTIVE_GLOW;
			ink     = TBD_UITheme.BRIGHT_INK;
			iconInk = TBD_UITheme.ROW_ACTIVE_GLOW;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill    = TBD_UITheme.NAV_HOVER_FILL;
			border  = TBD_UITheme.TRANSPARENT;
			accent  = TBD_UITheme.TRANSPARENT;
			ink     = TBD_UITheme.ON_SURFACE;
			iconInk = TBD_UITheme.MUTED_INK;
		}
		else
		{
			fill    = TBD_UITheme.TRANSPARENT;
			border  = TBD_UITheme.TRANSPARENT;
			accent  = TBD_UITheme.TRANSPARENT;
			ink     = TBD_UITheme.ChipInk(TBD_EUITint.NEUTRAL);
			iconInk = TBD_UITheme.DIM_INK;
		}

		// Rows sit on the TERRAINS panel; the chip sits on the row's own fill.
		int ground = TBD_UITheme.PanelGround();
		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.Paint(m_wAccent, accent);
		TBD_UITheme.Paint(m_wTitle, ink);
		TBD_UITheme.Paint(m_wIcon, iconInk);
		TBD_UITheme.Show(m_wChevron, m_bSelected);

		if (m_CountChip)
		{
			m_CountChip.SetGround(TBD_UITheme.Over(fill, ground));
			if (m_bSelected)
				m_CountChip.SetTint(TBD_EUITint.PRIMARY);
			else
				m_CountChip.SetTint(TBD_EUITint.NEUTRAL);
		}
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnRowActivated(m_iIndex);
	}

	//------------------------------------------------------------------------------------------------
	void SetRowVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
	}
}

class TBD_TerrainSelectorPanel
{
	protected TBD_PanelComponent m_Panel;
	protected Widget m_wBody;
	protected TBD_SearchBoxComponent m_Search;
	protected Widget m_wContent;
	protected Widget m_wEmptyState;
	protected ref TBD_UIScrollBar m_ScrollBar;

	protected TBD_MissionCatalog m_Catalog;
	protected ref array<TBD_TerrainRowComponent> m_aRows;   //!< pool; handlers owned by their widgets
	protected ref array<string> m_aRowKeys;                 //!< terrain key per live row
	protected int m_iLiveRows;
	protected string m_sSelectedKey;
	protected string m_sQuery;

	//! (TBD_TerrainSelectorPanel panel, string terrainKey)
	protected ref ScriptInvoker m_OnSelected;

	//------------------------------------------------------------------------------------------------
	//! `panelRoot` is a mounted `TBD_Panel.layout`. Returns false when the layout tree is missing
	//! pieces; the screen then shows an empty column rather than crashing.
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Pick a terrain. `notify` false = visual only (restoring state).
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

	//------------------------------------------------------------------------------------------------
	string GetSelectedKey()
	{
		return m_sSelectedKey;
	}

	//------------------------------------------------------------------------------------------------
	//! Key of the first terrain in catalog order, or empty.
	string GetFirstKey()
	{
		array<ref TBD_TerrainInfo> terrains = m_Catalog.GetTerrains();
		if (terrains.IsEmpty())
			return string.Empty;

		return terrains[0].m_sKey;
	}

	//------------------------------------------------------------------------------------------------
	//! Put focus on the selected row (else the first). False when there is no row to focus.
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

	//------------------------------------------------------------------------------------------------
	//! (TBD_TerrainSelectorPanel panel, string terrainKey)
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	// ── Called by TBD_TerrainRowComponent ───────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	void OnRowActivated(int index)
	{
		if (index < 0 || index >= m_iLiveRows)
			return;

		Select(m_aRowKeys[index], true);
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnSearchChanged(TBD_SearchBoxComponent box, string query)
	{
		m_sQuery = query;
		Refresh();
	}

	//------------------------------------------------------------------------------------------------
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
