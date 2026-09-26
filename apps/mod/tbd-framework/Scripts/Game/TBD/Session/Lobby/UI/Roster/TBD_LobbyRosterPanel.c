/**
 * @file TBD_LobbyRosterPanel.c
 * @brief The ROLES column of the Lobby screen: one squad card per squad of the chosen faction.
 *
 * Role: rebuilds squad cards and seat rows from TBD_LobbyCatalog for one faction, keeps the
 * selection, and turns seat clicks into select, claim or release.  Position: built by
 * TBD_LobbyScreen (editable) and TBD_BriefingOrbatPage (read-only with Locate buttons) into a
 * mounted TBD_PanelFill.layout; raises GetOnSelected and GetOnLocate to them.
 * State: the catalog, faction key, selected key, the cards, rows and Locate buttons of the last
 * rebuild, and the callsigns the player folded; client UI only.
 * Invariants: a faction switch or a catalog change rebuilds every card (no pooling); a folded card
 * stays folded across rebuilds unless it holds the player's own seat; read-only mode only selects.
 */

//! Controller of the ROLES column.
class TBD_LobbyRosterPanel
{
	protected TBD_PanelComponent m_Panel; //!< the host panel; null after Destroy
	protected Widget m_wBody; //!< TBD_LobbyRoster.layout body
	protected Widget m_wContent; //!< `Content`: the squad cards
	protected Widget m_wEmptyState; //!< `EmptyState`: shown when the faction has no squads
	protected ref TBD_UIScrollBar m_ScrollBar; //!< scroll bar over `Scroll`

	protected TBD_LobbyCatalog m_Catalog; //!< the data source; its changes rebuild the cards
	protected string m_sFactionKey; //!< the faction shown
	protected string m_sSelectedKey; //!< the selected slot key; empty for none
	protected ref array<TBD_LobbySquadCardComponent> m_aCards; //!< the cards of the last rebuild
	protected ref array<TBD_LobbySlotRowComponent> m_aRows; //!< the seat rows of the last rebuild
	protected ref map<string, bool> m_mCollapsed; //!< callsign -> the player folded it; survives rebuilds
	protected bool m_bReadOnly; //!< briefing ORBAT: select only, never claim or release; default false
	protected bool m_bShowLocate; //!< briefing ORBAT: a Locate button on every squad header; default false
	protected ref array<TBD_UIButton> m_aLocateButtons; //!< Locate buttons of the last rebuild, index-aligned with m_aLocateCallsigns
	protected ref array<string> m_aLocateCallsigns; //!< callsign of each Locate button
	protected ref ScriptInvoker m_OnLocate; //!< (TBD_LobbyRosterPanel panel, string callsign); created on first GetOnLocate

	protected ref ScriptInvoker m_OnSelected; //!< (TBD_LobbyRosterPanel panel, string slotKey); created on first GetOnSelected

	//! Mount the roster body into a TBD_PanelFill.layout and subscribe to `catalog`.
	//! @param panelRoot a mounted TBD_PanelFill.layout
	//! @param catalog the data source; may be null (nothing is drawn)
	//! @return false when the panel handler or the body layout is missing
	bool Build(Widget panelRoot, TBD_LobbyCatalog catalog)
	{
		m_Catalog = catalog;
		m_aCards = {};
		m_aRows = {};
		m_mCollapsed = new map<string, bool>();

		if (!panelRoot)
			return false;

		m_Panel = TBD_PanelComponent.Cast(panelRoot.FindHandler(TBD_PanelComponent));
		if (!m_Panel)
			return false;

		m_Panel.SetTitle("Roles");
		m_Panel.SetIcon("person");

		m_wBody = TBD_UILayouts.Create(TBD_UILayouts.LOBBY_ROSTER, m_Panel.GetBodyDock());
		if (!m_wBody)
			return false;

		m_wContent = m_wBody.FindAnyWidget("Content");
		m_wEmptyState = m_wBody.FindAnyWidget("EmptyState");
		TBD_UITheme.Paint(m_wEmptyState, TBD_UITheme.MUTED_INK);
		m_ScrollBar = TBD_UIScrollBar.Mount(m_wBody.FindAnyWidget("ScrollBarDock"), ScrollLayoutWidget.Cast(m_wBody.FindAnyWidget("Scroll")), m_wContent, m_Panel.GetGround());

		if (m_Catalog)
			m_Catalog.GetOnChanged().Insert(OnCatalogChanged);

		return true;
	}

	//! Unsubscribe from the catalog, drop the Locate buttons and scroll bar, and forget the widgets.
	void Destroy()
	{
		if (m_Catalog)
			m_Catalog.GetOnChanged().Remove(OnCatalogChanged);

		ClearLocateButtons();

		if (m_ScrollBar)
			m_ScrollBar.Destroy();

		m_ScrollBar = null;
		m_Catalog = null;
		m_Panel = null;
		m_wBody = null;
		m_wContent = null;
		if (m_aCards)
			m_aCards.Clear();
		if (m_aRows)
			m_aRows.Clear();
	}

	//! Read-only mode (the briefing ORBAT): clicks select, and the kit inspector follows, but never claim or release.
	void SetReadOnly(bool readOnly)
	{
		m_bReadOnly = readOnly;
	}

	//! Give every squad header a Locate button from the next rebuild on; clicks raise GetOnLocate.
	void SetShowLocate(bool show)
	{
		m_bShowLocate = show;
	}

	//! @return the (TBD_LobbyRosterPanel panel, string callsign) invoker raised by a Locate button
	ScriptInvoker GetOnLocate()
	{
		if (!m_OnLocate)
			m_OnLocate = new ScriptInvoker();

		return m_OnLocate;
	}

	//! Raise GetOnLocate with the callsign of the clicked Locate button.
	protected void OnLocate(TBD_UIButton button)
	{
		if (!m_aLocateButtons || !m_OnLocate)
			return;

		int index = m_aLocateButtons.Find(button);
		if (index < 0)
			return;

		m_OnLocate.Invoke(this, m_aLocateCallsigns[index]);
	}

	//! Unsubscribe and forget every Locate button.
	protected void ClearLocateButtons()
	{
		if (m_aLocateButtons)
		{
			foreach (TBD_UIButton button : m_aLocateButtons)
			{
				if (button)
					button.GetOnActivate().Remove(OnLocate);
			}
		}

		m_aLocateButtons = {};
		m_aLocateCallsigns = {};
	}

	//! Show `factionKey`'s squads, rebuilding every card.
	void SetFaction(string factionKey)
	{
		m_sFactionKey = factionKey;
		Rebuild();
	}

	//! Highlight a seat.
	//! @param slotKey the seat to select
	//! @param notify true raises GetOnSelected; false is visual only
	void Select(string slotKey, bool notify)
	{
		m_sSelectedKey = slotKey;
		foreach (TBD_LobbySlotRowComponent row : m_aRows)
		{
			row.SetSelected(row.GetKey() == slotKey);
		}

		if (notify && m_OnSelected)
			m_OnSelected.Invoke(this, slotKey);
	}

	//! @return the selected slot key; empty for none
	string GetSelectedKey()
	{
		return m_sSelectedKey;
	}

	//! Give keyboard focus to the selected seat, else the player's own, else the first row.
	//! @return false without a workspace, rows or a target widget
	bool FocusSelected()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace || m_aRows.IsEmpty())
			return false;

		Widget target;
		foreach (TBD_LobbySlotRowComponent row : m_aRows)
		{
			if (row.GetKey() == m_sSelectedKey || (m_Catalog && row.GetKey() == m_Catalog.GetOwnKey()))
			{
				target = row.GetRootWidget();
				break;
			}
		}

		if (!target)
			target = m_aRows[0].GetRootWidget();

		if (!target)
			return false;

		workspace.SetFocusedWidget(target);
		return true;
	}

	//! @return the (TBD_LobbyRosterPanel panel, string slotKey) invoker raised by a selection
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}


	//! Handle a seat click: the first click selects; a second click on an OPEN seat claims it; a second click on the player's own seat releases it; read-only mode only selects. The catalog answers through GetOnChanged, which rebuilds the cards.
	void OnSlotActivated(string slotKey)
	{
		if (!m_Catalog)
			return;

		if (m_bReadOnly)
		{
			Select(slotKey, true);
			return;
		}

		TBD_LobbySlotInfo slot = m_Catalog.GetSlot(slotKey);
		if (!slot)
			return;

		if (slot.m_bOwn && slotKey == m_sSelectedKey)
		{
			Print(string.Format("[TBD][lobby] RELEASE %1", slotKey));
			m_Catalog.Release();
			Select(slotKey, true);
			return;
		}

		if (slotKey == m_sSelectedKey && slot.IsOpen())
		{
			Print(string.Format("[TBD][lobby] CLAIM %1", slotKey));
			m_Catalog.Claim(slotKey);
			Select(slotKey, true);
			return;
		}

		Select(slotKey, true);
	}

	//! Remember a fold so the card stays folded across rebuilds.
	void OnCardToggled(string callsign, bool expanded)
	{
		m_mCollapsed.Set(callsign, !expanded);
	}


	//! Rebuild on any catalog change.
	protected void OnCatalogChanged(TBD_LobbyCatalog catalog)
	{
		Rebuild();
	}

	//! Recreate every squad card, Locate button and seat row for the current faction, keeping the selection and folds.
	protected void Rebuild()
	{
		if (!m_wContent || !m_Catalog)
			return;

		TBD_UILayouts.Clear(m_wContent);
		m_aCards.Clear();
		m_aRows.Clear();
		ClearLocateButtons();

		array<ref TBD_LobbySquadInfo> squads = m_Catalog.GetSquads(m_sFactionKey);
		TBD_UITheme.Show(m_wEmptyState, squads.IsEmpty());

		int ground = m_Panel.GetGround();
		TBD_EUITint tint = TBD_EUITint.PRIMARY;
		TBD_LobbyFactionInfo faction = m_Catalog.GetFaction(m_sFactionKey);
		if (faction && faction.m_eTint != TBD_EUITint.NEUTRAL)
			tint = faction.m_eTint;

		foreach (TBD_LobbySquadInfo squad : squads)
		{
			TBD_LobbySquadCardComponent card = TBD_LobbySquadCardComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.LOBBY_SQUAD_CARD, m_wContent, TBD_LobbySquadCardComponent));
			if (!card)
				continue;

			AlignableSlot.SetHorizontalAlign(card.GetRootWidget(), LayoutHorizontalAlign.Stretch);
			card.Bind(this, squad, ground, tint);
			m_aCards.Insert(card);

			if (m_bShowLocate)
			{
				Widget buttonRoot = TBD_UILayouts.Create(TBD_UILayouts.BUTTON, card.GetActionDock());
				TBD_UIButton locate;
				if (buttonRoot)
					locate = TBD_UIButton.Cast(buttonRoot.FindHandler(TBD_UIButton));
				if (locate)
				{
					locate.SetLabel("Locate");
					locate.SetPrimary(false);
					locate.GetOnActivate().Insert(OnLocate);
					m_aLocateButtons.Insert(locate);
					m_aLocateCallsigns.Insert(squad.m_sCallsign);
				}
			}

			int rowGround = card.GetBodyGround();
			int count = squad.m_aSlots.Count();
			for (int i = 0; i < count; i++)
			{
				TBD_LobbySlotRowComponent row = TBD_LobbySlotRowComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.LOBBY_SLOT_ROW, card.GetSlotsContent(), TBD_LobbySlotRowComponent));
				if (!row)
					continue;

				AlignableSlot.SetHorizontalAlign(row.GetRootWidget(), LayoutHorizontalAlign.Stretch);
				row.Bind(this, squad.m_aSlots[i], rowGround, i == count - 1);
				row.SetSelected(squad.m_aSlots[i].m_sKey == m_sSelectedKey);
				m_aRows.Insert(row);
			}

			// Every card open by default; your own squad always.
			bool collapsed;
			if (m_mCollapsed.Find(squad.m_sCallsign, collapsed) && collapsed && !squad.HasOwn())
				card.SetExpanded(false);
			else
				card.SetExpanded(true);
		}
	}
}
