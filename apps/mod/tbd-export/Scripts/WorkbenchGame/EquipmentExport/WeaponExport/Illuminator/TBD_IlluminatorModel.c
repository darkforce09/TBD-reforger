//------------------------------------------------------------------------------------------------
// TBD_IlluminatorModel.c
//
// Strong data models representing weapon-mounted tactical lights, IR illuminators,
// laser aiming modules, and combo devices for the TBD Reforger Universal Equipment Export System.
//
// Pure ground-truth extraction:
//   - ZERO synthetic defaults: omitted values serialize as JSON null.
//   - ZERO token mutation: raw engine localization strings (e.g. #AR-Item_Flashlight_Soviet_Name)
//     are preserved verbatim without stripping # or humanizing.
//   - ZERO hardcoded data: mounting keys, optical lenses, lighting metrics, and physical
//     properties reflect only what is genuinely declared on the BaseContainer and its container ancestry.
//   - NO synthetic fields: id, family, category, and addon are eliminated. Identification is
//     purely via canonical resource_name and file_path.
//------------------------------------------------------------------------------------------------

class TBD_TacticalIlluminatorLensInfo
{
	string m_sDescription;
	vector m_vColor = "1 1 1";
	float m_fLightValue = 0.0;

	//------------------------------------------------------------------------------------------------
	string SerializeJson(string indent = "          ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";

		if (!m_sDescription.IsEmpty())
			json += sub + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		else
			json += sub + "\"description\": null,\n";

		json += sub + "\"color\": [" + m_vColor[0].ToString() + ", " + m_vColor[1].ToString() + ", " + m_vColor[2].ToString() + "],\n";
		json += sub + "\"light_value\": " + m_fLightValue.ToString() + "\n";
		json += indent + "}";
		return json;
	}
}

class TBD_IlluminatorMountingInfo
{
	string m_sAttachmentType;
	ref array<string> m_aCompatibleAttachmentTypes = {};
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_IlluminatorCapabilityInfo
{
	bool m_bHasFlashlight = false;
	bool m_bHasLaser = false;

	bool m_bIsIR = false;
	bool m_bHasIsIR = false;

	float m_fEmissiveIntensity = -1.0;
	bool m_bHasEmissiveIntensity = false;

	float m_fLightNearPlane = -1.0;
	bool m_bHasLightNearPlane = false;

	vector m_vAdjustOffset = "0 0 0";
	bool m_bHasAdjustOffset = false;

	ref array<float> m_aLaserColor = {};
	bool m_bHasLaserColor = false;

	ref array<ref TBD_TacticalIlluminatorLensInfo> m_aLenses = {};
	bool m_bHasLenses = false;
}

class TBD_IlluminatorPhysicalInfo
{
	float m_fWeightKg = -1.0;
	bool m_bHasWeight = false;
	float m_fVolumeCm3 = -1.0;
	bool m_bHasVolume = false;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
}

class TBD_IlluminatorVisualsInfo
{
	string m_sModelMesh;
}

class TBD_IlluminatorInfo
{
	string m_sResourceName;
	string m_sFilePath;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_IlluminatorMountingInfo m_Mounting = new TBD_IlluminatorMountingInfo();
	ref TBD_IlluminatorCapabilityInfo m_Illumination = new TBD_IlluminatorCapabilityInfo();
	ref TBD_IlluminatorPhysicalInfo m_Physical = new TBD_IlluminatorPhysicalInfo();
	ref TBD_IlluminatorVisualsInfo m_Visuals = new TBD_IlluminatorVisualsInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize illuminator entry to clean, indented JSON string strictly adhering to Target Data Schema
	string SerializeJson(string indent = "    ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";
		string sub2 = sub + "  ";
		string sub3 = sub2 + "  ";

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

		// 3. Illumination & Laser Capabilities
		json += sub + "\"illumination\": {\n";

		string hasFlStr = "false";
		if (m_Illumination.m_bHasFlashlight) hasFlStr = "true";
		json += sub2 + "\"has_flashlight\": " + hasFlStr + ",\n";

		string hasLzStr = "false";
		if (m_Illumination.m_bHasLaser) hasLzStr = "true";
		json += sub2 + "\"has_laser\": " + hasLzStr + ",\n";

		if (m_Illumination.m_bHasIsIR)
		{
			string isIrStr = "false";
			if (m_Illumination.m_bIsIR) isIrStr = "true";
			json += sub2 + "\"is_ir\": " + isIrStr + ",\n";
		}
		else
		{
			json += sub2 + "\"is_ir\": null,\n";
		}

		if (m_Illumination.m_bHasEmissiveIntensity)
			json += sub2 + "\"emissive_intensity\": " + m_Illumination.m_fEmissiveIntensity.ToString() + ",\n";
		else
			json += sub2 + "\"emissive_intensity\": null,\n";

		if (m_Illumination.m_bHasLightNearPlane)
			json += sub2 + "\"light_near_plane\": " + m_Illumination.m_fLightNearPlane.ToString() + ",\n";
		else
			json += sub2 + "\"light_near_plane\": null,\n";

		if (m_Illumination.m_bHasAdjustOffset)
			json += sub2 + "\"adjust_offset\": [" + m_Illumination.m_vAdjustOffset[0].ToString() + ", " + m_Illumination.m_vAdjustOffset[1].ToString() + ", " + m_Illumination.m_vAdjustOffset[2].ToString() + "],\n";
		else
			json += sub2 + "\"adjust_offset\": null,\n";

		if (m_Illumination.m_bHasLaserColor && m_Illumination.m_aLaserColor.Count() > 0)
		{
			json += sub2 + "\"laser_color\": [";
			for (int lc = 0; lc < m_Illumination.m_aLaserColor.Count(); lc++)
			{
				json += m_Illumination.m_aLaserColor[lc].ToString();
				if (lc < m_Illumination.m_aLaserColor.Count() - 1)
					json += ", ";
			}
			json += "],\n";
		}
		else
		{
			json += sub2 + "\"laser_color\": null,\n";
		}

		if (m_Illumination.m_bHasLenses && m_Illumination.m_aLenses.Count() > 0)
		{
			json += sub2 + "\"lenses\": [\n";
			for (int l = 0; l < m_Illumination.m_aLenses.Count(); l++)
			{
				json += m_Illumination.m_aLenses[l].SerializeJson(sub3);
				if (l < m_Illumination.m_aLenses.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += sub2 + "]\n";
		}
		else
		{
			json += sub2 + "\"lenses\": null\n";
		}
		json += sub + "},\n";

		// 4. Physical
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

		// 5. Visuals (3D Mesh)
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
