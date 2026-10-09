//------------------------------------------------------------------------------------------------
// TBD_WeaponModel.c
//
// Strong data models representing weapon entities, ballistics, zeroing, muzzles, fire modes,
// and attachment slot specifications for the TBD Reforger Universal Weapon Export System.
//
// Pure intrinsic keys: exports pure foreign keys (magazine_wells, required_attachment_type,
// obstructed_attachment_types) without pre-computing candidate compatibility arrays.
//------------------------------------------------------------------------------------------------

class TBD_WeaponFireModeInfo
{
	string m_sName;
	string m_sSourceJson;
	string m_sFireModeType; // e.g. "Semiauto", "Auto", "Burst", "Safety", "Manual"
	int m_iBurstCount = 1; // 1 = semi, 0 = auto, 3 = burst
	int m_iRoundsPerMinute = 0;
	string m_sBurstType; // e.g. "Interruptable", "Uninterruptable", "InterruptableAndResetting"
}

class TBD_WeaponMuzzleInfo
{
	int m_iIndex = 0;
	string m_sInstanceId;
	string m_sSourceJson;
	string m_sDefaultProjectile;
	string m_sMuzzleClass;
	ref array<string> m_aMagazineWells = {};
	string m_sDefaultMagazineTemplate;
	bool m_bIsDisposable = false;
	ref array<ref TBD_WeaponFireModeInfo> m_aFireModes = {};
}

class TBD_WeaponAttachmentSlotInfo
{
	string m_sSlotName;
	string m_sInstanceId;
	string m_sSourceJson;
	string m_sPivotId;
	string m_sRequiredAttachmentType;
	string m_sDefaultAttachedPrefab;
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_WeaponSightsInfo
{
	string m_sSightsType;
	ref array<string> m_aInstances = {};
	ref array<string> m_aZeroingDistances = {};
	int m_iDefaultZeroIndex = 0;
	string m_sRearPivot;
	string m_sFrontPivot;
}

class TBD_WeaponBallisticsInfo
{
	float m_fInitSpeedCoef = 1.0;
	float m_fDispersionDiameter = -1.0;
	float m_fDispersionRange = -1.0;
}

class TBD_WeaponPhysicalInfo
{
	string m_sInventoryJson;
	string m_sPhysicsJson;
	string m_sDeploymentJson; //!< Native deployment-point configurations, separate from mass and dimensions.
	string m_sVisualsJson; //!< Source-backed icon, mesh and preview references serialized under visuals.
	string m_sMountingJson; //!< Native attachment ancestry for weapons that can themselves be installed in slots.
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	vector m_vDimensions = "0 0 0";
	string m_sInventorySize;
	bool m_bHasBipod = false;
	bool m_bIsDisposable = false;
}

class TBD_WeaponClassificationInfo
{
	string m_sSourceJson; //!< Per-component native weapon/slot enums with source identity; empty when inapplicable.
	string m_sWeaponType;
	string m_sWeaponSlotType;
}

class TBD_WeaponInfo
{
	string m_sResourceName;
	string m_sNamesJson;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	string m_sCategory; // "rifles", "machine_guns", "handguns", "launchers", "grenades", "explosives", "underbarrel"
	string m_sFamily;
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_WeaponClassificationInfo m_Classification = new TBD_WeaponClassificationInfo();
	ref TBD_WeaponPhysicalInfo m_Physical = new TBD_WeaponPhysicalInfo();
	ref TBD_WeaponSightsInfo m_Sights = new TBD_WeaponSightsInfo();
	ref TBD_WeaponBallisticsInfo m_Ballistics = new TBD_WeaponBallisticsInfo();
	ref array<ref TBD_WeaponMuzzleInfo> m_aMuzzles = {};
	ref array<ref TBD_WeaponAttachmentSlotInfo> m_aAttachmentSlots = {};

	//------------------------------------------------------------------------------------------------
	//! Serialize weapon entry to clean, indented JSON string
	string SerializeJson(string indent = "    ")
	{
		TBD_EquipmentExportJson.IncludeResource(m_sResourceName);
		string json = "{}";
		json = TBD_EquipmentExportJson.Member(json, "resource_name", TBD_EquipmentExportJson.Quote(m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "resource_guid", TBD_EquipmentResourceNames.GuidJson(m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "id", TBD_EquipmentExportJson.Quote(m_sId));
		json = TBD_EquipmentExportJson.Member(json, "names", m_sNamesJson);
		json = TBD_EquipmentExportJson.Member(json, "category", TBD_EquipmentExportJson.Quote(m_sCategory));
		json = TBD_EquipmentExportJson.Member(json, "family", TBD_EquipmentExportJson.Quote(m_sFamily));
		json = TBD_EquipmentExportJson.Member(json, "addon", TBD_EquipmentExportJson.Quote(m_sAddonId));
		json = TBD_EquipmentExportJson.Member(json, "file_path", TBD_EquipmentExportJson.Quote(m_sFilePath));
		string parent = "null";
		if (!m_sVariantOf.IsEmpty()) parent = TBD_EquipmentExportJson.Quote(m_sVariantOf);
		json = TBD_EquipmentExportJson.Member(json, "parent_prefab", parent);
		if (!m_Classification.m_sSourceJson.IsEmpty())
			json = TBD_EquipmentExportJson.Member(json, "classification", m_Classification.m_sSourceJson);
		if (!m_Physical.m_sInventoryJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "inventory", m_Physical.m_sInventoryJson);
		if (!m_Physical.m_sPhysicsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "physics", m_Physical.m_sPhysicsJson);
		if (!m_Physical.m_sDeploymentJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "deployment", m_Physical.m_sDeploymentJson);
		if (!m_Physical.m_sVisualsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "visuals", m_Physical.m_sVisualsJson);
		if (!m_Physical.m_sMountingJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "mounting", m_Physical.m_sMountingJson);
		json = TBD_EquipmentExportJson.Member(json, "sights", "[" + TBD_EquipmentExportJson.Join(m_Sights.m_aInstances) + "]");
		array<string> muzzles = {};
		foreach (TBD_WeaponMuzzleInfo muzzle : m_aMuzzles) muzzles.Insert(muzzle.m_sSourceJson);
		json = TBD_EquipmentExportJson.Member(json, "muzzles", "[" + TBD_EquipmentExportJson.Join(muzzles) + "]");
		array<string> slots = {};
		foreach (TBD_WeaponAttachmentSlotInfo slot : m_aAttachmentSlots) slots.Insert(slot.m_sSourceJson);
		json = TBD_EquipmentExportJson.Member(json, "attachment_slots", "[" + TBD_EquipmentExportJson.Join(slots) + "]");
		return TBD_EquipmentExportJson.Pretty(json, indent);
	}
}
