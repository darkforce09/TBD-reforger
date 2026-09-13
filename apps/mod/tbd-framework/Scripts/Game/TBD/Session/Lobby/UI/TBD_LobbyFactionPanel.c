//! Pre-game rebuild (2026-09-13) — the FACTIONS column of the Lobby (lobby_sidebar mockup).
//!
//! ```
//!   ┌ FACTIONS ─────────────────────────┐
//!   │ BLUFOR  [DEFENDING]      [0 / 92] │  <- selected: brighter tinted border
//!   │ OPFOR   [ATTACKING]      [0 / 95] │
//!   │                                   │
//!   │ Spectators               [0 / 10] │
//!   │ (VoiceDock — voice panel, later)  │
//!   └───────────────────────────────────┘
//! ```
//!
//! Two classes, one file:
//!   * `TBD_LobbyFactionRowComponent` — one pooled row (`TBD_LobbyFactionRow.layout`). Contract:
//!     `Border`, `Background`, `Name`, `RoleChipDock`, `CountChipDock`.
//!   * `TBD_LobbyFactionPanel` — the controller: fills a mounted `TBD_PanelFill` (title FACTIONS),
//!     owns the selection. `GetOnSelected()(panel, factionKey)` is its only output; the seat
//!     counts are recomputed from the catalog on every `Refresh()`.

class TBD_LobbyFactionRowComponent : TBD_UIInteractive
{
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected TextWidget m_wName;
	protected Widget m_wRoleDock;
	protected Widget m_wCountDock;
	protected TBD_ChipComponent m_RoleChip;
	protected TBD_ChipComponent m_CountChip;

	protected TBD_LobbyFactionPanel m_Owner; //!< weak
	protected int m_iIndex = -1;
	protected TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL;
	protected bool m_bSelected;
	protected int m_iGround;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wName = TextWidget.Cast(w.FindAnyWidget("Name"));
		m_wRoleDock = w.FindAnyWidget("RoleChipDock");
		m_wCountDock = w.FindAnyWidget("CountChipDock");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
	}

	//------------------------------------------------------------------------------------------------
	void Bind(TBD_LobbyFactionPanel owner, int index, TBD_LobbyFactionInfo faction, int claimed, int ground)
	{
		m_Owner = owner;
		m_iIndex = index;
		m_eTint = faction.m_eTint;
		m_iGround = ground;

		TBD_UITheme.Write(m_wName, faction.m_sName);

		if (faction.m_sRoleLabel.IsEmpty())
		{
			if (m_RoleChip)
				m_RoleChip.SetChipVisible(false);
		}
		else
		{
			if (!m_RoleChip)
				m_RoleChip = TBD_ChipComponent.Mount(m_wRoleDock, faction.m_sRoleLabel, TBD_EUITint.NEUTRAL);
			else
				m_RoleChip.SetText(faction.m_sRoleLabel);

			if (m_RoleChip)
				m_RoleChip.SetChipVisible(true);
		}

		string count = string.Format("%1 / %2", claimed, faction.m_iSeats);
		if (!m_CountChip)
			m_CountChip = TBD_ChipComponent.Mount(m_wCountDock, count, TBD_EUITint.NEUTRAL);
		else
			m_CountChip.SetText(count);

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

		bool hovered = IsHighlighted() && m_bInteractive;
		int fill = TBD_UITheme.FactionRowFill(m_eTint, hovered || m_bSelected); // selected = lit like hover
		int border = TBD_UITheme.FactionRowBorder(m_eTint, m_bSelected);
		int ink = TBD_UITheme.FactionRowInk(m_eTint);
		if (m_eTint == TBD_EUITint.NEUTRAL && (hovered || m_bSelected))
			ink = TBD_UITheme.ON_SURFACE;

		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.PanelGround();

		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.Paint(m_wName, ink);

		int rowGround = TBD_UITheme.Over(fill, ground);
		if (m_RoleChip)
			m_RoleChip.SetGround(rowGround);

		if (m_CountChip)
			m_CountChip.SetGround(rowGround);
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

class TBD_LobbyFactionPanel
{
	protected TBD_PanelComponent m_Panel;
	protected Widget m_wBody;
	protected Widget m_wContent;
	protected Widget m_wSpectatorDock;

	protected TBD_LobbyCatalog m_Catalog;
	protected ref array<TBD_LobbyFactionRowComponent> m_aRows; //!< handlers owned by their widgets
	protected ref array<string> m_aRowKeys;
	protected string m_sSelectedKey;

	//! (TBD_LobbyFactionPanel panel, string factionKey)
	protected ref ScriptInvoker m_OnSelected;

	//------------------------------------------------------------------------------------------------
	//! `panelRoot` is a mounted `TBD_PanelFill.layout`.
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Rebind every row from the catalog (seat counts included).
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

	//------------------------------------------------------------------------------------------------
	//! Pick a faction. `notify` false = visual only.
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

	//------------------------------------------------------------------------------------------------
	string GetSelectedKey()
	{
		return m_sSelectedKey;
	}

	//------------------------------------------------------------------------------------------------
	//! Key of the first faction in catalog order, or empty.
	string GetFirstKey()
	{
		array<ref TBD_LobbyFactionInfo> factions = m_Catalog.GetFactions();
		if (factions.IsEmpty())
			return string.Empty;

		return factions[0].m_sKey;
	}

	//------------------------------------------------------------------------------------------------
	//! Put focus on the selected row (else the first). False when there is no row to focus.
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

	//------------------------------------------------------------------------------------------------
	//! (TBD_LobbyFactionPanel panel, string factionKey)
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	// ── Called by TBD_LobbyFactionRowComponent ──────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	void OnRowActivated(int index)
	{
		if (index < 0 || index >= m_aRowKeys.Count())
			return;

		Select(m_aRowKeys[index], true);
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnCatalogChanged(TBD_LobbyCatalog catalog)
	{
		Refresh();
	}

	//------------------------------------------------------------------------------------------------
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
