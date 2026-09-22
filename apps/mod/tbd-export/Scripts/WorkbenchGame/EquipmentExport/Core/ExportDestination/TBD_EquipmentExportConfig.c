/**
 * TBD_EquipmentExportConfig.c
 *
 * User-configurable settings and dialog parameters for the TBD Workbench Equipment Data Exporter.
 */

class TBD_EquipmentExportConfig
{
	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Destination directory for equipment export (e.g. $profile:TBD_Export/equipment/)")]
	string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base template prefabs (*_base.et / '* Base')")]
	bool m_bIncludeAbstract;

	[Attribute("0", UIWidgets.CheckBox, "Verbose console logging during prefab inspection")]
	bool m_bVerboseLog;

	//------------------------------------------------------------------------------------------------
	void TBD_EquipmentExportConfig()
	{
		m_sDestinationDir = "$profile:TBD_Export/equipment/";
		m_bIncludeAbstract = true;
		m_bVerboseLog = false;
	}
}
