// Effective arrays come from the engine once. Ancestors are evidence, never appended installations.
class TBD_SourceContainerReader
{
	static const int MAX_NODES = 100000;
	static const int MAX_DEPTH = 128;
	string m_sResource;
	ref array<ref TBD_SourceExportNode> m_aNodes = {};
	ref array<ref TBD_SourceExportReference> m_aReferences = {};
	ref array<string> m_aErrors = {};
	ref TBD_SourceTypeHierarchy m_Types = new TBD_SourceTypeHierarchy();
	protected ref array<BaseContainer> m_aActive = {};
	protected ref array<BaseContainer> m_aSeen = {};
	protected ref array<string> m_aSeenIds = {};
	protected ref array<BaseContainer> m_aEffectiveSeen = {};
	protected ref array<string> m_aEffectiveIds = {};
	protected ref array<BaseContainer> m_aAncestorSeen = {};
	protected ref array<string> m_aAncestorIds = {};

	BaseContainer Container(string nodeId)
	{
		int index = m_aSeenIds.Find(nodeId);
		if (index < 0) return null;
		return m_aSeen[index];
	}

	string NodeId(BaseContainer container, string view)
	{
		int index;
		if (view == "effective")
		{
			index = m_aEffectiveSeen.Find(container);
			if (index >= 0) return m_aEffectiveIds[index];
		}
		else
		{
			index = m_aAncestorSeen.Find(container);
			if (index >= 0) return m_aAncestorIds[index];
		}
		return string.Empty;
	}

	string Capture(BaseContainer container, string id = "root", string view = "effective", int depth = 0)
	{
		if (!container) return string.Empty;
		if (m_aActive.Find(container) >= 0)
		{
			m_aErrors.Insert(id + ": container cycle");
			return string.Empty;
		}
		// A shared inherited container can also occur in an effective installation.
		string seenId = NodeId(container, view);
		if (!seenId.IsEmpty()) return seenId;
		if (depth > MAX_DEPTH || m_aNodes.Count() >= MAX_NODES)
		{
			m_aErrors.Insert(id + ": source traversal limit reached");
			return string.Empty;
		}
		m_aSeen.Insert(container);
		m_aSeenIds.Insert(id);
		if (view == "effective") { m_aEffectiveSeen.Insert(container); m_aEffectiveIds.Insert(id); }
		else { m_aAncestorSeen.Insert(container); m_aAncestorIds.Insert(id); }
		m_aActive.Insert(container);
		TBD_SourceExportNode node = new TBD_SourceExportNode();
		node.m_sId = id;
		node.m_sView = view;
		node.m_sClass = container.GetClassName();
		m_Types.Add(node.m_sClass);
		node.m_sName = container.GetName();
		node.m_sResource = container.GetResourceName();
		if (node.m_sResource.Length() == 18 && node.m_sResource.StartsWith("{") && node.m_sResource.EndsWith("}"))
			node.m_sNativeInstanceId = node.m_sResource;
		else if (node.m_sResource != m_sResource && IsGameplayResource(node.m_sResource))
		{
			TBD_SourceExportReference sourceLink = new TBD_SourceExportReference();
			sourceLink.m_sNode = id;
			sourceLink.m_sProperty = "resource_name";
			sourceLink.m_sMethod = "BaseContainer.GetResourceName";
			sourceLink.m_sResource = node.m_sResource;
			sourceLink.m_sKind = "gameplay";
			m_aReferences.Insert(sourceLink);
		}
		container.GetSourceAddons(node.m_aAddons);
		m_aNodes.Insert(node);

		array<string> properties = {};
		for (int i = 0; i < container.GetNumVars(); i++) properties.Insert(container.GetVarName(i));
		properties.Sort();
		foreach (string property : properties) CaptureProperty(container, node, property, depth);
		for (int child = 0; child < container.GetNumChildren(); child++)
		{
			string childId = Capture(container.GetChild(child), id + "/children/" + child.ToString(), view, depth + 1);
			if (!childId.IsEmpty() && node.m_aChildren.Find(childId) < 0) node.m_aChildren.Insert(childId);
		}
		BaseContainer ancestor = container.GetAncestor();
		if (ancestor) node.m_sAncestor = Capture(ancestor, id + "/ancestor", "ancestor", depth + 1);
		m_aActive.Remove(m_aActive.Count() - 1);
		return id;
	}

	protected void CaptureProperty(BaseContainer container, TBD_SourceExportNode node, string property, int depth)
	{
		int index = container.GetVarIndex(property);
		DataVarType nativeType = container.GetDataVarType(index);
		TBD_SourceExportFact fact = new TBD_SourceExportFact();
		fact.m_sNativeType = typename.EnumToString(DataVarType, nativeType);
		if (nativeType == DataVarType.OBJECT || nativeType == DataVarType.OBJECT_ARRAY)
		{
			fact.m_sObjectBaseClass = container.GetObjectBaseClass(index);
			m_Types.Add(fact.m_sObjectBaseClass);
		}
		if (node.m_sClass == "ItemPhysicalAttributes")
		{
			string getter;
			if (property == "Weight") { fact.m_sNativeUnit = "kg"; getter = "GetWeight"; }
			if (property == "ItemVolume") { fact.m_sNativeUnit = "cm3"; getter = "GetVolume"; }
			if (property == "ItemDimensions") { fact.m_sNativeUnit = "cm"; getter = "GetDimensions"; }
			if (!getter.IsEmpty()) fact.m_sUnitEvidence = "ArmaReforgerScriptAPIPublic/ItemPhysicalAttributes." + getter + " native attribute documentation";
		}
		fact.m_sResource = m_sResource;
		fact.m_sNode = node.m_sId;
		fact.m_sProperty = property;
		fact.m_sOrigin = "engine_default";
		if (container.IsVariableSet(property)) fact.m_sOrigin = "inherited";
		if (container.IsVariableSetDirectly(property))
		{
			fact.m_sOrigin = "declared";
			node.m_aDeclared.Insert(property);
		}
		node.m_mProperties.Insert(property, fact);
		string path = node.m_sId + "/properties/" + TBD_SourceExportJson.PointerPart(property);
		bool ok = true;
		if (nativeType == DataVarType.OBJECT)
		{
			fact.m_sMethod = "BaseContainer.GetObject";
			BaseContainer object = container.GetObject(property);
			if (object)
			{
				string objectId = Capture(object, path, node.m_sView, depth + 1);
				fact.m_sValue = "{\"node_id\":" + TBD_SourceExportJson.Quote(objectId) + "}";
				ok = !objectId.IsEmpty();
			}
		}
		else if (nativeType == DataVarType.OBJECT_ARRAY)
		{
			fact.m_sMethod = "BaseContainer.GetObjectArray";
			BaseContainerList objects = container.GetObjectArray(property);
			array<string> ids = {};
			if (objects)
			{
				for (int item = 0; item < objects.Count(); item++)
				{
					BaseContainer element = objects.Get(item);
					if (!element) { ids.Insert("null"); continue; }
					string itemId = Capture(element, path + "/" + item.ToString(), node.m_sView, depth + 1);
					ids.Insert("{\"node_id\":" + TBD_SourceExportJson.Quote(itemId) + "}");
					if (itemId.IsEmpty()) ok = false;
				}
			}
			fact.m_sValue = "[" + TBD_SourceExportJson.Join(ids) + "]";
		}
		else ok = TBD_SourcePropertyReader.Read(container, property, nativeType, fact.m_sValue);

		if (!ok)
		{
			fact.m_sStatus = "error";
			fact.m_sValue = "null";
			fact.m_sReason = "Native property read failed or type is not supported by the reader";
			m_aErrors.Insert(path + ": " + fact.m_sNativeType + " read failed");
		}
		CaptureEnums(container, index, fact);
		CaptureReferences(container, property, nativeType, node.m_sId);
	}

	protected void CaptureEnums(BaseContainer container, int index, TBD_SourceExportFact fact)
	{
		array<string> names = {};
		array<int> values = {};
		container.GetEnumValues(index, names, values);
		array<string> entries = {};
		if (names.Count() != values.Count())
		{
			m_aErrors.Insert(fact.m_sNode + ": invalid native enum metadata");
			return;
		}
		foreach (int i, string name : names)
			entries.Insert("{\"name\":" + TBD_SourceExportJson.Quote(name) + ",\"value\":" + values[i].ToString() + "}");
		fact.m_sEnumValues = "[" + TBD_SourceExportJson.Join(entries) + "]";
	}

	protected void CaptureReferences(BaseContainer container, string property, DataVarType nativeType, string nodeId)
	{
		array<string> resources = {};
		if (nativeType == DataVarType.RESOURCE_NAME)
		{
			string resource;
			if (container.Get(property, resource)) resources.Insert(resource);
		}
		else if (nativeType == DataVarType.RESOURCE_NAME_ARRAY) container.Get(property, resources);
		foreach (string name : resources)
		{
			if (name.IsEmpty()) continue;
			TBD_SourceExportReference link = new TBD_SourceExportReference();
			link.m_sNode = nodeId;
			link.m_sProperty = property;
			link.m_sResource = name;
			link.m_sKind = "binary";
			if (IsGameplayResource(name)) link.m_sKind = "gameplay";
			m_aReferences.Insert(link);
		}
	}

	static bool IsGameplayResource(string name)
	{
		return name.EndsWith(".et") || name.EndsWith(".conf") || name.EndsWith(".gamemat") || name.EndsWith(".ragdoll");
	}

	string Snapshot(string resourceId)
	{
		array<string> nodes = {};
		foreach (TBD_SourceExportNode node : m_aNodes) nodes.Insert(node.Json());
		string json = "{\"document_type\":\"source_snapshot\",\"schema_version\":2,\"resource_id\":" + TBD_SourceExportJson.Quote(resourceId);
			json += ",\"resource_name\":" + TBD_SourceExportJson.Quote(m_sResource) + ",\"root_node\":\"root\",\"nodes\":[" + TBD_SourceExportJson.Join(nodes) + "]}";
		return json;
	}
}
