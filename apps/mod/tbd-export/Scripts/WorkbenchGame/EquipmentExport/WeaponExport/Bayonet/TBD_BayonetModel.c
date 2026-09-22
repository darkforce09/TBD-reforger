//------------------------------------------------------------------------------------------------
// TBD_BayonetModel.c
//
// Strong data models representing rifle bayonets and blade attachments for the TBD Reforger
// Universal Equipment Export System.
//
// Pure ground-truth extraction:
//   - ZERO synthetic defaults: omitted values serialize as JSON null.
//   - ZERO token mutation: raw engine localization strings (e.g. #AR-Item_Bayonet_M9_Name)
//     are preserved verbatim without stripping # or humanizing.
//   - ZERO hardcoded data: mounting keys, obstruction lists, combat modifiers, and physical
//     properties reflect only what is genuinely declared on the BaseContainer and its container ancestry.
//   - NO synthetic fields: id, family, category, and addon are eliminated. Identification is
//     purely via canonical resource_name and file_path.
//------------------------------------------------------------------------------------------------

class TBD_BayonetMountingInfo
{
	string m_sAttachmentType;
	ref array<string> m_aCompatibleAttachmentTypes = {};
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_BayonetCombatInfo
{
	float m_fExtraObstructionLength = 0.0;
	bool m_bHasExtraObstructionLength = false;
	float m_fMeleeDamage = 0.0;
	bool m_bHasMeleeDamage = false;
}

class TBD_BayonetPhysicalInfo
{
	float m_fWeightKg = -1.0;
	bool m_bHasWeight = false;
	float m_fVolumeCm3 = -1.0;
	bool m_bHasVolume = false;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
}

class TBD_BayonetVisualsInfo
{
	string m_sModelMesh;
}

class TBD_BayonetInfo
{
	string m_sResourceName;
	string m_sFilePath;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_BayonetMountingInfo m_Mounting = new TBD_BayonetMountingInfo();
	ref TBD_BayonetCombatInfo m_Combat = new TBD_BayonetCombatInfo();
	ref TBD_BayonetPhysicalInfo m_Physical = new TBD_BayonetPhysicalInfo();
	ref TBD_BayonetVisualsInfo m_Visuals = new TBD_BayonetVisualsInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize bayonet device entry to clean, indented JSON string strictly adhering to Target Data Schema
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

		// 3. Combat & Handling
		json += sub + "\"combat_handling\": {\n";
		if (m_Combat.m_bHasExtraObstructionLength)
			json += sub2 + "\"extra_obstruction_length\": " + m_Combat.m_fExtraObstructionLength.ToString() + ",\n";
		else
			json += sub2 + "\"extra_obstruction_length\": null,\n";

		if (m_Combat.m_bHasMeleeDamage)
			json += sub2 + "\"melee_damage\": " + m_Combat.m_fMeleeDamage.ToString() + "\n";
		else
			json += sub2 + "\"melee_damage\": null\n";
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
