// NET API entry point for source-reader verification without changing an editor world.
class TBD_SourceExportRequest : JsonApiStruct
{
	string resource;
	string action;
	ref array<string> resources = {};
	void TBD_SourceExportRequest() { RegV("resource"); RegV("action"); RegV("resources"); }
}

class TBD_SourceExportResponse : JsonApiStruct
{
	string status;
	string snapshot;
	int error_count;
	ref array<string> errors = {};
	void TBD_SourceExportResponse()
	{
		RegV("status");
		RegV("snapshot");
		RegV("error_count");
		RegV("errors");
	}
}

class EMCP_WB_SourceExport : NetApiHandler
{
	override JsonApiStruct GetRequest() { return new TBD_SourceExportRequest(); }

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		TBD_SourceExportRequest req = TBD_SourceExportRequest.Cast(request);
		TBD_SourceExportResponse response = new TBD_SourceExportResponse();
		response.status = "error";
		if (req.action == "start" || req.action == "step" || req.action == "status" || req.action == "diagnostic")
		{
			if (req.action == "start" || req.action == "diagnostic")
			{
				if (TBD_SourceExportGeneration.s_Active && !TBD_SourceExportGeneration.s_Active.m_bFinished)
				{ response.errors.Insert("An export is already running"); return response; }
				TBD_SourceExportGeneration.s_Active = new TBD_SourceExportGeneration();
				string scope = "complete";
				if (req.action == "diagnostic") scope = "diagnostic";
				TBD_SourceExportGeneration.s_Active.Start(scope, req.resources);
			}
			TBD_SourceExportGeneration generation = TBD_SourceExportGeneration.s_Active;
			if (!generation) { response.errors.Insert("No generation is active"); return response; }
			if (req.action == "step") generation.Step();
			response.status = "running";
			if (generation.m_bFinished) response.status = "completed";
			response.snapshot = generation.m_sDirectory;
			response.error_count = generation.m_aErrors.Count();
			for (int e = 0; e < generation.m_aErrors.Count() && e < 20; e++) response.errors.Insert(generation.m_aErrors[e]);
			return response;
		}
		if (req.action == "read_script")
		{
			FileHandle input = FileIO.OpenFile(req.resource, FileMode.READ);
			if (!input) { response.errors.Insert("Script source cannot be opened"); return response; }
			string contents;
			string line;
			while (input.ReadLine(line) >= 0) contents += line + "\n";
			input.Close();
			response.snapshot = "$profile:TBD_Export/source_reader_probes/native_script.c";
			if (TBD_SourceExportJson.Write(response.snapshot, contents)) response.status = "ok";
			return response;
		}
		if (req.action == "discovery")
		{
			TBD_SourceResourceDiscovery discovery = new TBD_SourceResourceDiscovery();
			discovery.Inspect(req.resources);
			response.errors = discovery.m_aErrors;
			response.error_count = response.errors.Count();
			response.snapshot = "$profile:TBD_Export/source_reader_probes/discovery.json";
			string census = "{\"equipment\":" + TBD_SourceExportJson.Strings(discovery.m_aEquipment);
			census += ",\"vehicles\":" + TBD_SourceExportJson.Strings(discovery.m_aVehicles) + "}";
			if (TBD_SourceExportJson.Write(response.snapshot, census) && response.error_count == 0) response.status = "ok";
			return response;
		}
		if (req.action == "verify" || req.action == "verify_resources")
		{
			TBD_SourceReaderVerification verification = new TBD_SourceReaderVerification();
			bool passed;
			if (req.action == "verify") passed = verification.Run();
			else passed = verification.RunResources(req.resources);
			if (passed) response.status = "ok";
			response.errors = verification.m_aErrors;
			response.error_count = response.errors.Count();
			FileIO.MakeDirectory("$profile:TBD_Export/source_reader_probes");
			response.snapshot = "$profile:TBD_Export/source_reader_probes/verification.json";
			TBD_SourceExportJson.Write(response.snapshot, verification.Json());
			return response;
		}
		Resource resource = Resource.Load(req.resource);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			response.errors.Insert("Cannot load requested resource");
			return response;
		}
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root)
		{
			response.errors.Insert("Requested resource is not a source container");
			return response;
		}
		TBD_SourceContainerReader reader = new TBD_SourceContainerReader();
		reader.m_sResource = req.resource;
		reader.Capture(root);
		FileIO.MakeDirectory("$profile:TBD_Export/source_reader_probes");
		response.snapshot = "$profile:TBD_Export/source_reader_probes/latest.json";
		string content = reader.Snapshot(req.resource);
		if (req.action == "hierarchy")
		{
			response.snapshot = "$profile:TBD_Export/source_reader_probes/hierarchy.json";
			content = reader.m_Types.Json();
			foreach (string hierarchyError : reader.m_Types.m_aErrors) reader.m_aErrors.Insert(hierarchyError);
		}
		if (!TBD_SourceExportJson.Write(response.snapshot, content)) reader.m_aErrors.Insert("Snapshot file write failed");
		response.error_count = reader.m_aErrors.Count();
		for (int i = 0; i < reader.m_aErrors.Count() && i < 20; i++) response.errors.Insert(reader.m_aErrors[i]);
		if (response.error_count == 0) response.status = "ok";
		return response;
	}
}
