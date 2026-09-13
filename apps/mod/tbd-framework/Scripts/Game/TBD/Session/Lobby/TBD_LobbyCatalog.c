//! Pre-game rebuild (2026-09-13) — the read surface behind the Lobby screen.
//!
//! Shaped like the wire (`TBD_LobbyRoster`: sides -> groups -> slots, each slot a key / role /
//! state / holder / isOwn) plus the presentation the mockups draw and the wire does not carry yet:
//! a faction's role label (`DEFENDING`), a squad's vehicle, a slot's weapon chips and tags
//! (`MED`, `ENG`), and the kit behind a slot. Today `TBD_LobbyMock` builds it; the adapter that
//! wraps `TBD_LobbyClient` implements the same class and calls `Set()`, and no screen notices.
//!
//! Screens hold a catalog and never a raw array. Mutations (`Claim`, `Release`) go through here
//! too, so the mock can answer synchronously and the live adapter asynchronously behind one
//! `GetOnChanged()`.

//! One row of the FACTIONS panel (plus the Spectators row: `m_bSpectators`).
class TBD_LobbyFactionInfo
{
	string m_sKey;          //!< "BLUFOR" — matches TBD_LobbySide.m_sKey
	string m_sName;         //!< "BLUFOR" / "Spectators"
	string m_sRoleLabel;    //!< "DEFENDING"; empty = no chip
	TBD_EUITint m_eTint;    //!< BLUFOR / OPFOR / NEUTRAL
	int m_iSeats;
	bool m_bSpectators;

	void TBD_LobbyFactionInfo(string key, string name, string roleLabel, TBD_EUITint tint, int seats, bool spectators = false)
	{
		m_sKey = key;
		m_sName = name;
		m_sRoleLabel = roleLabel;
		m_eTint = tint;
		m_iSeats = seats;
		m_bSpectators = spectators;
	}
}

//! One seat. `m_sState` is the wire vocabulary verbatim: OPEN | HELD | DEAD.
class TBD_LobbySlotInfo
{
	string m_sKey;          //!< durable slot key — the string Claim() takes
	int m_iIndex;           //!< 1-based position in the squad ("1: Platoon Commander")
	string m_sRole;         //!< "Platoon Commander"
	ref array<string> m_aWeapons; //!< weapon chips: "AK-74", "RPG-7"
	ref array<string> m_aTags;    //!< role tags: "MED", "ENG"
	string m_sState = "OPEN";
	string m_sHolder;       //!< display name; empty when OPEN
	bool m_bOwn;
	string m_sKitKey;       //!< TBD_LobbyCatalog.GetKit() key

	void TBD_LobbySlotInfo(string key, int index, string role, string kitKey)
	{
		m_sKey = key;
		m_iIndex = index;
		m_sRole = role;
		m_sKitKey = kitKey;
		m_aWeapons = {};
		m_aTags = {};
	}

	bool IsOpen()
	{
		return m_sState == "OPEN";
	}

	bool IsDead()
	{
		return m_sState == "DEAD";
	}

	//! "8: Rifleman (AT)" — the roster row and the kit inspector headline.
	string Headline()
	{
		return string.Format("%1: %2", m_iIndex, m_sRole);
	}
}

//! One squad card: callsign chip, vehicle chip, `filled/total`, collapsible slot rows.
class TBD_LobbySquadInfo
{
	string m_sCallsign;     //!< "Alpha 2-1"
	string m_sVehicle;      //!< "BMP-2"; empty = no chip
	ref array<ref TBD_LobbySlotInfo> m_aSlots;

	void TBD_LobbySquadInfo(string callsign, string vehicle)
	{
		m_sCallsign = callsign;
		m_sVehicle = vehicle;
		m_aSlots = {};
	}

	TBD_LobbySlotInfo AddSlot(string role, string kitKey, string weapons = "", string tags = "", string holder = "")
	{
		int index = m_aSlots.Count() + 1;
		string key = string.Format("%1:%2", m_sCallsign, index);
		TBD_LobbySlotInfo slot = new TBD_LobbySlotInfo(key, index, role, kitKey);
		if (!weapons.IsEmpty())
			weapons.Split(",", slot.m_aWeapons, true);
		if (!tags.IsEmpty())
			tags.Split(",", slot.m_aTags, true);
		if (!holder.IsEmpty())
		{
			slot.m_sHolder = holder;
			slot.m_sState = "HELD";
		}

		m_aSlots.Insert(slot);
		return slot;
	}

	int Filled()
	{
		int filled;
		foreach (TBD_LobbySlotInfo slot : m_aSlots)
		{
			if (!slot.IsOpen())
				filled++;
		}

		return filled;
	}

	bool HasOwn()
	{
		foreach (TBD_LobbySlotInfo slot : m_aSlots)
		{
			if (slot.m_bOwn)
				return true;
		}

		return false;
	}
}

//! One labelled line of a kit: `Helmet` / `SSh-68 Steel Helmet`, `Bandages` / `x4`.
class TBD_KitEntry
{
	string m_sLabel;
	string m_sValue;        //!< "None" reads dim
	int m_iCount;           //!< 0 = no count shown
	TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL; //!< WARNING for the mockup's amber `PG-7VL HEAT`

	void TBD_KitEntry(string label, string value, int count = 0, TBD_EUITint tint = TBD_EUITint.NEUTRAL)
	{
		m_sLabel = label;
		m_sValue = value;
		m_iCount = count;
		m_eTint = tint;
	}

	bool IsNone()
	{
		return m_sValue.IsEmpty() || m_sValue == "None";
	}
}

//! One WEAPON SLOT card: name, mounted attachments, ammunition rows + summary.
class TBD_KitWeapon
{
	string m_sSlotLabel;    //!< "WEAPON SLOT 1"
	string m_sName;         //!< "AK-74"
	string m_sAmmoSummary;  //!< "7 MAGS" / "4 ROCKETS"
	ref array<ref TBD_KitEntry> m_aAttachments;
	ref array<ref TBD_KitEntry> m_aAmmo;

	void TBD_KitWeapon(string slotLabel, string name, string ammoSummary)
	{
		m_sSlotLabel = slotLabel;
		m_sName = name;
		m_sAmmoSummary = ammoSummary;
		m_aAttachments = {};
		m_aAmmo = {};
	}
}

//! Everything the KIT INSPECTOR draws for one slot. Sections in mockup order.
class TBD_KitInfo
{
	string m_sKey;
	ref array<ref TBD_KitEntry> m_aGear;
	ref array<ref TBD_KitWeapon> m_aWeapons;
	ref array<ref TBD_KitEntry> m_aGrenades;
	ref array<ref TBD_KitEntry> m_aGadgets;
	ref array<ref TBD_KitEntry> m_aTools;
	ref array<ref TBD_KitEntry> m_aMedical;
	ref array<ref TBD_KitEntry> m_aMisc;

	void TBD_KitInfo(string key)
	{
		m_sKey = key;
		m_aGear = {};
		m_aWeapons = {};
		m_aGrenades = {};
		m_aGadgets = {};
		m_aTools = {};
		m_aMedical = {};
		m_aMisc = {};
	}
}

//! The read + intent surface. Mock until an adapter calls Set().
class TBD_LobbyCatalog
{
	protected static ref TBD_LobbyCatalog s_Instance;

	string m_sMissionId;    //!< top-bar title in mono; overridden by TBD_SessionSelection when set
	ref TBD_SessionIdentity m_Identity;
	ref array<ref TBD_LobbyFactionInfo> m_aFactions;
	ref map<string, ref array<ref TBD_LobbySquadInfo>> m_mSquads; //!< faction key -> squads
	ref map<string, ref TBD_KitInfo> m_mKits;
	protected string m_sOwnKey;

	//! (TBD_LobbyCatalog catalog) — after any roster change (claim / release / live update)
	protected ref ScriptInvoker m_OnChanged;

	//------------------------------------------------------------------------------------------------
	void TBD_LobbyCatalog()
	{
		m_aFactions = {};
		m_mSquads = new map<string, ref array<ref TBD_LobbySquadInfo>>();
		m_mKits = new map<string, ref TBD_KitInfo>();
	}

	//------------------------------------------------------------------------------------------------
	static TBD_LobbyCatalog Get()
	{
		if (!s_Instance)
			s_Instance = TBD_LobbyMock.Build();

		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	//! Replace the catalog (the live adapter, or a fixture). Null restores the mock on the next Get().
	static void Set(TBD_LobbyCatalog catalog)
	{
		s_Instance = catalog;
	}

	// ── Reads ───────────────────────────────────────────────────────────────────────────────

	array<ref TBD_LobbyFactionInfo> GetFactions()
	{
		return m_aFactions;
	}

	TBD_LobbyFactionInfo GetFaction(string key)
	{
		foreach (TBD_LobbyFactionInfo faction : m_aFactions)
		{
			if (faction.m_sKey == key)
				return faction;
		}

		return null;
	}

	//! Squads of a faction in ORBAT order; empty for spectators / unknown keys.
	array<ref TBD_LobbySquadInfo> GetSquads(string factionKey)
	{
		array<ref TBD_LobbySquadInfo> squads;
		if (m_mSquads.Find(factionKey, squads))
			return squads;

		return {};
	}

	//! Seats taken in a faction (for the `0 / 92` chip).
	int CountClaimed(string factionKey)
	{
		int claimed;
		foreach (TBD_LobbySquadInfo squad : GetSquads(factionKey))
		{
			claimed += squad.Filled();
		}

		return claimed;
	}

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

	//! The squad a slot sits in, null when unknown.
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

	TBD_KitInfo GetKit(string kitKey)
	{
		TBD_KitInfo kit;
		if (m_mKits.Find(kitKey, kit))
			return kit;

		return null;
	}

	string GetOwnKey()
	{
		return m_sOwnKey;
	}

	TBD_SessionIdentity GetIdentity()
	{
		return m_Identity;
	}

	//! Mono top-bar title: the scenario picked in the selector, else the catalog's own.
	string GetMissionId()
	{
		if (!TBD_SessionSelection.s_sMissionId.IsEmpty())
			return TBD_SessionSelection.s_sMissionId;

		return m_sMissionId;
	}

	// ── Intents (mock: synchronous; live: the adapter overrides) ────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Take an OPEN seat; releases the current one first. False when the seat is not claimable.
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	ScriptInvoker GetOnChanged()
	{
		if (!m_OnChanged)
			m_OnChanged = new ScriptInvoker();

		return m_OnChanged;
	}

	//------------------------------------------------------------------------------------------------
	protected void NotifyChanged()
	{
		if (m_OnChanged)
			m_OnChanged.Invoke(this);
	}
}
