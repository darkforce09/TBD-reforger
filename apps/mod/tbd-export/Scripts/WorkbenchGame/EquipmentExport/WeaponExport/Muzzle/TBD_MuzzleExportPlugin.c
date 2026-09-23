[WorkbenchPluginAttribute(name: "Diagnostic Export: Muzzle Devices", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_MuzzleExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"AttachmentMuzzle"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
