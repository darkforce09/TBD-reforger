[WorkbenchPluginAttribute(name: "Diagnostic Export: M16 Resources", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_M16ExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "Prefabs/Weapons/Rifles/M16/");
	}
}
