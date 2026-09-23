// A generation is private staging data until the Rust publisher validates and seals it.
class TBD_SourceExportGeneration
{
	static const string DESTINATION = "$profile:TBD_Export/equipment_vehicle_exports/generations/";
	static ref TBD_SourceExportGeneration s_Active;
	string m_sGenerationId;
	string m_sDirectory;
	string m_sScope;
	bool m_bFinished;
	int m_iCursor;
	ref array<string> m_aErrors = {};
	ref array<string> m_aQueue = {};
	protected ref array<string> m_aDiscovered = {};
	protected ref array<string> m_aEquipment = {};
	protected ref array<string> m_aVehicles = {};
	protected ref array<string> m_aEquipmentIds = {};
	protected ref array<string> m_aVehicleIds = {};
	protected ref array<string> m_aEntries = {};
	protected ref map<string, string> m_mIdentities = new map<string, string>();
	protected ref TBD_SourceReaderVerification m_Verification;
	protected ref TBD_SourceTypeHierarchy m_Types = new TBD_SourceTypeHierarchy();
	protected string m_sStarted;
	protected string m_sEnvironment;

	bool Start(string scope = "complete", array<string> resources = null)
	{
		m_sScope = scope;
		m_sGenerationId = Workbench.GenerateGloballyUniqueID64();
		m_sDirectory = DESTINATION + m_sGenerationId;
		m_sStarted = TBD_SourceExportEnvironment.Timestamp();
		m_sEnvironment = TBD_SourceExportEnvironment.Capture();
		if (!FileIO.MakeDirectory(m_sDirectory)) { m_aErrors.Insert("Cannot create generation directory"); m_bFinished = true; return false; }
		m_Verification = new TBD_SourceReaderVerification();
		if (!m_Verification.Run())
		{
			foreach (string failure : m_Verification.m_aErrors) m_aErrors.Insert(failure);
			Finish(); return false;
		}
		if (scope == "complete")
		{
			TBD_SourceResourceDiscovery discovery = new TBD_SourceResourceDiscovery();
			discovery.Scan();
			m_aEquipment = discovery.m_aEquipment;
			m_aVehicles = discovery.m_aVehicles;
			foreach (string error : discovery.m_aErrors) m_aErrors.Insert(error);
			foreach (string equipment : m_aEquipment) AddDiscovered(equipment);
			foreach (string vehicle : m_aVehicles) AddDiscovered(vehicle);
		}
		else if (resources)
			foreach (string diagnostic : resources) AddDiscovered(diagnostic);
		m_aDiscovered.Sort();
		foreach (string name : m_aDiscovered) Enqueue(name);
		if (m_aQueue.IsEmpty()) { m_aErrors.Insert("No resources were discovered"); Finish(); return false; }
		WriteProgress();
		return true;
	}

	protected void AddDiscovered(string resource)
	{
		if (m_aDiscovered.Find(resource) < 0) m_aDiscovered.Insert(resource);
	}

	protected void Enqueue(string name)
	{
		if (name.IsEmpty()) return;
		string id = TBD_SourceCapabilityBuilder.ResourceIdentity(name);
		if (m_mIdentities.Contains(id))
		{
			if (m_mIdentities.Get(id) != name) m_aErrors.Insert("One native identity has different exact resource names: " + name);
			return;
		}
		if (m_aQueue.Count() >= 100000) { m_aErrors.Insert("Resource dependency traversal limit reached"); return; }
		m_mIdentities.Insert(id, name);
		m_aQueue.Insert(name);
	}

	bool Step()
	{
		if (m_bFinished) return false;
		if (m_iCursor >= m_aQueue.Count()) { Finish(); return false; }
		string name = m_aQueue[m_iCursor];
		CaptureResource(name);
		m_iCursor++;
		if (m_iCursor % 25 == 0) WriteProgress();
		if (m_iCursor >= m_aQueue.Count()) { Finish(); return false; }
		return true;
	}

	protected void CaptureResource(string name)
	{
		Resource resource = Resource.Load(name);
		if (!resource || !resource.IsValid() || !resource.GetResource()) { m_aErrors.Insert("Cannot load gameplay resource: " + name); return; }
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root) { m_aErrors.Insert("Gameplay resource lacks a readable source container: " + name); return; }
		TBD_SourceContainerReader reader = new TBD_SourceContainerReader();
		reader.m_sResource = name;
		reader.Capture(root);
		foreach (string typeName : reader.m_Types.m_aTypes) m_Types.Add(typeName);
		foreach (string error : reader.m_aErrors) m_aErrors.Insert(name + ": " + error);
		if (reader.m_aNodes.IsEmpty()) return;
		TBD_SourceCapabilityBuilder builder = new TBD_SourceCapabilityBuilder();
		string previousLocale;
		WidgetManager.GetLanguage(previousLocale);
		if (previousLocale != "en_us") WidgetManager.SetLanguage("en_us");
		string record = builder.Build(reader);
		if (previousLocale != "en_us") WidgetManager.SetLanguage(previousLocale);
		foreach (string buildError : builder.m_aErrors) m_aErrors.Insert(name + ": " + buildError);
		string identity = TBD_SourceCapabilityBuilder.ResourceIdentity(name);
		string path = ResourceFile(reader);
		string recordFile = "records/" + path;
		string sourceFile = "sources/" + path;
		if (!WriteDocument(recordFile, record) || !WriteDocument(sourceFile, reader.Snapshot(identity))) return;
		array<string> domains = {};
		if (m_aEquipment.Find(name) >= 0) { domains.Insert("equipment"); m_aEquipmentIds.Insert(identity); }
		if (m_aVehicles.Find(name) >= 0) { domains.Insert("vehicle"); m_aVehicleIds.Insert(identity); }
		if (domains.IsEmpty()) domains.Insert("dependency");
		string entry = "{\"resource_id\":" + TBD_SourceExportJson.Quote(identity);
		entry += ",\"resource_name\":" + TBD_SourceExportJson.Quote(name);
		entry += ",\"record_file\":" + TBD_SourceExportJson.Quote(recordFile);
		entry += ",\"source_file\":" + TBD_SourceExportJson.Quote(sourceFile);
		entry += ",\"domains\":" + TBD_SourceExportJson.Strings(domains) + "}";
		m_aEntries.Insert(entry);
		foreach (TBD_SourceExportReference link : reader.m_aReferences)
			if (link.m_sKind == "gameplay") Enqueue(link.m_sResource);
	}

	protected string ResourceFile(TBD_SourceContainerReader reader)
	{
		string addon = "unidentified_addon";
		if (!reader.m_aNodes[0].m_aAddons.IsEmpty()) addon = TBD_SourceCapabilityRules.Field(reader.m_aNodes[0].m_aAddons[0]);
		string file = TBD_SourceCapabilityBuilder.Guid(reader.m_sResource);
		if (file.IsEmpty()) file = "resource_" + m_iCursor.ToString();
		return addon + "/" + file.Substring(0, 2) + "/" + file + ".json";
	}

	protected bool WriteDocument(string relativePath, string content)
	{
		string destination = m_sDirectory + "/" + relativePath;
		string parent = destination.Substring(0, destination.LastIndexOf("/"));
		if (!FileIO.MakeDirectory(parent) || !TBD_SourceExportJson.Write(destination, content))
		{ m_aErrors.Insert("Generation file write failed: " + relativePath); return false; }
		return true;
	}

	protected void WriteProgress()
	{
		string json = "{\"generation_id\":" + TBD_SourceExportJson.Quote(m_sGenerationId);
		json += ",\"completed_resources\":" + m_iCursor.ToString() + ",\"queued_resources\":" + m_aQueue.Count().ToString();
		json += ",\"error_count\":" + m_aErrors.Count().ToString() + "}";
		// Progress sits outside the immutable generation's validated file set.
		TBD_SourceExportJson.Write("$profile:TBD_Export/equipment_vehicle_exports/progress.json", json);
		Print("[TBD Source Export] " + json);
	}

	void Finish()
	{
		m_bFinished = true;
		m_aEquipmentIds.Sort();
		m_aVehicleIds.Sort();
		string hierarchy = m_Types.Json();
		foreach (string hierarchyError : m_Types.m_aErrors) m_aErrors.Insert(hierarchyError);
		string status = "completed";
		if (!m_aErrors.IsEmpty()) status = "failed";
		string json = "{\"document_type\":\"export_generation\",\"schema_version\":2,\"generation_id\":" + TBD_SourceExportJson.Quote(m_sGenerationId);
		json += ",\"scope\":" + TBD_SourceExportJson.Quote(m_sScope) + ",\"status\":" + TBD_SourceExportJson.Quote(status);
		json += ",\"started_at\":" + TBD_SourceExportJson.Quote(m_sStarted) + ",\"finished_at\":" + TBD_SourceExportJson.Quote(TBD_SourceExportEnvironment.Timestamp());
		json += ",\"environment\":" + m_sEnvironment + ",\"resources\":[" + TBD_SourceExportJson.Join(m_aEntries) + "]";
		json += ",\"equipment_ids\":" + TBD_SourceExportJson.Strings(m_aEquipmentIds);
		json += ",\"vehicle_ids\":" + TBD_SourceExportJson.Strings(m_aVehicleIds);
		json += ",\"discovered_resources\":" + TBD_SourceExportJson.Strings(m_aDiscovered);
		json += ",\"errors\":" + TBD_SourceExportJson.Strings(m_aErrors);
		json += ",\"reader_verification\":" + m_Verification.Json();
		json += ",\"type_hierarchy\":" + hierarchy + "}";
		WriteDocument("generation.json", json);
		WriteProgress();
	}
}
