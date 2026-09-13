//! Pre-game rebuild (2026-09-13) — the Lobby, second Dock & Sub-Layout screen.
//!
//! ```
//!   TopDock     TBD_SessionTopBar        wog_187_chollima… · tabs (Lobby) · Mission Maker ADMIN · 👥 1
//!   LeftDock    TBD_PanelFill + TBD_LobbyFactionPanel   FACTIONS (+ VoiceDock, later pass)
//!   CenterDock  TBD_PanelFill + TBD_LobbyRosterPanel    ROLES
//!   RightDock   TBD_KitInspector + TBD_KitInspectorPanel
//!   BottomDock  TBD_SessionBottomBar     [ Lock Lobby ] [ Ready & Continue ]
//!   OverlayDock popovers
//! ```
//!
//! The screen owns wiring and nothing else: faction -> roster -> kit inspector is two invokers,
//! claim / release live in the catalog. Data comes from `TBD_LobbyCatalog.Get()` (mock today, a
//! `TBD_LobbyClient` adapter later); this file never touches a widget by name outside the docks.
//!
//! Opened through `TBD_MenuStack` (preset `TBD_UILobby`, bound in `Configs/System/chimeraMenus.conf`
//! to `TBD_LobbyScreen.layout` — the shell that replaced the monolith at the same GUID). Reached
//! from the selector's top-bar tab, from the pause menu ("Change slot"), or by `TBD_LobbyStage`.
class TBD_LobbyScreen : TBD_DockScreen
{
	protected ref TBD_LobbyFactionPanel m_Factions;
	protected ref TBD_LobbyRosterPanel m_Roster;
	protected ref TBD_KitInspectorPanel m_Kit;
	protected TBD_LobbyCatalog m_Catalog;

	protected bool m_bLocked;
	protected bool m_bReady;

	static const string ACTION_LOCK = "lock_lobby";
	static const string ACTION_READY = "ready";

	//------------------------------------------------------------------------------------------------
	static void OpenFromPause()
	{
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			return;

		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UILobby);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenOpen()
	{
		m_Catalog = TBD_LobbyCatalog.Get();
		super.OnScreenOpen(); // bars are up after this; they read GetSessionIdentity()

		m_Factions = new TBD_LobbyFactionPanel();
		if (m_Factions.Build(Mount("LeftDock", TBD_UILayouts.PANEL_FILL), m_Catalog))
			m_Factions.GetOnSelected().Insert(OnFactionSelected);

		m_Roster = new TBD_LobbyRosterPanel();
		if (m_Roster.Build(Mount("CenterDock", TBD_UILayouts.PANEL_FILL), m_Catalog))
			m_Roster.GetOnSelected().Insert(OnSlotSelected);

		m_Kit = new TBD_KitInspectorPanel();
		m_Kit.Build(Mount("RightDock", TBD_UILayouts.LOBBY_KIT_INSPECTOR), m_Catalog);

		if (m_BottomBar)
		{
			m_BottomBar.AddAction(ACTION_LOCK, "Lock Lobby");
			m_BottomBar.AddAction(ACTION_READY, "Ready & Continue", true);
		}

		// Your own side first, else the first faction; the cascade fills the roster.
		string factionKey = OwnFactionKey();
		if (factionKey.IsEmpty())
			factionKey = m_Factions.GetFirstKey();

		m_Factions.Select(factionKey, true);

		if (!m_Catalog.GetOwnKey().IsEmpty())
			m_Roster.Select(m_Catalog.GetOwnKey(), true);

		Print("[TBD][lobby] Lobby opened (dock shell).");
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		if (m_Factions)
		{
			m_Factions.GetOnSelected().Remove(OnFactionSelected);
			m_Factions.Destroy();
		}

		if (m_Roster)
		{
			m_Roster.GetOnSelected().Remove(OnSlotSelected);
			m_Roster.Destroy();
		}

		if (m_Kit)
			m_Kit.Destroy();

		m_Factions = null;
		m_Roster = null;
		m_Kit = null;

		Print("[TBD][lobby] Lobby closed.");
		super.OnScreenClose();
	}

	//------------------------------------------------------------------------------------------------
	//! Focus lands on your seat, else the first seat: the next click is "pick a slot".
	override void FocusDefault()
	{
		if (m_Roster && m_Roster.FocusSelected())
			return;

		if (m_Factions && m_Factions.FocusSelected())
			return;

		super.FocusDefault();
	}

	// ── TBD_DockScreen hooks ────────────────────────────────────────────────────────────────

	override protected string GetScreenTitle()
	{
		if (m_Catalog)
			return m_Catalog.GetMissionId();

		return "Lobby";
	}

	override protected bool IsTitleMono()
	{
		return true;
	}

	override protected int GetSessionTab()
	{
		return TBD_ESessionTab.LOBBY;
	}

	override protected TBD_SessionIdentity GetSessionIdentity()
	{
		if (!m_Catalog)
			return null;

		return m_Catalog.GetIdentity();
	}

	//------------------------------------------------------------------------------------------------
	//! Mock pass: the two toggles are the mockup's script (label + tint flip, one log line each).
	//! The wire step routes them to the lobby service; the labels and tints stay.
	override protected void OnBottomAction(TBD_SessionBottomBar bar, string id)
	{
		if (id == ACTION_LOCK)
		{
			m_bLocked = !m_bLocked;
			TBD_UIButton button = bar.GetAction(ACTION_LOCK);
			if (m_bLocked)
			{
				bar.SetActionLabel(ACTION_LOCK, "Unlock Lobby");
				if (button)
					button.SetTint(TBD_EUITint.WARNING);
			}
			else
			{
				bar.SetActionLabel(ACTION_LOCK, "Lock Lobby");
				if (button)
					button.SetTint(TBD_EUITint.PRIMARY);
			}

			Print(string.Format("[TBD][lobby] LOCK LOBBY -> %1", m_bLocked));
			return;
		}

		if (id == ACTION_READY)
		{
			m_bReady = !m_bReady;
			TBD_UIButton button = bar.GetAction(ACTION_READY);
			if (m_bReady)
			{
				bar.SetActionLabel(ACTION_READY, "Ready (Waiting for Admin)");
				if (button)
					button.SetTint(TBD_EUITint.SUCCESS);
			}
			else
			{
				bar.SetActionLabel(ACTION_READY, "Ready & Continue");
				if (button)
					button.SetTint(TBD_EUITint.PRIMARY);
			}

			Print(string.Format("[TBD][lobby] READY -> %1 (seat %2)", m_bReady, m_Catalog.GetOwnKey()));
		}
	}

	// ── Wiring ──────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnFactionSelected(TBD_LobbyFactionPanel panel, string factionKey)
	{
		if (m_Roster)
			m_Roster.SetFaction(factionKey);

		if (m_Kit)
			m_Kit.Show(null, null);
	}

	//------------------------------------------------------------------------------------------------
	protected void OnSlotSelected(TBD_LobbyRosterPanel panel, string slotKey)
	{
		if (!m_Kit || !m_Catalog)
			return;

		m_Kit.Show(m_Catalog.GetSlot(slotKey), m_Catalog.GetSquadOf(slotKey));
	}

	//------------------------------------------------------------------------------------------------
	//! Faction key of the seat you hold, empty when unslotted.
	protected string OwnFactionKey()
	{
		if (!m_Catalog || m_Catalog.GetOwnKey().IsEmpty())
			return string.Empty;

		TBD_LobbySquadInfo own = m_Catalog.GetSquadOf(m_Catalog.GetOwnKey());
		if (!own)
			return string.Empty;

		foreach (TBD_LobbyFactionInfo faction : m_Catalog.GetFactions())
		{
			foreach (TBD_LobbySquadInfo squad : m_Catalog.GetSquads(faction.m_sKey))
			{
				if (squad == own)
					return faction.m_sKey;
			}
		}

		return string.Empty;
	}
}

modded enum ChimeraMenuPreset
{
	TBD_UILobby
}

// -- Pause Menu Integration for Slot Change --------------------------------------------------
modded class PauseMenuUI
{
	protected SCR_ButtonTextComponent m_TbdChangeSlotButton;

	override void OnMenuOpen()
	{
		super.OnMenuOpen();
		HookTbdChangeSlot();
	}

	override void OnMenuClose()
	{
		if (m_TbdChangeSlotButton)
		{
			m_TbdChangeSlotButton.m_OnClicked.Remove(OnTbdChangeSlot);
			m_TbdChangeSlotButton = null;
		}
		super.OnMenuClose();
	}

	protected void HookTbdChangeSlot()
	{
		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		TBD_EGameStage stage = fm.GetStage();
		if (stage != TBD_EGameStage.BRIEFING
			&& stage != TBD_EGameStage.SAFE_START
			&& stage != TBD_EGameStage.LIVE)
			return;

		Widget root = GetRootWidget();
		if (!root)
			return;

		SCR_ButtonTextComponent btn = SCR_ButtonTextComponent.GetButtonText("LeaveFaction", root);
		if (!btn)
			return;

		Widget row = btn.GetRootWidget();
		if (row)
			row.SetVisible(true);

		btn.SetText("Change slot");
		btn.SetEnabled(true);
		btn.m_OnClicked.Insert(OnTbdChangeSlot);
		m_TbdChangeSlotButton = btn;
	}

	protected void OnTbdChangeSlot()
	{
		Close();
		GetGame().GetCallqueue().CallLater(TBD_LobbyScreen.OpenFromPause, 0, false);
	}
}
