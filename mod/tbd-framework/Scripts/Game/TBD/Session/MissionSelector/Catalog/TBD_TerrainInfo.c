/**
 * @file TBD_TerrainInfo.c
 * @brief One terrain in the Mission Selector's TERRAINS list.
 *
 * Role: names a terrain, its icon and its inspector hero art.  Position: TBD_MissionSelectorMock
 * builds them into TBD_MissionCatalog; TBD_TerrainSelectorPanel and TBD_MissionInspectorPanel read
 * them.
 * State: none beyond its fields.  Invariants: m_sKey matches TBD_MissionSummary.m_sTerrainKey.
 */

//! One entry of the TERRAINS list.
class TBD_TerrainInfo
{
	string m_sKey;           //!< "everon" -- matches TBD_MissionSummary.m_sTerrainKey
	string m_sName;          //!< "Everon"
	string m_sIcon;          //!< TBD_UIIcons key ("water", "landscape", "ac_unit")
	ResourceName m_sHeroImage; //!< inspector hero texture (TBD_UILayouts.HERO_*); empty = the topo-grid art

	//! Build a terrain entry; an empty heroImage uses the topo-grid art.
	void TBD_TerrainInfo(string key, string name, string icon, ResourceName heroImage = "")
	{
		m_sKey = key;
		m_sName = name;
		m_sIcon = icon;
		m_sHeroImage = heroImage;
	}
}
