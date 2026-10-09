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

//! Source-backed medical capability values.
class TBD_ItemMedicalInfo
{
	string m_sJson; //!< Empty when inapplicable; plain medical JSON object otherwise.
	bool m_bIsMedical = false;
	string m_sConsumableType;
	float m_fEffectValue = -1.0;
}

//! Source-backed radio capability values.
class TBD_ItemRadioInfo
{
	string m_sJson; //!< Empty when inapplicable; plain radio JSON object otherwise.
	bool m_bIsRadio = false;
	string m_sRadioType;
	float m_fMinFrequency = -1.0;
	float m_fMaxFrequency = -1.0;
	int m_iChannelCount = -1;
	float m_fTransmissionPower = -1.0;
	bool m_bHasEncryption = false;
}

//! Source-backed gadget capability values.
class TBD_ItemGadgetInfo
{
	string m_sJson; //!< Empty when inapplicable; plain gadget JSON object otherwise.
	bool m_bIsGadget = false;
	string m_sGadgetType;
	float m_fMagnification = -1.0;
	float m_fFieldOfView = -1.0;
}

//! Source-backed explosive capability values.
class TBD_ItemExplosiveInfo
{
	string m_sJson; //!< Empty when inapplicable; plain explosive JSON object otherwise.
	bool m_bIsExplosive = false;
	string m_sExplosiveType;
	float m_fArmedDelay = -1.0;
	float m_fTriggerPressureKg = -1.0;
}

//! Source-backed tool capability values.
class TBD_ItemToolInfo
{
	string m_sJson; //!< Empty when inapplicable; plain tool JSON object otherwise.
	bool m_bIsTool = false;
	string m_sToolType;
	string m_sActionType;
}

//! Source-backed survival capability values.
class TBD_ItemSurvivalInfo
{
	string m_sJson; //!< Empty when inapplicable; plain survival JSON object otherwise.
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

	string m_sInventoryJson; //!< Native inventory measurements and restrictions.
	string m_sPhysicsJson; //!< Native physics body measurements.
	string m_sStorageJson; //!< Individual storage capacities, slots and restrictions.
	string m_sNamesJson; //!< Authored names and separately resolved English names.
	string m_sVisualsJson; //!< Separate icon, preview and model resource references.

	ref TBD_ItemPhysicalInfo m_Physical = new TBD_ItemPhysicalInfo();
	ref TBD_ItemMedicalInfo m_Medical = new TBD_ItemMedicalInfo();
	ref TBD_ItemRadioInfo m_Radio = new TBD_ItemRadioInfo();
	ref TBD_ItemGadgetInfo m_Gadget = new TBD_ItemGadgetInfo();
	ref TBD_ItemExplosiveInfo m_Explosive = new TBD_ItemExplosiveInfo();
	ref TBD_ItemToolInfo m_Tool = new TBD_ItemToolInfo();
	ref TBD_ItemSurvivalInfo m_Survival = new TBD_ItemSurvivalInfo();

	//! Encode one item; only extracted capabilities contribute sections.
	string SerializeJson(string indent = "    ")
	{
		TBD_EquipmentExportJson.IncludeResource(m_sResourceName);
		array<string> fields = {};
		AddText(fields, "resource_name", m_sResourceName);
		AddText(fields, "id", m_sId);
		string guid;
		if (m_sResourceName.Length() >= 18 && m_sResourceName.StartsWith("{"))
			guid = m_sResourceName.Substring(1, 16);
		if (guid.IsEmpty()) fields.Insert("\"resource_guid\": null");
		else AddText(fields, "resource_guid", guid);
		AddText(fields, "category", m_sCategory);
		AddText(fields, "family", m_sFamily);
		AddText(fields, "addon", m_sAddonId);
		AddText(fields, "file_path", m_sFilePath);
		if (!m_sVariantOf.IsEmpty()) AddText(fields, "parent_prefab", m_sVariantOf);
		else fields.Insert("\"parent_prefab\": null");
		AddSection(fields, "names", m_sNamesJson);
		AddSection(fields, "inventory", m_sInventoryJson);
		AddSection(fields, "physics", m_sPhysicsJson);
		AddSection(fields, "storage", m_sStorageJson);
		AddSection(fields, "medical", m_Medical.m_sJson);
		AddSection(fields, "radio", m_Radio.m_sJson);
		AddSection(fields, "gadget", m_Gadget.m_sJson);
		AddSection(fields, "explosive", m_Explosive.m_sJson);
		AddSection(fields, "tool", m_Tool.m_sJson);
		AddSection(fields, "survival", m_Survival.m_sJson);
		AddSection(fields, "visuals", m_sVisualsJson);
		return indent + "{\n" + indent + "  " + TBD_EquipmentExportJson.Join(fields, ",\n" + indent + "  ") + "\n" + indent + "}";
	}

	//! Append an escaped, ordinary text value.
	protected void AddText(array<string> fields, string name, string value)
	{
		fields.Insert(TBD_EquipmentExportJson.Quote(name) + ": " + TBD_EquipmentExportJson.Quote(value));
	}

	//! Empty extraction strings denote inapplicable sections, not JSON null values.
	protected void AddSection(array<string> fields, string name, string json)
	{
		if (!json.IsEmpty()) fields.Insert(TBD_EquipmentExportJson.Quote(name) + ": " + json);
	}
}
