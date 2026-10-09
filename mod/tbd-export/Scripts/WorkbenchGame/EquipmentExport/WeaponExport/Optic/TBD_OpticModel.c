/**
 * Standard optic records retain mounting rules, individual sight installations,
 * inventory measurements and visual references without combining source instances.
 */

class TBD_OpticMountingInfo
{
	string m_sAttachmentType;
	ref array<string> m_aCompatibleAttachmentTypes = {};
	ref array<string> m_aNativeTypeHierarchy = {}; //!< Native self/base types; separate from authored compatibility rules.
	ref array<string> m_aObstructedAttachmentTypes = {};
}

//! One native sight installation, with its own zeroing, optical and aiming configuration.
class TBD_OpticSightInstance
{
	string m_sInstanceId; //!< Native instance ID or explicit structural location.
	string m_sClassName; //!< Native sight component class.
	string m_sInstanceName; //!< Exact authored container name.
	string m_sSettingsJson; //!< Source-backed general sight and aiming settings.
	string m_sFieldOfViewJson; //!< Native FOV and zoom configuration; no unit conversion.
	string m_sZeroingJson; //!< Full ordered range tuples and configured selection.
	string m_sReticleJson; //!< Reticle resources, dimensions, colors and illumination.
	string m_sAlignmentJson; //!< Authored sight point offsets and angles.

	//! Serialize separate installations without merging their settings.
	string SerializeJson(string indent)
	{
		string sub = indent + "  ";
		string json = indent + "{\n";
		json += sub + "\"instance_id\":" + TBD_EquipmentExportJson.Quote(m_sInstanceId) + ",\n";
		json += sub + "\"native_class\":" + TBD_EquipmentExportJson.Quote(m_sClassName) + ",\n";
		json += sub + "\"instance_name\":" + TBD_EquipmentExportJson.Quote(m_sInstanceName) + ",\n";
		json += sub + "\"settings\":" + m_sSettingsJson + ",\n";
		json += sub + "\"field_of_view\":" + m_sFieldOfViewJson + ",\n";
		json += sub + "\"zeroing\":" + m_sZeroingJson + ",\n";
		json += sub + "\"reticle\":" + m_sReticleJson + ",\n";
		json += sub + "\"alignment\":" + m_sAlignmentJson + "\n";
		return json + indent + "}";
	}
}

//! Ordered effective sight instances belonging to this optic resource.
class TBD_OpticSightsInfo
{
	ref array<ref TBD_OpticSightInstance> m_aInstances = {}; //!< JSON sights array, including repeated classes.

	//! Serialize every sight instance in its collected order.
	string SerializeJson(string indent)
	{
		array<string> entries = {};
		foreach (TBD_OpticSightInstance instance : m_aInstances)
			entries.Insert(instance.SerializeJson(indent + "  "));
		if (entries.IsEmpty())
			return "[]";
		return "[\n" + TBD_EquipmentExportJson.Join(entries, ",\n") + "\n" + indent + "]";
	}
}

class TBD_OpticPhysicalInfo
{
	string m_sInventoryJson;
	string m_sPhysicsJson;
	float m_fWeightKg = -1.0;
	bool m_bHasWeight = false;
	float m_fVolumeCm3 = -1.0;
	bool m_bHasVolume = false;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
}

class TBD_OpticVisualsInfo
{
	string m_sModelMesh;
}

class TBD_OpticInfo
{
	string m_sNamesJson;
	string m_sResourceName;
	string m_sFilePath;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_OpticMountingInfo m_Mounting = new TBD_OpticMountingInfo();
	ref TBD_OpticSightsInfo m_Sights = new TBD_OpticSightsInfo();
	ref TBD_OpticPhysicalInfo m_Physical = new TBD_OpticPhysicalInfo();
	ref TBD_OpticVisualsInfo m_Visuals = new TBD_OpticVisualsInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize optic entry to clean, indented JSON string strictly adhering to Target Data Schema
	string SerializeJson(string indent = "    ")
	{
		TBD_EquipmentExportJson.IncludeResource(m_sResourceName);
		string json = "{}";
		json = TBD_EquipmentExportJson.Member(json, "resource_name", TBD_EquipmentExportJson.Quote(m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "resource_guid", TBD_EquipmentResourceNames.GuidJson(m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "file_path", TBD_EquipmentExportJson.Quote(m_sFilePath));
		json = TBD_EquipmentExportJson.Member(json, "id", TBD_EquipmentExportJson.Quote(TBD_EquipmentResourceNames.GenerateSlug(m_sFilePath)));
		string addon;
		int colon = m_sFilePath.IndexOf(":");
		if (m_sFilePath.StartsWith("$") && colon > 1) addon = m_sFilePath.Substring(1, colon - 1);
		json = TBD_EquipmentExportJson.Member(json, "addon", TBD_EquipmentExportJson.Quote(addon));
		json = TBD_EquipmentExportJson.Member(json, "names", m_sNamesJson);
		string parent = "null";
		if (!m_sVariantOf.IsEmpty()) parent = TBD_EquipmentExportJson.Quote(m_sVariantOf);
		json = TBD_EquipmentExportJson.Member(json, "parent_prefab", parent);
		string mounting = "{}";
		mounting = TBD_EquipmentExportJson.Member(mounting, "attachment_type", TBD_EquipmentExportJson.Quote(m_Mounting.m_sAttachmentType));
		mounting = TBD_EquipmentExportJson.Member(mounting, "native_type_hierarchy", TBD_EquipmentExportJson.Strings(m_Mounting.m_aNativeTypeHierarchy));
		mounting = TBD_EquipmentExportJson.Member(mounting, "compatible_attachment_types", TBD_EquipmentExportJson.Strings(m_Mounting.m_aCompatibleAttachmentTypes));
		mounting = TBD_EquipmentExportJson.Member(mounting, "exclusions", TBD_EquipmentExportJson.Strings(m_Mounting.m_aObstructedAttachmentTypes));
		json = TBD_EquipmentExportJson.Member(json, "mounting", mounting);
		json = TBD_EquipmentExportJson.Member(json, "sights", m_Sights.SerializeJson(""));
		if (!m_Physical.m_sInventoryJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "inventory", m_Physical.m_sInventoryJson);
		if (!m_Physical.m_sPhysicsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "physics", m_Physical.m_sPhysicsJson);
		string visuals = "{}";
		visuals = TBD_EquipmentExportJson.Member(visuals, "icon", TBD_EquipmentExportJson.Quote(m_sIcon));
		visuals = TBD_EquipmentExportJson.Member(visuals, "item_model", TBD_EquipmentExportJson.Quote(m_Visuals.m_sModelMesh));
		json = TBD_EquipmentExportJson.Member(json, "visuals", visuals);
		return TBD_EquipmentExportJson.Pretty(json, indent);
	}
}
