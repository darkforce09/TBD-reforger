/**
 * TBD_EquipmentScanItem.c
 *
 * Data record representing one discovered equipment prefab in the unfiltered scan.
 */

class TBD_EquipmentScanItem
{
	string m_sResourceName;
	string m_sFilePath;
	string m_sId;
	string m_sDisplayName;
	string m_sAddonId;
	string m_sCategoryPath;
	string m_sRootClass;
	bool m_bIsAbstract;
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	ref array<string> m_aDetectedComponents = {};
	ref array<string> m_aSignals = {};

	//------------------------------------------------------------------------------------------------
	string ToJson(string indent = "    ")
	{
		string json = indent + "{\n";
		json += indent + "  \"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		json += indent + "  \"id\": \"" + TBD_EquipmentExportJson.Escape(m_sId) + "\",\n";
		json += indent + "  \"display_name\": \"" + TBD_EquipmentExportJson.Escape(m_sDisplayName) + "\",\n";
		json += indent + "  \"addon\": \"" + TBD_EquipmentExportJson.Escape(m_sAddonId) + "\",\n";
		json += indent + "  \"category_path\": \"" + TBD_EquipmentExportJson.Escape(m_sCategoryPath) + "\",\n";
		json += indent + "  \"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";
		json += indent + "  \"root_class\": \"" + TBD_EquipmentExportJson.Escape(m_sRootClass) + "\",\n";

		string isAbsStr = "false";
		if (m_bIsAbstract)
			isAbsStr = "true";
		json += indent + "  \"is_abstract\": " + isAbsStr;

		if (m_fWeightKg >= 0)
			json += ",\n" + indent + "  \"weight_kg\": " + m_fWeightKg.ToString();
		if (m_fVolumeCm3 >= 0)
			json += ",\n" + indent + "  \"volume_cm3\": " + m_fVolumeCm3.ToString();

		// Detected signals
		json += ",\n" + indent + "  \"signals\": [";
		for (int s = 0; s < m_aSignals.Count(); s++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_aSignals[s]) + "\"";
			if (s < m_aSignals.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		// Detected components
		json += indent + "  \"components\": [";
		for (int c = 0; c < m_aDetectedComponents.Count(); c++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_aDetectedComponents[c]) + "\"";
			if (c < m_aDetectedComponents.Count() - 1)
				json += ", ";
		}
		json += "]\n";

		json += indent + "}";
		return json;
	}
}
