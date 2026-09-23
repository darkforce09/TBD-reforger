[WorkbenchPluginAttribute(name: "Diagnostic Export: Wearables and Gear", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_WearableExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"BaseLoadoutClothComponent"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
