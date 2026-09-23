[WorkbenchPluginAttribute(name: "Diagnostic Export: Ammunition and Magazines", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_AmmoExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"MagazineComponent", "Projectile", "ProjectileMoveComponent"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
