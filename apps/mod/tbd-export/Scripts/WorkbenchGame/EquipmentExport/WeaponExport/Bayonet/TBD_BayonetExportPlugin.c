[WorkbenchPluginAttribute(name: "Diagnostic Export: Bayonets", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_BayonetExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"AttachmentBayonet"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
