/**
 * TBD_ItemModel.c
 *
 * Strongly typed data models representing inventory items, medical supplies, radios,
 * navigation instruments, optics, tools, explosives, and survival gear for the
 * TBD Reforger Universal Equipment Export System.
 */

class TBD_ItemPhysicalInfo
{
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	vector m_vDimensions = "0 0 0";
	string m_sInventorySize;
	bool m_bIsStackable = false;
	int m_iMaxStack = 1;
}

class TBD_ItemMedicalInfo
{
	bool m_bIsMedical = false;
	string m_sConsumableType;
	float m_fEffectValue = -1.0;
}

class TBD_ItemRadioInfo
{
	bool m_bIsRadio = false;
	string m_sRadioType;
	float m_fMinFrequency = -1.0;
	float m_fMaxFrequency = -1.0;
	int m_iChannelCount = -1;
	float m_fTransmissionPower = -1.0;
	bool m_bHasEncryption = false;
}

class TBD_ItemGadgetInfo
{
	bool m_bIsGadget = false;
	string m_sGadgetType;
	float m_fMagnification = -1.0;
	float m_fFieldOfView = -1.0;
}

class TBD_ItemExplosiveInfo
{
	bool m_bIsExplosive = false;
	string m_sExplosiveType;
	float m_fArmedDelay = -1.0;
	float m_fTriggerPressureKg = -1.0;
}

class TBD_ItemToolInfo
{
	bool m_bIsTool = false;
	string m_sToolType;
	string m_sActionType;
}

class TBD_ItemSurvivalInfo
{
	bool m_bIsSurvival = false;
	string m_sSurvivalType;
	float m_fCapacity = -1.0;
}

class TBD_ItemInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	string m_sCategory; // "medical", "radios", "navigation", "binoculars", "flashlights", "tools", "explosives", "throwables", "weapon_parts", "survival", "intel_and_misc"
	string m_sFamily;
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_ItemPhysicalInfo m_Physical = new TBD_ItemPhysicalInfo();
	ref TBD_ItemMedicalInfo m_Medical = new TBD_ItemMedicalInfo();
	ref TBD_ItemRadioInfo m_Radio = new TBD_ItemRadioInfo();
	ref TBD_ItemGadgetInfo m_Gadget = new TBD_ItemGadgetInfo();
	ref TBD_ItemExplosiveInfo m_Explosive = new TBD_ItemExplosiveInfo();
	ref TBD_ItemToolInfo m_Tool = new TBD_ItemToolInfo();
	ref TBD_ItemSurvivalInfo m_Survival = new TBD_ItemSurvivalInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize inventory item entry to clean, indented JSON string.
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
		json += sub + "\"family\": \"" + TBD_EquipmentExportJson.Escape(m_sFamily) + "\",\n";
		json += sub + "\"addon\": \"" + TBD_EquipmentExportJson.Escape(m_sAddonId) + "\",\n";
		json += sub + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";
		json += sub + "\"is_abstract\": " + m_bIsAbstract.ToString() + ",\n";

		if (!m_sVariantOf.IsEmpty())
			json += sub + "\"variant_of\": \"" + TBD_EquipmentExportJson.Escape(m_sVariantOf) + "\",\n";
		else
			json += sub + "\"variant_of\": null,\n";

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
		json += sub2 + "\"inventory_size\": \"" + TBD_EquipmentExportJson.Escape(m_Physical.m_sInventorySize) + "\",\n";
		json += sub2 + "\"is_stackable\": " + m_Physical.m_bIsStackable.ToString() + ",\n";
		json += sub2 + "\"max_stack\": " + m_Physical.m_iMaxStack.ToString() + "\n";
		json += sub + "},\n";

		// Medical section
		if (m_Medical.m_bIsMedical)
		{
			json += sub + "\"medical\": {\n";
			json += sub2 + "\"consumable_type\": \"" + TBD_EquipmentExportJson.Escape(m_Medical.m_sConsumableType) + "\",\n";
			if (m_Medical.m_fEffectValue >= 0)
				json += sub2 + "\"effect_value\": " + m_Medical.m_fEffectValue.ToString() + "\n";
			else
				json += sub2 + "\"effect_value\": null\n";
			json += sub + "},\n";
		}
		else
		{
			json += sub + "\"medical\": null,\n";
		}

		// Radio section
		if (m_Radio.m_bIsRadio)
		{
			json += sub + "\"radio\": {\n";
			json += sub2 + "\"radio_type\": \"" + TBD_EquipmentExportJson.Escape(m_Radio.m_sRadioType) + "\",\n";
			if (m_Radio.m_fMinFrequency >= 0)
				json += sub2 + "\"min_frequency\": " + m_Radio.m_fMinFrequency.ToString() + ",\n";
			else
				json += sub2 + "\"min_frequency\": null,\n";
			if (m_Radio.m_fMaxFrequency >= 0)
				json += sub2 + "\"max_frequency\": " + m_Radio.m_fMaxFrequency.ToString() + ",\n";
			else
				json += sub2 + "\"max_frequency\": null,\n";
			if (m_Radio.m_iChannelCount >= 0)
				json += sub2 + "\"channel_count\": " + m_Radio.m_iChannelCount.ToString() + ",\n";
			else
				json += sub2 + "\"channel_count\": null,\n";
			if (m_Radio.m_fTransmissionPower >= 0)
				json += sub2 + "\"power_watts\": " + m_Radio.m_fTransmissionPower.ToString() + ",\n";
			else
				json += sub2 + "\"power_watts\": null,\n";
			json += sub2 + "\"encryption\": " + m_Radio.m_bHasEncryption.ToString() + "\n";
			json += sub + "},\n";
		}
		else
		{
			json += sub + "\"radio\": null,\n";
		}

		// Gadget section
		if (m_Gadget.m_bIsGadget)
		{
			json += sub + "\"gadget\": {\n";
			json += sub2 + "\"gadget_type\": \"" + TBD_EquipmentExportJson.Escape(m_Gadget.m_sGadgetType) + "\",\n";
			if (m_Gadget.m_fMagnification >= 0)
				json += sub2 + "\"magnification\": " + m_Gadget.m_fMagnification.ToString() + ",\n";
			else
				json += sub2 + "\"magnification\": null,\n";
			if (m_Gadget.m_fFieldOfView >= 0)
				json += sub2 + "\"field_of_view\": " + m_Gadget.m_fFieldOfView.ToString() + "\n";
			else
				json += sub2 + "\"field_of_view\": null\n";
			json += sub + "},\n";
		}
		else
		{
			json += sub + "\"gadget\": null,\n";
		}

		// Explosive section
		if (m_Explosive.m_bIsExplosive)
		{
			json += sub + "\"explosive\": {\n";
			json += sub2 + "\"explosive_type\": \"" + TBD_EquipmentExportJson.Escape(m_Explosive.m_sExplosiveType) + "\",\n";
			if (m_Explosive.m_fArmedDelay >= 0)
				json += sub2 + "\"armed_delay\": " + m_Explosive.m_fArmedDelay.ToString() + ",\n";
			else
				json += sub2 + "\"armed_delay\": null,\n";
			if (m_Explosive.m_fTriggerPressureKg >= 0)
				json += sub2 + "\"trigger_pressure_kg\": " + m_Explosive.m_fTriggerPressureKg.ToString() + "\n";
			else
				json += sub2 + "\"trigger_pressure_kg\": null\n";
			json += sub + "},\n";
		}
		else
		{
			json += sub + "\"explosive\": null,\n";
		}

		// Tool section
		if (m_Tool.m_bIsTool)
		{
			json += sub + "\"tool\": {\n";
			json += sub2 + "\"tool_type\": \"" + TBD_EquipmentExportJson.Escape(m_Tool.m_sToolType) + "\",\n";
			json += sub2 + "\"action_type\": \"" + TBD_EquipmentExportJson.Escape(m_Tool.m_sActionType) + "\"\n";
			json += sub + "},\n";
		}
		else
		{
			json += sub + "\"tool\": null,\n";
		}

		// Survival section
		if (m_Survival.m_bIsSurvival)
		{
			json += sub + "\"survival\": {\n";
			json += sub2 + "\"survival_type\": \"" + TBD_EquipmentExportJson.Escape(m_Survival.m_sSurvivalType) + "\",\n";
			if (m_Survival.m_fCapacity >= 0)
				json += sub2 + "\"capacity\": " + m_Survival.m_fCapacity.ToString() + "\n";
			else
				json += sub2 + "\"capacity\": null\n";
			json += sub + "}\n";
		}
		else
		{
			json += sub + "\"survival\": null\n";
		}

		json += indent + "}";
		return json;
	}
}
