[WorkbenchPluginAttribute(name: "Diagnostic Export: BTR-70 Resources", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_BTRExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {};
		TBD_SourceDiagnosticExport.Run("vehicle", nativeTypes, "Prefabs/Vehicles/Wheeled/BTR70/");
	}
}
