[WorkbenchPluginAttribute(name: "Diagnostic Export: All Equipment", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_EquipmentExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
