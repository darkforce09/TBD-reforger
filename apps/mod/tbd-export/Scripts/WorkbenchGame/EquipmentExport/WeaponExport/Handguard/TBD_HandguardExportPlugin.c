[WorkbenchPluginAttribute(name: "Diagnostic Export: Handguards and Foregrips", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_HandguardExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"AttachmentHandGuard"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
