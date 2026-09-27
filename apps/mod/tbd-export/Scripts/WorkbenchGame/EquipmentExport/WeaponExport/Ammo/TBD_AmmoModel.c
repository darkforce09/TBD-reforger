//------------------------------------------------------------------------------------------------
// TBD_AmmoModel.c
//
// Strong data models representing magazines, ammunition, cartridges, tracer ratios,
// and projectile ballistics for the TBD Reforger Universal Equipment Export System.
//
// Pure intrinsic keys: exports pure relational keys (magazine_wells, ammo_resources)
// without pre-computing candidate compatibility arrays.
//------------------------------------------------------------------------------------------------

class TBD_TracerRatioInfo
{
	bool m_bHasTracers = false;
	string m_sRatioString = "none"; // e.g. "4:1", "1:1", "1:0" (all tracer), "none"
	int m_iTracerCount = 0;
	int m_iStandardCount = 0;
	int m_iInterval = 0; // e.g. 5 = every 5th round
	int m_iTerminalTracerCluster = 0; // consecutive tracers at belt end
	ref array<int> m_aAmmoMapping = {}; // raw sequence from AmmoMapping
}

class TBD_MagazineCapacityInfo
{
	int m_iRoundCapacity = 0; // from MaxAmmo
	string m_sAmmoStyle = "box"; // "box", "belt", "drum", "clip", "single_round", "internal"
	float m_fWeightPerRoundKg = -1.0; // from WeightPerAmmo
}

class TBD_MagazineCaliberInfo
{
	string m_sCaliberName; // e.g. "5.56x45mm NATO", "7.62x54mmR", "40x46mm"
	string m_sAmmoType; // e.g. "Ball", "APTracer", "HEDP"
	int m_iAmmoTypeFlags = 0; // e.g. 6 = AP + Tracer
}

class TBD_MagazinePhysicalInfo
{
	float m_fWeightEmptyKg = -1.0;
	float m_fWeightFullKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	vector m_vDimensions = "0 0 0";
	string m_sInventorySize; // e.g. "SLOT_1x1", "SLOT_2x2"
}

class TBD_MagazineInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	string m_sCategory; // "magazines_rifle", "magazines_mg", "magazines_handgun", "magazines_heavy", "magazines_grenades", "magazines_rockets"
	string m_sFamily;
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	// Relational foreign keys matching weapon muzzle magazine_wells
	ref array<string> m_aMagazineWells = {};

	// Sub-components
	ref TBD_MagazineCapacityInfo m_Capacity = new TBD_MagazineCapacityInfo();
	ref TBD_MagazineCaliberInfo m_Caliber = new TBD_MagazineCaliberInfo();
	ref TBD_TracerRatioInfo m_Tracers = new TBD_TracerRatioInfo();
	ref TBD_MagazinePhysicalInfo m_Physical = new TBD_MagazinePhysicalInfo();

	// Relational foreign keys matching projectile resource names
	ref array<string> m_aAmmoResources = {};

	string m_sConfigurationJson = "[]"; //!< Per-installation magazine source configuration.
	ref map<string, string> m_FieldEvidence; //!< Evidence owned by this magazine projection.
	string m_sInventoryJson = "null"; //!< Native inventory measurements and ammunition mass fields.
	string m_sNamesJson = "null"; //!< Original and resolved name evidence.

	//! Serialize directly readable magazine configuration without derived ammunition statistics.
	string SerializeJson(string indent = "    ")
	{
		TBD_EquipmentExportJson.IncludeResourceMetadata(m_sResourceName, m_FieldEvidence);
		string sub = indent + "  ";
		string json = indent + "{\n";
		json += sub + "\"resource_name\":" + TBD_EquipmentExportJson.Quote(m_sResourceName) + ",\n";
		json += sub + "\"resource_guid\":" + TBD_EquipmentResourceNames.GuidJson(m_sResourceName) + ",\n";
		json += sub + "\"id\":" + TBD_EquipmentExportJson.Quote(m_sId) + ",\n";
		json += sub + "\"names\":" + m_sNamesJson + ",\n";
		json += sub + "\"category\":" + TBD_EquipmentExportJson.Quote(m_sCategory) + ",\n";
		json += sub + "\"addon\":" + TBD_EquipmentExportJson.Quote(m_sAddonId) + ",\n";
		json += sub + "\"file_path\":" + TBD_EquipmentExportJson.Quote(m_sFilePath) + ",\n";
		string parent = "null";
		if (!m_sVariantOf.IsEmpty()) parent = TBD_EquipmentExportJson.Quote(m_sVariantOf);
		json += sub + "\"parent_prefab\":" + parent + ",\n";
		if (!m_sInventoryJson.IsEmpty() && m_sInventoryJson != "null")
			json += sub + "\"inventory\":" + m_sInventoryJson + ",\n";
		json += sub + "\"magazines\":" + m_sConfigurationJson + ",\n";
		json += sub + "\"icon\":" + TBD_EquipmentExportJson.Quote(m_sIcon) + "\n";
		return TBD_EquipmentExportJson.Pretty(json + indent + "}", indent);
	}
}

class TBD_ProjectileBallisticsInfo
{
	float m_fInitSpeedMps = -1.0;
	float m_fInitSpeedVariation = 0.0;
	float m_fAirDrag = -1.0;
	float m_fMassKg = -1.0;
	float m_fMaxPenetration = -1.0;
	string m_sBallisticTableConfig;
}

class TBD_ProjectileWarheadInfo
{
	bool m_bIsExplosive = false;
	float m_fDamageValue = -1.0;
	float m_fSafetyDistanceMeters = 0.0;
	string m_sEffectPrefab;
	string m_sSoundEvent;
	string m_sParticleEffect;
}

class TBD_ProjectileTracerInfo
{
	bool m_bIsTracer = false;
	string m_sTracerColor;
	float m_fTracerStartDistance = 0.0;
	float m_fTracerBurnTime = 0.0;
}

class TBD_ProjectileVisualsInfo
{
	string m_sProjectileModel;
	string m_sCartridgeModel;
}

class TBD_ProjectileInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sCategory; // "projectiles_bullets", "projectiles_heavy", "projectiles_grenades", "projectiles_rockets", "projectiles_mortar"
	string m_sCaliber;
	string m_sBulletType; // "Ball", "AP", "Tracer", "HEDP", "HE", "HEAT", "Smoke", "Illum"
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;

	ref TBD_ProjectileBallisticsInfo m_Ballistics = new TBD_ProjectileBallisticsInfo();
	ref TBD_ProjectileWarheadInfo m_Warhead = new TBD_ProjectileWarheadInfo();
	ref TBD_ProjectileTracerInfo m_Tracer = new TBD_ProjectileTracerInfo();
	ref TBD_ProjectileVisualsInfo m_Visuals = new TBD_ProjectileVisualsInfo();

	string m_sConfigurationJson = "null"; //!< Source-backed projectile motion and damage structures.
	ref map<string, string> m_FieldEvidence; //!< Evidence owned by this projectile projection.
	string m_sTracerJson = "null"; //!< Native burn timing; no inferred tracer ratio or distance.
	string m_sNamesJson = "null"; //!< Original and resolved name evidence.
	string m_sInventoryJson = "null"; //!< Carried-round measurements, separate from in-flight mass.

	//! Serialize original projectile systems without universal damage/penetration ratings.
	string SerializeJson(string indent = "    ")
	{
		TBD_EquipmentExportJson.IncludeResourceMetadata(m_sResourceName, m_FieldEvidence);
		string sub = indent + "  ";
		string json = indent + "{\n";
		json += sub + "\"resource_name\":" + TBD_EquipmentExportJson.Quote(m_sResourceName) + ",\n";
		json += sub + "\"resource_guid\":" + TBD_EquipmentResourceNames.GuidJson(m_sResourceName) + ",\n";
		json += sub + "\"id\":" + TBD_EquipmentExportJson.Quote(m_sId) + ",\n";
		json += sub + "\"names\":" + m_sNamesJson + ",\n";
		json += sub + "\"category\":" + TBD_EquipmentExportJson.Quote(m_sCategory) + ",\n";
		json += sub + "\"addon\":" + TBD_EquipmentExportJson.Quote(m_sAddonId) + ",\n";
		json += sub + "\"file_path\":" + TBD_EquipmentExportJson.Quote(m_sFilePath) + ",\n";
		if (!m_sInventoryJson.IsEmpty() && m_sInventoryJson != "null")
			json += sub + "\"inventory\":" + m_sInventoryJson + ",\n";
		json += sub + "\"projectile\":" + m_sConfigurationJson + ",\n";
		json += sub + "\"tracer\":" + m_sTracerJson + "\n";
		return TBD_EquipmentExportJson.Pretty(json + indent + "}", indent);
	}
}
