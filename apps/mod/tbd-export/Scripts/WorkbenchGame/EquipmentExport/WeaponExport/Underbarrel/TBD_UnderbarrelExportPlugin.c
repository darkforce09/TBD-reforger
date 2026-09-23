[WorkbenchPluginAttribute(name: "Diagnostic Export: Underbarrel Devices", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_UnderbarrelExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"AttachmentUnderBarrel"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
