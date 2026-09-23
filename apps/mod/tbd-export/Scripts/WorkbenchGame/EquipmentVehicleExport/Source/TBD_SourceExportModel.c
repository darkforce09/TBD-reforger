// Source values retain native property names; organized facts reference these nodes.
class TBD_SourceExportFact
{
	string m_sStatus = "present";
	string m_sValue = "null";
	string m_sNativeType;
	string m_sOrigin = "unknown";
	string m_sMethod = "BaseContainer.Get";
	string m_sReason;
	string m_sResource;
	string m_sNode;
	string m_sProperty;
	string m_sEnumValues = "[]";
	string m_sObjectBaseClass;
	string m_sNativeUnit;
	string m_sUnitEvidence;

	string Json()
	{
		string json = "{\"status\":" + TBD_SourceExportJson.Quote(m_sStatus);
			json += ",\"value\":" + m_sValue + ",\"native_type\":" + TBD_SourceExportJson.Nullable(m_sNativeType);
			json += ",\"native_unit\":" + TBD_SourceExportJson.Nullable(m_sNativeUnit);
			json += ",\"unit_evidence\":" + TBD_SourceExportJson.Nullable(m_sUnitEvidence);
			json += ",\"object_base_class\":" + TBD_SourceExportJson.Nullable(m_sObjectBaseClass);
			json += ",\"origin\":" + TBD_SourceExportJson.Quote(m_sOrigin);
			json += ",\"source\":{\"resource_name\":" + TBD_SourceExportJson.Quote(m_sResource);
			json += ",\"node_id\":" + TBD_SourceExportJson.Quote(m_sNode);
			json += ",\"property\":" + TBD_SourceExportJson.Quote(m_sProperty);
			json += ",\"method\":" + TBD_SourceExportJson.Quote(m_sMethod) + "}";
			json += ",\"reason\":" + TBD_SourceExportJson.Nullable(m_sReason) + ",\"enum_values\":" + m_sEnumValues + "}";
		return json;
	}
}

class TBD_SourceExportNode
{
	string m_sId;
	string m_sView;
	string m_sClass;
	string m_sName;
	string m_sResource;
	string m_sAncestor;
	string m_sNativeInstanceId;
	ref array<string> m_aAddons = {};
	ref array<string> m_aChildren = {};
	ref array<string> m_aDeclared = {};
	ref map<string, ref TBD_SourceExportFact> m_mProperties = new map<string, ref TBD_SourceExportFact>();

	string Json()
	{
		array<string> keys = {};
		foreach (string key, TBD_SourceExportFact fact : m_mProperties) keys.Insert(key);
		keys.Sort();
		array<string> fields = {};
		foreach (string name : keys) fields.Insert(TBD_SourceExportJson.Quote(name) + ":" + m_mProperties.Get(name).Json());
		string json = "{\"node_id\":" + TBD_SourceExportJson.Quote(m_sId);
			json += ",\"view\":" + TBD_SourceExportJson.Quote(m_sView);
			json += ",\"class_name\":" + TBD_SourceExportJson.Quote(m_sClass);
			json += ",\"instance_name\":" + TBD_SourceExportJson.Quote(m_sName);
			json += ",\"native_instance_id\":" + TBD_SourceExportJson.Nullable(m_sNativeInstanceId);
			string identityKind = "exporter_structural_path";
			if (!m_sNativeInstanceId.IsEmpty()) identityKind = "native_container_id";
			json += ",\"instance_identity_kind\":" + TBD_SourceExportJson.Quote(identityKind);
			json += ",\"resource_name\":" + TBD_SourceExportJson.Quote(m_sResource);
			json += ",\"source_addons\":" + TBD_SourceExportJson.Strings(m_aAddons);
			json += ",\"ancestor_id\":" + TBD_SourceExportJson.Nullable(m_sAncestor);
			json += ",\"children\":" + TBD_SourceExportJson.Strings(m_aChildren);
			json += ",\"declared_properties\":" + TBD_SourceExportJson.Strings(m_aDeclared);
			json += ",\"properties\":{" + TBD_SourceExportJson.Join(fields) + "}}";
		return json;
	}
}

class TBD_SourceExportReference
{
	string m_sNode;
	string m_sProperty;
	string m_sResource;
	string m_sKind;
	string m_sMethod = "BaseContainer.Get";

	string Json()
	{
		string json = "{\"node_id\":" + TBD_SourceExportJson.Quote(m_sNode);
			json += ",\"property\":" + TBD_SourceExportJson.Quote(m_sProperty);
			json += ",\"resource_name\":" + TBD_SourceExportJson.Quote(m_sResource);
			json += ",\"kind\":" + TBD_SourceExportJson.Quote(m_sKind);
			json += ",\"method\":" + TBD_SourceExportJson.Quote(m_sMethod) + "}";
		return json;
	}
}
