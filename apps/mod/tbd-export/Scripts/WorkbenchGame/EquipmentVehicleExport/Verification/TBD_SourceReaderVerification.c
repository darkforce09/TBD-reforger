// Isolated fixtures exercise serialization; installed resources exercise native inheritance.
[BaseContainerProps()]
class TBD_SourceReaderFixture
{
	[Attribute("17")] int Number;
	[Attribute("1")] bool Toggle;
	[Attribute("authored")] string Text;
	[Attribute()] ref array<int> Values;
}

class TBD_SourceReaderVerification
{
	ref array<string> m_aChecks = {};
	ref array<string> m_aErrors = {};

	bool Run()
	{
		VerifyLargeJson();
		VerifyExplicitEmpty();
		VerifyRemovedComponents();
		VerifyResource("{00C9BBE426F7D459}Prefabs/Vehicles/Wheeled/M998/M997_maxi_ambulance.et");
		VerifyResource("{5A987A8A13763769}Prefabs/Weapons/Rifles/M16/Rifle_M16A2_M203.et");
		VerifyResource("{95D4766BBE46F23D}Prefabs/Items/Equipment/Backpacks/Backpack_IIFS_FieldPack.et");
		return m_aErrors.IsEmpty();
	}

	bool RunResources(array<string> resources)
	{
		foreach (string resource : resources) VerifyResource(resource, false);
		return m_aErrors.IsEmpty();
	}

	protected void VerifyRemovedComponents()
	{
		string name = "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et";
		Resource original = Resource.Load(name);
		if (!original || !original.GetResource()) { m_aErrors.Insert("Cannot load component override fixture ancestor"); return; }
		BaseContainer source = original.GetResource().ToBaseContainer();
		Resource isolated = BaseContainerTools.CreateContainer(source.GetClassName());
		if (!isolated || !isolated.GetResource()) { m_aErrors.Insert("Cannot create isolated component override fixture"); return; }
		BaseContainer child = isolated.GetResource().ToBaseContainer();
		child.SetAncestor(name);
		BaseContainerList components = child.SetObjectArray("components");
		if (!components) { m_aErrors.Insert("Cannot override isolated components array"); return; }
		while (components.Count() > 0)
			if (!components.Remove(components.Get(0))) { m_aErrors.Insert("Cannot remove isolated component"); return; }
		TBD_SourceContainerReader reader = new TBD_SourceContainerReader();
		reader.m_sResource = "verification:removed_components";
		reader.Capture(child);
		foreach (string failure : reader.m_aErrors) m_aErrors.Insert(failure);
		CheckValue(reader, "components", "[]");
		if (!child.GetAncestor() || source.GetObjectArray("components").Count() == 0)
			m_aErrors.Insert("Component removal fixture lacks inherited components");
		else m_aChecks.Insert("Isolated child removes inherited components without changing the installed prefab");
	}

	protected void VerifyLargeJson()
	{
		array<string> expected = {};
		string item = "Native string with \"quotes\", \\ slash, newline\n and Unicode: Žluťoučký";
		for (int i = 0; i < 2000; i++) expected.Insert(item + i.ToString());
		string longText;
		for (int repeat = 0; repeat < 200; repeat++) longText += item;
		expected.Insert(longText);
		string json = "{\"values\":" + TBD_SourceExportJson.Strings(expected) + "}";
		JsonLoadContext context = new JsonLoadContext(false);
		array<string> actual = {};
		if (!context.LoadFromString(json) || !context.ReadValue("values", actual) || actual.Count() != expected.Count())
		{ m_aErrors.Insert("Large native string array serialization loses data"); return; }
		for (int n = 0; n < expected.Count(); n++)
			if (expected[n] != actual[n]) { m_aErrors.Insert("Escaped or Unicode string changed in serialization"); return; }
		m_aChecks.Insert("Large string arrays preserve all 2001 values, long text, escapes and Unicode");
	}

	protected void VerifyExplicitEmpty()
	{
		Resource resource = BaseContainerTools.CreateContainer("TBD_SourceReaderFixture");
		if (!resource || !resource.GetResource()) { m_aErrors.Insert("Cannot create isolated reader fixture"); return; }
		BaseContainer root = resource.GetResource().ToBaseContainer();
		array<int> empty = {};
		if (!root || !root.Set("Number", 0) || !root.Set("Toggle", false) || !root.Set("Text", "") || !root.Set("Values", empty))
		{ m_aErrors.Insert("Cannot set fixture properties"); return; }
		TBD_SourceContainerReader reader = new TBD_SourceContainerReader();
		reader.m_sResource = "verification:explicit_empty";
		reader.Capture(root);
		foreach (string error : reader.m_aErrors) m_aErrors.Insert(error);
		CheckValue(reader, "Number", "0");
		CheckValue(reader, "Toggle", "false");
		CheckValue(reader, "Text", TBD_SourceExportJson.Quote(""));
		CheckValue(reader, "Values", "[]");
		TBD_SourceContainerReader limited = new TBD_SourceContainerReader();
		limited.Capture(root, "root", "effective", TBD_SourceContainerReader.MAX_DEPTH + 1);
		if (limited.m_aErrors.IsEmpty() || !limited.m_aNodes.IsEmpty()) m_aErrors.Insert("Traversal limit does not fail explicitly");
		else m_aChecks.Insert("Traversal limit reports an extraction failure");
	}

	protected void CheckValue(TBD_SourceContainerReader reader, string property, string expected)
	{
		if (reader.m_aNodes.IsEmpty()) { m_aErrors.Insert("Empty fixture snapshot"); return; }
		TBD_SourceExportFact fact = reader.m_aNodes[0].m_mProperties.Get(property);
		if (!fact || fact.m_sStatus != "present" || fact.m_sValue != expected || fact.m_sOrigin != "declared")
			m_aErrors.Insert("Explicit fixture value not preserved: " + property);
		else m_aChecks.Insert("Explicit fixture value preserved: " + property);
	}

	protected void VerifyResource(string name, bool requireOverrides = true)
	{
		Resource resource = Resource.Load(name);
		if (!resource || !resource.IsValid() || !resource.GetResource()) { m_aErrors.Insert("Verification resource cannot load: " + name); return; }
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root) { m_aErrors.Insert("Verification resource lacks container: " + name); return; }
		TBD_SourceContainerReader reader = new TBD_SourceContainerReader();
		reader.m_sResource = name;
		reader.Capture(root);
		foreach (string error : reader.m_aErrors) m_aErrors.Insert(name + ": " + error);
		int overrides;
		int properties;
		int objectProperties;
		foreach (TBD_SourceExportNode node : reader.m_aNodes)
		{
			BaseContainer container = reader.Container(node.m_sId);
			if (!container || container.GetNumVars() != node.m_mProperties.Count())
			{ m_aErrors.Insert(name + ": incomplete property enumeration"); continue; }
			foreach (string property, TBD_SourceExportFact fact : node.m_mProperties)
			{
				DataVarType kind = container.GetDataVarType(container.GetVarIndex(property));
				if (kind == DataVarType.OBJECT || kind == DataVarType.OBJECT_ARRAY)
				{
					VerifyObjects(reader, container, node, property, kind, fact);
					objectProperties++;
					continue;
				}
				string direct;
				if (!TBD_SourcePropertyReader.Read(container, property, kind, direct) || fact.m_sValue != direct)
					m_aErrors.Insert(name + ": native value mismatch at " + node.m_sId + "/" + property);
				properties++;
				BaseContainer ancestor = container.GetAncestor();
				string parentValue;
				if (node.m_sView == "effective" && ancestor && ancestor.GetVarIndex(property) >= 0 && fact.m_sOrigin == "declared")
					if (TBD_SourcePropertyReader.Read(ancestor, property, kind, parentValue) && parentValue != direct) overrides++;
			}
		}
		if (properties == 0 || requireOverrides && overrides == 0) m_aErrors.Insert(name + ": representative override verification has no coverage");
		else
		{
			string check = name + ": " + properties.ToString() + " native properties checked; " + overrides.ToString() + " child overrides preserved";
			m_aChecks.Insert(check + "; " + objectProperties.ToString() + " ordered object relationships checked");
		}
	}

	protected void VerifyObjects(TBD_SourceContainerReader reader, BaseContainer container, TBD_SourceExportNode node, string property, DataVarType kind, TBD_SourceExportFact fact)
	{
		string expected = "null";
		if (kind == DataVarType.OBJECT)
		{
			BaseContainer object = container.GetObject(property);
			if (object) expected = "{\"node_id\":" + TBD_SourceExportJson.Quote(reader.NodeId(object, node.m_sView)) + "}";
		}
		else
		{
			array<string> entries = {};
			BaseContainerList objects = container.GetObjectArray(property);
			if (objects)
				for (int i = 0; i < objects.Count(); i++)
				{
					if (!objects.Get(i)) entries.Insert("null");
					else entries.Insert("{\"node_id\":" + TBD_SourceExportJson.Quote(reader.NodeId(objects.Get(i), node.m_sView)) + "}");
				}
			expected = "[" + TBD_SourceExportJson.Join(entries) + "]";
		}
		if (fact.m_sValue != expected) m_aErrors.Insert(reader.m_sResource + ": native object relationship mismatch at " + node.m_sId + "/" + property);
	}

	string Json()
	{
		string status = "passed";
		if (!m_aErrors.IsEmpty()) status = "failed";
		return "{\"status\":" + TBD_SourceExportJson.Quote(status) + ",\"checks\":" + TBD_SourceExportJson.Strings(m_aChecks) + "}";
	}
}
