[WorkbenchPluginAttribute(name: "Diagnostic Export: Weapons", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_WeaponExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"WeaponComponent"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
