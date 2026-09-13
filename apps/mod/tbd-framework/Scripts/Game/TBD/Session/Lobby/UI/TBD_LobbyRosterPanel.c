//! Pre-game rebuild (2026-09-13) — the ROLES column of the Lobby (orbat_panel mockup).
//!
//! ```
//!   ┌ 👤 ROLES ───────────────────────────────────┐
//!   │ ┌ [Alpha 1-1] [UAZ-3151]          [2/3] v ┐ │  <- squad card, header toggles the rows
//!   │ │ 1: PLATOON COMMANDER [AK-74]            │ │
//!   │ │ [1stID] Miller                          │ │  <- HELD: amber holder
//!   │ │ 2: PLATOON SERGEANT  [AK-74]            │ │
//!   │ │ [Unslotted]                             │ │  <- OPEN
//!   │ └─────────────────────────────────────────┘ │
//!   └─────────────────────────────────────────────┘
//! ```
//!
//! Three classes, one file:
//!   * `TBD_LobbySlotRowComponent` — one seat (`TBD_LobbySlotRow.layout`). Contract: `Background`,
//!     `RoleText`, `ChipsDock`, `HolderText`, `StatusChipDock`, `RowRule`.
//!   * `TBD_LobbySquadCardComponent` — one squad (`TBD_LobbySquadCard.layout`). Contract: `Border`,
//!     `Background`, `HeaderButton`, `HeaderBG`, `CallsignChipDock`, `VehicleChipDock`,
//!     `CountChipDock`, `Chevron`, `HeaderRule`, `SlotsContent`.
//!   * `TBD_LobbyRosterPanel` — the controller: `SetFaction(key)` rebuilds the cards (a faction
//!     switch is user-paced; rebuild-not-pool). Click a seat = select it (the kit inspector shows
//!     it); click the selected OPEN seat again = claim; click your own (selected) seat again = release.
//!     Output: `GetOnSelected()(panel, slotKey)`.

class TBD_LobbySlotRowComponent : TBD_UIInteractive
{
	protected Widget m_wBackground;
	protected TextWidget m_wRole;
	protected Widget m_wChipsDock;
	protected TextWidget m_wHolder;
	protected Widget m_wStatusDock;
	protected Widget m_wRule;
	protected TBD_ChipComponent m_StatusChip;
	protected ref array<TBD_ChipComponent> m_aChips;

	protected TBD_LobbyRosterPanel m_Owner; //!< weak
	protected string m_sKey;
	protected bool m_bSelected;
	protected bool m_bOwn;
	protected bool m_bOpen = true;
	protected bool m_bDead;
	protected int m_iGround;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wBackground = w.FindAnyWidget("Background");
		m_wRole = TextWidget.Cast(w.FindAnyWidget("RoleText"));
		m_wChipsDock = w.FindAnyWidget("ChipsDock");
		m_wHolder = TextWidget.Cast(w.FindAnyWidget("HolderText"));
		m_wStatusDock = w.FindAnyWidget("StatusChipDock");
		m_wRule = w.FindAnyWidget("RowRule");
		m_aChips = {};
	}

	//------------------------------------------------------------------------------------------------
	void Bind(TBD_LobbyRosterPanel owner, TBD_LobbySlotInfo slot, int ground, bool last)
	{
		m_Owner = owner;
		m_sKey = slot.m_sKey;
		m_bOwn = slot.m_bOwn;
		m_bOpen = slot.IsOpen();
		m_bDead = slot.IsDead();
		m_iGround = ground;

		string headline = slot.Headline();
		headline.ToUpper();
		TBD_UITheme.Write(m_wRole, headline);

		// Weapon chips (neutral mono) then role tags (MED emerald, ENG cyan-ish -> PRIMARY).
		TBD_UILayouts.Clear(m_wChipsDock);
		m_aChips.Clear();
		foreach (string weapon : slot.m_aWeapons)
		{
			AddChip(weapon, TBD_EUITint.NEUTRAL);
		}

		foreach (string tag : slot.m_aTags)
		{
			TBD_EUITint tint = TBD_EUITint.PRIMARY;
			if (tag == "MED")
				tint = TBD_EUITint.SUCCESS;

			AddChip(tag, tint);
		}

		TBD_UITheme.Write(m_wHolder, slot.m_sHolder);
		TBD_UITheme.Show(m_wRule, !last);

		string status;
		TBD_EUITint statusTint = TBD_EUITint.NEUTRAL;
		if (m_bDead)
		{
			status = "DEAD";
			statusTint = TBD_EUITint.DANGER;
		}
		else if (m_bOpen)
		{
			status = "Unslotted";
		}

		if (status.IsEmpty())
		{
			if (m_StatusChip)
				m_StatusChip.SetChipVisible(false);
		}
		else
		{
			if (!m_StatusChip)
			{
				m_StatusChip = TBD_ChipComponent.Mount(m_wStatusDock, status, statusTint);
				if (m_StatusChip)
					m_StatusChip.SetUppercase(false);
			}
			else
			{
				m_StatusChip.Set(status, statusTint);
			}

			if (m_StatusChip)
				m_StatusChip.SetChipVisible(true);
		}

		SetInteractive(!m_bDead);
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	protected void AddChip(string text, TBD_EUITint tint)
	{
		TBD_ChipComponent chip = TBD_ChipComponent.Mount(m_wChipsDock, text, tint);
		if (!chip)
			return;

		AlignableSlot.SetPadding(chip.GetRootWidget(), 0, 0, 6, 0);
		m_aChips.Insert(chip);
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

		int fill = TBD_UITheme.TRANSPARENT;
		int roleInk = TBD_UITheme.MUTED_INK;
		int holderInk = TBD_UITheme.HOLDER_INK;

		if (m_bOwn)
		{
			fill = TBD_UITheme.CARD_SELECTED_FILL;
			roleInk = TBD_UITheme.PRIMARY_FIXED;
			holderInk = TBD_UITheme.HOLDER_INK;
		}
		else if (m_bSelected)
		{
			fill = TBD_UITheme.CARD_HOVER_FILL;
			roleInk = TBD_UITheme.ON_SURFACE;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill = TBD_UITheme.SLOT_HOVER_FILL;
			roleInk = TBD_UITheme.ON_SURFACE;
		}
		else if (m_bOpen)
		{
			fill = TBD_UITheme.SLOT_OPEN_FILL;
			roleInk = TBD_UITheme.DIM_INK;
		}

		if (m_bDead)
			roleInk = TBD_UITheme.DIM_INK;

		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.PanelGround();

		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wRule, TBD_UITheme.SLOT_RULE, ground);
		TBD_UITheme.Paint(m_wRole, roleInk);
		TBD_UITheme.Paint(m_wHolder, holderInk);
		TBD_UITheme.Show(m_wHolder, !m_bOpen && !m_bDead);

		int rowGround = TBD_UITheme.Over(fill, ground);
		foreach (TBD_ChipComponent chip : m_aChips)
		{
			chip.SetGround(rowGround);
		}

		if (m_StatusChip)
			m_StatusChip.SetGround(rowGround);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnSlotActivated(m_sKey);
	}

	//------------------------------------------------------------------------------------------------
	string GetKey()
	{
		return m_sKey;
	}
}

class TBD_LobbySquadCardComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wHeaderButton;
	protected Widget m_wHeaderBG;
	protected Widget m_wCallsignDock;
	protected Widget m_wVehicleDock;
	protected Widget m_wCountDock;
	protected TextWidget m_wChevron;
	protected Widget m_wHeaderRule;
	protected Widget m_wSlotsContent;

	protected TBD_ChipComponent m_CallsignChip;
	protected TBD_ChipComponent m_VehicleChip;
	protected TBD_ChipComponent m_CountChip;

	protected bool m_bExpanded = true;
	protected int m_iGround;
	protected string m_sCallsign;
	protected TBD_LobbyRosterPanel m_Owner; //!< weak — told when the player folds / unfolds the card

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wHeaderButton = w.FindAnyWidget("HeaderButton");
		m_wHeaderBG = w.FindAnyWidget("HeaderBG");
		m_wCallsignDock = w.FindAnyWidget("CallsignChipDock");
		m_wVehicleDock = w.FindAnyWidget("VehicleChipDock");
		m_wCountDock = w.FindAnyWidget("CountChipDock");
		m_wChevron = TextWidget.Cast(w.FindAnyWidget("Chevron"));
		m_wHeaderRule = w.FindAnyWidget("HeaderRule");
		m_wSlotsContent = w.FindAnyWidget("SlotsContent");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
		// Header band: rounded top, square bottom (its overlay clips the extra 8 px).
		TBD_UILayouts.MountRounded(m_wHeaderBG, TBD_UITheme.RADIUS_ROW - 1);
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	void Bind(TBD_LobbyRosterPanel owner, TBD_LobbySquadInfo squad, int ground, TBD_EUITint tint)
	{
		m_Owner = owner;
		m_sCallsign = squad.m_sCallsign;
		m_iGround = ground;
		int headerGround = TBD_UITheme.Over(TBD_UITheme.SQUAD_HEADER_FILL, TBD_UITheme.Over(TBD_UITheme.SQUAD_CARD_FILL, ground));

		// The callsign wears the side's colour: blue on BLUFOR, red on OPFOR (operator, run 1).
		if (!m_CallsignChip)
			m_CallsignChip = TBD_ChipComponent.Mount(m_wCallsignDock, squad.m_sCallsign, tint, headerGround);
		else
			m_CallsignChip.Set(squad.m_sCallsign, tint);

		if (m_CallsignChip)
			m_CallsignChip.SetUppercase(false);

		if (squad.m_sVehicle.IsEmpty())
		{
			if (m_VehicleChip)
				m_VehicleChip.SetChipVisible(false);
		}
		else
		{
			if (!m_VehicleChip)
				m_VehicleChip = TBD_ChipComponent.Mount(m_wVehicleDock, squad.m_sVehicle, TBD_EUITint.NEUTRAL, headerGround);
			else
				m_VehicleChip.SetText(squad.m_sVehicle);

			if (m_VehicleChip)
				m_VehicleChip.SetChipVisible(true);
		}

		string count = string.Format("%1/%2", squad.Filled(), squad.m_aSlots.Count());
		if (!m_CountChip)
			m_CountChip = TBD_ChipComponent.Mount(m_wCountDock, count, TBD_EUITint.NEUTRAL, headerGround);
		else
			m_CountChip.SetText(count);

		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	void SetExpanded(bool expanded)
	{
		m_bExpanded = expanded;
		TBD_UITheme.Show(m_wSlotsContent, expanded);
		TBD_UITheme.Show(m_wHeaderRule, expanded);
		if (expanded)
			TBD_UITheme.Write(m_wChevron, "v");
		else
			TBD_UITheme.Write(m_wChevron, ">");
	}

	//------------------------------------------------------------------------------------------------
	bool IsExpanded()
	{
		return m_bExpanded;
	}

	//------------------------------------------------------------------------------------------------
	Widget GetSlotsContent()
	{
		return m_wSlotsContent;
	}

	//------------------------------------------------------------------------------------------------
	//! Opaque colour under the slot rows.
	int GetBodyGround()
	{
		return TBD_UITheme.Over(TBD_UITheme.SQUAD_BODY_FILL, TBD_UITheme.Over(TBD_UITheme.SQUAD_CARD_FILL, m_iGround));
	}

	//------------------------------------------------------------------------------------------------
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//------------------------------------------------------------------------------------------------
	//! Clicks bubble up from the header button; a slot row consumes its own.
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (w != m_wHeaderButton || button != 0)
			return false;

		SetExpanded(!m_bExpanded);
		if (m_Owner)
			m_Owner.OnCardToggled(m_sCallsign, m_bExpanded);

		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void Repaint()
	{
		int cardGround = TBD_UITheme.Over(TBD_UITheme.SQUAD_CARD_FILL, m_iGround);
		TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.STRIP_BORDER, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.SQUAD_BODY_FILL, cardGround);
		TBD_UITheme.PaintOver(m_wHeaderBG, TBD_UITheme.SQUAD_HEADER_FILL, cardGround);
		TBD_UITheme.PaintOver(m_wHeaderRule, TBD_UITheme.STRIP_BORDER, cardGround);
		TBD_UITheme.Paint(m_wChevron, TBD_UITheme.MUTED_INK);
	}
}

class TBD_LobbyRosterPanel
{
	protected TBD_PanelComponent m_Panel;
	protected Widget m_wBody;
	protected Widget m_wContent;
	protected Widget m_wEmptyState;
	protected ref TBD_UIScrollBar m_ScrollBar;

	protected TBD_LobbyCatalog m_Catalog;
	protected string m_sFactionKey;
	protected string m_sSelectedKey;
	protected ref array<TBD_LobbySquadCardComponent> m_aCards;
	protected ref array<TBD_LobbySlotRowComponent> m_aRows;
	protected ref map<string, bool> m_mCollapsed; //!< callsign -> user collapsed it (survives rebuilds)

	//! (TBD_LobbyRosterPanel panel, string slotKey)
	protected ref ScriptInvoker m_OnSelected;

	//------------------------------------------------------------------------------------------------
	//! `panelRoot` is a mounted `TBD_PanelFill.layout`.
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

	//------------------------------------------------------------------------------------------------
	void Destroy()
	{
		if (m_Catalog)
			m_Catalog.GetOnChanged().Remove(OnCatalogChanged);

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

	//------------------------------------------------------------------------------------------------
	//! Point the roster at a faction and rebuild its squad cards.
	void SetFaction(string factionKey)
	{
		m_sFactionKey = factionKey;
		Rebuild();
	}

	//------------------------------------------------------------------------------------------------
	//! Highlight a seat. `notify` false = visual only.
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

	//------------------------------------------------------------------------------------------------
	string GetSelectedKey()
	{
		return m_sSelectedKey;
	}

	//------------------------------------------------------------------------------------------------
	//! Focus the selected seat, else your own, else the first visible one.
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

	//------------------------------------------------------------------------------------------------
	//! (TBD_LobbyRosterPanel panel, string slotKey)
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	// ── Called by TBD_LobbySlotRowComponent ────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! First click selects; the second click on an OPEN seat claims it; the second click on your
	//! own seat releases it. The catalog answers through GetOnChanged(), which rebuilds the cards.
	void OnSlotActivated(string slotKey)
	{
		if (!m_Catalog)
			return;

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

	//------------------------------------------------------------------------------------------------
	//! A folded card stays folded across rebuilds (claims rebuild the whole list).
	void OnCardToggled(string callsign, bool expanded)
	{
		m_mCollapsed.Set(callsign, !expanded);
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnCatalogChanged(TBD_LobbyCatalog catalog)
	{
		Rebuild();
	}

	//------------------------------------------------------------------------------------------------
	protected void Rebuild()
	{
		if (!m_wContent || !m_Catalog)
			return;

		TBD_UILayouts.Clear(m_wContent);
		m_aCards.Clear();
		m_aRows.Clear();

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
