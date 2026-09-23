[WorkbenchPluginAttribute(name: "Diagnostic Export: Vehicle Systems", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_VehicleDeepExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {};
		TBD_SourceDiagnosticExport.Run("vehicle", nativeTypes, "");
	}
}
