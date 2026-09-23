[WorkbenchPluginAttribute(name: "Diagnostic Export: Buttstocks", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_StockExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"AttachmentStock"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
