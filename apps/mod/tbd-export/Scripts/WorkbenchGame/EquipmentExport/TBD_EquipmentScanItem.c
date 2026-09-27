/**
 * TBD_EquipmentScanItem.c
 *
 * Lightweight discovery record with native identity, root capability evidence and common measurements.
 */

//! Catalog measurements use the same source readers and field paths as specialist records.
class TBD_EquipmentScanItem
{
	string m_sResourceName; //!< Exact engine resource name; relationships use this native identity.
	string m_sFilePath; //!< Exact path supplied during discovery.
	string m_sId; //!< Filename alias, not resource identity.
	string m_sAddonId; //!< Loaded addon identifier.
	string m_sCategoryPath; //!< Source directory beneath Prefabs, not an inferred gameplay category.
	string m_sRootClass; //!< Native root entity class.
	string m_sNamesJson; //!< Authored names and independently resolved English text.
	string m_sInventoryJson; //!< Native inventory measurements without unit conversion.
	string m_sPhysicsJson; //!< Root-owned rigid-body measurements, separate from inventory mass.
	ref array<string> m_aDetectedComponents = {}; //!< Native component classes across the resource and its children.
	ref array<string> m_aRootComponents = {}; //!< Native component classes owned by the resource root.
	ref array<string> m_aSignals = {}; //!< Capabilities evidenced by the root and its own components.

	//! Serializes ordinary domain values and registers identity for the metadata sidecar.
	string ToJson(string indent = "    ")
	{
		TBD_EquipmentExportJson.IncludeResource(m_sResourceName);
		string json = "{}";
		json = TBD_EquipmentExportJson.Member(json, "resource_name", TBD_EquipmentExportJson.Quote(m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "resource_guid", TBD_EquipmentResourceNames.GuidJson(m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "id", TBD_EquipmentExportJson.Quote(m_sId));
		json = TBD_EquipmentExportJson.Member(json, "id_kind", "\"filename_alias\"");
		json = TBD_EquipmentExportJson.Member(json, "addon", TBD_EquipmentExportJson.Quote(m_sAddonId));
		json = TBD_EquipmentExportJson.Member(json, "category_path", TBD_EquipmentExportJson.Quote(m_sCategoryPath));
		json = TBD_EquipmentExportJson.Member(json, "file_path", TBD_EquipmentExportJson.Quote(m_sFilePath));
		json = TBD_EquipmentExportJson.Member(json, "root_class", TBD_EquipmentExportJson.Quote(m_sRootClass));
		if (!m_sNamesJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "names", m_sNamesJson);
		if (!m_sInventoryJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "inventory", m_sInventoryJson);
		if (!m_sPhysicsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "physics", m_sPhysicsJson);
		json = TBD_EquipmentExportJson.Member(json, "signals", TBD_EquipmentExportJson.Strings(m_aSignals));
		json = TBD_EquipmentExportJson.Member(json, "root_components", TBD_EquipmentExportJson.Strings(m_aRootComponents));
		json = TBD_EquipmentExportJson.Member(json, "components", TBD_EquipmentExportJson.Strings(m_aDetectedComponents));
		return TBD_EquipmentExportJson.Pretty(json, indent);
	}
}
