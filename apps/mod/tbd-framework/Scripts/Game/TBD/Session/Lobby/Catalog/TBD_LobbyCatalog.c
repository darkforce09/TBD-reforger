/**
 * @file TBD_LobbyCatalog.c
 * @brief The read and intent surface behind the Lobby screen: factions, squads, seats and kits.
 *
 * Role: serves the screen the roster shape (sides, squads, seats with state, holder and own flag)
 * plus what the roster wire does not carry: a faction's role label, a squad's vehicle, a seat's
 * weapon and tag chips, and the kit behind a seat; Claim and Release change it and announce it.
 * Position: Get() builds TBD_LobbyMock until something calls Set(); read by TBD_LobbyScreen, its
 * panels, TBD_BriefingScreen, TBD_BriefingOrbatPage and TBD_PlayersPanel.
 * State: the process-wide instance, and the catalog's factions, squads, kits and own seat; client
 * UI only.
 * Invariants: screens hold a catalog, never a raw array; every change raises GetOnChanged; Claim
 * takes only an OPEN seat and gives up the current one first; Claim and Release change this copy
 * only and reach no server.
 */

//! The Lobby screen's data and intents; the mock until Set() replaces it.
class TBD_LobbyCatalog
{
	protected static ref TBD_LobbyCatalog s_Instance; //!< the process-wide catalog; null until the first Get or after Set(null)

	string m_sMissionId; //!< top-bar title in mono; TBD_SessionSelection's pick wins when set
	ref TBD_SessionIdentity m_Identity; //!< the viewer's identity for the session bars and the holder name
	ref array<ref TBD_LobbyFactionInfo> m_aFactions; //!< faction rows in display order
	ref map<string, ref array<ref TBD_LobbySquadInfo>> m_mSquads; //!< faction key -> squads
	ref map<string, ref TBD_KitInfo> m_mKits; //!< kit key -> kit sheet
	protected string m_sOwnKey; //!< the viewer's seat; empty when unslotted

	protected ref ScriptInvoker m_OnChanged; //!< (TBD_LobbyCatalog catalog) after any change; created on first GetOnChanged

	//! Create an empty catalog.
	void TBD_LobbyCatalog()
	{
		m_aFactions = {};
		m_mSquads = new map<string, ref array<ref TBD_LobbySquadInfo>>();
		m_mKits = new map<string, ref TBD_KitInfo>();
	}

	//! @return the catalog, building TBD_LobbyMock on first use
	static TBD_LobbyCatalog Get()
	{
		if (!s_Instance)
			s_Instance = TBD_LobbyMock.Build();

		return s_Instance;
	}

	//! Replace the catalog (a live adapter or a fixture); null restores the mock on the next Get.
	static void Set(TBD_LobbyCatalog catalog)
	{
		s_Instance = catalog;
	}


	//! @return the faction rows in display order
	array<ref TBD_LobbyFactionInfo> GetFactions()
	{
		return m_aFactions;
	}

	//! @return the faction row with `key`, or null
	TBD_LobbyFactionInfo GetFaction(string key)
	{
		foreach (TBD_LobbyFactionInfo faction : m_aFactions)
		{
			if (faction.m_sKey == key)
				return faction;
		}

		return null;
	}

	//! @return the squads of a faction in ORBAT order; empty for spectators and unknown keys
	array<ref TBD_LobbySquadInfo> GetSquads(string factionKey)
	{
		array<ref TBD_LobbySquadInfo> squads;
		if (m_mSquads.Find(factionKey, squads))
			return squads;

		return {};
	}

	//! @return seats of a faction that are not OPEN, for the `0 / 92` chip
	int CountClaimed(string factionKey)
	{
		int claimed;
		foreach (TBD_LobbySquadInfo squad : GetSquads(factionKey))
		{
			claimed += squad.Filled();
		}

		return claimed;
	}

	//! @return the seat with `slotKey`, or null
	TBD_LobbySlotInfo GetSlot(string slotKey)
	{
		foreach (TBD_LobbyFactionInfo faction : m_aFactions)
		{
			foreach (TBD_LobbySquadInfo squad : GetSquads(faction.m_sKey))
			{
				foreach (TBD_LobbySlotInfo slot : squad.m_aSlots)
				{
					if (slot.m_sKey == slotKey)
						return slot;
				}
			}
		}

		return null;
	}

	//! @return the squad holding `slotKey`, or null
	TBD_LobbySquadInfo GetSquadOf(string slotKey)
	{
		foreach (TBD_LobbyFactionInfo faction : m_aFactions)
		{
			foreach (TBD_LobbySquadInfo squad : GetSquads(faction.m_sKey))
			{
				foreach (TBD_LobbySlotInfo slot : squad.m_aSlots)
				{
					if (slot.m_sKey == slotKey)
						return squad;
				}
			}
		}

		return null;
	}

	//! @return the kit sheet with `kitKey`, or null
	TBD_KitInfo GetKit(string kitKey)
	{
		TBD_KitInfo kit;
		if (m_mKits.Find(kitKey, kit))
			return kit;

		return null;
	}

	//! @return the viewer's seat key; empty when unslotted
	string GetOwnKey()
	{
		return m_sOwnKey;
	}

	//! @return the viewer's session identity; may be null
	TBD_SessionIdentity GetIdentity()
	{
		return m_Identity;
	}

	//! @return the mono top-bar title: the mission picked in the selector, else the catalog's own
	string GetMissionId()
	{
		if (!TBD_SessionSelection.s_sMissionId.IsEmpty())
			return TBD_SessionSelection.s_sMissionId;

		return m_sMissionId;
	}


	//! Take an OPEN seat for the viewer, giving up the current one first, and raise GetOnChanged.
	//! @return false when the seat is unknown or not OPEN
	bool Claim(string slotKey)
	{
		TBD_LobbySlotInfo slot = GetSlot(slotKey);
		if (!slot || !slot.IsOpen())
			return false;

		Release();

		slot.m_sState = "HELD";
		slot.m_bOwn = true;
		if (m_Identity)
			slot.m_sHolder = string.Format("%1 (You)", m_Identity.m_sName);
		else
			slot.m_sHolder = "You";

		m_sOwnKey = slotKey;
		NotifyChanged();
		return true;
	}

	//! Give up the viewer's seat and raise GetOnChanged; no-op when unslotted.
	void Release()
	{
		if (m_sOwnKey.IsEmpty())
			return;

		TBD_LobbySlotInfo slot = GetSlot(m_sOwnKey);
		if (slot)
		{
			slot.m_sState = "OPEN";
			slot.m_bOwn = false;
			slot.m_sHolder = string.Empty;
		}

		m_sOwnKey = string.Empty;
		NotifyChanged();
	}

	//! @return the (TBD_LobbyCatalog catalog) invoker raised after any change
	ScriptInvoker GetOnChanged()
	{
		if (!m_OnChanged)
			m_OnChanged = new ScriptInvoker();

		return m_OnChanged;
	}

	//! Raise GetOnChanged.
	protected void NotifyChanged()
	{
		if (m_OnChanged)
			m_OnChanged.Invoke(this);
	}
}
