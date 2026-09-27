//! Runs the standard scanners into an isolated verification destination.
class TBD_StandardExportVerificationRequest : JsonApiStruct
{
	string section;
	string run_id;
	string resource_name;
	string category;
	void TBD_StandardExportVerificationRequest()
	{
		RegV("section");
		RegV("run_id");
		RegV("resource_name");
		RegV("category");
	}
}

class TBD_StandardExportVerificationResponse : JsonApiStruct
{
	string destination;
	string status;
	void TBD_StandardExportVerificationResponse()
	{
		RegV("destination");
		RegV("status");
	}
}

class TBD_StandardExportVerification : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new TBD_StandardExportVerificationRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		TBD_StandardExportVerificationRequest input = TBD_StandardExportVerificationRequest.Cast(request);
		TBD_StandardExportVerificationResponse result = new TBD_StandardExportVerificationResponse();
		if (input.run_id.IsEmpty() || input.run_id.Contains("/") || input.run_id.Contains("\\") || input.run_id.Contains("..") || input.run_id.Contains(":"))
		{
			result.status = "invalid_run_id";
			return result;
		}
		if (input.section == "serialization")
		{
			JsonSaveContainer writer = new JsonSaveContainer();
			ContainerSerializationSaveContext context = new ContainerSerializationSaveContext(false);
			context.SetContainer(writer);
			context.WriteValue("v", 1.25);
			result.destination = writer.SaveToString();
			result.status = TBD_EquipmentExportJson.Pretty("{\"zero\":" + TBD_EquipmentExportJson.Number(0) + ",\"fraction\":" + TBD_EquipmentExportJson.Number(1.25) + "}");
			return result;
		}
		if (input.section == "fire_mode_probe")
		{
			result.destination = TBD_WeaponMuzzleExtractor.VerifyInitializedFireModes(input.resource_name);
			result.status = "finished";
			return result;
		}
		if (input.section == "door_probe")
		{
			result.destination = TBD_VehicleCompartmentExtractor.VerifyInitializedDoors(input.resource_name);
			result.status = "finished";
			return result;
		}
		if (input.section == "source_properties")
		{
			Resource inspected = Resource.Load(input.resource_name);
			if (!inspected || !inspected.IsValid()) { result.status = "load_failed"; return result; }
			BaseContainer source = inspected.GetResource().ToBaseContainer();
			if (!source) { result.status = "no_source"; return result; }
			array<string> entries = {};
			array<BaseContainer> visited = {};
			InspectSourceProperties(source, entries, visited, 0);
			result.destination = "[" + TBD_EquipmentExportJson.Join(entries) + "]";
			result.status = "finished";
			return result;
		}

		string destination = "$profile:TBD_Export/standard_verification/" + input.run_id + "/";
		TBD_EquipmentExportPaths.EnsureDestinationDir(destination);
		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		config.m_sDestinationDir = destination + "equipment/";
		config.m_bIncludeAbstract = true;
		TBD_EquipmentExportJson.BeginRun();
		if (input.section == "weapon_sample")
		{
			TBD_WeaponScanner sample = new TBD_WeaponScanner(config);
			sample.RunResource(input.resource_name, input.category);
		}
		else if (input.section == "weapons")
		{
			TBD_WeaponScanner weapons = new TBD_WeaponScanner(config);
			weapons.RunScan();
		}
		else if (input.section == "equipment")
		{
			TBD_EquipmentExportPlugin.ExportTo(config.m_sDestinationDir);
		}
		else if (input.section == "vehicle_sample")
		{
			TBD_VehicleDeepExtractor vehicleSample = new TBD_VehicleDeepExtractor();
			vehicleSample.ScanResource(input.resource_name);
			string samplePath = destination + "vehicles_all.json";
			TBD_EquipmentExportJson.BeginCatalog(samplePath);
			string vehicles = TBD_VehicleDeepSerializer.SerializeAllToJson(vehicleSample.m_aPlatforms, vehicleSample.m_aAllVariants.Count());
			FileHandle sampleFile = FileIO.OpenFile(samplePath, FileMode.WRITE);
			if (sampleFile) { TBD_EquipmentExportJson.Write(sampleFile, vehicles, "[TBD][Verification]"); sampleFile.Close(); }
			else TBD_EquipmentExportJson.ExtractionError(input.resource_name, samplePath, "Cannot open verification file");
			FileHandle sampleMeta = FileIO.OpenFile(destination + "vehicles_meta.json", FileMode.WRITE);
			if (sampleMeta) { TBD_EquipmentExportJson.Write(sampleMeta, TBD_EquipmentExportJson.AddMetadata("{}"), "[TBD][Verification]"); sampleMeta.Close(); }
		}
		else if (input.section == "vehicles")
		{
			TBD_VehicleDeepExportPlugin.ExportTo(destination + "vehicles/");
		}
		else
		{
			result.status = "invalid_section";
			return result;
		}
		TBD_EquipmentExportJson.CompleteRun(destination);
		result.destination = destination;
		result.status = "finished";
		return result;
	}

	//! Verification-only native property inventory distinguishes absent fields from unimplemented catalog bindings.
	protected static void InspectSourceProperties(BaseContainer source, array<string> entries, array<BaseContainer> visited, int depth)
	{
		if (!source || visited.Find(source) >= 0 || depth > 128) return;
		visited.Insert(source);
		array<string> properties = {};
		for (int i = 0; i < source.GetNumVars(); i++)
		{
			string name = source.GetVarName(i);
			string nativeType = typename.EnumToString(DataVarType, source.GetDataVarType(i));
			properties.Insert(TBD_EquipmentExportJson.Quote(name) + ":" + TBD_EquipmentExportJson.Quote(nativeType));
			if (nativeType == "OBJECT") InspectSourceProperties(source.GetObject(name), entries, visited, depth + 1);
			if (nativeType == "OBJECT_ARRAY")
			{
				BaseContainerList children = source.GetObjectArray(name);
				if (children) for (int j = 0; j < children.Count(); j++) InspectSourceProperties(children.Get(j), entries, visited, depth + 1);
			}
		}
		string entry = TBD_EquipmentExportJson.Context(source);
		entry = TBD_EquipmentExportJson.Member(entry, "properties", "{" + TBD_EquipmentExportJson.Join(properties) + "}");
		entries.Insert(entry);
		for (int c = 0; c < source.GetNumChildren(); c++) InspectSourceProperties(source.GetChild(c), entries, visited, depth + 1);
	}
}
