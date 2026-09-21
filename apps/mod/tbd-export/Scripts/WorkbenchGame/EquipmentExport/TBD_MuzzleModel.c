//------------------------------------------------------------------------------------------------
// TBD_MuzzleModel.c
//
// Strong data models representing muzzle devices (sound suppressors, silencers, flash hiders,
// muzzle brakes, and compensators) for the TBD Reforger Universal Equipment Export System.
//
// Pure ground-truth extraction:
//   - ZERO synthetic defaults: omitted values serialize as JSON null.
//   - ZERO token mutation: raw engine localization strings (e.g. #AR-Item_Mark3Suppressor_Name)
//     are preserved verbatim without stripping # or humanizing.
//   - ZERO hardcoded data: mounting keys and modifier properties reflect only what is genuinely
//     declared on the BaseContainer and its container ancestry.
//------------------------------------------------------------------------------------------------

class TBD_MuzzleMountingInfo
{
	string m_sAttachmentType;
	ref array<string> m_aCompatibleAttachmentTypes = {};
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_MuzzleModifiersInfo
{
	bool m_bIsSuppressed = false;
	float m_fMuzzleSpeedCoefficient = 1.0;
	bool m_bHasMuzzleSpeedCoefficient = false;
	float m_fMuzzleDispersionFactor = 1.0;
	bool m_bHasMuzzleDispersionFactor = false;
	float m_fExtraObstructionLength = 0.0;
	bool m_bHasExtraObstructionLength = false;
	bool m_bOverrideMuzzleEffects = false;
	bool m_bHasOverrideMuzzleEffects = false;
	bool m_bOverrideShot = false;
	bool m_bHasOverrideShot = false;
	vector m_vRecoilAngularFactors = "0 0 0";
	bool m_bHasRecoilAngularFactors = false;
	vector m_vRecoilLinearFactors = "0 0 0";
	bool m_bHasRecoilLinearFactors = false;
	vector m_vTurnFactors = "0 0 0";
	bool m_bHasTurnFactors = false;
}

class TBD_MuzzlePhysicalInfo
{
	float m_fWeightKg = -1.0;
	bool m_bHasWeight = false;
	float m_fVolumeCm3 = -1.0;
	bool m_bHasVolume = false;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
}

class TBD_MuzzleVisualsInfo
{
	string m_sModelMesh;
}

class TBD_MuzzleInfo
{
	string m_sResourceName;
	string m_sFilePath;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_MuzzleMountingInfo m_Mounting = new TBD_MuzzleMountingInfo();
	ref TBD_MuzzleModifiersInfo m_Modifiers = new TBD_MuzzleModifiersInfo();
	ref TBD_MuzzlePhysicalInfo m_Physical = new TBD_MuzzlePhysicalInfo();
	ref TBD_MuzzleVisualsInfo m_Visuals = new TBD_MuzzleVisualsInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize muzzle device entry to clean, indented JSON string
	string SerializeJson(string indent = "    ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";
		string sub2 = sub + "  ";

		// 1. Identity & Raw Engine Strings
		json += sub + "\"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		json += sub + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";

		if (!m_sDisplayName.IsEmpty())
			json += sub + "\"display_name\": \"" + TBD_EquipmentExportJson.Escape(m_sDisplayName) + "\",\n";
		else
			json += sub + "\"display_name\": null,\n";

		if (!m_sDescription.IsEmpty())
			json += sub + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		else
			json += sub + "\"description\": null,\n";

		if (!m_sIcon.IsEmpty())
			json += sub + "\"icon\": \"" + TBD_EquipmentExportJson.Escape(m_sIcon) + "\",\n";
		else
			json += sub + "\"icon\": null,\n";

		string isAbsStr = "false";
		if (m_bIsAbstract) isAbsStr = "true";
		json += sub + "\"is_abstract\": " + isAbsStr + ",\n";

		if (!m_sVariantOf.IsEmpty())
			json += sub + "\"variant_of\": \"" + TBD_EquipmentExportJson.Escape(m_sVariantOf) + "\",\n";
		else
			json += sub + "\"variant_of\": null,\n";

		// 2. Mounting & Relational Keys
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

		// 3. Muzzle Acoustics & Ballistics Modifiers
		json += sub + "\"acoustics_ballistics\": {\n";
		string isSupStr = "false";
		if (m_Modifiers.m_bIsSuppressed) isSupStr = "true";
		json += sub2 + "\"is_suppressed\": " + isSupStr + ",\n";

		if (m_Modifiers.m_bHasMuzzleSpeedCoefficient)
			json += sub2 + "\"muzzle_speed_coefficient\": " + m_Modifiers.m_fMuzzleSpeedCoefficient.ToString() + ",\n";
		else
			json += sub2 + "\"muzzle_speed_coefficient\": null,\n";

		if (m_Modifiers.m_bHasMuzzleDispersionFactor)
			json += sub2 + "\"muzzle_dispersion_factor\": " + m_Modifiers.m_fMuzzleDispersionFactor.ToString() + ",\n";
		else
			json += sub2 + "\"muzzle_dispersion_factor\": null,\n";

		if (m_Modifiers.m_bHasExtraObstructionLength)
			json += sub2 + "\"extra_obstruction_length\": " + m_Modifiers.m_fExtraObstructionLength.ToString() + ",\n";
		else
			json += sub2 + "\"extra_obstruction_length\": null,\n";

		if (m_Modifiers.m_bHasOverrideMuzzleEffects)
		{
			string ovrEffectsStr = "false";
			if (m_Modifiers.m_bOverrideMuzzleEffects) ovrEffectsStr = "true";
			json += sub2 + "\"override_muzzle_effects\": " + ovrEffectsStr + ",\n";
		}
		else
		{
			json += sub2 + "\"override_muzzle_effects\": null,\n";
		}

		if (m_Modifiers.m_bHasOverrideShot)
		{
			string ovrShotStr = "false";
			if (m_Modifiers.m_bOverrideShot) ovrShotStr = "true";
			json += sub2 + "\"override_shot\": " + ovrShotStr + ",\n";
		}
		else
		{
			json += sub2 + "\"override_shot\": null,\n";
		}

		if (m_Modifiers.m_bHasRecoilAngularFactors)
			json += sub2 + "\"recoil_angular_factors\": [" + m_Modifiers.m_vRecoilAngularFactors[0].ToString() + ", " + m_Modifiers.m_vRecoilAngularFactors[1].ToString() + ", " + m_Modifiers.m_vRecoilAngularFactors[2].ToString() + "],\n";
		else
			json += sub2 + "\"recoil_angular_factors\": null,\n";

		if (m_Modifiers.m_bHasRecoilLinearFactors)
			json += sub2 + "\"recoil_linear_factors\": [" + m_Modifiers.m_vRecoilLinearFactors[0].ToString() + ", " + m_Modifiers.m_vRecoilLinearFactors[1].ToString() + ", " + m_Modifiers.m_vRecoilLinearFactors[2].ToString() + "],\n";
		else
			json += sub2 + "\"recoil_linear_factors\": null,\n";

		if (m_Modifiers.m_bHasTurnFactors)
			json += sub2 + "\"turn_factors\": [" + m_Modifiers.m_vTurnFactors[0].ToString() + ", " + m_Modifiers.m_vTurnFactors[1].ToString() + ", " + m_Modifiers.m_vTurnFactors[2].ToString() + "]\n";
		else
			json += sub2 + "\"turn_factors\": null\n";

		json += sub + "},\n";

		// 4. Physical & Visuals
		json += sub + "\"physical\": {\n";
		if (m_Physical.m_bHasWeight)
			json += sub2 + "\"weight_kg\": " + m_Physical.m_fWeightKg.ToString() + ",\n";
		else
			json += sub2 + "\"weight_kg\": null,\n";

		if (m_Physical.m_bHasVolume)
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
		json += sub + "}\n";

		json += indent + "}";
		return json;
	}
}
