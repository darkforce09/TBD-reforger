/**
 * TBD_StaticWeaponModel.c
 *
 * Data model for static and crew-served weapons (mortars, tripod-mounted HMGs/GPMGs,
 * and standalone tripod mount frames).
 *
 * Introspects turret limits, optical sights, compartment crew positions, mounted armament,
 * and multi-part dismantle linkages.
 */

class TBD_StaticWeaponInfo
{
	// Identity
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sCategory;     // "mortars", "tripods", "mounts"
	string m_sFamily;       // "2B14", "M252", "M2HB", "NSV", "PKM", "M60"
	string m_sFaction;      // "US", "USSR", "FIA"
	string m_sFilePath;
	string m_sAddonId;
	bool   m_bIsAbstract;
	string m_sVariantOf;

	// Turret Specs
	bool   m_bHasTurret;
	float  m_fTraverseMin;
	float  m_fTraverseMax;
	float  m_fElevationMin;
	float  m_fElevationMax;
	float  m_fAimingMaxSpeed;
	string m_sSightsType;
	float  m_fMagnification;
	string m_sReticleTexture;
	bool   m_bHasIllumination;

	// Crew & Compartment
	bool   m_bHasCrewSlot;
	string m_sCompartmentSlot;
	vector m_vPassengerOffset;
	string m_sDefaultOccupantPrefab;

	// Armament
	bool   m_bHasArmament;
	bool   m_bIsIntegral;
	string m_sMountedWeaponTemplate;
	string m_sCaliber;
	string m_sMagazineWell;
	ref array<string> m_aSupportedAmmo = {};
	ref array<string> m_aInitialMagazines = {};

	// Deployment & Multi-part linkages
	bool   m_bIsDeployable;
	string m_sDismantleAction;
	string m_sReplacementBase;
	string m_sBarrelPrefab;
	string m_sBipodPrefab;

	// Physical
	float  m_fMassKg;
	string m_sDebrisModel;

	string m_sFactionJson; //!< Source-authored faction affiliation; empty when unavailable.
	string m_sNamesJson; //!< Original and resolved names, supplied by the standard scanner.
	string m_sInventoryJson; //!< Native inventory mass, volume and dimensions.
	string m_sPhysicsJson; //!< Native bodies and rigid-body mass, including source-authored zero.
	string m_sStorageJson; //!< Individual storage compartments and restrictions.
	string m_sVisualsJson; //!< Native icon, mesh and inventory preview references.
	string m_sTurretJson; //!< Separate turret installations and controller settings.
	string m_sCrewJson; //!< Individual stations with native access and occupant relationships.
	string m_sArmamentJson; //!< Mounted weapon references and native slot state.
	string m_sDeploymentJson; //!< All configured deployment variants and parts.
	string m_sBallisticConfigurationsJson; //!< Authored ballistic configuration, without generated trajectories.
	ref array<string> m_aMountedWeaponsJson = {}; //!< Individual enabled mounted weapon installations.
	ref TBD_WeaponSightsInfo m_Sights = new TBD_WeaponSightsInfo();
	ref array<ref TBD_WeaponMuzzleInfo> m_aMuzzles = {};
	ref array<ref TBD_WeaponAttachmentSlotInfo> m_aAttachmentSlots = {};

	//! Serialize native domain sections and omit capabilities which do not apply.
	string SerializeJson(string indent = "    ")
	{
		TBD_EquipmentExportJson.IncludeResource(m_sResourceName);
		string json = "{}";
		json = TBD_EquipmentExportJson.Member(json, "resource_name", TBD_EquipmentExportJson.Quote(m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "resource_guid", TBD_EquipmentResourceNames.GuidJson(m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "id", TBD_EquipmentExportJson.Quote(m_sId));
		json = TBD_EquipmentExportJson.Member(json, "category", TBD_EquipmentExportJson.Quote(m_sCategory));
		json = TBD_EquipmentExportJson.Member(json, "family", TBD_EquipmentExportJson.Quote(m_sFamily));
		json = TBD_EquipmentExportJson.Member(json, "addon", TBD_EquipmentExportJson.Quote(m_sAddonId));
		json = TBD_EquipmentExportJson.Member(json, "file_path", TBD_EquipmentExportJson.Quote(m_sFilePath));
		string parent = "null";
		if (!m_sVariantOf.IsEmpty()) parent = TBD_EquipmentExportJson.Quote(m_sVariantOf);
		json = TBD_EquipmentExportJson.Member(json, "parent_prefab", parent);
		AppendSection(json, "names", m_sNamesJson);
		AppendSection(json, "faction", m_sFactionJson);
		AppendSection(json, "inventory", m_sInventoryJson);
		AppendSection(json, "physics", m_sPhysicsJson);
		AppendSection(json, "storage", m_sStorageJson);
		AppendSection(json, "visuals", m_sVisualsJson);
		AppendSection(json, "turret", m_sTurretJson);
		AppendSection(json, "crew", m_sCrewJson);
		AppendSection(json, "armament", m_sArmamentJson);
		if (!m_aMountedWeaponsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "mounted_weapons", "[" + TBD_EquipmentExportJson.Join(m_aMountedWeaponsJson) + "]");
		AppendSection(json, "deployment", m_sDeploymentJson);
		AppendSection(json, "ballistic_configurations", m_sBallisticConfigurationsJson);
		if (!m_Sights.m_aInstances.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "sights", "[" + TBD_EquipmentExportJson.Join(m_Sights.m_aInstances) + "]");
		array<string> muzzles = {};
		foreach (TBD_WeaponMuzzleInfo muzzle : m_aMuzzles) muzzles.Insert(muzzle.m_sSourceJson);
		if (!muzzles.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "muzzles", "[" + TBD_EquipmentExportJson.Join(muzzles) + "]");
		array<string> slots = {};
		foreach (TBD_WeaponAttachmentSlotInfo slot : m_aAttachmentSlots) slots.Insert(slot.m_sSourceJson);
		if (!slots.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "attachment_slots", "[" + TBD_EquipmentExportJson.Join(slots) + "]");
		return TBD_EquipmentExportJson.Pretty(json, indent);
	}

	//! Empty extraction strings denote capabilities which do not apply.
	protected void AppendSection(inout string json, string key, string value)
	{
		if (!value.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, key, value);
	}
}
