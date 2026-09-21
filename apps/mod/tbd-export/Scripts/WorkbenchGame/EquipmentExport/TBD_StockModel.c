//------------------------------------------------------------------------------------------------
// TBD_StockModel.c
//
// Strong data models representing weapon buttstocks and stock assemblies for the TBD Reforger
// Universal Equipment Export System.
//
// Pure ground-truth extraction:
//   - ZERO synthetic defaults: omitted values serialize as JSON null.
//   - ZERO token mutation: raw engine localization strings (e.g. #AR-Item_Stock_Vz58_Wood_Name)
//     are preserved verbatim without stripping # or humanizing.
//   - ZERO hardcoded data: mounting keys, child slots, handling modifiers, and physical
//     properties reflect only what is genuinely declared on the BaseContainer and its container ancestry.
//   - NO synthetic fields: id, family, category, and addon are eliminated. Identification is
//     purely via canonical resource_name and file_path.
//   - NESTED ATTACHMENT HOST: modular stocks declare child slots (cheek pads, cheek risers,
//     sling swivel attachment points) via AttachmentSlotComponent.
//------------------------------------------------------------------------------------------------

class TBD_StockMountingInfo
{
	string m_sAttachmentType;
	ref array<string> m_aCompatibleAttachmentTypes = {};
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_StockSlotInfo
{
	string m_sSlotName;
	ref array<string> m_aCompatibleAttachmentTypes = {};

	//------------------------------------------------------------------------------------------------
	//! Serialize nested child slot entry to clean, indented JSON string
	string SerializeJson(string indent = "        ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";

		json += sub + "\"slot_name\": \"" + TBD_EquipmentExportJson.Escape(m_sSlotName) + "\",\n";
		json += sub + "\"compatible_attachment_types\": [";
		for (int c = 0; c < m_aCompatibleAttachmentTypes.Count(); c++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_aCompatibleAttachmentTypes[c]) + "\"";
			if (c < m_aCompatibleAttachmentTypes.Count() - 1)
				json += ", ";
		}
		json += "]\n";

		json += indent + "}";
		return json;
	}
}

class TBD_StockHandlingInfo
{
	vector m_vRecoilAngularFactors = "0 0 0";
	bool m_bHasRecoilAngularFactors = false;
	vector m_vRecoilLinearFactors = "0 0 0";
	bool m_bHasRecoilLinearFactors = false;
	vector m_vTurnFactors = "0 0 0";
	bool m_bHasTurnFactors = false;
	float m_fExtraObstructionLength = 0.0;
	bool m_bHasExtraObstructionLength = false;
}

class TBD_StockPhysicalInfo
{
	float m_fWeightKg = -1.0;
	bool m_bHasWeight = false;
	float m_fVolumeCm3 = -1.0;
	bool m_bHasVolume = false;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
}

class TBD_StockVisualsInfo
{
	string m_sModelMesh;
}

class TBD_StockInfo
{
	string m_sResourceName;
	string m_sFilePath;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_StockMountingInfo m_Mounting = new TBD_StockMountingInfo();
	ref array<ref TBD_StockSlotInfo> m_aNestedSlots = {};
	ref TBD_StockHandlingInfo m_Handling = new TBD_StockHandlingInfo();
	ref TBD_StockPhysicalInfo m_Physical = new TBD_StockPhysicalInfo();
	ref TBD_StockVisualsInfo m_Visuals = new TBD_StockVisualsInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize buttstock entry to clean, indented JSON string strictly adhering to Target Data Schema
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

		// 3. Nested Attachment Slots (child slots hosted by this stock: cheek pads, risers, sling points)
		if (m_aNestedSlots.Count() > 0)
		{
			json += sub + "\"nested_attachment_slots\": [\n";
			for (int s = 0; s < m_aNestedSlots.Count(); s++)
			{
				json += m_aNestedSlots[s].SerializeJson(sub2);
				if (s < m_aNestedSlots.Count() - 1)
					json += ",\n";
				else
					json += "\n";
			}
			json += sub + "],\n";
		}
		else
		{
			json += sub + "\"nested_attachment_slots\": [],\n";
		}

		// 4. Handling & Recoil Modifiers
		json += sub + "\"handling_modifiers\": {\n";
		if (m_Handling.m_bHasRecoilAngularFactors)
			json += sub2 + "\"recoil_angular_factors\": [" + m_Handling.m_vRecoilAngularFactors[0].ToString() + ", " + m_Handling.m_vRecoilAngularFactors[1].ToString() + ", " + m_Handling.m_vRecoilAngularFactors[2].ToString() + "],\n";
		else
			json += sub2 + "\"recoil_angular_factors\": null,\n";

		if (m_Handling.m_bHasRecoilLinearFactors)
			json += sub2 + "\"recoil_linear_factors\": [" + m_Handling.m_vRecoilLinearFactors[0].ToString() + ", " + m_Handling.m_vRecoilLinearFactors[1].ToString() + ", " + m_Handling.m_vRecoilLinearFactors[2].ToString() + "],\n";
		else
			json += sub2 + "\"recoil_linear_factors\": null,\n";

		if (m_Handling.m_bHasTurnFactors)
			json += sub2 + "\"turn_factors\": [" + m_Handling.m_vTurnFactors[0].ToString() + ", " + m_Handling.m_vTurnFactors[1].ToString() + ", " + m_Handling.m_vTurnFactors[2].ToString() + "],\n";
		else
			json += sub2 + "\"turn_factors\": null,\n";

		if (m_Handling.m_bHasExtraObstructionLength)
			json += sub2 + "\"extra_obstruction_length\": " + m_Handling.m_fExtraObstructionLength.ToString() + "\n";
		else
			json += sub2 + "\"extra_obstruction_length\": null\n";
		json += sub + "},\n";

		// 5. Physical
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

		// 6. Visuals (3D Mesh)
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
