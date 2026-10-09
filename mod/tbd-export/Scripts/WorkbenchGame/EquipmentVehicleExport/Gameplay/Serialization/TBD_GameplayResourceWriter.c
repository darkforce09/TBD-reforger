class TBD_GameplayResourceWriter
{
	ref TBD_GameplayFieldDefinitions m_Definitions;
	ref TBD_GameplaySelectionPolicy m_Policy;
	protected ref map<string, ref array<string>> m_mSections = new map<string, ref array<string>>();

	string Build(TBD_SourceContainerReader reader)
	{
		array<string> nodes = {};
		foreach (TBD_SourceExportNode node : reader.m_aNodes) nodes.Insert(NodeJson(node));
		array<string> sectionNames = {};
		foreach (string section, array<string> nodeIds : m_mSections) sectionNames.Insert(section);
		sectionNames.Sort();
		array<string> sections = {};
		foreach (string name : sectionNames)
			sections.Insert(TBD_SourceExportJson.Quote(name) + ":" + TBD_SourceExportJson.Strings(m_mSections.Get(name)));
		array<string> references = {};
		foreach (TBD_SourceExportReference link : reader.m_aReferences) references.Insert(link.Json());
		TBD_SourceCapabilityBuilder names = new TBD_SourceCapabilityBuilder();
		string previousLocale;
		WidgetManager.GetLanguage(previousLocale);
		if (previousLocale != "en_us") WidgetManager.SetLanguage("en_us");
		string translated = names.CompactNames(reader);
		if (previousLocale != "en_us") WidgetManager.SetLanguage(previousLocale);
		string guid = TBD_SourceCapabilityBuilder.Guid(reader.m_sResource);
		string identityKind = "resource_name";
		if (!guid.IsEmpty()) identityKind = "resource_guid";
		string json = "{\"document_type\":\"gameplay_resource\",\"schema_version\":1,\"resource_id\":";
		json += TBD_SourceExportJson.Quote(TBD_SourceCapabilityBuilder.ResourceIdentity(reader.m_sResource));
		json += ",\"resource_name\":" + TBD_SourceExportJson.Quote(reader.m_sResource);
		json += ",\"resource_guid\":" + TBD_SourceExportJson.Nullable(guid);
		json += ",\"identity_kind\":" + TBD_SourceExportJson.Quote(identityKind);
		json += ",\"source_addons\":" + TBD_SourceExportJson.Strings(reader.m_aNodes[0].m_aAddons);
		json += ",\"root_node\":\"root\",\"nodes\":[" + TBD_SourceExportJson.Join(nodes) + "]";
		json += ",\"capabilities\":{" + TBD_SourceExportJson.Join(sections) + "}";
		json += ",\"names\":" + translated + ",\"references\":[" + TBD_SourceExportJson.Join(references) + "]";
		json += ",\"parent_prefab\":" + TBD_SourceExportJson.Nullable(reader.m_aNodes[0].m_sAncestorResource) + "}";
		return json;
	}

	protected string NodeJson(TBD_SourceExportNode node)
	{
		array<string> properties = {};
		foreach (string property, TBD_SourceExportFact fact : node.m_mProperties) properties.Insert(property);
		properties.Sort();
		array<string> fields = {};
		foreach (string name : properties)
		{
			TBD_SourceExportFact value = node.m_mProperties.Get(name);
			TBD_GameplaySelectionRule rule = m_Policy.Rule(node.m_sClass, name, value.m_sNativeType);
			if (!rule) continue;
			string definitionId = m_Definitions.RegisterField(node.m_sClass, name, value, rule);
			string encoded = "{\"definition_id\":" + TBD_SourceExportJson.Quote(definitionId);
			encoded += ",\"status\":" + TBD_SourceExportJson.Quote(value.m_sStatus);
			encoded += ",\"value\":" + value.m_sValue;
			encoded += ",\"origin\":" + TBD_SourceExportJson.Quote(value.m_sOrigin);
			encoded += ",\"reason\":" + TBD_SourceExportJson.Nullable(value.m_sReason) + "}";
			fields.Insert(TBD_SourceExportJson.Quote(name) + ":" + encoded);
			m_Definitions.m_iFacts++;
			AddSection(rule.m_sSection, node.m_sId);
		}
		m_Definitions.m_iNodes++;
		string json = "{\"node_id\":" + TBD_SourceExportJson.Quote(node.m_sId);
		json += ",\"class_name\":" + TBD_SourceExportJson.Quote(node.m_sClass);
		json += ",\"instance_name\":" + TBD_SourceExportJson.Quote(node.m_sName);
		json += ",\"native_instance_id\":" + TBD_SourceExportJson.Nullable(node.m_sNativeInstanceId);
		string identity = "exporter_structural_path";
		if (!node.m_sNativeInstanceId.IsEmpty()) identity = "native_container_id";
		json += ",\"instance_identity_kind\":" + TBD_SourceExportJson.Quote(identity);
		json += ",\"resource_name\":" + TBD_SourceExportJson.Quote(node.m_sResource);
		json += ",\"source_addons\":" + TBD_SourceExportJson.Strings(node.m_aAddons);
		json += ",\"view\":\"effective\",\"ancestor_id\":null,\"ancestor_resource_name\":" + TBD_SourceExportJson.Nullable(node.m_sAncestorResource);
		json += ",\"selection\":" + TBD_SourceExportJson.Quote(node.m_sSelection);
		json += ",\"children\":" + TBD_SourceExportJson.Strings(node.m_aChildren);
		json += ",\"declared_properties\":" + TBD_SourceExportJson.Strings(node.m_aDeclared);
		json += ",\"properties\":{" + TBD_SourceExportJson.Join(fields) + "}}";
		return json;
	}

	protected void AddSection(string section, string nodeId)
	{
		array<string> nodes = m_mSections.Get(section);
		if (!nodes) { nodes = {}; m_mSections.Insert(section, nodes); }
		if (nodes.Find(nodeId) < 0) nodes.Insert(nodeId);
	}
}
