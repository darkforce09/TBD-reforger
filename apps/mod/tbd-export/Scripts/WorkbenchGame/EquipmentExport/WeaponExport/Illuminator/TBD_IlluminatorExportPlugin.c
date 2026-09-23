[WorkbenchPluginAttribute(name: "Diagnostic Export: Lights and Pointers", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_IlluminatorExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"SCR_FlashlightComponent", "SCR_LaserComponent", "SCR_LaserPointerComponent", "FlashlightComponent", "LaserComponent"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
