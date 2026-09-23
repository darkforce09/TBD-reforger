[WorkbenchPluginAttribute(name: "Diagnostic Export: Weapon Attachments", description: "Source records for inspection; diagnostic generations cannot become the current bundle", category: "TBD Diagnostics")]
class TBD_AttachmentExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		array<string> nativeTypes = {"WeaponAttachmentAttributes"};
		TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, "");
	}
}
