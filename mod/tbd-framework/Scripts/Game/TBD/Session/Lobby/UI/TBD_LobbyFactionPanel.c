/**
 * @file TBD_LobbyFactionPanel.c
 * @brief The FACTIONS column of the Lobby screen: one pooled row per catalog faction.
 *
 * Role: fills a mounted TBD_PanelFill.layout titled FACTIONS with TBD_LobbyFactionRowComponent rows,
 * spectators in their own dock, and owns the faction selection.  Position: built by TBD_LobbyScreen
 * from TBD_LobbyCatalog; raises GetOnSelected, which the screen routes to TBD_LobbyRosterPanel.
 * State: the catalog, the pooled rows and their faction keys, and the selected key; client UI only.
 * Invariants: rows are pooled by index and surplus rows hidden; claimed counts are recomputed from
 * the catalog on every Refresh.
 */

//! Controller of the FACTIONS column.
class TBD_LobbyFactionPanel
{
	protected TBD_PanelComponent m_Panel; //!< the host panel; null after Destroy
	protected Widget m_wBody; //!< TBD_LobbyFactionList.layout body
	protected Widget m_wContent; //!< `Content`: the faction rows
	protected Widget m_wSpectatorDock; //!< `SpectatorDock`: the spectator row

	protected TBD_LobbyCatalog m_Catalog; //!< the data source; its changes refresh the rows
	protected ref array<TBD_LobbyFactionRowComponent> m_aRows; //!< pooled row handlers, owned by their widgets
	protected ref array<string> m_aRowKeys; //!< faction key of each visible row, index-aligned with m_aRows
	protected string m_sSelectedKey; //!< the selected faction key; empty for none

	protected ref ScriptInvoker m_OnSelected; //!< (TBD_LobbyFactionPanel panel, string factionKey); created on first GetOnSelected

	//! Mount the faction list into a TBD_PanelFill.layout, subscribe to `catalog` and fill the rows.
	//! @param panelRoot a mounted TBD_PanelFill.layout
	//! @param catalog the data source; may be null (nothing is drawn)
	//! @return false when the panel handler or the body layout is missing
	bool Build(Widget panelRoot, TBD_LobbyCatalog catalog)
	{
		m_Catalog = catalog;
		m_aRows = {};
		m_aRowKeys = {};

		if (!panelRoot)
			return false;

		m_Panel = TBD_PanelComponent.Cast(panelRoot.FindHandler(TBD_PanelComponent));
		if (!m_Panel)
			return false;

		m_Panel.SetTitle("Factions");

		m_wBody = TBD_UILayouts.Create(TBD_UILayouts.LOBBY_FACTION_LIST, m_Panel.GetBodyDock());
		if (!m_wBody)
			return false;

		m_wContent = m_wBody.FindAnyWidget("Content");
		m_wSpectatorDock = m_wBody.FindAnyWidget("SpectatorDock");

		if (m_Catalog)
			m_Catalog.GetOnChanged().Insert(OnCatalogChanged);

		Refresh();
		return true;
	}

	//! Unsubscribe from the catalog and forget the widgets and rows.
	void Destroy()
	{
		if (m_Catalog)
			m_Catalog.GetOnChanged().Remove(OnCatalogChanged);

		m_Catalog = null;
		m_Panel = null;
		m_wBody = null;
		m_wContent = null;
		m_wSpectatorDock = null;
		if (m_aRows)
			m_aRows.Clear();
	}

	//! Rebind every row from the catalog, claimed counts included, and hide surplus pooled rows.
	void Refresh()
	{
		if (!m_wContent || !m_Catalog)
			return;

		m_aRowKeys.Clear();
		int cursor;
		int ground = m_Panel.GetGround();

		foreach (TBD_LobbyFactionInfo faction : m_Catalog.GetFactions())
		{
			Widget container = m_wContent;
			if (faction.m_bSpectators)
				container = m_wSpectatorDock;

			TBD_LobbyFactionRowComponent row = AcquireRow(cursor, container);
			if (!row)
				continue;

			row.Bind(this, cursor, faction, m_Catalog.CountClaimed(faction.m_sKey), ground);
			row.SetSelected(faction.m_sKey == m_sSelectedKey);
			row.SetRowVisible(true);
			m_aRowKeys.Insert(faction.m_sKey);
			cursor++;
		}

		for (int i = cursor; i < m_aRows.Count(); i++)
		{
			m_aRows[i].SetRowVisible(false);
		}
	}

	//! Pick a faction.
	//! @param factionKey the faction to select
	//! @param notify true raises GetOnSelected; false is visual only
	void Select(string factionKey, bool notify)
	{
		m_sSelectedKey = factionKey;

		for (int i = 0; i < m_aRowKeys.Count(); i++)
		{
			m_aRows[i].SetSelected(m_aRowKeys[i] == factionKey);
		}

		if (notify && m_OnSelected)
			m_OnSelected.Invoke(this, factionKey);
	}

	//! @return the selected faction key; empty for none
	string GetSelectedKey()
	{
		return m_sSelectedKey;
	}

	//! @return the key of the first faction in catalog order, or empty; requires a catalog
	string GetFirstKey()
	{
		array<ref TBD_LobbyFactionInfo> factions = m_Catalog.GetFactions();
		if (factions.IsEmpty())
			return string.Empty;

		return factions[0].m_sKey;
	}

	//! Give keyboard focus to the selected row, else the first.
	//! @return false when there is no row to focus
	bool FocusSelected()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace || m_aRowKeys.IsEmpty())
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

	//! @return the (TBD_LobbyFactionPanel panel, string factionKey) invoker raised by a selection
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}


	//! Select the faction of row `index` and notify; out-of-range indices are ignored.
	void OnRowActivated(int index)
	{
		if (index < 0 || index >= m_aRowKeys.Count())
			return;

		Select(m_aRowKeys[index], true);
	}


	//! Refresh on any catalog change.
	protected void OnCatalogChanged(TBD_LobbyCatalog catalog)
	{
		Refresh();
	}

	//! The pooled row at `index`, created in `container` when the pool is shorter.
	//! @return the row, or null when the row layout fails
	protected TBD_LobbyFactionRowComponent AcquireRow(int index, Widget container)
	{
		if (index < m_aRows.Count())
			return m_aRows[index];

		TBD_LobbyFactionRowComponent row = TBD_LobbyFactionRowComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.LOBBY_FACTION_ROW, container, TBD_LobbyFactionRowComponent));
		if (!row)
			return null;

		AlignableSlot.SetHorizontalAlign(row.GetRootWidget(), LayoutHorizontalAlign.Stretch);
		m_aRows.Insert(row);
		return row;
	}
}
