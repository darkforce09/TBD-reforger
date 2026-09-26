/**
 * @file TBD_KitInfo.c
 * @brief Everything the KIT INSPECTOR draws for one seat, and the two inputs its 3D preview dresses.
 *
 * Role: the kit sheet of one seat.  Position: built by TBD_LobbyMock into TBD_LobbyCatalog; read
 * by TBD_KitInspectorPanel and TBD_KitPreviewComponent.
 * State: plain data.  Invariants: m_sKitAlias and m_Loadout are the two inputs the server's slot
 * body spawn applies (the alias through TBD_Registry, then TBD_LoadoutApplication layers the
 * loadout), so the preview doll wears what the seat spawns; the display arrays are mock text and
 * never null.
 */

//! The kit sheet of one seat, sections in mockup order.
class TBD_KitInfo
{
	string m_sKey; //!< TBD_LobbySlotInfo.m_sKitKey this sheet answers
	string m_sKitAlias; //!< the slot's `kit`, e.g. `kit:sov_rifleman`; resolved through TBD_Registry
	ResourceName m_sBasePrefab;      //!< resolved character prefab; empty = resolve m_sKitAlias on demand
	ref TBD_SlotLoadoutStruct m_Loadout; //!< the JSON loadout the server applies; null = kit-only slot
	ref array<ref TBD_KitEntry> m_aGear; //!< GEAR cells
	ref array<ref TBD_KitWeapon> m_aWeapons; //!< WEAPONS cards; the inspector draws up to three
	ref array<ref TBD_KitEntry> m_aGrenades; //!< GRENADES cells
	ref array<ref TBD_KitEntry> m_aGadgets; //!< GADGETS cells
	ref array<ref TBD_KitEntry> m_aTools; //!< TOOLS cells
	ref array<ref TBD_KitEntry> m_aMedical; //!< MEDICAL cells
	ref array<ref TBD_KitEntry> m_aMisc; //!< MISC cells

	//! Create an empty sheet.
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

	//! The character prefab the preview dresses: the pinned one, else the alias through the registry (the addon ships `Data/registry.json`, so this resolves on clients too).
	//! @return the prefab; empty for an unknown kit
	ResourceName BasePrefab()
	{
		if (!m_sBasePrefab.IsEmpty())
			return m_sBasePrefab;

		if (m_sKitAlias.IsEmpty())
			return string.Empty;

		bool ok;
		ResourceName resolved = TBD_Registry.Resolve(m_sKitAlias, ok);
		if (!ok)
			return string.Empty;

		return resolved;
	}
}
