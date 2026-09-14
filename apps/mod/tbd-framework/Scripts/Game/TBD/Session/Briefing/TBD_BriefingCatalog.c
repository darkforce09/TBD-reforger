//! Briefing pass (2026-09-14) — the read surface behind the rebuilt Briefing screen.
//!
//! Presentation catalog, wire-shaped: every model here is what ONE briefing page draws, in the
//! words the mockups use, and each has an obvious source on the wire when the adapter lands —
//! `TBD_BriefingPayload` (orders → Background, zones + win mode → Objectives, end conditions →
//! Rules), `TBD_RadioClient` (nets → Frequencies), `TBD_MarkerClient` (plans → Markers),
//! `TBD_LobbyCatalog` (ORBAT, identity), `TBD_MissionVehicleRoster` (assets — server-only today,
//! so that one needs a wire line first). `TBD_BriefingMock.Build()` fills it until then; screens
//! never hold the mock class (`Get()` / `Set()` is the swap point, as for the lobby).
//!
//! Faction discipline: the briefing is written for the reader's side. `GetOwnFaction()` is the
//! side whose assets / uniforms are "friendly"; `GetEnemyFaction()` the other. Tints come from
//! `TBD_EUITint.BLUFOR / OPFOR`, so an enemy page is the friendly builder painted red.

//! One side as the briefing names it.
class TBD_BriefingFaction
{
	string m_sKey;        //!< "BLUFOR"
	string m_sName;       //!< "BLUFOR"
	string m_sRoleLabel;  //!< "DEFENDERS" / "ATTACKERS" — the header chip
	TBD_EUITint m_eTint;

	void TBD_BriefingFaction(string key, string name, string roleLabel, TBD_EUITint tint)
	{
		m_sKey = key;
		m_sName = name;
		m_sRoleLabel = roleLabel;
		m_eTint = tint;
	}
}

//! One radio net row: `A1-2  1st Platoon, 2nd Squad   163.7 MHz` + aux channels.
class TBD_NetInfo
{
	string m_sCallsign;   //!< "A1-1"; empty on the LR net
	string m_sLabel;      //!< "Company HQ (Mission Maker)"
	float m_fFreqMHz;
	ref array<float> m_aAux;
	bool m_bLongRange;
	bool m_bOwn;          //!< the reader's own squad net (highlighted)

	void TBD_NetInfo(string callsign, string label, float freqMHz, bool longRange = false, bool own = false)
	{
		m_sCallsign = callsign;
		m_sLabel = label;
		m_fFreqMHz = freqMHz;
		m_bLongRange = longRange;
		m_bOwn = own;
		m_aAux = {};
	}

	TBD_NetInfo Aux(float a, float b, float c)
	{
		m_aAux.Insert(a);
		m_aAux.Insert(b);
		m_aAux.Insert(c);
		return this;
	}

	string Title()
	{
		if (m_sCallsign.IsEmpty())
			return m_sLabel;

		return m_sCallsign + " " + m_sLabel;
	}

	string FreqText()
	{
		return string.Format("%1 MHz", m_fFreqMHz.ToString(-1, 1));
	}

	string AuxText()
	{
		if (m_aAux.IsEmpty())
			return string.Empty;

		string text;
		foreach (int i, float aux : m_aAux)
		{
			if (i > 0)
				text += ", ";
			text += aux.ToString(-1, 1);
		}

		return text + " MHz";
	}
}

//! One objective card.
class TBD_ObjectiveInfo
{
	int m_iIndex;
	string m_sTitle;        //!< "Southern Zone"
	string m_sRoleLabel;    //!< "Defend" / "Capture" — the pill
	string m_sType;         //!< "Sector"
	string m_sCaptureTime;  //!< "60s"
	string m_sRetake;       //!< "Permanent (Locked)"
	float m_fX;             //!< world X for Locate (map pan)
	float m_fZ;

	void TBD_ObjectiveInfo(int index, string title, string roleLabel, string type, string captureTime, string retake, float x, float z)
	{
		m_iIndex = index;
		m_sTitle = title;
		m_sRoleLabel = roleLabel;
		m_sType = type;
		m_sCaptureTime = captureTime;
		m_sRetake = retake;
		m_fX = x;
		m_fZ = z;
	}
}

class TBD_RuleInfo
{
	string m_sTitle;
	string m_sBody;

	void TBD_RuleInfo(string title, string body)
	{
		m_sTitle = title;
		m_sBody = body;
	}
}

//! "Mission Rules" / "General Rules" — a collapsible group of numbered rules.
class TBD_RuleGroup
{
	string m_sTitle;
	bool m_bOpen;
	ref array<ref TBD_RuleInfo> m_aRules;

	void TBD_RuleGroup(string title, bool open = true)
	{
		m_sTitle = title;
		m_bOpen = open;
		m_aRules = {};
	}

	void Add(string title, string body)
	{
		m_aRules.Insert(new TBD_RuleInfo(title, body));
	}
}

//! One parameters row: icon · label · mono value.
class TBD_ParamInfo
{
	string m_sIcon;   //!< TBD_UIIcons key
	string m_sLabel;
	string m_sValue;
	TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL; //!< WARNING for the amber Safe Start value

	void TBD_ParamInfo(string icon, string label, string value, TBD_EUITint tint = TBD_EUITint.NEUTRAL)
	{
		m_sIcon = icon;
		m_sLabel = label;
		m_sValue = value;
		m_eTint = tint;
	}
}

//! One vehicle of a type: `BTR-70 Alpha 1-1` with its ammunition and cargo inventory.
class TBD_AssetInstanceInfo
{
	string m_sCallsign;
	float m_fX;
	float m_fZ;
	ref array<ref TBD_KitEntry> m_aAmmo;        //!< "14.5mm KPVT Belts" / "500 rnds"
	ref array<ref TBD_KitEntry> m_aInvAmmo;     //!< INVENTORY sections, counts in m_iCount
	ref array<ref TBD_KitEntry> m_aInvWeapons;
	ref array<ref TBD_KitEntry> m_aInvGrenades;
	ref array<ref TBD_KitEntry> m_aInvMedical;
	ref array<ref TBD_KitEntry> m_aInvMisc;

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

//! One vehicle TYPE group of the assets page: header (name · count), Vehicle Info, instances.
class TBD_AssetTypeInfo
{
	string m_sName;           //!< "BTR-70"
	ResourceName m_sPrefab;   //!< rendered in the Vehicle Info box; empty = no preview (crates)
	int m_iCount;             //!< the header badge (instances, or an explicit count for crates)
	ref array<string> m_aWeapons;
	string m_sSpeedRoad;      //!< "80 km/h"
	string m_sAmphibious;     //!< "Yes" / "No"
	string m_sSpeedWater;     //!< "10 km/h" / ""
	string m_sCrew;           //!< "2 + Dismount 7 Troops"
	bool m_bExpanded;         //!< first group starts open, as the mockup
	ref array<ref TBD_AssetInstanceInfo> m_aInstances;

	void TBD_AssetTypeInfo(string name, ResourceName prefab, int count)
	{
		m_sName = name;
		m_sPrefab = prefab;
		m_iCount = count;
		m_aWeapons = {};
		m_aInstances = {};
	}

	TBD_AssetTypeInfo Specs(string speedRoad, string amphibious, string speedWater, string crew)
	{
		m_sSpeedRoad = speedRoad;
		m_sAmphibious = amphibious;
		m_sSpeedWater = speedWater;
		m_sCrew = crew;
		return this;
	}

	bool HasInfo()
	{
		return !m_aWeapons.IsEmpty() || !m_sSpeedRoad.IsEmpty() || !m_sCrew.IsEmpty();
	}
}

//! One uniform card: a faction component, its doll, its weapon chips and camo name.
class TBD_UniformInfo
{
	string m_sName;        //!< "CDF (12th Mechanized)"
	string m_sKitAlias;    //!< "kit:sov_rifleman" — the doll's prefab through TBD_Registry
	ResourceName m_sPrefab; //!< pinned prefab; empty = resolve the alias
	ref array<string> m_aChips; //!< "MAG", "AK"
	string m_sCamo;        //!< "TTsKO / Dubok"

	void TBD_UniformInfo(string name, string kitAlias, ResourceName prefab, string camo)
	{
		m_sName = name;
		m_sKitAlias = kitAlias;
		m_sPrefab = prefab;
		m_sCamo = camo;
		m_aChips = {};
	}

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

class TBD_PlanInfo
{
	string m_sId;
	string m_sLabel;

	void TBD_PlanInfo(string id, string label)
	{
		m_sId = id;
		m_sLabel = label;
	}
}

//! The read surface. Mock until an adapter calls Set().
class TBD_BriefingCatalog
{
	protected static ref TBD_BriefingCatalog s_Instance;

	string m_sMissionId;
	ref TBD_BriefingFaction m_Own;
	ref TBD_BriefingFaction m_Enemy;
	string m_sTimeLimit;        //!< "60 Minutes"
	string m_sCaptureSummary;   //!< "Capture 2 of 2"
	ref array<ref TBD_NetInfo> m_aNets;
	ref array<ref TBD_ObjectiveInfo> m_aObjectives;
	ref array<ref TBD_RuleGroup> m_aRuleGroups;
	ref array<string> m_aLore;
	ref array<ref TBD_ParamInfo> m_aParams;
	ref array<ref TBD_AssetTypeInfo> m_aFriendlyAssets;
	ref array<ref TBD_AssetTypeInfo> m_aEnemyAssets;
	ref array<ref TBD_UniformInfo> m_aFriendlyUniforms;
	ref array<ref TBD_UniformInfo> m_aEnemyUniforms;
	ref array<ref TBD_PlanInfo> m_aPlans;

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	static TBD_BriefingCatalog Get()
	{
		if (!s_Instance)
			s_Instance = TBD_BriefingMock.Build();

		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	//! Replace the catalog (the live adapter, or a fixture). Null restores the mock on the next Get().
	static void Set(TBD_BriefingCatalog catalog)
	{
		s_Instance = catalog;
	}

	// ── Reads ───────────────────────────────────────────────────────────────────────────────

	//! Mono top-bar title: the scenario picked in the selector, else the catalog's own.
	string GetMissionId()
	{
		if (!TBD_SessionSelection.s_sMissionId.IsEmpty())
			return TBD_SessionSelection.s_sMissionId;

		return m_sMissionId;
	}

	TBD_BriefingFaction GetOwnFaction()   { return m_Own; }
	TBD_BriefingFaction GetEnemyFaction() { return m_Enemy; }
	string GetTimeLimit()                 { return m_sTimeLimit; }
	string GetCaptureSummary()            { return m_sCaptureSummary; }
	array<ref TBD_NetInfo> GetNets()      { return m_aNets; }
	array<ref TBD_ObjectiveInfo> GetObjectives() { return m_aObjectives; }
	array<ref TBD_RuleGroup> GetRuleGroups()     { return m_aRuleGroups; }
	array<string> GetLore()               { return m_aLore; }
	array<ref TBD_ParamInfo> GetParams()  { return m_aParams; }
	array<ref TBD_PlanInfo> GetPlans()    { return m_aPlans; }

	//! Assets / uniforms of one side; `friendly` picks the reader's side.
	array<ref TBD_AssetTypeInfo> GetAssets(bool friendly)
	{
		if (friendly)
			return m_aFriendlyAssets;

		return m_aEnemyAssets;
	}

	array<ref TBD_UniformInfo> GetUniforms(bool friendly)
	{
		if (friendly)
			return m_aFriendlyUniforms;

		return m_aEnemyUniforms;
	}

	TBD_BriefingFaction GetFaction(bool friendly)
	{
		if (friendly)
			return m_Own;

		return m_Enemy;
	}

	//! Total vehicles of a side (the page header count).
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
