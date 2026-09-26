/**
 * @file TBD_BriefingCatalog.c
 * @brief The read surface every Briefing page draws from.
 *
 * Role: one briefing's presentation data: mission id, both factions, radio nets, objectives, rule
 * groups, lore, parameters, assets and uniforms for each side, and tactical plans.
 * Position: `Get()` builds it from TBD_BriefingMock until something calls `Set()`; the Briefing screen, its
 * pages and the Markers panel read it on the client.
 * State: one static instance on the client, for the life of the script VM.  Invariants: pages
 * never hold the mock class, so `Set()` is the one swap point; `friendly` selects the reader's side
 * (`GetOwnFaction()`) and false the other; the arrays are allocated in the constructor.
 */

//! Briefing presentation data, served from TBD_BriefingMock until `Set()` replaces it.
class TBD_BriefingCatalog
{
	protected static ref TBD_BriefingCatalog s_Instance; //!< the served catalog; null until first `Get()`

	string m_sMissionId; //!< mission id shown in the top bar when no selection overrides it
	ref TBD_BriefingFaction m_Own; //!< the reader's side
	ref TBD_BriefingFaction m_Enemy; //!< the other side
	string m_sTimeLimit;        //!< "60 Minutes"
	string m_sCaptureSummary;   //!< "Capture 2 of 2"
	ref array<ref TBD_NetInfo> m_aNets; //!< radio nets, long-range first
	ref array<ref TBD_ObjectiveInfo> m_aObjectives; //!< objective cards
	ref array<ref TBD_RuleGroup> m_aRuleGroups; //!< rule groups
	ref array<string> m_aLore; //!< Background paragraphs
	ref array<ref TBD_ParamInfo> m_aParams; //!< Parameters rows
	ref array<ref TBD_AssetTypeInfo> m_aFriendlyAssets; //!< the reader's side's vehicle types
	ref array<ref TBD_AssetTypeInfo> m_aEnemyAssets; //!< the other side's vehicle types
	ref array<ref TBD_UniformInfo> m_aFriendlyUniforms; //!< the reader's side's uniform cards
	ref array<ref TBD_UniformInfo> m_aEnemyUniforms; //!< the other side's uniform cards
	ref array<ref TBD_PlanInfo> m_aPlans; //!< tactical plans for the Markers panel

	//! An empty catalog with every array allocated.
	void TBD_BriefingCatalog()
	{
		m_aNets = {};
		m_aObjectives = {};
		m_aRuleGroups = {};
		m_aLore = {};
		m_aParams = {};
		m_aFriendlyAssets = {};
		m_aEnemyAssets = {};
		m_aFriendlyUniforms = {};
		m_aEnemyUniforms = {};
		m_aPlans = {};
	}

	//! @return the served catalog, built from TBD_BriefingMock on first use
	static TBD_BriefingCatalog Get()
	{
		if (!s_Instance)
			s_Instance = TBD_BriefingMock.Build();

		return s_Instance;
	}

	//! Replace the served catalog. Null restores the mock on the next `Get()`.
	//! @param catalog the catalog to serve, or null
	static void Set(TBD_BriefingCatalog catalog)
	{
		s_Instance = catalog;
	}


	//! @return the mono top-bar title: the mission picked in the selector, else `m_sMissionId`
	string GetMissionId()
	{
		if (!TBD_SessionSelection.s_sMissionId.IsEmpty())
			return TBD_SessionSelection.s_sMissionId;

		return m_sMissionId;
	}

	//! @return the reader's side
	TBD_BriefingFaction GetOwnFaction()   { return m_Own; }
	//! @return the other side
	TBD_BriefingFaction GetEnemyFaction() { return m_Enemy; }
	//! @return the time-limit text
	string GetTimeLimit()                 { return m_sTimeLimit; }
	//! @return the capture summary text
	string GetCaptureSummary()            { return m_sCaptureSummary; }
	//! @return the radio nets
	array<ref TBD_NetInfo> GetNets()      { return m_aNets; }
	//! @return the objective cards
	array<ref TBD_ObjectiveInfo> GetObjectives() { return m_aObjectives; }
	//! @return the rule groups
	array<ref TBD_RuleGroup> GetRuleGroups()     { return m_aRuleGroups; }
	//! @return the Background paragraphs
	array<string> GetLore()               { return m_aLore; }
	//! @return the Parameters rows
	array<ref TBD_ParamInfo> GetParams()  { return m_aParams; }
	//! @return the tactical plans
	array<ref TBD_PlanInfo> GetPlans()    { return m_aPlans; }

	//! @param friendly true for the reader's side
	//! @return that side's vehicle types
	array<ref TBD_AssetTypeInfo> GetAssets(bool friendly)
	{
		if (friendly)
			return m_aFriendlyAssets;

		return m_aEnemyAssets;
	}

	//! @param friendly true for the reader's side
	//! @return that side's uniform cards
	array<ref TBD_UniformInfo> GetUniforms(bool friendly)
	{
		if (friendly)
			return m_aFriendlyUniforms;

		return m_aEnemyUniforms;
	}

	//! @param friendly true for the reader's side
	//! @return that side
	TBD_BriefingFaction GetFaction(bool friendly)
	{
		if (friendly)
			return m_Own;

		return m_Enemy;
	}

	//! @param friendly true for the reader's side
	//! @return the sum of that side's type counts, shown in the page header
	int CountAssets(bool friendly)
	{
		int total;
		foreach (TBD_AssetTypeInfo type : GetAssets(friendly))
		{
			total += type.m_iCount;
		}

		return total;
	}
}
