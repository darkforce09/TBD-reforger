[WorkbenchPluginAttribute(name: "Diagnostic Export: Optics and Sights", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_OpticExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"SightsComponent", "SCR_2DOpticsComponent"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
