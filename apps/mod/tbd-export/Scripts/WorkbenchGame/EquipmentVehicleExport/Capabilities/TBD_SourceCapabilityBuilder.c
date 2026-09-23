class TBD_SourceCapabilityBuilder
{
	protected ref map<string, ref array<string>> m_mCapabilities = new map<string, ref array<string>>();
	protected ref map<string, string> m_mNodeCapabilities = new map<string, string>();
	protected ref array<string> m_aNames = {};
	ref array<string> m_aErrors = {};

	string Build(TBD_SourceContainerReader reader)
	{
		foreach (TBD_SourceExportNode node : reader.m_aNodes)
		{
			if (node.m_sView != "effective") continue;
			string capability = TBD_SourceCapabilityRules.Capability(node.m_sClass);
			if (capability.IsEmpty()) capability = ParentCapability(node.m_sId);
			if (!capability.IsEmpty())
			{
				m_mNodeCapabilities.Set(node.m_sId, capability);
				AddCapability(capability, node);
			}
			if (TBD_SourceCapabilityRules.IsA(node.m_sClass, "UIInfo")) AddName(node, reader.Container(node.m_sId));
		}
		array<string> keys = {};
		foreach (string key, array<string> instances : m_mCapabilities) keys.Insert(key);
		keys.Sort();
		array<string> capabilities = {};
		foreach (string category : keys)
			capabilities.Insert(TBD_SourceExportJson.Quote(category) + ":[" + TBD_SourceExportJson.Join(m_mCapabilities.Get(category)) + "]");
		array<string> links = {};
		foreach (TBD_SourceExportReference link : reader.m_aReferences) links.Insert(link.Json());
		string guid = Guid(reader.m_sResource);
		string identityKind = "resource_name";
		if (!guid.IsEmpty()) identityKind = "resource_guid";
		string json = "{\"document_type\":\"resource_record\",\"schema_version\":2,\"resource_id\":" + TBD_SourceExportJson.Quote(TBD_SourceCapabilityBuilder.ResourceIdentity(reader.m_sResource));
		json += ",\"resource_name\":" + TBD_SourceExportJson.Quote(reader.m_sResource);
		json += ",\"resource_guid\":" + TBD_SourceExportJson.Nullable(guid);
		json += ",\"identity_kind\":" + TBD_SourceExportJson.Quote(identityKind);
		json += ",\"source_addons\":" + TBD_SourceExportJson.Strings(reader.m_aNodes[0].m_aAddons);
		json += ",\"capabilities\":{" + TBD_SourceExportJson.Join(capabilities) + "}";
		json += ",\"names\":[" + TBD_SourceExportJson.Join(m_aNames) + "]";
		json += ",\"references\":[" + TBD_SourceExportJson.Join(links) + "]";
		reader.m_Types.m_aTypes.Sort();
		json += ",\"type_names\":" + TBD_SourceExportJson.Strings(reader.m_Types.m_aTypes);
		return json + "}";
	}

	protected string ParentCapability(string id)
	{
		string parent = id;
		while (parent.Contains("/"))
		{
			parent = parent.Substring(0, parent.LastIndexOf("/"));
			if (m_mNodeCapabilities.Contains(parent)) return m_mNodeCapabilities.Get(parent);
		}
		return string.Empty;
	}

	protected void AddCapability(string capability, TBD_SourceExportNode node)
	{
		array<string> instances = m_mCapabilities.Get(capability);
		if (!instances) { instances = {}; m_mCapabilities.Insert(capability, instances); }
		array<string> properties = {};
		foreach (string key, TBD_SourceExportFact value : node.m_mProperties) properties.Insert(key);
		properties.Sort();
		array<string> names = {};
		array<string> facts = {};
		foreach (string property : properties)
		{
			string name = TBD_SourceCapabilityRules.Field(property);
			if (names.Find(name) >= 0) { m_aErrors.Insert(node.m_sId + ": normalized field collision: " + name); continue; }
			names.Insert(name);
			facts.Insert(TBD_SourceExportJson.Quote(name) + ":" + node.m_mProperties.Get(property).Json());
		}
		string json = "{\"node_id\":" + TBD_SourceExportJson.Quote(node.m_sId);
		instances.Insert(json + ",\"facts\":{" + TBD_SourceExportJson.Join(facts) + "}}");
	}

	protected void AddName(TBD_SourceExportNode node, BaseContainer container)
	{
		TBD_SourceExportFact original = node.m_mProperties.Get("Name");
		if (!original) return;
		TBD_SourceExportFact english = new TBD_SourceExportFact();
		english.m_sResource = original.m_sResource;
		english.m_sNode = node.m_sId;
		english.m_sProperty = "Name";
		english.m_sMethod = "WidgetManager.Translate";
		english.m_sNativeType = "STRING";
		english.m_sOrigin = "native_getter";
		string sourceText;
		string locale;
		WidgetManager.GetLanguage(locale);
		if (!container.Get("Name", sourceText))
		{
			english.m_sStatus = "error";
			english.m_sReason = "Native name property could not be read";
		}
		else
		{
			string translated = WidgetManager.Translate(sourceText);
			if (locale != "en_us" || translated.IsEmpty() || translated.StartsWith("#") || sourceText.StartsWith("#") && translated == sourceText)
			{
				english.m_sStatus = "unavailable";
				english.m_sReason = "No resolved English name is provided by WidgetManager.Translate";
			}
			else english.m_sValue = TBD_SourceExportJson.Quote(translated);
		}
		string json = "{\"node_id\":" + TBD_SourceExportJson.Quote(node.m_sId);
		json += ",\"source_text\":" + original.Json() + ",\"display_name_en\":" + english.Json();
		m_aNames.Insert(json + ",\"locale\":\"en_us\"}");
	}

	static string Guid(string name)
	{
		if (name.Length() > 18 && name.StartsWith("{") && name.Substring(17, 1) == "}") return name.Substring(1, 16);
		return string.Empty;
	}

	static string ResourceIdentity(string name)
	{
		string guid = Guid(name);
		if (!guid.IsEmpty()) return "guid:" + guid;
		return "resource:" + name;
	}
}
