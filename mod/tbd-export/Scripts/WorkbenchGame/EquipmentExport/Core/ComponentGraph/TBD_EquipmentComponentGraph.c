/** Collects effective installed components and child entities without accumulating ancestors. */
class TBD_EquipmentComponentGraph
{
	protected static const int ANCESTOR_CAP = 16;

	//------------------------------------------------------------------------------------------------
	//! Collect the effective prefab configuration supplied by Workbench.
	static void CollectComponentChain(BaseContainer prefabRoot, notnull map<string, ref array<BaseContainer>> outComps, int componentDepthCap)
	{
		m_Active.Clear();
		m_Visited.Clear();
		m_Paths.Clear();
		m_CurrentResource = prefabRoot.GetResourceName();
		m_PrefabResource = m_CurrentResource;
		CollectEffective(prefabRoot, outComps, "root", 0);
	}

	//------------------------------------------------------------------------------------------------
	//! Collect one container's components, descending into nested component arrays.
	protected static void CollectComponentsRec(BaseContainer holder, notnull map<string, ref array<BaseContainer>> outComps, int depth, int componentDepthCap)
	{
		CollectEffective(holder, outComps, "root", depth);
	}

	//------------------------------------------------------------------------------------------------
	//! Test whether the collected set contains an instance of the native base type.
	static bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (IsA(cls, suffix))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Append a class name to an accumulator, skipping empties and repeats.
	static void AddUniqueType(notnull array<string> list, string item)
	{
		if (item.IsEmpty())
			return;

		if (list.Find(item) == -1)
			list.Insert(item);
	}

	protected static ref array<BaseContainer> m_Active = {};
	protected static ref array<BaseContainer> m_Visited = {};
	protected static ref array<string> m_Paths = {};
	static string m_CurrentResource;
	static string m_PrefabResource;

	static TBD_EquipmentComponentContext SaveContext()
	{
		TBD_EquipmentComponentContext context = new TBD_EquipmentComponentContext();
		context.m_sResource = m_CurrentResource;
		context.m_sPrefabResource = m_PrefabResource;
		context.m_Visited.Copy(m_Visited);
		context.m_Paths.Copy(m_Paths);
		return context;
	}

	static void RestoreContext(TBD_EquipmentComponentContext context)
	{
		m_CurrentResource = context.m_sResource;
		m_PrefabResource = context.m_sPrefabResource;
		m_Visited.Copy(context.m_Visited);
		m_Paths.Copy(context.m_Paths);
		m_Active.Clear();
	}

	//! Tests the engine's type relationship without interpreting class-name fragments.
	static bool IsA(string className, string baseName)
	{
		if (baseName == "CompartmentManagerComponent") baseName = "BaseCompartmentManagerComponent";
		if (baseName == "LoadoutClothComponent" || baseName == "ClothComponent") baseName = "BaseLoadoutClothComponent";
		if (baseName == "StorageComponent") baseName = "BaseInventoryStorageComponent";
		if (baseName == "RadioComponent") baseName = "BaseRadioComponent";
		if (baseName == "GadgetComponent") baseName = "SCR_GadgetComponent";
		if (className == baseName) return true;
		typename concrete = className.ToType();
		typename baseType = baseName.ToType();
		return concrete && baseType && concrete.IsInherited(baseType);
	}

	//! Returns native instance identity, or an explicitly structural location.
	static string InstanceId(BaseContainer container)
	{
		string nativeName = container.GetResourceName();
		if (nativeName.Length() == 18 && nativeName.StartsWith("{")) return nativeName;
		int index = m_Visited.Find(container);
		if (index >= 0) return m_Paths[index];
		return container.GetName();
	}

	static string StructuralPath(BaseContainer container)
	{
		int index = m_Visited.Find(container);
		if (index >= 0) return m_Paths[index];
		return string.Empty;
	}

	//! Resolve the owning entity from collector-ownerComponents paths, independently of repeated prefab classes.
	static BaseContainer OwningEntitySource(BaseContainer component)
	{
		string path = StructuralPath(component);
		string ownerPath = "root";
		int childIndex = path.IndexOf("/children/");
		while (childIndex >= 0)
		{
			int end = path.IndexOfFrom(childIndex + 10, "/");
			if (end < 0) { ownerPath = path; break; }
			ownerPath = path.Substring(0, end);
			childIndex = path.IndexOfFrom(end, "/children/");
		}
		int ownerIndex = m_Paths.Find(ownerPath);
		if (ownerIndex >= 0) return m_Visited[ownerIndex];
		return null;
	}

	//! Select the components belonging to one entity so child installations cannot change its catalog membership.
	static void ComponentsOwnedBy(BaseContainer entity, map<string, ref array<BaseContainer>> components, notnull map<string, ref array<BaseContainer>> ownerComponents)
	{
		foreach (string className, array<BaseContainer> instances : components)
		{
			array<BaseContainer> matches = {};
			foreach (BaseContainer instance : instances)
				if (OwningEntitySource(instance) == entity) matches.Insert(instance);
			if (!matches.IsEmpty()) ownerComponents.Insert(className, matches);
		}
	}

	//! Walks the effective component and child-entity lists; ancestors are not installations.
	protected static void CollectEffective(BaseContainer holder, map<string, ref array<BaseContainer>> outComps, string path, int depth)
	{
		if (!holder) return;
		if (m_Active.Find(holder) >= 0 || depth > 128)
		{
			TBD_EquipmentExportJson.ExtractionError(m_CurrentResource, path, "Component cycle or traversal limit");
			return;
		}
		if (m_Visited.Find(holder) >= 0) return;
		m_Visited.Insert(holder);
		m_Paths.Insert(path);
		m_Active.Insert(holder);
		BaseContainerList components = holder.GetObjectArray("components");
		if (components)
		{
			for (int i = 0; i < components.Count(); i++)
			{
				BaseContainer component = components.Get(i);
				if (!component) continue;
				string cls = component.GetClassName();
				array<BaseContainer> bucket = outComps.Get(cls);
				if (!bucket) { bucket = {}; outComps.Insert(cls, bucket); }
				if (bucket.Find(component) < 0) bucket.Insert(component);
				CollectEffective(component, outComps, path + "/components/" + i.ToString(), depth + 1);
			}
		}
		for (int child = 0; child < holder.GetNumChildren(); child++)
			CollectEffective(holder.GetChild(child), outComps, path + "/children/" + child.ToString(), depth + 1);
		m_Active.Remove(m_Active.Count() - 1);
	}
}

//! Preserves the calling resource's source locations while inspecting a referenced installation.
class TBD_EquipmentComponentContext
{
	string m_sResource;
	string m_sPrefabResource;
	ref array<BaseContainer> m_Visited = {};
	ref array<string> m_Paths = {};
}
