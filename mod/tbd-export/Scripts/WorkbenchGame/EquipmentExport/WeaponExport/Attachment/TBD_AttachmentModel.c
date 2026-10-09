/** Standard attachment records preserve native mounting rules and directly readable gameplay sections. */

class TBD_AttachmentSlotInfo
{
	string m_sSlotName;
	string m_sSourceJson; //!< Native slot component identity and source resource.
	string m_sInstanceId;
	string m_sPivotId;
	string m_sRequiredAttachmentType;
	string m_sDefaultAttachedPrefab;
	ref array<string> m_aObstructedAttachmentTypes = {};

	//------------------------------------------------------------------------------------------------
	string SerializeJson(string indent = "        ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";

		json += sub + "\"source\":" + m_sSourceJson + ",\n";
		json += sub + "\"instance_id\":" + TBD_EquipmentExportJson.Quote(m_sInstanceId) + ",\n";
		json += sub + "\"slot_name\": \"" + TBD_EquipmentExportJson.Escape(m_sSlotName) + "\",\n";
		json += sub + "\"pivot_id\": \"" + TBD_EquipmentExportJson.Escape(m_sPivotId) + "\",\n";
		json += sub + "\"required_attachment_type\": \"" + TBD_EquipmentExportJson.Escape(m_sRequiredAttachmentType) + "\",\n";

		if (!m_sDefaultAttachedPrefab.IsEmpty())
			json += sub + "\"default_attachment\": \"" + TBD_EquipmentExportJson.Escape(m_sDefaultAttachedPrefab) + "\",\n";
		else
			json += sub + "\"default_attachment\": null,\n";

		json += sub + "\"obstructed_attachment_types\": [";
		for (int ob = 0; ob < m_aObstructedAttachmentTypes.Count(); ob++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_aObstructedAttachmentTypes[ob]) + "\"";
			if (ob < m_aObstructedAttachmentTypes.Count() - 1)
				json += ", ";
		}
		json += "]\n";

		json += indent + "}";
		return json;
	}
}

class TBD_AttachmentMountingInfo
{
	string m_sAttachmentType;
	ref array<string> m_aCompatibleAttachmentTypes = {};
	ref array<string> m_aNativeTypeHierarchy = {}; //!< Native self/base types; separate from authored compatibility rules.
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_AttachmentPhysicalInfo
{
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
	string m_sInventoryJson; //!< Effective raw inventory measurements; empty when inapplicable.
	string m_sPhysicsJson; //!< Separate rigid-body measurements; empty when inapplicable.
}

class TBD_AttachmentVisualsInfo
{
	string m_sModelMesh;
}

class TBD_MuzzleAttachmentInfo
{
	string m_sSourceJson; //!< Selected suppressor attribute values and their source status.
	bool m_bIsSuppressed = false;
	bool m_bHasSuppressorAttributes = false;
	float m_fMuzzleSpeedCoefficient = 1.0;
	float m_fMuzzleDispersionFactor = 1.0;
	float m_fExtraObstructionLength = 0.0;
	bool m_bOverrideMuzzleEffects = false;
	bool m_bOverrideShot = false;
	vector m_vAngularFactors = "0 0 0";
	bool m_bHasAngularFactors = false;
	vector m_vLinearFactors = "0 0 0";
	bool m_bHasLinearFactors = false;
	vector m_vTurnFactors = "0 0 0";
	bool m_bHasTurnFactors = false;
}

class TBD_BayonetAttachmentInfo
{
	string m_sSourceJson; //!< Selected bayonet attribute values and their source status.
	bool m_bIsBayonet = false;
	bool m_bHasBayonetAttributes = false;
	float m_fDamageModificationFactor = 1.0;
	float m_fExtraObstructionLength = 0.0;
	float m_fPrecisionModificationFactor = 1.0;
	float m_fRangeModificationFactor = 1.0;
}

class TBD_IlluminatorLensInfo
{
	string m_sDescription;
	vector m_vLenseColor = "1 1 1";
	float m_fLightValue = 0.0;

	string SerializeJson(string indent = "          ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";
		json += sub + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		json += sub + "\"color\": [" + m_vLenseColor[0].ToString() + ", " + m_vLenseColor[1].ToString() + ", " + m_vLenseColor[2].ToString() + "],\n";
		json += sub + "\"light_value\": " + m_fLightValue.ToString() + "\n";
		json += indent + "}";
		return json;
	}
}

class TBD_IlluminatorAttachmentInfo
{
	string m_sSourceJson; //!< Individual light/laser configurations and ordered lens values.
	bool m_bHasLight = false;
	bool m_bHasLaser = false;
	float m_fEmissiveIntensity = -1.0;
	vector m_vFlashlightAdjustOffset = "0 0 0";
	bool m_bHasAdjustOffset = false;
	float m_fLightNearPlaneHand = -1.0;
	ref array<ref TBD_IlluminatorLensInfo> m_aLenses = {};
}

class TBD_HandguardAttachmentInfo
{
	ref array<ref TBD_AttachmentSlotInfo> m_aNestedSlots = {};
}

class TBD_MountAttachmentInfo
{
	ref array<ref TBD_AttachmentSlotInfo> m_aNestedSlots = {};
}

class TBD_StockAttachmentInfo
{
	ref array<ref TBD_AttachmentSlotInfo> m_aNestedSlots = {};
}

class TBD_CamouflageAttachmentInfo
{
	string m_sTargetType; // "optic" or "weapon"
	string m_sSourceJson; //!< Native camouflage configuration when supplied by the source.
}

class TBD_BipodAttachmentInfo
{
	ref array<ref TBD_AttachmentSlotInfo> m_aNestedSlots = {};
}

class TBD_AttachmentInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	string m_sCategory; // "muzzles", "bipods", "handguards", "illuminators", "bayonets", "stocks", "mounts", "camouflage"
	string m_sFamily;
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_AttachmentMountingInfo m_Mounting = new TBD_AttachmentMountingInfo();
	ref TBD_AttachmentPhysicalInfo m_Physical = new TBD_AttachmentPhysicalInfo();
	ref TBD_AttachmentVisualsInfo m_Visuals = new TBD_AttachmentVisualsInfo();

	// Category technical payloads
	ref TBD_MuzzleAttachmentInfo m_Muzzle = new TBD_MuzzleAttachmentInfo();
	ref TBD_BipodAttachmentInfo m_Bipod = new TBD_BipodAttachmentInfo();
	ref TBD_HandguardAttachmentInfo m_Handguard = new TBD_HandguardAttachmentInfo();
	ref TBD_IlluminatorAttachmentInfo m_Illuminator = new TBD_IlluminatorAttachmentInfo();
	ref TBD_BayonetAttachmentInfo m_Bayonet = new TBD_BayonetAttachmentInfo();
	ref TBD_StockAttachmentInfo m_Stock = new TBD_StockAttachmentInfo();
	ref TBD_MountAttachmentInfo m_Mount = new TBD_MountAttachmentInfo();
	ref TBD_CamouflageAttachmentInfo m_Camouflage = new TBD_CamouflageAttachmentInfo();

	string m_sNamesJson = "null"; //!< Original name text/key and official English resolution evidence.

	//! Serialize readable source-backed sections without derived classifications or missing-value sentinels.
	string SerializeJson(string indent = "    ")
	{
		TBD_EquipmentExportJson.IncludeResource(m_sResourceName);
		string sub = indent + "  ";
		array<string> fields = {};
		fields.Insert(sub + "\"resource_name\":" + TBD_EquipmentExportJson.Quote(m_sResourceName));
		fields.Insert(sub + "\"resource_guid\":" + TBD_EquipmentResourceNames.GuidJson(m_sResourceName));
		fields.Insert(sub + "\"id\":" + TBD_EquipmentExportJson.Quote(m_sId));
		fields.Insert(sub + "\"names\":" + m_sNamesJson);
		fields.Insert(sub + "\"category\":" + TBD_EquipmentExportJson.Quote(m_sCategory));
		fields.Insert(sub + "\"family\":" + TBD_EquipmentExportJson.Quote(m_sFamily));
		fields.Insert(sub + "\"addon\":" + TBD_EquipmentExportJson.Quote(m_sAddonId));
		fields.Insert(sub + "\"file_path\":" + TBD_EquipmentExportJson.Quote(m_sFilePath));
		string parent = "null";
		if (!m_sVariantOf.IsEmpty()) parent = TBD_EquipmentExportJson.Quote(m_sVariantOf);
		fields.Insert(sub + "\"parent_prefab\":" + parent);
		fields.Insert(sub + "\"mounting\":" + MountingJson());
		if (!m_Physical.m_sInventoryJson.IsEmpty())
			fields.Insert(sub + "\"inventory\":" + m_Physical.m_sInventoryJson);
		if (!m_Physical.m_sPhysicsJson.IsEmpty())
			fields.Insert(sub + "\"physics\":" + m_Physical.m_sPhysicsJson);
		string visuals = "{\"icon\":" + TBD_EquipmentExportJson.Quote(m_sIcon);
		visuals += ",\"model_mesh\":" + TBD_EquipmentExportJson.Quote(m_Visuals.m_sModelMesh) + "}";
		fields.Insert(sub + "\"visuals\":" + visuals);
		AddTechnicalSections(fields, sub);
		return indent + "{\n" + TBD_EquipmentExportJson.Join(fields, ",\n") + "\n" + indent + "}";
	}

	//! Native type ancestry and exclusions remain rules rather than item-to-item compatibility guesses.
	string MountingJson()
	{
		string nativeType = "null";
		if (!m_Mounting.m_sAttachmentType.IsEmpty()) nativeType = TBD_EquipmentExportJson.Quote(m_Mounting.m_sAttachmentType);
		string json = "{\"attachment_type\":" + nativeType;
		json += ",\"native_type_hierarchy\":" + TBD_EquipmentExportJson.Strings(m_Mounting.m_aNativeTypeHierarchy);
		json += ",\"compatible_attachment_types\":" + TBD_EquipmentExportJson.Strings(m_Mounting.m_aCompatibleAttachmentTypes);
		json += ",\"obstructed_attachment_types\":" + TBD_EquipmentExportJson.Strings(m_Mounting.m_aObstructedAttachmentTypes);
		return json + "}";
	}

	//! All applicable source sections survive regardless of the display category.
	void AddTechnicalSections(array<string> fields, string indent)
	{
		if (!m_Muzzle.m_sSourceJson.IsEmpty()) fields.Insert(indent + "\"muzzle_data\":" + m_Muzzle.m_sSourceJson);
		if (!m_Bayonet.m_sSourceJson.IsEmpty()) fields.Insert(indent + "\"bayonet_data\":" + m_Bayonet.m_sSourceJson);
		if (!m_Illuminator.m_sSourceJson.IsEmpty()) fields.Insert(indent + "\"illuminator_data\":" + m_Illuminator.m_sSourceJson);
		if (!m_Camouflage.m_sSourceJson.IsEmpty()) fields.Insert(indent + "\"camouflage_data\":" + m_Camouflage.m_sSourceJson);
		if (m_sCategory == "handguards") fields.Insert(indent + "\"handguard_data\":" + SlotsJson(m_Handguard.m_aNestedSlots, indent));
		if (m_sCategory == "mounts") fields.Insert(indent + "\"mount_data\":" + SlotsJson(m_Mount.m_aNestedSlots, indent));
		if (m_sCategory == "stocks") fields.Insert(indent + "\"stock_data\":" + SlotsJson(m_Stock.m_aNestedSlots, indent));
		if (m_sCategory == "bipods") fields.Insert(indent + "\"bipod_data\":" + SlotsJson(m_Bipod.m_aNestedSlots, indent));
	}

	//! Repeated slot classes retain their individual native identities and default installations.
	string SlotsJson(array<ref TBD_AttachmentSlotInfo> slots, string indent)
	{
		array<string> entries = {};
		foreach (TBD_AttachmentSlotInfo slot : slots) entries.Insert(slot.SerializeJson(indent + "  "));
		return "{\"nested_attachment_slots\":[\n" + TBD_EquipmentExportJson.Join(entries, ",\n") + "\n" + indent + "]}";
	}
}
