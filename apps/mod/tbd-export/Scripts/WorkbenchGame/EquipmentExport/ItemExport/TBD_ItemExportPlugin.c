[WorkbenchPluginAttribute(name: "Diagnostic Export: Inventory Items", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_ItemExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"InventoryItemComponent"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
