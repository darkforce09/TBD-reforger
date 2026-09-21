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

	//------------------------------------------------------------------------------------------------
	//! Serialize magazine entry to clean, indented JSON string
	string SerializeJson(string indent = "    ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";
		string sub2 = sub + "  ";
		string sub3 = sub2 + "  ";

		// Identity
		json += sub + "\"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		json += sub + "\"id\": \"" + TBD_EquipmentExportJson.Escape(m_sId) + "\",\n";
		json += sub + "\"display_name\": \"" + TBD_EquipmentExportJson.Escape(m_sDisplayName) + "\",\n";

		if (!m_sDescription.IsEmpty())
			json += sub + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		else
			json += sub + "\"description\": null,\n";

		if (!m_sIcon.IsEmpty())
			json += sub + "\"icon\": \"" + TBD_EquipmentExportJson.Escape(m_sIcon) + "\",\n";
		else
			json += sub + "\"icon\": null,\n";

		json += sub + "\"category\": \"" + TBD_EquipmentExportJson.Escape(m_sCategory) + "\",\n";
		json += sub + "\"family\": \"" + TBD_EquipmentExportJson.Escape(m_sFamily) + "\",\n";
		json += sub + "\"addon\": \"" + TBD_EquipmentExportJson.Escape(m_sAddonId) + "\",\n";
		json += sub + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";

		string isAbsStr = "false";
		if (m_bIsAbstract) isAbsStr = "true";
		json += sub + "\"is_abstract\": " + isAbsStr + ",\n";

		if (!m_sVariantOf.IsEmpty())
			json += sub + "\"variant_of\": \"" + TBD_EquipmentExportJson.Escape(m_sVariantOf) + "\",\n";
		else
			json += sub + "\"variant_of\": null,\n";

		// Magazine Wells (Pure relational foreign keys to weapon muzzles)
		json += sub + "\"magazine_wells\": [";
		for (int mw = 0; mw < m_aMagazineWells.Count(); mw++)
		{
			json += "\"" + m_aMagazineWells[mw] + "\"";
			if (mw < m_aMagazineWells.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		// Capacity
		json += sub + "\"capacity\": {\n";
		json += sub2 + "\"round_capacity\": " + m_Capacity.m_iRoundCapacity.ToString() + ",\n";
		json += sub2 + "\"ammo_style\": \"" + TBD_EquipmentExportJson.Escape(m_Capacity.m_sAmmoStyle) + "\",\n";
		if (m_Capacity.m_fWeightPerRoundKg >= 0)
			json += sub2 + "\"weight_per_round_kg\": " + m_Capacity.m_fWeightPerRoundKg.ToString() + "\n";
		else
			json += sub2 + "\"weight_per_round_kg\": null\n";
		json += sub + "},\n";

		// Caliber
		json += sub + "\"caliber\": {\n";
		json += sub2 + "\"caliber_name\": \"" + TBD_EquipmentExportJson.Escape(m_Caliber.m_sCaliberName) + "\",\n";
		if (!m_Caliber.m_sAmmoType.IsEmpty())
			json += sub2 + "\"ammo_type\": \"" + TBD_EquipmentExportJson.Escape(m_Caliber.m_sAmmoType) + "\",\n";
		else
			json += sub2 + "\"ammo_type\": null,\n";
		json += sub2 + "\"ammo_type_flags\": " + m_Caliber.m_iAmmoTypeFlags.ToString() + "\n";
		json += sub + "},\n";

		// Tracers
		json += sub + "\"tracers\": {\n";
		string hasTracerStr = "false";
		if (m_Tracers.m_bHasTracers) hasTracerStr = "true";
		json += sub2 + "\"has_tracers\": " + hasTracerStr + ",\n";
		json += sub2 + "\"tracer_ratio\": \"" + TBD_EquipmentExportJson.Escape(m_Tracers.m_sRatioString) + "\",\n";
		json += sub2 + "\"tracer_count\": " + m_Tracers.m_iTracerCount.ToString() + ",\n";
		json += sub2 + "\"standard_count\": " + m_Tracers.m_iStandardCount.ToString() + ",\n";
		json += sub2 + "\"interval\": " + m_Tracers.m_iInterval.ToString() + ",\n";
		json += sub2 + "\"terminal_tracer_cluster\": " + m_Tracers.m_iTerminalTracerCluster.ToString() + ",\n";
		json += sub2 + "\"ammo_mapping\": [";
		for (int am = 0; am < m_Tracers.m_aAmmoMapping.Count(); am++)
		{
			json += m_Tracers.m_aAmmoMapping[am].ToString();
			if (am < m_Tracers.m_aAmmoMapping.Count() - 1)
				json += ", ";
		}
		json += "]\n";
		json += sub + "},\n";

		// Ammo Resources (Relational foreign keys to projectile catalog)
		json += sub + "\"ammo_resources\": [";
		for (int ar = 0; ar < m_aAmmoResources.Count(); ar++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_aAmmoResources[ar]) + "\"";
			if (ar < m_aAmmoResources.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		// Physical
		json += sub + "\"physical\": {\n";
		if (m_Physical.m_fWeightEmptyKg >= 0)
			json += sub2 + "\"weight_empty_kg\": " + m_Physical.m_fWeightEmptyKg.ToString() + ",\n";
		else
			json += sub2 + "\"weight_empty_kg\": null,\n";

		if (m_Physical.m_fWeightFullKg >= 0)
			json += sub2 + "\"weight_full_kg\": " + m_Physical.m_fWeightFullKg.ToString() + ",\n";
		else
			json += sub2 + "\"weight_full_kg\": null,\n";

		if (m_Physical.m_fVolumeCm3 >= 0)
			json += sub2 + "\"volume_cm3\": " + m_Physical.m_fVolumeCm3.ToString() + ",\n";
		else
			json += sub2 + "\"volume_cm3\": null,\n";

		json += sub2 + "\"dimensions\": [" + m_Physical.m_vDimensions[0].ToString() + ", " + m_Physical.m_vDimensions[1].ToString() + ", " + m_Physical.m_vDimensions[2].ToString() + "],\n";

		if (!m_Physical.m_sInventorySize.IsEmpty())
			json += sub2 + "\"inventory_size\": \"" + TBD_EquipmentExportJson.Escape(m_Physical.m_sInventorySize) + "\"\n";
		else
			json += sub2 + "\"inventory_size\": null\n";
		json += sub + "}\n";

		json += indent + "}";
		return json;
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

	//------------------------------------------------------------------------------------------------
	//! Serialize projectile entry to clean, indented JSON string
	string SerializeJson(string indent = "    ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";
		string sub2 = sub + "  ";

		// Identity
		json += sub + "\"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		json += sub + "\"id\": \"" + TBD_EquipmentExportJson.Escape(m_sId) + "\",\n";
		json += sub + "\"display_name\": \"" + TBD_EquipmentExportJson.Escape(m_sDisplayName) + "\",\n";

		if (!m_sDescription.IsEmpty())
			json += sub + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		else
			json += sub + "\"description\": null,\n";

		json += sub + "\"category\": \"" + TBD_EquipmentExportJson.Escape(m_sCategory) + "\",\n";
		json += sub + "\"caliber\": \"" + TBD_EquipmentExportJson.Escape(m_sCaliber) + "\",\n";
		json += sub + "\"bullet_type\": \"" + TBD_EquipmentExportJson.Escape(m_sBulletType) + "\",\n";
		json += sub + "\"addon\": \"" + TBD_EquipmentExportJson.Escape(m_sAddonId) + "\",\n";
		json += sub + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";

		string isAbsStr = "false";
		if (m_bIsAbstract) isAbsStr = "true";
		json += sub + "\"is_abstract\": " + isAbsStr + ",\n";

		// Ballistics
		json += sub + "\"ballistics\": {\n";
		if (m_Ballistics.m_fInitSpeedMps >= 0)
			json += sub2 + "\"init_speed_mps\": " + m_Ballistics.m_fInitSpeedMps.ToString() + ",\n";
		else
			json += sub2 + "\"init_speed_mps\": null,\n";

		json += sub2 + "\"init_speed_variation\": " + m_Ballistics.m_fInitSpeedVariation.ToString() + ",\n";

		if (m_Ballistics.m_fAirDrag >= 0)
			json += sub2 + "\"air_drag\": " + m_Ballistics.m_fAirDrag.ToString() + ",\n";
		else
			json += sub2 + "\"air_drag\": null,\n";

		if (m_Ballistics.m_fMassKg >= 0)
			json += sub2 + "\"mass_kg\": " + m_Ballistics.m_fMassKg.ToString() + ",\n";
		else
			json += sub2 + "\"mass_kg\": null,\n";

		if (m_Ballistics.m_fMaxPenetration >= 0)
			json += sub2 + "\"max_penetration\": " + m_Ballistics.m_fMaxPenetration.ToString() + ",\n";
		else
			json += sub2 + "\"max_penetration\": null,\n";

		if (!m_Ballistics.m_sBallisticTableConfig.IsEmpty())
			json += sub2 + "\"ballistic_table_config\": \"" + TBD_EquipmentExportJson.Escape(m_Ballistics.m_sBallisticTableConfig) + "\"\n";
		else
			json += sub2 + "\"ballistic_table_config\": null\n";
		json += sub + "},\n";

		// Warhead
		json += sub + "\"warhead\": {\n";
		string isExpStr = "false";
		if (m_Warhead.m_bIsExplosive) isExpStr = "true";
		json += sub2 + "\"is_explosive\": " + isExpStr + ",\n";

		if (m_Warhead.m_fDamageValue >= 0)
			json += sub2 + "\"damage_value\": " + m_Warhead.m_fDamageValue.ToString() + ",\n";
		else
			json += sub2 + "\"damage_value\": null,\n";

		json += sub2 + "\"safety_distance_meters\": " + m_Warhead.m_fSafetyDistanceMeters.ToString() + ",\n";

		if (!m_Warhead.m_sEffectPrefab.IsEmpty())
			json += sub2 + "\"effect_prefab\": \"" + TBD_EquipmentExportJson.Escape(m_Warhead.m_sEffectPrefab) + "\",\n";
		else
			json += sub2 + "\"effect_prefab\": null,\n";

		if (!m_Warhead.m_sSoundEvent.IsEmpty())
			json += sub2 + "\"sound_event\": \"" + TBD_EquipmentExportJson.Escape(m_Warhead.m_sSoundEvent) + "\",\n";
		else
			json += sub2 + "\"sound_event\": null,\n";

		if (!m_Warhead.m_sParticleEffect.IsEmpty())
			json += sub2 + "\"particle_effect\": \"" + TBD_EquipmentExportJson.Escape(m_Warhead.m_sParticleEffect) + "\"\n";
		else
			json += sub2 + "\"particle_effect\": null\n";
		json += sub + "},\n";

		// Tracer
		json += sub + "\"tracer\": {\n";
		string isTracerStr = "false";
		if (m_Tracer.m_bIsTracer) isTracerStr = "true";
		json += sub2 + "\"is_tracer\": " + isTracerStr + ",\n";

		if (!m_Tracer.m_sTracerColor.IsEmpty())
			json += sub2 + "\"tracer_color\": \"" + TBD_EquipmentExportJson.Escape(m_Tracer.m_sTracerColor) + "\",\n";
		else
			json += sub2 + "\"tracer_color\": null,\n";

		json += sub2 + "\"tracer_start_distance\": " + m_Tracer.m_fTracerStartDistance.ToString() + ",\n";
		json += sub2 + "\"tracer_burn_time\": " + m_Tracer.m_fTracerBurnTime.ToString() + "\n";
		json += sub + "},\n";

		// Visuals
		json += sub + "\"visuals\": {\n";
		if (!m_Visuals.m_sProjectileModel.IsEmpty())
			json += sub2 + "\"projectile_model\": \"" + TBD_EquipmentExportJson.Escape(m_Visuals.m_sProjectileModel) + "\",\n";
		else
			json += sub2 + "\"projectile_model\": null,\n";

		if (!m_Visuals.m_sCartridgeModel.IsEmpty())
			json += sub2 + "\"cartridge_model\": \"" + TBD_EquipmentExportJson.Escape(m_Visuals.m_sCartridgeModel) + "\"\n";
		else
			json += sub2 + "\"cartridge_model\": null\n";
		json += sub + "}\n";

		json += indent + "}";
		return json;
	}
}
