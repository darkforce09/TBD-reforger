//------------------------------------------------------------------------------------------------
// TBD_AttachmentModel.c
//
// Strong data models representing weapon attachments (muzzle devices & suppressors,
// bipods & grips, handguards & rail systems, tactical lights & lasers, bayonets, stocks,
// mounts, and camouflage wraps) for the TBD Reforger Universal Equipment Export System.
//
// Pure intrinsic keys: exports pure relational keys (attachment_type, compatible_attachment_types,
// obstructed_attachment_types) without pre-computing weapon-attachment pairing tables.
// Zero fake/mock data: every field is introspected from real engine components and attributes.
//------------------------------------------------------------------------------------------------

class TBD_AttachmentSlotInfo
{
	string m_sSlotName;
	string m_sPivotId;
	string m_sRequiredAttachmentType;
	string m_sDefaultAttachedPrefab;
	ref array<string> m_aObstructedAttachmentTypes = {};

	//------------------------------------------------------------------------------------------------
	string SerializeJson(string indent = "        ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";

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
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_AttachmentPhysicalInfo
{
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
}

class TBD_AttachmentVisualsInfo
{
	string m_sModelMesh;
}

class TBD_MuzzleAttachmentInfo
{
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

	//------------------------------------------------------------------------------------------------
	//! Serialize attachment entry to clean, indented JSON string
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

		// Mounting (Pure relational foreign keys to weapon attachment slots)
		json += sub + "\"mounting\": {\n";
		if (!m_Mounting.m_sAttachmentType.IsEmpty())
			json += sub2 + "\"attachment_type\": \"" + TBD_EquipmentExportJson.Escape(m_Mounting.m_sAttachmentType) + "\",\n";
		else
			json += sub2 + "\"attachment_type\": null,\n";

		json += sub2 + "\"compatible_attachment_types\": [";
		for (int c = 0; c < m_Mounting.m_aCompatibleAttachmentTypes.Count(); c++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_Mounting.m_aCompatibleAttachmentTypes[c]) + "\"";
			if (c < m_Mounting.m_aCompatibleAttachmentTypes.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		json += sub2 + "\"obstructed_attachment_types\": [";
		for (int o = 0; o < m_Mounting.m_aObstructedAttachmentTypes.Count(); o++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_Mounting.m_aObstructedAttachmentTypes[o]) + "\"";
			if (o < m_Mounting.m_aObstructedAttachmentTypes.Count() - 1)
				json += ", ";
		}
		json += "]\n";
		json += sub + "},\n";

		// Physical attributes
		json += sub + "\"physical\": {\n";
		if (m_Physical.m_fWeightKg >= 0)
			json += sub2 + "\"weight_kg\": " + m_Physical.m_fWeightKg.ToString() + ",\n";
		else
			json += sub2 + "\"weight_kg\": null,\n";

		if (m_Physical.m_fVolumeCm3 >= 0)
			json += sub2 + "\"volume_cm3\": " + m_Physical.m_fVolumeCm3.ToString() + ",\n";
		else
			json += sub2 + "\"volume_cm3\": null,\n";

		if (m_Physical.m_bHasDimensions)
			json += sub2 + "\"dimensions\": [" + m_Physical.m_vDimensions[0].ToString() + ", " + m_Physical.m_vDimensions[1].ToString() + ", " + m_Physical.m_vDimensions[2].ToString() + "],\n";
		else
			json += sub2 + "\"dimensions\": null,\n";

		if (!m_Physical.m_sInventorySize.IsEmpty())
			json += sub2 + "\"inventory_size\": \"" + TBD_EquipmentExportJson.Escape(m_Physical.m_sInventorySize) + "\"\n";
		else
			json += sub2 + "\"inventory_size\": null\n";
		json += sub + "},\n";

		// Visuals (3D Mesh)
		json += sub + "\"visuals\": {\n";
		if (!m_Visuals.m_sModelMesh.IsEmpty())
			json += sub2 + "\"model_mesh\": \"" + TBD_EquipmentExportJson.Escape(m_Visuals.m_sModelMesh) + "\"\n";
		else
			json += sub2 + "\"model_mesh\": null\n";
		json += sub + "},\n";

		// Category Payload Serialization
		if (m_sCategory == "muzzles")
		{
			json += sub + "\"muzzle_data\": {\n";
			string isSupStr = "false";
			if (m_Muzzle.m_bIsSuppressed) isSupStr = "true";
			json += sub2 + "\"is_suppressed\": " + isSupStr + ",\n";

			if (m_Muzzle.m_bHasSuppressorAttributes)
			{
				json += sub2 + "\"muzzle_speed_coefficient\": " + m_Muzzle.m_fMuzzleSpeedCoefficient.ToString() + ",\n";
				json += sub2 + "\"muzzle_dispersion_factor\": " + m_Muzzle.m_fMuzzleDispersionFactor.ToString() + ",\n";
				json += sub2 + "\"extra_obstruction_length\": " + m_Muzzle.m_fExtraObstructionLength.ToString() + ",\n";

				string ovrEffectsStr = "false";
				if (m_Muzzle.m_bOverrideMuzzleEffects) ovrEffectsStr = "true";
				json += sub2 + "\"override_muzzle_effects\": " + ovrEffectsStr + ",\n";

				string ovrShotStr = "false";
				if (m_Muzzle.m_bOverrideShot) ovrShotStr = "true";
				json += sub2 + "\"override_shot\": " + ovrShotStr + ",\n";

				if (m_Muzzle.m_bHasAngularFactors)
					json += sub2 + "\"recoil_angular_factors\": [" + m_Muzzle.m_vAngularFactors[0].ToString() + ", " + m_Muzzle.m_vAngularFactors[1].ToString() + ", " + m_Muzzle.m_vAngularFactors[2].ToString() + "],\n";
				else
					json += sub2 + "\"recoil_angular_factors\": null,\n";

				if (m_Muzzle.m_bHasLinearFactors)
					json += sub2 + "\"recoil_linear_factors\": [" + m_Muzzle.m_vLinearFactors[0].ToString() + ", " + m_Muzzle.m_vLinearFactors[1].ToString() + ", " + m_Muzzle.m_vLinearFactors[2].ToString() + "],\n";
				else
					json += sub2 + "\"recoil_linear_factors\": null,\n";

				if (m_Muzzle.m_bHasTurnFactors)
					json += sub2 + "\"turn_factors\": [" + m_Muzzle.m_vTurnFactors[0].ToString() + ", " + m_Muzzle.m_vTurnFactors[1].ToString() + ", " + m_Muzzle.m_vTurnFactors[2].ToString() + "]\n";
				else
					json += sub2 + "\"turn_factors\": null\n";
			}
			else
			{
				json += sub2 + "\"muzzle_speed_coefficient\": null,\n";
				json += sub2 + "\"muzzle_dispersion_factor\": null,\n";
				json += sub2 + "\"extra_obstruction_length\": null,\n";
				json += sub2 + "\"override_muzzle_effects\": null,\n";
				json += sub2 + "\"override_shot\": null,\n";
				json += sub2 + "\"recoil_angular_factors\": null,\n";
				json += sub2 + "\"recoil_linear_factors\": null,\n";
				json += sub2 + "\"turn_factors\": null\n";
			}
			json += sub + "}\n";
		}
		else if (m_sCategory == "bayonets")
		{
			json += sub + "\"bayonet_data\": {\n";
			string isBayStr = "false";
			if (m_Bayonet.m_bIsBayonet) isBayStr = "true";
			json += sub2 + "\"is_bayonet\": " + isBayStr + ",\n";

			if (m_Bayonet.m_bHasBayonetAttributes)
			{
				json += sub2 + "\"damage_modification_factor\": " + m_Bayonet.m_fDamageModificationFactor.ToString() + ",\n";
				json += sub2 + "\"extra_obstruction_length\": " + m_Bayonet.m_fExtraObstructionLength.ToString() + ",\n";
				json += sub2 + "\"precision_modification_factor\": " + m_Bayonet.m_fPrecisionModificationFactor.ToString() + ",\n";
				json += sub2 + "\"range_modification_factor\": " + m_Bayonet.m_fRangeModificationFactor.ToString() + "\n";
			}
			else
			{
				json += sub2 + "\"damage_modification_factor\": null,\n";
				json += sub2 + "\"extra_obstruction_length\": null,\n";
				json += sub2 + "\"precision_modification_factor\": null,\n";
				json += sub2 + "\"range_modification_factor\": null\n";
			}
			json += sub + "}\n";
		}
		else if (m_sCategory == "illuminators")
		{
			json += sub + "\"illuminator_data\": {\n";
			string hlStr = "false";
			if (m_Illuminator.m_bHasLight) hlStr = "true";
			json += sub2 + "\"has_light\": " + hlStr + ",\n";
			string hlzStr = "false";
			if (m_Illuminator.m_bHasLaser) hlzStr = "true";
			json += sub2 + "\"has_laser\": " + hlzStr + ",\n";

			if (m_Illuminator.m_fEmissiveIntensity >= 0)
				json += sub2 + "\"emissive_intensity\": " + m_Illuminator.m_fEmissiveIntensity.ToString() + ",\n";
			else
				json += sub2 + "\"emissive_intensity\": null,\n";

			if (m_Illuminator.m_bHasAdjustOffset)
				json += sub2 + "\"flashlight_adjust_offset\": [" + m_Illuminator.m_vFlashlightAdjustOffset[0].ToString() + ", " + m_Illuminator.m_vFlashlightAdjustOffset[1].ToString() + ", " + m_Illuminator.m_vFlashlightAdjustOffset[2].ToString() + "],\n";
			else
				json += sub2 + "\"flashlight_adjust_offset\": null,\n";

			if (m_Illuminator.m_fLightNearPlaneHand >= 0)
				json += sub2 + "\"light_near_plane_hand\": " + m_Illuminator.m_fLightNearPlaneHand.ToString() + ",\n";
			else
				json += sub2 + "\"light_near_plane_hand\": null,\n";

			json += sub2 + "\"lenses\": [\n";
			for (int l = 0; l < m_Illuminator.m_aLenses.Count(); l++)
			{
				json += m_Illuminator.m_aLenses[l].SerializeJson(sub3);
				if (l < m_Illuminator.m_aLenses.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += sub2 + "]\n";
			json += sub + "}\n";
		}
		else if (m_sCategory == "handguards")
		{
			json += sub + "\"handguard_data\": {\n";
			string hasHgSlotsStr = "false";
			if (m_Handguard.m_aNestedSlots.Count() > 0) hasHgSlotsStr = "true";
			json += sub2 + "\"has_nested_slots\": " + hasHgSlotsStr + ",\n";

			json += sub2 + "\"nested_attachment_slots\": [\n";
			for (int ns = 0; ns < m_Handguard.m_aNestedSlots.Count(); ns++)
			{
				json += m_Handguard.m_aNestedSlots[ns].SerializeJson(sub3);
				if (ns < m_Handguard.m_aNestedSlots.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += sub2 + "]\n";
			json += sub + "}\n";
		}
		else if (m_sCategory == "mounts")
		{
			json += sub + "\"mount_data\": {\n";
			string hasMountSlotsStr = "false";
			if (m_Mount.m_aNestedSlots.Count() > 0) hasMountSlotsStr = "true";
			json += sub2 + "\"has_nested_slots\": " + hasMountSlotsStr + ",\n";

			json += sub2 + "\"nested_attachment_slots\": [\n";
			for (int nsm = 0; nsm < m_Mount.m_aNestedSlots.Count(); nsm++)
			{
				json += m_Mount.m_aNestedSlots[nsm].SerializeJson(sub3);
				if (nsm < m_Mount.m_aNestedSlots.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += sub2 + "]\n";
			json += sub + "}\n";
		}
		else if (m_sCategory == "stocks")
		{
			json += sub + "\"stock_data\": {\n";
			string hasStockSlotsStr = "false";
			if (m_Stock.m_aNestedSlots.Count() > 0) hasStockSlotsStr = "true";
			json += sub2 + "\"has_nested_slots\": " + hasStockSlotsStr + ",\n";

			json += sub2 + "\"nested_attachment_slots\": [\n";
			for (int nss = 0; nss < m_Stock.m_aNestedSlots.Count(); nss++)
			{
				json += m_Stock.m_aNestedSlots[nss].SerializeJson(sub3);
				if (nss < m_Stock.m_aNestedSlots.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += sub2 + "]\n";
			json += sub + "}\n";
		}
		else if (m_sCategory == "camouflage")
		{
			json += sub + "\"camouflage_data\": {\n";
			if (!m_Camouflage.m_sTargetType.IsEmpty())
				json += sub2 + "\"target_type\": \"" + TBD_EquipmentExportJson.Escape(m_Camouflage.m_sTargetType) + "\"\n";
			else
				json += sub2 + "\"target_type\": null\n";
			json += sub + "}\n";
		}
		else if (m_sCategory == "bipods")
		{
			json += sub + "\"bipod_data\": {\n";
			string hasBipodSlotsStr = "false";
			if (m_Bipod.m_aNestedSlots.Count() > 0) hasBipodSlotsStr = "true";
			json += sub2 + "\"has_nested_slots\": " + hasBipodSlotsStr + ",\n";

			json += sub2 + "\"nested_attachment_slots\": [\n";
			for (int nsb = 0; nsb < m_Bipod.m_aNestedSlots.Count(); nsb++)
			{
				json += m_Bipod.m_aNestedSlots[nsb].SerializeJson(sub3);
				if (nsb < m_Bipod.m_aNestedSlots.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += sub2 + "]\n";
			json += sub + "}\n";
		}
		else
		{
			json += sub + "\"attachment_data\": {}\n";
		}

		json += indent + "}";
		return json;
	}
}
