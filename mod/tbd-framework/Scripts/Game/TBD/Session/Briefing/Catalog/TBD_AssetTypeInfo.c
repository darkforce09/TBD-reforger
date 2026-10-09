/**
 * @file TBD_AssetTypeInfo.c
 * @brief One vehicle type group of the Assets page, and its vehicles.
 *
 * Role: a vehicle type's header, specifications and weapons, with one TBD_AssetInstanceInfo
 * per vehicle and its ammunition and inventory.  Position: TBD_BriefingMock fills it into TBD_BriefingCatalog; the Briefing pages read it.
 * State: plain data.  Invariants: an empty prefab shows no 3D preview; `HasInfo` is false when
 * there are no weapons, road speed or crew, and the Vehicle Info section is then omitted.
 */

//! One vehicle of a type: `BTR-70 Alpha 1-1` with its ammunition and cargo inventory.
class TBD_AssetInstanceInfo
{
	string m_sCallsign; //!< vehicle callsign, such as `Alpha 1-1`
	float m_fX; //!< world X for Locate, metres; 0 with m_fZ 0 means no position
	float m_fZ; //!< world Z for Locate, metres
	ref array<ref TBD_KitEntry> m_aAmmo;        //!< "14.5mm KPVT Belts" / "500 rnds"
	ref array<ref TBD_KitEntry> m_aInvAmmo;     //!< INVENTORY sections, counts in m_iCount
	ref array<ref TBD_KitEntry> m_aInvWeapons; //!< inventory weapons
	ref array<ref TBD_KitEntry> m_aInvGrenades; //!< inventory grenades
	ref array<ref TBD_KitEntry> m_aInvMedical; //!< inventory medical items
	ref array<ref TBD_KitEntry> m_aInvMisc; //!< inventory miscellaneous items

	//! One vehicle with empty ammunition and inventory lists.
	//! @param callsign the vehicle callsign
	//! @param x world X, metres
	//! @param z world Z, metres
	void TBD_AssetInstanceInfo(string callsign, float x = 0, float z = 0)
	{
		m_sCallsign = callsign;
		m_fX = x;
		m_fZ = z;
		m_aAmmo = {};
		m_aInvAmmo = {};
		m_aInvWeapons = {};
		m_aInvGrenades = {};
		m_aInvMedical = {};
		m_aInvMisc = {};
	}
}

//! One vehicle TYPE group of the assets page: header (name - count), Vehicle Info, instances.
class TBD_AssetTypeInfo
{
	string m_sName;           //!< "BTR-70"
	ResourceName m_sPrefab;   //!< rendered in the Vehicle Info box; empty = no preview (crates)
	int m_iCount;             //!< the header badge (instances, or an explicit count for crates)
	ref array<string> m_aWeapons; //!< vehicle weapon names
	string m_sSpeedRoad;      //!< "80 km/h"
	string m_sAmphibious;     //!< "Yes" / "No"
	string m_sSpeedWater;     //!< "10 km/h" / ""
	string m_sCrew;           //!< "2 + Dismount 7 Troops"
	bool m_bExpanded;         //!< first group starts open, as the mockup
	ref array<ref TBD_AssetInstanceInfo> m_aInstances; //!< one entry per vehicle

	//! One vehicle type with no weapons or vehicles yet.
	//! @param name the type name
	//! @param prefab the preview prefab; empty for none
	//! @param count the header count
	void TBD_AssetTypeInfo(string name, ResourceName prefab, int count)
	{
		m_sName = name;
		m_sPrefab = prefab;
		m_iCount = count;
		m_aWeapons = {};
		m_aInstances = {};
	}

	//! Set the mobility and crew specifications.
	//! @return this type, for chaining
	TBD_AssetTypeInfo Specs(string speedRoad, string amphibious, string speedWater, string crew)
	{
		m_sSpeedRoad = speedRoad;
		m_sAmphibious = amphibious;
		m_sSpeedWater = speedWater;
		m_sCrew = crew;
		return this;
	}

	//! @return true when there are weapons, a road speed or a crew line to show
	bool HasInfo()
	{
		return !m_aWeapons.IsEmpty() || !m_sSpeedRoad.IsEmpty() || !m_sCrew.IsEmpty();
	}
}
