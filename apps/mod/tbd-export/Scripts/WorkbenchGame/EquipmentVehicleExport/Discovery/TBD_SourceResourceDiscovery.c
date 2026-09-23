// Discovery preserves the established prefab search scope. Facts are read separately.
class TBD_SourceResourceDiscovery
{
	ref array<string> m_aEquipment = {};
	ref array<string> m_aVehicles = {};
	ref array<string> m_aErrors = {};
	protected ref array<string> m_aVisited = {};
	protected static const ref array<string> DENY_PATHS = {
		"/Structures/", "/Rocks/", "/Trees/", "/Debris/", "/Foliage/", "Prefabs/Editor/",
		"Prefabs/Systems/", "Prefabs/Waypoints/", "Prefabs/Triggers/", "Prefabs/Compositions/", "Prefabs/Sounds/", "Prefabs/UI/"
	};

	bool Scan()
	{
		array<string> addons = {};
		GameProject.GetLoadedAddons(addons);
		array<string> extensions = {"et"};
		foreach (string addon : addons)
		{
			string rootPath = "$" + GameProject.GetAddonID(addon) + ":Prefabs";
			if (!Workbench.SearchResources(OnResource, extensions, null, rootPath, true)) m_aErrors.Insert("Resource search failed: " + rootPath);
		}
		if (addons.IsEmpty()) m_aErrors.Insert("No loaded addons are available");
		m_aEquipment.Sort();
		m_aVehicles.Sort();
		if (m_aEquipment.IsEmpty() || m_aVehicles.IsEmpty()) m_aErrors.Insert("Discovery returned an empty equipment or vehicle inventory");
		return m_aErrors.IsEmpty();
	}

	void Inspect(array<string> resources)
	{
		foreach (string name : resources) OnResource(name);
	}

	protected void OnResource(ResourceName name, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty()) path = name;
		if (!path.Contains("Prefabs/")) return;
		foreach (string denied : DENY_PATHS) if (path.Contains(denied)) return;
		if (m_aVisited.Find(name) >= 0) return;
		m_aVisited.Insert(name);
		Resource resource = Resource.Load(name);
		if (!resource || !resource.IsValid() || !resource.GetResource()) { m_aErrors.Insert("Cannot inspect prefab: " + name); return; }
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root) { m_aErrors.Insert("Prefab lacks a container: " + name); return; }
		array<string> classes = {};
		array<BaseContainer> active = {};
		Collect(root, classes, active, 0);
		string rootClass = root.GetClassName();
		if (TBD_SourceCapabilityRules.IsA(rootClass, "ChimeraCharacter") || Has(classes, "CharacterControllerComponent")) return;
		bool vehicle = TBD_SourceCapabilityRules.IsA(rootClass, "Vehicle") || Has(classes, "VehicleWheeledSimulation") || Has(classes, "VehicleHelicopterSimulation");
		vehicle = vehicle || Has(classes, "VehicleBoatSimulation") || Has(classes, "VehicleTrackedSimulation") || Has(classes, "VehiclePlaneSimulation");
		vehicle = vehicle || Has(classes, "VehicleFixedWingSimulation") || Has(classes, "VehicleControllerComponent") || Has(classes, "SCR_CarControllerComponent");
		bool compartments = Has(classes, "BaseCompartmentManagerComponent") || Has(classes, "CompartmentManagerComponent");
		if (vehicle)
		{
			if (compartments && !path.EndsWith("_dst.et") && !path.Contains("_wreck_") && !path.Contains("_Wreck")) m_aVehicles.Insert(name);
			return;
		}
		bool inventory = Has(classes, "InventoryItemComponent");
		bool staticWeapon = rootClass == "Turret" || path.Contains("/Tripods/") || path.Contains("/Mortars/");
		bool assembly = path.Contains("/VehParts/") || path.Contains("Prefabs/Vehicles/");
		if (!inventory && compartments && (assembly || !staticWeapon)) return;
		bool equipment = inventory || Has(classes, "WeaponComponent") || Has(classes, "MagazineComponent") || Has(classes, "BaseLoadoutClothComponent") || Has(classes, "LoadoutClothComponent");
		equipment = equipment || Has(classes, "SCR_GadgetComponent") || Has(classes, "SCR_BinocularsComponent") || Has(classes, "WeaponAttachmentAttributes");
		equipment = equipment || Has(classes, "SCR_FastTravelAction") || Has(classes, "TurretComponent") || staticWeapon || path.Contains("/Ammo/");
		if (equipment) m_aEquipment.Insert(name);
	}

	protected void Collect(BaseContainer node, array<string> classes, array<BaseContainer> active, int depth)
	{
		if (depth > 128 || active.Find(node) >= 0) { m_aErrors.Insert("Discovery container cycle or depth limit"); return; }
		active.Insert(node);
		string name = node.GetClassName();
		if (classes.Find(name) < 0) classes.Insert(name);
		BaseContainerList components = node.GetObjectArray("components");
		if (components)
			for (int i = 0; i < components.Count(); i++) if (components.Get(i)) Collect(components.Get(i), classes, active, depth + 1);
		BaseContainerList actions = node.GetObjectArray("additionalActions");
		if (actions)
			for (int action = 0; action < actions.Count(); action++) if (actions.Get(action)) Collect(actions.Get(action), classes, active, depth + 1);
		BaseContainer attributes = node.GetObject("Attributes");
		if (attributes)
		{
			BaseContainerList custom = attributes.GetObjectArray("CustomAttributes");
			if (custom)
				for (int j = 0; j < custom.Count(); j++) if (custom.Get(j)) Collect(custom.Get(j), classes, active, depth + 1);
		}
		active.Remove(active.Count() - 1);
	}

	protected bool Has(array<string> classes, string baseName)
	{
		foreach (string name : classes)
			if (TBD_SourceCapabilityRules.IsA(name, baseName)) return true;
		return false;
	}
}
