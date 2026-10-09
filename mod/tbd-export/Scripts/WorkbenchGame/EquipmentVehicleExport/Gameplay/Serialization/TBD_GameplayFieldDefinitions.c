// Metadata is interned once; fact values remain in their owning resource instance.
class TBD_GameplayFieldDefinitions
{
	ref map<string, string> m_mDefinitionIds = new map<string, string>();
	ref array<string> m_aDefinitions = {};
	int m_iFacts;
	int m_iNodes;

	string RegisterField(string className, string property, TBD_SourceExportFact fact, TBD_GameplaySelectionRule rule)
	{
		string json = "{\"class_name\":" + TBD_SourceExportJson.Quote(className);
		json += ",\"property\":" + TBD_SourceExportJson.Quote(property);
		json += ",\"field_name\":" + TBD_SourceExportJson.Quote(TBD_SourceCapabilityRules.Field(property));
		json += ",\"section\":" + TBD_SourceExportJson.Quote(rule.m_sSection);
		json += ",\"disposition\":" + TBD_SourceExportJson.Quote(rule.m_sDisposition);
		string follow = "false";
		if (rule.m_bFollowReference) follow = "true";
		json += ",\"follow_reference\":" + follow;
		json += ",\"method\":" + TBD_SourceExportJson.Quote(fact.m_sMethod);
		json += ",\"native_type\":" + TBD_SourceExportJson.Quote(fact.m_sNativeType);
		json += ",\"native_unit\":" + TBD_SourceExportJson.Nullable(fact.m_sNativeUnit);
		json += ",\"unit_evidence\":" + TBD_SourceExportJson.Nullable(fact.m_sUnitEvidence);
		json += ",\"object_base_class\":" + TBD_SourceExportJson.Nullable(fact.m_sObjectBaseClass);
		json += ",\"enum_values\":" + fact.m_sEnumValues + "}";
		string id = m_mDefinitionIds.Get(json);
		if (!id.IsEmpty()) return id;
		id = "field_" + m_aDefinitions.Count().ToString();
		m_mDefinitionIds.Insert(json, id);
		m_aDefinitions.Insert(TBD_SourceExportJson.Quote(id) + ":" + json);
		return id;
	}

	string Json()
	{
		return "{\"schema_version\":1,\"fields\":{" + TBD_SourceExportJson.Join(m_aDefinitions) + "}}";
	}
}
