/**
 * TBD_EquipmentExportJson.c
 *
 * JSON string escaping, checked file writes, and UTC timestamps. Every catalog
 * and every _meta.json sidecar the exporter writes is serialized through this
 * class, so escaping and the generation timestamp have exactly one definition.
 */

class TBD_EquipmentExportJson
{
	protected static ref array<string> m_Errors = {};
	protected static ref map<string, ref map<string, string>> m_FieldMetadata = new map<string, ref map<string, string>>();
	protected static ref array<string> m_CatalogResources = {};
	protected static ref map<string, ref map<string, string>> m_CatalogFieldMetadata = new map<string, ref map<string, string>>();
	protected static ref array<string> m_Files = {};
	static string m_sExportRunId;
	static string m_sGeneratedAt;
	static string m_sFieldPrefix;

	static void BeginRun()
	{
		m_Errors.Clear();
		m_sFieldPrefix = string.Empty;
		m_FieldMetadata.Clear();
		m_CatalogResources.Clear();
		m_Files.Clear();
		m_sGeneratedAt = IsoNowUtc();
		m_sExportRunId = m_sGeneratedAt + "-" + System.GetTickCount().ToString();
	}

	static void BeginCatalog(string filePath)
	{
		if (m_sExportRunId.IsEmpty()) BeginRun();
		m_CatalogResources.Clear();
		m_CatalogFieldMetadata.Clear();
		RegisterFile(filePath);
	}

	//! A specialist catalog owns its field paths; another domain's overlapping resources cannot pollute its sidecar.
	static void BeginDomainMetadata()
	{
		m_FieldMetadata.Clear();
		m_CatalogResources.Clear();
		m_sFieldPrefix = string.Empty;
	}

	static void RegisterFile(string filePath)
	{
		if (m_Files.Find(filePath) < 0) m_Files.Insert(filePath);
	}

	static void IncludeResource(string resource)
	{
		if (m_CatalogResources.Find(resource) < 0) m_CatalogResources.Insert(resource);
	}

	static string CatalogMetadata()
	{
		array<string> resources = {};
		foreach (string resource : m_CatalogResources)
		{
			map<string, string> evidence = m_CatalogFieldMetadata.Get(resource);
			if (!evidence) { resources.Insert(Quote(resource) + ":" + Metadata(resource)); continue; }
			array<string> fields = {};
			foreach (string path, string detail : evidence) fields.Insert(Quote(path) + ":" + detail);
			resources.Insert(Quote(resource) + ":{" + Join(fields) + "}");
		}
		return "{\n" + Join(resources, ",\n") + "\n}";
	}

	//! Transfer one extracted projection's evidence so a second capability cannot contaminate its fields.
	static map<string, string> TakeResourceMetadata(string resource)
	{
		ref map<string, string> fields = m_FieldMetadata.Get(resource);
		m_FieldMetadata.Remove(resource);
		return fields;
	}

	//! A catalog merges only the evidence of the projections actually serialized into that catalog.
	static void IncludeResourceMetadata(string resource, map<string, string> fields)
	{
		IncludeResource(resource);
		map<string, string> catalogFields = m_CatalogFieldMetadata.Get(resource);
		if (!catalogFields) { catalogFields = new map<string, string>(); m_CatalogFieldMetadata.Set(resource, catalogFields); }
		if (fields) foreach (string path, string detail : fields) catalogFields.Set(path, detail);
	}

	static string AddMetadata(string metadata)
	{
		metadata.TrimInPlace();
		metadata = Member(metadata, "version", "2");
		metadata = Member(metadata, "export_run_id", Quote(m_sExportRunId));
		metadata = Member(metadata, "fields", CatalogMetadata());
		return metadata + "\n";
	}

	static string Envelope(int count)
	{
		string json = "{\"version\":2";
		json += ",\"generated_at\":" + Quote(m_sGeneratedAt);
		json += ",\"export_run_id\":" + Quote(m_sExportRunId);
		return json + ",\"total_count\":" + count.ToString() + "}";
	}

	static void CompleteRun(string destination)
	{
		foreach (string path : m_Files)
		{
			JsonLoadContainer check = new JsonLoadContainer();
			if (!check.LoadFromFile(path))
				ExtractionError(string.Empty, path, "Written catalog is missing or invalid JSON");
		}
		string json = Envelope(m_Files.Count());
		json = Member(json, "files", Strings(m_Files));
		json = Member(json, "errors", Strings(m_Errors));
		array<string> addons = {};
		GameProject.GetLoadedAddons(addons);
		json = Member(json, "loaded_addons", Strings(addons));
		array<string> addonDetails = {};
		foreach (string addon : addons)
		{
			string details = "{\"guid\":" + Quote(addon) + ",\"id\":" + Quote(GameProject.GetAddonID(addon));
			details += ",\"title\":" + Quote(GameProject.GetAddonTitle(addon));
			details += ",\"version\":null,\"version_status\":\"unavailable\",\"version_explanation\":\"GameProject addon API does not expose versions\"}";
			addonDetails.Insert(details);
		}
		json = Member(json, "addon_details", "[" + Join(addonDetails) + "]");
		string build = "null";
		if (GetGame()) build = Quote(GetGame().GetBuildVersion());
		json = Member(json, "game_build", build);
		json = Member(json, "exporter_revision", "null");
		json = Member(json, "exporter_revision_status", Quote("unavailable"));
		json = Member(json, "completed_at", Quote(IsoNowUtc()));
		string status = "complete";
		if (!m_Errors.IsEmpty()) status = "error";
		json = Member(json, "status", Quote(status));
		FileHandle file = FileIO.OpenFile(destination + "export_run_meta.json", FileMode.WRITE);
		if (file) { Write(file, Pretty(json), "[TBD][Export]"); file.Close(); }
		else ExtractionError(string.Empty, destination, "Cannot write completion metadata");
	}

	//! Records an extraction failure without manufacturing a replacement value.
	static void ExtractionError(string resource, string location, string reason)
	{
		string message = resource + " " + location + ": " + reason;
		m_Errors.Insert(message);
		Print("[TBD][Export] " + message, LogLevel.ERROR);
	}

	static string Quote(string value)
	{
		return "\"" + Escape(value) + "\"";
	}

	static string Strings(array<string> values)
	{
		array<string> encoded = {};
		foreach (string value : values) encoded.Insert(Quote(value));
		return "[" + Join(encoded) + "]";
	}

	static string Context(BaseContainer source)
	{
		if (!source) return "null";
		string json = "{\"instance_id\":" + Quote(TBD_EquipmentComponentGraph.InstanceId(source));
		json += ",\"native_class\":" + Quote(source.GetClassName());
		json += ",\"native_name\":" + Quote(source.GetName());
		json += ",\"source_resource\":" + Quote(source.GetResourceName()) + "}";
		return json;
	}

	//! Adds members to a domain object without parsing or changing numeric text.
	static string Member(string object, string key, string value)
	{
		if (object.IsEmpty() || object == "null") object = "{}";
		string result = PreserveSubstring(object, 0, object.Length() - 1);
		if (object != "{}") result += ",";
		return result + Quote(key) + ":" + value + "}";
	}

	//! Enfusion Substring returns at most 8191 characters; join bounded slices to preserve long JSON values.
	static string PreserveSubstring(string value, int start, int length)
	{
		if (length <= 0) return string.Empty;
		array<string> chunks = {};
		for (int position = start; position < start + length; position += 4096)
		{
			int count = Math.Min(4096, start + length - position);
			chunks.Insert(value.Substring(position, count));
		}
		return string.Join(string.Empty, chunks, false);
	}

	//! Indents JSON tokens while preserving the original scalar representations.
	static string Pretty(string json, string indent = "")
	{
		array<string> chunks = {};
		string output = indent;
		string padding = indent;
		bool quoted;
		bool escaped;
		for (int start = 0; start < json.Length(); start += 4096)
		{
			string inputChunk = json.Substring(start, Math.Min(4096, json.Length() - start));
			for (int i = 0; i < inputChunk.Length(); i++)
			{
				if (output.Length() >= 4096) { chunks.Insert(output); output = string.Empty; }
				string character = inputChunk.Substring(i, 1);
				if (quoted)
				{
					output += character;
					if (escaped) escaped = false;
					else if (character == "\\") escaped = true;
					else if (character == "\"") quoted = false;
					continue;
				}
				if (character == "\"") { quoted = true; output += character; }
				else if (character == "{" || character == "[")
				{
					padding += "  ";
					output += character + "\n" + padding;
				}
				else if (character == "}" || character == "]")
				{
					if (padding.Length() < indent.Length() + 2)
					{
						ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, "serialization", "Unbalanced JSON value: " + json);
						return json;
					}
					if (padding.Length() == 2) padding = string.Empty;
					else padding = padding.Substring(0, padding.Length() - 2);
					output += "\n" + padding + character;
				}
				else if (character == ",") output += ",\n" + padding;
				else if (character == ":") output += ": ";
				else if (character != " " && character != "\n" && character != "\r" && character != "\t") output += character;
			}
		}
		chunks.Insert(output);
		return string.Join(string.Empty, chunks, false);
	}

	static string Join(array<string> values, string separator = ",")
	{
		return string.Join(separator, values, false);
	}

	//! Serializes the native float through the engine JSON writer, without decimal rounding.
	static string Number(float value)
	{
		JsonSaveContainer writer = new JsonSaveContainer();
		ContainerSerializationSaveContext context = new ContainerSerializationSaveContext(false);
		context.SetContainer(writer);
		context.WriteValue("v", value);
		string encoded = writer.SaveToString();
		return UnwrapNativeValue(encoded);
	}

	//! Native writers may append whitespace; unwrap the value by its enclosing object delimiters.
	static string UnwrapNativeValue(string encoded)
	{
		int colon = encoded.IndexOf(":");
		int closingBrace = encoded.LastIndexOf("}");
		if (colon < 0 || closingBrace <= colon + 1)
		{
			ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, "serialization", "Native JSON writer did not return a value envelope");
			return "null";
		}
		string value = PreserveSubstring(encoded, colon + 1, closingBrace - colon - 1);
		value.TrimInPlace();
		return value;
	}

	static string VectorJson(vector value)
	{
		return "[" + Number(value[0]) + "," + Number(value[1]) + "," + Number(value[2]) + "]";
	}

	//! Reads only an explicitly requested native property and records its source separately.
	static string Field(BaseContainer source, string property, string outputPath)
	{
		outputPath.Replace("[", "/");
		outputPath.Replace("]", "");
		outputPath.Replace(".", "/");
		if (!outputPath.StartsWith("/")) outputPath = "/" + outputPath;
		outputPath = m_sFieldPrefix + outputPath;
		string value = "null";
		string status = "not_present";
		string nativeType;
		string origin;
		if (source && source.GetVarIndex(property) >= 0)
		{
			nativeType = typename.EnumToString(DataVarType, source.GetDataVarType(source.GetVarIndex(property)));
			status = "present";
			origin = "engine_default";
			if (source.IsVariableSet(property)) origin = "inherited";
			if (source.IsVariableSetDirectly(property)) origin = "declared";
			if (!TBD_EquipmentNativeJson.Read(source, property, nativeType, value))
			{
				status = "error";
				ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, outputPath, "Cannot read " + property + " (" + nativeType + ")");
			}
		}
		string metadata = "{\"status\":" + Quote(status) + ",\"native_property\":" + Quote(property);
		metadata += ",\"native_type\":" + Quote(nativeType) + ",\"origin\":" + Quote(origin);
		metadata += ",\"method\":\"BaseContainer.Get\"," + UnitMetadata(source, property);
		if (source)
		{
			metadata += ",\"component_class\":" + Quote(source.GetClassName());
			metadata += ",\"container_name\":" + Quote(source.GetName());
			metadata += ",\"container_resource\":" + Quote(source.GetResourceName());
			metadata += ",\"instance_id\":" + Quote(TBD_EquipmentComponentGraph.InstanceId(source));
			if (source.GetVarIndex(property) >= 0)
			{
				array<string> names = {};
				array<int> numbers = {};
				source.GetEnumValues(source.GetVarIndex(property), names, numbers);
				array<string> enumEntries = {};
				for (int entry = 0; entry < names.Count() && entry < numbers.Count(); entry++)
					enumEntries.Insert("{\"name\":" + Quote(names[entry]) + ",\"value\":" + numbers[entry].ToString() + "}");
				if (!enumEntries.IsEmpty()) metadata += ",\"enum_values\":[" + Join(enumEntries) + "]";
			}
		}
		metadata += "}";
		map<string, string> resourceFields = m_FieldMetadata.Get(TBD_EquipmentComponentGraph.m_CurrentResource);
		if (!resourceFields)
		{
			resourceFields = new map<string, string>();
			m_FieldMetadata.Set(TBD_EquipmentComponentGraph.m_CurrentResource, resourceFields);
		}
		resourceFields.Set(outputPath, metadata);
		return value;
	}

	//! Attach units only where the installed native API explicitly documents their meaning.
	protected static string UnitMetadata(BaseContainer source, string property)
	{
		string unit;
		string evidence;
		if (source && TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "ItemPhysicalAttributes"))
		{
			if (property == "Weight") { unit = "kg"; evidence = "ItemPhysicalAttributes.GetWeight: returns weight in kg"; }
			if (property == "ItemVolume") { unit = "cm3"; evidence = "ItemPhysicalAttributes.GetVolume: returns volume in cm3"; }
			if (property == "ItemDimensions") { unit = "cm"; evidence = "ItemPhysicalAttributes.GetDimensions: returns dimensions in cm"; }
		}
		if (source && TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SCR_UniversalInventoryStorageComponent"))
		{
			if (property == "m_fMaxWeight") { unit = "kg"; evidence = "SCR_UniversalInventoryStorageComponent compares this limit with InventoryItemComponent.GetTotalWeight"; }
			if (property == "MaxCumulativeVolume") { unit = "cm3"; evidence = "SCR_UniversalInventoryStorageComponent compares capacity with ItemPhysicalAttributes.GetVolume"; }
			if (property == "MaxItemSize") { unit = "cm"; evidence = "Storage item-size limits compare against ItemPhysicalAttributes.GetDimensions"; }
		}
		if (source && TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SCR_FuelConsumptionComponent"))
		{
			if (property == "m_fFuelConsumption" || property == "m_fFuelConsumptionIdle")
			{
				unit = "l/h";
				evidence = "SCR_FuelConsumptionComponent attributes: fuel consumption in liters per hour";
			}
		}
		if (source && TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SCR_FuelNode"))
		{
			if (property == "MaxFuel" || property == "Fuel") { unit = "l"; evidence = "SCR_FuelConsumptionComponent subtracts liters consumed from fuel-node amount"; }
		}
		if (source && TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SCR_VehicleDamageManagerComponent"))
		{
			if (property == "m_fVehicleDamageSpeedThreshold" || property == "m_fVehicleSpeedDestroy" || property == "m_fOccupantsDamageSpeedThreshold" || property == "m_fOccupantsSpeedDeath")
			{ unit = "km/h"; evidence = "SCR_VehicleDamageManagerComponent speed-threshold attribute descriptions"; }
			if (property == "m_fSecondaryFireDamageDelay") { unit = "s"; evidence = "SCR_VehicleDamageManagerComponent fire-delay attribute description"; }
		}
		if (source && TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SCR_BaseSupportStationComponent") && property == "m_fRange")
		{ unit = "m"; evidence = "SCR_BaseSupportStationComponent.m_fRange attribute documents meters; -1 is the authored local-only sentinel"; }
		if (unit.IsEmpty()) return "\"unit\":null,\"unit_status\":\"unspecified\"";
		return "\"unit\":" + Quote(unit) + ",\"unit_status\":\"documented\",\"unit_evidence\":" + Quote(evidence);
	}

	//! Maps an explicit set of catalog fields to source properties, retaining null and empty values.
	static string Fields(BaseContainer source, string bindings, string outputPath)
	{
		array<string> pairs = {};
		bindings.Split("|", pairs, true);
		array<string> fields = {};
		foreach (string pair : pairs)
		{
			array<string> parts = {};
			pair.Split("=", parts, false);
			if (parts.Count() != 2) continue;
			fields.Insert(Quote(parts[0]) + ":" + Field(source, parts[1], outputPath + "/" + parts[0]));
		}
		return "{" + Join(fields) + "}";
	}

	//! Exports a typed, ordered object list through explicit domain-owned field bindings.
	static string ObjectArray(BaseContainer source, string property, string bindings, string outputPath)
	{
		if (!source || source.GetVarIndex(property) < 0) return "null";
		BaseContainerList objects = source.GetObjectArray(property);
		if (!objects) return "null";
		array<string> values = {};
		for (int i = 0; i < objects.Count(); i++)
		{
			BaseContainer object = objects.Get(i);
			if (!object) { values.Insert("null"); continue; }
			values.Insert(Fields(object, bindings, outputPath + "/" + i.ToString()));
		}
		return "[" + Join(values) + "]";
	}

	static string Metadata(string resource)
	{
		array<string> fields = {};
		map<string, string> resourceFields = m_FieldMetadata.Get(resource);
		if (!resourceFields) return "{}";
		foreach (string key, string value : resourceFields)
			fields.Insert(Quote(key) + ":" + value);
		return "{\n" + Join(fields, ",\n") + "\n}";
	}

	//! Records native getter evidence beside the ordinary scalar without adding per-value wrappers.
	static void RecordNativeField(BaseContainer source, string path, string status, string method, string explanation, string enumName, string nativeType = "ENUM")
	{
		string metadata = "{\"status\":" + Quote(status) + ",\"method\":" + Quote(method);
		metadata += ",\"explanation\":" + Quote(explanation) + ",\"native_type\":" + Quote(nativeType);
		metadata += ",\"unit\":null,\"unit_status\":\"unspecified\"";
		metadata += ",\"component_class\":" + Quote(source.GetClassName()) + ",\"container_resource\":" + Quote(source.GetResourceName());
		metadata += ",\"instance_id\":" + Quote(TBD_EquipmentComponentGraph.InstanceId(source));
		metadata += ",\"enum_name\":" + Quote(enumName) + "}";
		map<string, string> resourceFields = m_FieldMetadata.Get(TBD_EquipmentComponentGraph.m_CurrentResource);
		if (!resourceFields)
		{
			resourceFields = new map<string, string>();
			m_FieldMetadata.Set(TBD_EquipmentComponentGraph.m_CurrentResource, resourceFields);
		}
		resourceFields.Set(m_sFieldPrefix + path, metadata);
	}
	//------------------------------------------------------------------------------------------------
	//! Escape special characters for valid JSON strings.
	static string Escape(string s)
	{
		s.Replace("\\", "\\\\");
		s.Replace("\"", "\\\"");
		s.Replace("\n", "\\n");
		s.Replace("\r", "\\r");
		s.Replace("\t", "\\t");
		return s;
	}

	//------------------------------------------------------------------------------------------------
	//! Checked write helper that logs an error and aborts on failure.
	static bool Write(FileHandle f, string data, string logTag)
	{
		if (data.IsEmpty())
			return true;

		int wrote = f.Write(data);
		if (wrote <= 0)
		{
			ExtractionError(string.Empty, logTag, "FileHandle.Write failed; wrote=" + wrote.ToString());
			return false;
		}
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Helper for formatting current UTC time in ISO 8601 format (YYYY-MM-DDTHH:MM:SSZ).
	static string IsoNowUtc()
	{
		int y, mo, d, h, mi, s;
		System.GetYearMonthDayUTC(y, mo, d);
		System.GetHourMinuteSecondUTC(h, mi, s);
		return string.Format("%1-%2-%3T%4:%5:%6Z", y, Pad2(mo), Pad2(d), Pad2(h), Pad2(mi), Pad2(s));
	}

	//------------------------------------------------------------------------------------------------
	static string Pad2(int v)
	{
		if (v < 10)
			return "0" + v.ToString();
		return v.ToString();
	}
}
