/**
 * @file TBD_LobbyScreen.c
 * @brief The Lobby dock screen: factions, roles and kit inspector wired together, Lock and Ready.
 *
 * Role: mounts TBD_LobbyFactionPanel (LeftDock), TBD_LobbyRosterPanel (CenterDock) and
 * TBD_KitInspectorPanel (RightDock) over TBD_LobbyCatalog and wires faction -> roster -> kit
 * through their invokers; TopDock and BottomDock are TBD_DockScreen's session bars.
 * Position: opened through TBD_MenuStack on the TBD_UILobby preset (TBD_LobbyScreen.layout) from the
 * top-bar Lobby tab or from PauseMenuUI's "Change slot"; TBD_LobbyStage closes it after LOBBY.
 * State: the three panels, the catalog, and the local Lock and Ready toggles; client UI only.
 * Invariants: the screen owns wiring only and touches no widget by name outside the docks; claim and
 * release live in the catalog; Lock and Ready change labels and log, and reach no server.
 */

//! Lobby screen on the TBD_UILobby preset.
class TBD_LobbyScreen : TBD_DockScreen
{
	protected ref TBD_LobbyFactionPanel m_Factions; //!< FACTIONS column; null when closed
	protected ref TBD_LobbyRosterPanel m_Roster; //!< ROLES column; null when closed
	protected ref TBD_KitInspectorPanel m_Kit; //!< KIT INSPECTOR column; null when closed
	protected TBD_LobbyCatalog m_Catalog; //!< TBD_LobbyCatalog.Get() at open

	protected bool m_bLocked; //!< Lock Lobby toggle; default false
	protected bool m_bReady; //!< Ready & Continue toggle; default false

	static const string ACTION_LOCK = "lock_lobby"; //!< bottom-bar action id of Lock Lobby
	static const string ACTION_READY = "ready"; //!< bottom-bar action id of Ready & Continue

	//! Open the lobby unless it is already open; PauseMenuUI calls it a frame after the pause menu closes.
	//! @authority client
	static void OpenFromPause()
	{
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			return;

		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UILobby);
	}

	//! Build the three panels and the two bottom actions, then select the player's own faction (else the first) and own seat.
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

	//! Unsubscribe and destroy the three panels.
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

	//! Focus the player's seat, else the first seat, else the selected faction.
	override void FocusDefault()
	{
		if (m_Roster && m_Roster.FocusSelected())
			return;

		if (m_Factions && m_Factions.FocusSelected())
			return;

		super.FocusDefault();
	}


	//! @return the catalog's mission id, or `Lobby` without a catalog
	override protected string GetScreenTitle()
	{
		if (m_Catalog)
			return m_Catalog.GetMissionId();

		return "Lobby";
	}

	//! @return true: the title is a mission id, set in the mono face
	override protected bool IsTitleMono()
	{
		return true;
	}

	//! @return TBD_ESessionTab.LOBBY
	override protected int GetSessionTab()
	{
		return TBD_ESessionTab.LOBBY;
	}

	//! @return the catalog's session identity, or null without a catalog
	override protected TBD_SessionIdentity GetSessionIdentity()
	{
		if (!m_Catalog)
			return null;

		return m_Catalog.GetIdentity();
	}

	//! Flip Lock Lobby or Ready & Continue: label, tint and one log line each; neither reaches the server.
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


	//! Show `factionKey`'s squads and clear the kit inspector.
	protected void OnFactionSelected(TBD_LobbyFactionPanel panel, string factionKey)
	{
		if (m_Roster)
			m_Roster.SetFaction(factionKey);

		if (m_Kit)
			m_Kit.Show(null, null);
	}

	//! Show the kit of `slotKey` and its squad.
	protected void OnSlotSelected(TBD_LobbyRosterPanel panel, string slotKey)
	{
		if (!m_Kit || !m_Catalog)
			return;

		m_Kit.Show(m_Catalog.GetSlot(slotKey), m_Catalog.GetSquadOf(slotKey));
	}

	//! @return the faction key of the seat the player holds; empty when unslotted
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
