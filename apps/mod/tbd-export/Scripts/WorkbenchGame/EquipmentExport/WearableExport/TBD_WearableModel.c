/**
 * TBD_WearableModel.c
 *
 * Strongly typed data models representing wearable equipment, clothing, load-bearing gear,
 * body armor, storage capacities, modular attachment slots, and protection metrics for
 * the TBD Reforger Universal Equipment Export System.
 */

class TBD_WearablePhysicalInfo
{
	string m_sInventoryJson;
	string m_sPhysicsJson;
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	vector m_vDimensions = "0 0 0";
	string m_sInventorySize;
}

class TBD_WearableStorageInfo
{
	string m_sJson;
	bool m_bHasStorage = false;
	float m_fMaxWeightKg = -1.0;
	float m_fMaxVolumeCm3 = -1.0;
	int m_iCargoGridW = -1;
	int m_iCargoGridH = -1;
}

class TBD_WearableArmorInfo
{
	string m_sJson;
	bool m_bHasArmor = false;
	string m_sProtectionLevel;
	float m_fPassedDamageScale = -1.0;
	ref array<string> m_aProtectedHitZones = new array<string>();
}

class TBD_WearableSlotInfo
{
	string m_sJson; //!< Selected native slot fields and owning cloth identity.
	string m_sSlotName;
	string m_sAreaType;
	string m_sDefaultPrefab;
}

class TBD_WearableVisualInfo
{
	string m_sJson; //!< Separate model, icon and preview resource references.
	string m_sWornModel;
	string m_sItemModel;
	string m_sDeflatedModel;
}

class TBD_WearableInfo
{
	string m_sResourceName;
	string m_sNamesJson;
	string m_sLoadoutJson; //!< Per-cloth native areas and restrictions.
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
		TBD_EquipmentExportJson.IncludeResource(m_sResourceName);
		string json = indent + "{\n";
		string sub = indent + "  ";
		string sub2 = sub + "  ";

		json += sub + "\"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		json += sub + "\"id\": \"" + TBD_EquipmentExportJson.Escape(m_sId) + "\",\n";
		json += sub + "\"icon\": \"" + TBD_EquipmentExportJson.Escape(m_sIcon) + "\",\n";
		json += sub + "\"category\": \"" + TBD_EquipmentExportJson.Escape(m_sCategory) + "\",\n";
		json += sub + "\"area_type\": \"" + TBD_EquipmentExportJson.Escape(m_sAreaType) + "\",\n";
		json += sub + "\"addon\": \"" + TBD_EquipmentExportJson.Escape(m_sAddonId) + "\",\n";
		json += sub + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";
		json += sub + "\"resource_guid\": " + TBD_EquipmentResourceNames.GuidJson(m_sResourceName) + ",\n";
		json += sub + "\"names\": " + m_sNamesJson + ",\n";

		if (!m_sVariantOf.IsEmpty())
			json += sub + "\"parent_prefab\": \"" + TBD_EquipmentExportJson.Escape(m_sVariantOf) + "\",\n";
		else
			json += sub + "\"parent_prefab\": null,\n";

		// Blocked slots array
		json += sub + "\"blocked_slots\": [";
		for (int i = 0; i < m_aBlockedSlots.Count(); i++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_aBlockedSlots[i]) + "\"";
			if (i < m_aBlockedSlots.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		if (!m_Physical.m_sInventoryJson.IsEmpty()) json += sub + "\"inventory\": " + m_Physical.m_sInventoryJson + ",\n";
		if (!m_Physical.m_sPhysicsJson.IsEmpty()) json += sub + "\"physics\": " + m_Physical.m_sPhysicsJson + ",\n";
		if (!m_Storage.m_sJson.IsEmpty()) json += sub + "\"storage\": " + m_Storage.m_sJson + ",\n";
		if (!m_sLoadoutJson.IsEmpty()) json += sub + "\"loadout\": " + m_sLoadoutJson + ",\n";
		if (!m_Armor.m_sJson.IsEmpty()) json += sub + "\"armor\": " + m_Armor.m_sJson + ",\n";

		// Slots section
		json += sub + "\"slots\": [\n";
		for (int s = 0; s < m_aSlots.Count(); s++)
		{
			TBD_WearableSlotInfo slot = m_aSlots[s];
			json += sub2 + slot.m_sJson;
			if (s < m_aSlots.Count() - 1)
				json += ",";
			json += "\n";
		}
		json += sub + "],\n";

		json += sub + "\"visuals\": " + m_Visual.m_sJson + "\n";

		json += indent + "}";
		return json;
	}
}
