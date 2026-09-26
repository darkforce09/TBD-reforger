/**
 * @file TBD_UniformInfo.c
 * @brief One uniform card of the Uniforms page.
 *
 * Role: name, doll prefab, weapon chips and camouflage name of one faction component.  Position: TBD_BriefingMock fills it into TBD_BriefingCatalog; the Briefing pages read it.
 * State: plain data.  Invariants: a pinned prefab wins over the kit alias; an alias that
 * TBD_Registry cannot resolve yields an empty prefab, and the doll shows nothing.
 */

//! One uniform card: a faction component, its doll, its weapon chips and camo name.
class TBD_UniformInfo
{
	string m_sName;        //!< "CDF (12th Mechanized)"
	string m_sKitAlias;    //!< "kit:sov_rifleman" -- the doll's prefab through TBD_Registry
	ResourceName m_sPrefab; //!< pinned prefab; empty = resolve the alias
	ref array<string> m_aChips; //!< "MAG", "AK"
	string m_sCamo;        //!< "TTsKO / Dubok"

	//! One uniform card with no chips.
	//! @param name the card name
	//! @param kitAlias the kit alias resolved when no prefab is pinned
	//! @param prefab the pinned prefab; empty to resolve the alias
	//! @param camo the camouflage name
	void TBD_UniformInfo(string name, string kitAlias, ResourceName prefab, string camo)
	{
		m_sName = name;
		m_sKitAlias = kitAlias;
		m_sPrefab = prefab;
		m_sCamo = camo;
		m_aChips = {};
	}

	//! @return the pinned prefab, else the alias resolved through TBD_Registry, else empty
	ResourceName Prefab()
	{
		if (!m_sPrefab.IsEmpty())
			return m_sPrefab;

		bool ok;
		ResourceName resolved = TBD_Registry.Resolve(m_sKitAlias, ok);
		if (!ok)
			return string.Empty;

		return resolved;
	}
}
