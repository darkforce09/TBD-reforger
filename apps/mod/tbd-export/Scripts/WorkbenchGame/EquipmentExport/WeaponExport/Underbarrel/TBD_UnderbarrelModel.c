//------------------------------------------------------------------------------------------------
// TBD_UnderbarrelModel.c
//
// Strong data models representing underbarrel devices (secondary weapon systems, grenade
// launchers, underbarrel attachments, and accessories) for the TBD Reforger Universal Equipment
// Export System.
//
// Pure ground-truth extraction:
//   - ZERO synthetic defaults: omitted values serialize as JSON null.
//   - ZERO token mutation: raw engine localization strings (e.g. #AR-Weapon_M203_Name)
//     are preserved verbatim without stripping # or humanizing.
//   - ZERO hardcoded data: mounting keys and launcher attributes reflect only what is genuinely
//     declared on the BaseContainer and its container ancestry.
//   - NO synthetic fields: id, family, category, and addon are eliminated. Identification is
//     purely via canonical resource_name and file_path.
//------------------------------------------------------------------------------------------------

class TBD_UnderbarrelMountingInfo
{
	string m_sAttachmentType;
	ref array<string> m_aCompatibleAttachmentTypes = {};
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_UnderbarrelLauncherInfo
{
	bool m_bIsLauncher = false;
	ref array<string> m_aMagazineWells = {};
	int m_iChamberCapacity = 0;
	bool m_bHasChamberCapacity = false;
	ref array<int> m_aZeroingDistances = {};
	bool m_bHasZeroingDistances = false;
}

class TBD_UnderbarrelPhysicalInfo
{
	float m_fWeightKg = -1.0;
	bool m_bHasWeight = false;
	float m_fVolumeCm3 = -1.0;
	bool m_bHasVolume = false;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
}

class TBD_UnderbarrelVisualsInfo
{
	string m_sModelMesh;
}

class TBD_UnderbarrelInfo
{
	string m_sResourceName;
	string m_sFilePath;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_UnderbarrelMountingInfo m_Mounting = new TBD_UnderbarrelMountingInfo();
	ref TBD_UnderbarrelLauncherInfo m_Launcher = new TBD_UnderbarrelLauncherInfo();
	ref TBD_UnderbarrelPhysicalInfo m_Physical = new TBD_UnderbarrelPhysicalInfo();
	ref TBD_UnderbarrelVisualsInfo m_Visuals = new TBD_UnderbarrelVisualsInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize underbarrel device entry to clean, indented JSON string strictly adhering to Target Data Schema
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

		// 3. Launcher / Secondary Weapon System Properties
		json += sub + "\"launcher\": {\n";
		string isLauncherStr = "false";
		if (m_Launcher.m_bIsLauncher) isLauncherStr = "true";
		json += sub2 + "\"is_launcher\": " + isLauncherStr + ",\n";

		json += sub2 + "\"magazine_wells\": [";
		for (int w = 0; w < m_Launcher.m_aMagazineWells.Count(); w++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_Launcher.m_aMagazineWells[w]) + "\"";
			if (w < m_Launcher.m_aMagazineWells.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		if (m_Launcher.m_bHasChamberCapacity)
			json += sub2 + "\"chamber_capacity\": " + m_Launcher.m_iChamberCapacity.ToString() + ",\n";
		else
			json += sub2 + "\"chamber_capacity\": null,\n";

		if (m_Launcher.m_bHasZeroingDistances && m_Launcher.m_aZeroingDistances.Count() > 0)
		{
			json += sub2 + "\"zeroing_distances\": [";
			for (int zd = 0; zd < m_Launcher.m_aZeroingDistances.Count(); zd++)
			{
				json += m_Launcher.m_aZeroingDistances[zd].ToString();
				if (zd < m_Launcher.m_aZeroingDistances.Count() - 1)
					json += ", ";
			}
			json += "]\n";
		}
		else
		{
			json += sub2 + "\"zeroing_distances\": null\n";
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
