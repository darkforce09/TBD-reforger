// Compact and diagnostic exports share discovery and native reading, with separate publications.
class TBD_GameplayExportGeneration : TBD_SourceExportGeneration
{
	protected ref TBD_GameplaySelectionPolicy m_Policy = new TBD_GameplaySelectionPolicy();
	protected ref TBD_GameplayFieldDefinitions m_Definitions = new TBD_GameplayFieldDefinitions();

	override protected string Destination()
	{
		return "$profile:TBD_Export/equipment_vehicle_exports/gameplay/generations/";
	}

	override protected void CaptureResource(string name)
	{
		Resource resource = Resource.Load(name);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{ m_aErrors.Insert("Cannot load gameplay resource: " + name); return; }
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root) { m_aErrors.Insert("Gameplay resource has no source container: " + name); return; }
		TBD_SourceContainerReader reader = new TBD_SourceContainerReader();
		reader.m_sResource = name;
		reader.m_GameplayPolicy = m_Policy;
		reader.Capture(root);
		foreach (string error : reader.m_aErrors) m_aErrors.Insert(name + ": " + error);
		foreach (string typeName : reader.m_Types.m_aTypes) m_Types.Add(typeName);
		if (reader.m_aNodes.IsEmpty()) return;
		TBD_GameplayResourceWriter writer = new TBD_GameplayResourceWriter();
		writer.m_Definitions = m_Definitions;
		writer.m_Policy = m_Policy;
		string identity = TBD_SourceCapabilityBuilder.ResourceIdentity(name);
		array<string> domains = {};
		if (m_aEquipment.Find(name) >= 0) { domains.Insert("equipment"); m_aEquipmentIds.Insert(identity); }
		if (m_aVehicles.Find(name) >= 0) { domains.Insert("vehicle"); m_aVehicleIds.Insert(identity); }
		string prefix = "resources/";
		if (domains.IsEmpty()) { domains.Insert("dependency"); prefix = "shared_configurations/"; }
		string relative = prefix + ResourceFile(reader);
		if (!WriteDocument(relative, writer.Build(reader))) return;
		string entry = "{\"resource_id\":" + TBD_SourceExportJson.Quote(identity);
		entry += ",\"resource_name\":" + TBD_SourceExportJson.Quote(name);
		entry += ",\"resource_file\":" + TBD_SourceExportJson.Quote(relative);
		entry += ",\"domains\":" + TBD_SourceExportJson.Strings(domains) + "}";
		m_aEntries.Insert(entry);
		foreach (TBD_SourceExportReference link : reader.m_aReferences)
			if (link.m_sKind == "gameplay") Enqueue(link.m_sResource);
	}

	override void Finish()
	{
		m_bFinished = true;
		m_aEquipmentIds.Sort();
		m_aVehicleIds.Sort();
		string hierarchy = m_Types.Json();
		foreach (string hierarchyError : m_Types.m_aErrors) m_aErrors.Insert(hierarchyError);
		foreach (string policyError : m_Policy.m_aErrors) m_aErrors.Insert(policyError);
		WriteDocument("resource_index.json", "{\"schema_version\":1,\"resources\":[" + TBD_SourceExportJson.Join(m_aEntries) + "]}");
		WriteDocument("field_definitions.json", m_Definitions.Json());
		WriteDocument("native_types.json", hierarchy);
		WriteDocument("selection_report.json", m_Policy.Report(m_aEntries.Count(), m_Definitions.m_iNodes, m_Definitions.m_iFacts));
		string status = "completed";
		if (!m_aErrors.IsEmpty()) status = "failed";
		string json = "{\"document_type\":\"gameplay_generation\",\"schema_version\":1,\"dataset_kind\":\"gameplay\",\"generation_id\":" + TBD_SourceExportJson.Quote(m_sGenerationId);
		json += ",\"scope\":" + TBD_SourceExportJson.Quote(m_sScope) + ",\"status\":" + TBD_SourceExportJson.Quote(status);
		json += ",\"started_at\":" + TBD_SourceExportJson.Quote(m_sStarted);
		json += ",\"finished_at\":" + TBD_SourceExportJson.Quote(TBD_SourceExportEnvironment.Timestamp());
		json += ",\"environment\":" + m_sEnvironment;
		json += ",\"equipment_ids\":" + TBD_SourceExportJson.Strings(m_aEquipmentIds);
		json += ",\"vehicle_ids\":" + TBD_SourceExportJson.Strings(m_aVehicleIds);
		json += ",\"errors\":" + TBD_SourceExportJson.Strings(m_aErrors);
		json += ",\"reader_verification\":" + m_Verification.Json();
		json += ",\"policy_version\":1,\"policy_sha256\":" + TBD_SourceExportJson.Quote(TBD_GameplayPolicyGenerated.DIGEST);
		json += ",\"extraction_method\":\"workbench_native\"}";
		WriteDocument("generation.json", json);
		WriteProgress();
	}
}
