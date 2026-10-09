/**
 * Preserves existing native ballistic and wind tables referenced by standard ammunition.
 * Each configuration is written once; no trajectory simulation or table generation occurs.
 */
//! Owns run-local table collection and the standard ammunition table catalog.
class TBD_AmmoBallisticTableExporter
{
	protected static ref array<string> m_TableResources = {}; //!< Exact resources in catalog order.
	protected static ref array<string> m_TableRecords = {}; //!< Source-backed records parallel to resources.
	protected static ref array<string> m_SeenTables = {}; //!< Native identities already attempted this run.
	protected static ref array<string> m_SeenSources = {}; //!< Completed projectile or effect resources.
	protected static ref array<string> m_ActiveSources = {}; //!< Active gameplay references for cycle checks.
	protected static ref array<BaseContainer> m_ActiveContainers = {}; //!< Active nested effect containers.

	//! Clear all caches at the start of an ammunition scan.
	static void Reset()
	{
		m_TableResources.Clear();
		m_TableRecords.Clear();
		m_SeenTables.Clear();
		m_SeenSources.Clear();
		m_ActiveSources.Clear();
		m_ActiveContainers.Clear();
	}

	//! Collect table dependencies from an already-resolved projectile and its gameplay effects.
	static void Collect(string resourceName, map<string, ref array<BaseContainer>> components)
	{
		CollectComponents(resourceName, components, 0);
	}

	//! Resolve native identity without deriving it from a filename or category.
	protected static string ResourceIdentity(string resourceName)
	{
		if (resourceName.Length() >= 18 && resourceName.StartsWith("{")) return resourceName.Substring(0, 18);
		return resourceName;
	}

	//! Follow actual motion components and trigger effects while bounding dependency traversal.
	protected static void CollectComponents(string resourceName, map<string, ref array<BaseContainer>> components, int depth)
	{
		string identity = ResourceIdentity(resourceName);
		if (depth > 64 || m_ActiveSources.Find(identity) >= 0)
		{
			TBD_EquipmentExportJson.ExtractionError(resourceName, "ballistic_tables", "Gameplay reference cycle or traversal limit");
			return;
		}
		if (m_SeenSources.Find(identity) >= 0) return;
		m_ActiveSources.Insert(identity);
		foreach (string cls, array<BaseContainer> bucket : components)
		{
			foreach (BaseContainer component : bucket)
			{
				if (TBD_EquipmentComponentGraph.IsA(cls, "ProjectileMoveComponent"))
				{
					CollectTableProperty(component, "BallisticTableConfig");
					CollectTableProperty(component, "ProjectileWindTableConfig");
					CollectEffects(component, "ProjectileEffects", depth);
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "BaseTriggerComponent"))
					CollectEffects(component, "PROJECTILE_EFFECTS", depth);
			}
		}
		m_ActiveSources.Remove(m_ActiveSources.Count() - 1);
		m_SeenSources.Insert(identity);
	}

	//! Read a configured table reference without falling back over an explicit empty value.
	protected static void CollectTableProperty(BaseContainer component, string property)
	{
		if (component.GetVarIndex(property) < 0) return;
		string name;
		if (!component.Get(property, name))
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, property, "Cannot read configured table reference");
			return;
		}
		if (!name.IsEmpty()) CollectTable(name);
	}

	//! Visit only damage, explosion and submunition effect relationships relevant to table closure.
	protected static void CollectEffects(BaseContainer source, string property, int depth)
	{
		if (source.GetVarIndex(property) < 0) return;
		BaseContainerList effects = source.GetObjectArray(property);
		if (!effects)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, property, "Cannot read configured effect list");
			return;
		}
		for (int i = 0; i < effects.Count(); i++)
		{
			BaseContainer effect = effects.Get(i);
			if (!effect) continue;
			if (depth > 64 || m_ActiveContainers.Find(effect) >= 0)
			{
				TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, property, "Effect cycle or traversal limit");
				continue;
			}
			m_ActiveContainers.Insert(effect);
			CollectEffects(effect, "DamageEffects", depth + 1);
			CollectEffects(effect, "DamageEffect", depth + 1);
			CollectEffects(effect, "ExplosionEffects", depth + 1);
			if (TBD_EquipmentComponentGraph.IsA(effect.GetClassName(), "ExplosionEffect"))
				CollectEffectResource(effect, "EffectPrefab", depth + 1);
			if (TBD_EquipmentComponentGraph.IsA(effect.GetClassName(), "SubmunitionEffect"))
				CollectEffectResource(effect, "Prefab", depth + 1);
			m_ActiveContainers.Remove(m_ActiveContainers.Count() - 1);
		}
	}

	//! Inspect the referenced gameplay prefab with the existing standard component collector.
	protected static void CollectEffectResource(BaseContainer effect, string property, int depth)
	{
		if (effect.GetVarIndex(property) < 0) return;
		string name;
		if (!effect.Get(property, name))
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, property, "Cannot read effect resource reference");
			return;
		}
		if (name.IsEmpty() || m_SeenSources.Find(ResourceIdentity(name)) >= 0) return;
		Resource resource = Resource.Load(name);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			TBD_EquipmentExportJson.ExtractionError(name, property, "Cannot load required gameplay reference");
			return;
		}
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root)
		{
			TBD_EquipmentExportJson.ExtractionError(name, property, "Gameplay reference has no configuration container");
			return;
		}
		TBD_EquipmentComponentContext context = TBD_EquipmentComponentGraph.SaveContext();
		map<string, ref array<BaseContainer>> components = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, components, 128);
		CollectComponents(name, components, depth);
		TBD_EquipmentComponentGraph.RestoreContext(context);
	}

	//! Preserve the selected native table fields once for every referenced configuration identity.
	protected static void CollectTable(string resourceName)
	{
		string identity = ResourceIdentity(resourceName);
		if (m_SeenTables.Find(identity) >= 0) return;
		m_SeenTables.Insert(identity);
		Resource resource = Resource.Load(resourceName);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			TBD_EquipmentExportJson.ExtractionError(resourceName, "ballistic_tables", "Cannot load required table configuration");
			return;
		}
		BaseContainer source = resource.GetResource().ToBaseContainer();
		if (!source)
		{
			TBD_EquipmentExportJson.ExtractionError(resourceName, "ballistic_tables", "Table resource has no configuration container");
			return;
		}
		string owner = TBD_EquipmentComponentGraph.m_CurrentResource;
		string prefix = TBD_EquipmentExportJson.m_sFieldPrefix;
		TBD_EquipmentComponentGraph.m_CurrentResource = resourceName;
		TBD_EquipmentExportJson.m_sFieldPrefix = string.Empty;
		string json = "{\"resource_name\":" + TBD_EquipmentExportJson.Quote(resourceName);
		json += ",\"resource_guid\":" + TBD_EquipmentResourceNames.GuidJson(resourceName);
		json += ",\"native_class\":" + TBD_EquipmentExportJson.Quote(source.GetClassName());
		if (TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "BallisticTableArray"))
		{
			json += ",\"kind\":\"ballistic\"";
			json += ",\"direct_fire\":" + ReadEntries(source, "Table data", "/direct_fire", false);
			json += ",\"indirect_fire\":" + ReadEntries(source, "Indirect fire Table data", "/indirect_fire", false);
		}
		else if (TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SCR_ProjectileWindTable"))
		{
			json += ",\"kind\":\"wind\"";
			json += ",\"entries\":" + ReadEntries(source, "m_aData", "/entries", true);
		}
		else
		{
			TBD_EquipmentExportJson.ExtractionError(resourceName, "ballistic_tables", "Unsupported table class " + source.GetClassName());
			json += ",\"status\":\"error\"";
		}
		m_TableResources.Insert(resourceName);
		m_TableRecords.Insert(json + "}");
		TBD_EquipmentComponentGraph.m_CurrentResource = owner;
		TBD_EquipmentExportJson.m_sFieldPrefix = prefix;
	}

	//! Keep native entry order, empty arrays, null entries and unconverted vector rows.
	protected static string ReadEntries(BaseContainer source, string property, string path, bool wind)
	{
		int variable = source.GetVarIndex(property);
		if (variable < 0)
		{
			TBD_EquipmentExportJson.RecordNativeField(source, path, "not_present", "BaseContainer.GetObjectArray", property, string.Empty, string.Empty);
			return "null";
		}
		string nativeType = typename.EnumToString(DataVarType, source.GetDataVarType(variable));
		BaseContainerList entries = source.GetObjectArray(property);
		if (!entries)
		{
			TBD_EquipmentExportJson.RecordNativeField(source, path, "error", "BaseContainer.GetObjectArray", property, string.Empty, nativeType);
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Cannot read " + property);
			return "null";
		}
		TBD_EquipmentExportJson.RecordNativeField(source, path, "present", "BaseContainer.GetObjectArray", property, string.Empty, nativeType);
		array<string> records = {};
		for (int i = 0; i < entries.Count(); i++)
		{
			BaseContainer entry = entries.Get(i);
			if (!entry) { records.Insert("null"); continue; }
			string entryPath = path + "/" + i.ToString();
			string expected = "BallisticTable";
			if (wind) expected = "SCR_ProjectileWindData";
			if (!TBD_EquipmentComponentGraph.IsA(entry.GetClassName(), expected))
				TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, entryPath, "Unexpected native table entry " + entry.GetClassName());
			string bindings = "initial_speed_coefficient=InitSpeedCoefficient|data=Table data";
			if (wind)
				bindings = "initial_speed_coefficient=m_fInitSpeedCoef|wind_speed=m_fWindSpeed|firing_solution=m_aFiringSolution|values=m_aValues";
			string json = TBD_EquipmentExportJson.Fields(entry, bindings, entryPath);
			records.Insert(TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(entry)));
		}
		return "[" + TBD_EquipmentExportJson.Join(records) + "]";
	}

	//! Write the shared source tables and matching sidecar through the standard checked writer.
	static void WriteCatalog(string destination)
	{
		string path = TBD_EquipmentExportPaths.BuildCategoryPath(destination, "ammunition", "ballistic_tables.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destination, "ammunition", "ballistic_tables_meta.json");
		TBD_EquipmentExportJson.BeginCatalog(path);
		FileHandle file = FileIO.OpenFile(path, FileMode.WRITE);
		if (!file)
		{
			TBD_EquipmentExportJson.ExtractionError(string.Empty, path, "Cannot open ballistic table catalog");
			return;
		}
		string envelope = TBD_EquipmentExportJson.Envelope(m_TableRecords.Count());
		string header = TBD_EquipmentExportJson.PreserveSubstring(envelope, 0, envelope.Length() - 1) + ",\"tables\":[\n";
		bool success = TBD_EquipmentExportJson.Write(file, header, "[TBD][BallisticTables]");
		for (int i = 0; i < m_TableRecords.Count() && success; i++)
		{
			TBD_EquipmentExportJson.IncludeResource(m_TableResources[i]);
			string record = TBD_EquipmentExportJson.Pretty(m_TableRecords[i], "    ");
			if (i + 1 < m_TableRecords.Count()) record += ",";
			success = TBD_EquipmentExportJson.Write(file, record + "\n", "[TBD][BallisticTables]");
		}
		if (success) TBD_EquipmentExportJson.Write(file, "]}\n", "[TBD][BallisticTables]");
		file.Close();
		TBD_EquipmentExportJson.RegisterFile(metaPath);
		FileHandle metadata = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (!metadata)
		{
			TBD_EquipmentExportJson.ExtractionError(string.Empty, metaPath, "Cannot open ballistic table metadata");
			return;
		}
		string meta = "{\"category\":\"ballistic_tables\",\"total_count\":" + m_TableRecords.Count().ToString();
		meta += ",\"generated_at\":" + TBD_EquipmentExportJson.Quote(TBD_EquipmentExportJson.m_sGeneratedAt) + "}";
		TBD_EquipmentExportJson.Write(metadata, TBD_EquipmentExportJson.AddMetadata(meta), "[TBD][BallisticTables]");
		metadata.Close();
	}
}
