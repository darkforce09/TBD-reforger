/**
 * TBD_WearableModel.c
 *
 * Strongly typed data models representing wearable equipment, clothing, load-bearing gear,
 * body armor, storage capacities, modular attachment slots, and protection metrics for
 * the TBD Reforger Universal Equipment Export System.
 */

class TBD_WearablePhysicalInfo
{
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	vector m_vDimensions = "0 0 0";
	string m_sInventorySize;
}

class TBD_WearableStorageInfo
{
	bool m_bHasStorage = false;
	float m_fMaxWeightKg = -1.0;
	float m_fMaxVolumeCm3 = -1.0;
	int m_iCargoGridW = -1;
	int m_iCargoGridH = -1;
}

class TBD_WearableArmorInfo
{
	bool m_bHasArmor = false;
	string m_sProtectionLevel;
	float m_fPassedDamageScale = -1.0;
	ref array<string> m_aProtectedHitZones = new array<string>();
}

class TBD_WearableSlotInfo
{
	string m_sSlotName;
	string m_sAreaType;
	string m_sDefaultPrefab;
}

class TBD_WearableVisualInfo
{
	string m_sWornModel;
	string m_sItemModel;
	string m_sDeflatedModel;
}

class TBD_WearableInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	string m_sCategory; // "headgear", "face_cover", "eyewear", "jackets", "pants", "boots", "gloves", "armored_vests", "vests_and_rigs", "backpacks", "accessories"
	string m_sAreaType; // Exact LoadoutAreaType class name
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref array<string> m_aBlockedSlots = new array<string>();
	ref TBD_WearablePhysicalInfo m_Physical = new TBD_WearablePhysicalInfo();
	ref TBD_WearableStorageInfo m_Storage = new TBD_WearableStorageInfo();
	ref TBD_WearableArmorInfo m_Armor = new TBD_WearableArmorInfo();
	ref array<ref TBD_WearableSlotInfo> m_aSlots = new array<ref TBD_WearableSlotInfo>();
	ref TBD_WearableVisualInfo m_Visual = new TBD_WearableVisualInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize wearable entry to clean, indented JSON string.
	string SerializeJson(string indent = "    ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";
		string sub2 = sub + "  ";

		json += sub + "\"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		json += sub + "\"id\": \"" + TBD_EquipmentExportJson.Escape(m_sId) + "\",\n";
		json += sub + "\"display_name\": \"" + TBD_EquipmentExportJson.Escape(m_sDisplayName) + "\",\n";
		json += sub + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		json += sub + "\"icon\": \"" + TBD_EquipmentExportJson.Escape(m_sIcon) + "\",\n";
		json += sub + "\"category\": \"" + TBD_EquipmentExportJson.Escape(m_sCategory) + "\",\n";
		json += sub + "\"area_type\": \"" + TBD_EquipmentExportJson.Escape(m_sAreaType) + "\",\n";
		json += sub + "\"addon_id\": \"" + TBD_EquipmentExportJson.Escape(m_sAddonId) + "\",\n";
		json += sub + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";
		json += sub + "\"is_abstract\": " + m_bIsAbstract.ToString() + ",\n";

		if (!m_sVariantOf.IsEmpty())
			json += sub + "\"variant_of\": \"" + TBD_EquipmentExportJson.Escape(m_sVariantOf) + "\",\n";
		else
			json += sub + "\"variant_of\": null,\n";

		// Blocked slots array
		json += sub + "\"blocked_slots\": [";
		for (int i = 0; i < m_aBlockedSlots.Count(); i++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_aBlockedSlots[i]) + "\"";
			if (i < m_aBlockedSlots.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		// Physical section
		json += sub + "\"physical\": {\n";
		if (m_Physical.m_fWeightKg >= 0)
			json += sub2 + "\"weight_kg\": " + m_Physical.m_fWeightKg.ToString() + ",\n";
		else
			json += sub2 + "\"weight_kg\": null,\n";

		if (m_Physical.m_fVolumeCm3 >= 0)
			json += sub2 + "\"volume_cm3\": " + m_Physical.m_fVolumeCm3.ToString() + ",\n";
		else
			json += sub2 + "\"volume_cm3\": null,\n";

		json += sub2 + "\"dimensions\": \"" + m_Physical.m_vDimensions.ToString(false) + "\",\n";
		json += sub2 + "\"inventory_size\": \"" + TBD_EquipmentExportJson.Escape(m_Physical.m_sInventorySize) + "\"\n";
		json += sub + "},\n";

		// Storage section
		json += sub + "\"storage\": {\n";
		json += sub2 + "\"has_storage\": " + m_Storage.m_bHasStorage.ToString() + ",\n";
		if (m_Storage.m_fMaxWeightKg >= 0)
			json += sub2 + "\"max_weight_kg\": " + m_Storage.m_fMaxWeightKg.ToString() + ",\n";
		else
			json += sub2 + "\"max_weight_kg\": null,\n";

		if (m_Storage.m_fMaxVolumeCm3 >= 0)
			json += sub2 + "\"max_volume_cm3\": " + m_Storage.m_fMaxVolumeCm3.ToString() + ",\n";
		else
			json += sub2 + "\"max_volume_cm3\": null,\n";

		if (m_Storage.m_iCargoGridW >= 0)
			json += sub2 + "\"cargo_grid_w\": " + m_Storage.m_iCargoGridW.ToString() + ",\n";
		else
			json += sub2 + "\"cargo_grid_w\": null,\n";

		if (m_Storage.m_iCargoGridH >= 0)
			json += sub2 + "\"cargo_grid_h\": " + m_Storage.m_iCargoGridH.ToString() + "\n";
		else
			json += sub2 + "\"cargo_grid_h\": null\n";
		json += sub + "},\n";

		// Armor section
		json += sub + "\"armor\": {\n";
		json += sub2 + "\"has_armor\": " + m_Armor.m_bHasArmor.ToString() + ",\n";
		json += sub2 + "\"protection_level\": \"" + TBD_EquipmentExportJson.Escape(m_Armor.m_sProtectionLevel) + "\",\n";
		if (m_Armor.m_fPassedDamageScale >= 0)
			json += sub2 + "\"passed_damage_scale\": " + m_Armor.m_fPassedDamageScale.ToString() + ",\n";
		else
			json += sub2 + "\"passed_damage_scale\": null,\n";

		json += sub2 + "\"protected_hit_zones\": [";
		for (int j = 0; j < m_Armor.m_aProtectedHitZones.Count(); j++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_Armor.m_aProtectedHitZones[j]) + "\"";
			if (j < m_Armor.m_aProtectedHitZones.Count() - 1)
				json += ", ";
		}
		json += "]\n";
		json += sub + "},\n";

		// Slots section
		json += sub + "\"slots\": [\n";
		for (int s = 0; s < m_aSlots.Count(); s++)
		{
			TBD_WearableSlotInfo slot = m_aSlots[s];
			json += sub2 + "{\n";
			string sub3 = sub2 + "  ";
			json += sub3 + "\"slot_name\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sSlotName) + "\",\n";
			json += sub3 + "\"area_type\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sAreaType) + "\",\n";
			json += sub3 + "\"default_prefab\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sDefaultPrefab) + "\"\n";
			json += sub2 + "}";
			if (s < m_aSlots.Count() - 1)
				json += ",";
			json += "\n";
		}
		json += sub + "],\n";

		// Visual section
		json += sub + "\"visual\": {\n";
		json += sub2 + "\"worn_model\": \"" + TBD_EquipmentExportJson.Escape(m_Visual.m_sWornModel) + "\",\n";
		json += sub2 + "\"item_model\": \"" + TBD_EquipmentExportJson.Escape(m_Visual.m_sItemModel) + "\",\n";
		json += sub2 + "\"deflated_model\": \"" + TBD_EquipmentExportJson.Escape(m_Visual.m_sDeflatedModel) + "\"\n";
		json += sub + "}\n";

		json += indent + "}";
		return json;
	}
}
