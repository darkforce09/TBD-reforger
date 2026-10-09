/**
 * TBD_VehicleDeepExtractor.c
 *
 * Discovers vehicle variants and coordinates the standard domain extractors.
 * Effective configurations preserve installed instances and source-authored values.
 */

class TBD_VehicleDeepExtractor
{
	protected static const string TAG = "[TBD][VehicleDeepExtractor]";
	protected static const int COMPONENT_DEPTH_CAP = 6;

	protected static const ref array<string> DENY_HARD = {
		"/VehParts/",
		"/Turret/",
		"/Turrets/",
		"/Seats/",
		"/WeapSystems/",
		"/WeapMounts/",
		"/Lights/",
		"/Probes/",
		"/VirtualSlots/",
		"/Dst/",
		"/Wrecks/",
		"/Crater/",
		"Prefabs/Editor/",
		"Prefabs/Systems/",
		"Prefabs/Waypoints/",
		"Prefabs/Triggers/",
		"Prefabs/Compositions/",
		"Prefabs/Sounds/",
		"Prefabs/UI/",
		"Prefabs/Characters/"
	};

	ref array<ref TBD_VehicleDeepPlatform> m_aPlatforms = {};
	ref map<string, ref TBD_VehicleDeepPlatform> m_mPlatformMap = new map<string, ref TBD_VehicleDeepPlatform>();
	ref array<ref TBD_VehicleDeepVariant> m_aAllVariants = {};

	protected string m_sCurrentAddonId;
	protected string m_sPlatformFilter;

	//! Runs the same candidate extraction for a single installed resource during verification.
	void ScanResource(string resourceName)
	{
		ProcessCandidate(resourceName, resourceName, "ArmaReforger");
	}

	//------------------------------------------------------------------------------------------------
	//! Execute full dynamic vehicle discovery and deep engineering extraction across addons.
	bool ScanAllAddons(string platformFilter = "")
	{
		int startMs = System.GetTickCount();
		m_sPlatformFilter = platformFilter;
		m_sPlatformFilter.ToLower();

		Print(TAG + " Starting Universal Vehicle Deep Introspection Scan...", LogLevel.NORMAL);

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			m_sCurrentAddonId = addonId;

			string rootPath = "$" + addonId + ":Prefabs/Vehicles";
			Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);
			Workbench.SearchResources(OnResourceFound, extEt, null, "$" + addonId + ":Prefabs/MP", true);
			Workbench.SearchResources(OnResourceFound, extEt, null, "$" + addonId + ":Prefabs/Scenarios", true);
		}

		if (m_aAllVariants.IsEmpty())
		{
			m_sCurrentAddonId = string.Empty;
			Workbench.SearchResources(OnResourceFound, extEt, null, string.Empty, true);
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 Completed in %2 ms: %3 platforms, %4 variants extracted.",
			TAG, elapsedMs, m_aPlatforms.Count(), m_aAllVariants.Count()), LogLevel.NORMAL);

		return !m_aAllVariants.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty()) path = resName;

		if (!path.Contains("Prefabs/")) return;

		foreach (string deny : DENY_HARD)
		{
			if (path.Contains(deny)) return;
		}

		if (path.EndsWith("_dst.et") || path.Contains("_wreck_") || path.Contains("_Wreck"))
			return;

		string addonId = m_sCurrentAddonId;
		if (addonId.IsEmpty() && path.StartsWith("$"))
		{
			int colon = path.IndexOf(":");
			if (colon > 1) addonId = path.Substring(1, colon - 1);
		}
		if (addonId.IsEmpty()) addonId = "ArmaReforger";

		ProcessCandidate(resName, path, addonId);
	}

	//------------------------------------------------------------------------------------------------
	protected void ProcessCandidate(string resName, string filePath, string addonId)
	{
		foreach (TBD_VehicleDeepVariant ex : m_aAllVariants)
		{
			if (ex.m_sResourceName == resName) return;
		}

		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid()) return;
		BaseResourceObject resObj = res.GetResource();
		if (!resObj) return;
		BaseContainer root = resObj.ToBaseContainer();
		if (!root) return;

		string rootClass = root.GetClassName();
		if (rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter")
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		CollectComponentChain(root, comps);
		map<string, ref array<BaseContainer>> rootComponents = new map<string, ref array<BaseContainer>>();
		foreach (string componentClass, array<BaseContainer> instances : comps)
		{
			array<BaseContainer> rootInstances = {};
			foreach (BaseContainer instance : instances)
				if (!TBD_EquipmentComponentGraph.StructuralPath(instance).Contains("/children/")) rootInstances.Insert(instance);
			if (!rootInstances.IsEmpty()) rootComponents.Insert(componentClass, rootInstances);
		}

		if (HasCompSuffix(comps, "CharacterControllerComponent"))
			return;

		bool isVehicle = (TBD_EquipmentComponentGraph.IsA(rootClass, "Vehicle")
			|| HasCompSuffix(rootComponents, "VehicleWheeledSimulation")
			|| HasCompSuffix(rootComponents, "VehicleHelicopterSimulation")
			|| HasCompSuffix(rootComponents, "VehicleBoatSimulation")
			|| HasCompSuffix(rootComponents, "VehicleTrackedSimulation")
			|| HasCompSuffix(rootComponents, "SCR_CarControllerComponent")
			|| HasCompSuffix(rootComponents, "VehicleControllerComponent")
			|| HasCompSuffix(rootComponents, "HelicopterControllerComponent"));

		if (!isVehicle || !HasCompSuffix(rootComponents, "BaseCompartmentManagerComponent"))
			return;

		// Dynamic platform resolution via directory taxonomy and inheritance
		string platformId = TBD_VehicleCatalogClassifier.ResolvePlatformId(filePath, resName, root);

		if (!m_sPlatformFilter.IsEmpty())
		{
			string lowerPlat = platformId;
			lowerPlat.ToLower();
			string lowerPath = filePath;
			lowerPath.ToLower();
			if (!lowerPlat.Contains(m_sPlatformFilter) && !lowerPath.Contains(m_sPlatformFilter))
				return;
		}

		TBD_VehicleDeepVariant varData = new TBD_VehicleDeepVariant();
		varData.m_sResourceName = resName;
		varData.m_sFilePath = filePath;
		varData.m_sAddonId = addonId;
		varData.m_sPlatform = platformId;

		int lastSlash = filePath.LastIndexOf("/");
		if (lastSlash != -1)
		{
			string slug = filePath.Substring(lastSlash + 1, filePath.Length() - lastSlash - 1);
			slug.Replace(".et", "");
			varData.m_sId = slug;
		}
		else
		{
			varData.m_sId = "Vehicle";
		}

		BaseContainer anc = root.GetAncestor();
		if (anc) varData.m_sParentPrefab = anc.GetResourceName();

		// Component-driven introspection pipeline
		ExtractIdentityAndRole(comps, root, varData);
		varData.m_sNamesJson = TBD_EquipmentDisplayAttributes.Names(comps);
		varData.m_sInventoryJson = TBD_ItemInventoryExtractor.Inventory(comps);
		ExtractPhysicsProperties(comps, varData);

		// Subsystem modular extractors
		TBD_VehicleDrivetrainExtractor.Extract(comps, varData);
		TBD_VehicleDrivetrainExtractor.ExtractSource(comps, varData);
		TBD_VehicleSystemsExtractor.Extract(comps, varData);
		TBD_VehicleArmorExtractor.Extract(comps, varData);
		TBD_VehicleCompartmentExtractor.Extract(comps, varData);
		TBD_VehicleTurretExtractor.Extract(comps, varData);

		AssignToPlatform(platformId, varData);
		m_aAllVariants.Insert(varData);
	}

	//------------------------------------------------------------------------------------------------
	protected void ExtractIdentityAndRole(map<string, ref array<BaseContainer>> comps, BaseContainer root, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleVariantInfo temp = new TBD_VehicleVariantInfo();
		temp.m_sId = varData.m_sId;
		TBD_VehicleCatalogClassifier.ExtractIdentity(comps, temp);
		TBD_VehicleCatalogClassifier.ExtractDomainAndRole(comps, temp);

		varData.m_sDisplayName = TBD_EquipmentDisplayAttributes.English(TBD_EquipmentDisplayAttributes.RawDisplayNameFor(comps));
		varData.m_sDescription = TBD_EquipmentDisplayAttributes.English(TBD_EquipmentDisplayAttributes.RawDescriptionFor(comps));
		varData.m_sFaction = temp.m_sFaction;
		varData.m_sVehicleDomain = temp.m_sVehicleDomain;

	}

	//------------------------------------------------------------------------------------------------
	//! Keeps vehicle-body physics separate from inventory mass and installed child bodies.
	protected void ExtractPhysicsProperties(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		map<string, ref array<BaseContainer>> rootBodies = new map<string, ref array<BaseContainer>>();
		array<BaseContainer> bodies = {};
		array<BaseContainer> candidates = comps.Get("RigidBody");
		if (candidates)
		{
			foreach (BaseContainer body : candidates)
			{
				if (!body || TBD_EquipmentComponentGraph.StructuralPath(body).Contains("/children/")) continue;
				bodies.Insert(body);
			}
		}
		rootBodies.Insert("RigidBody", bodies);
		varData.m_sPhysicsJson = TBD_ItemInventoryExtractor.Physics(rootBodies);
		if (bodies.Count() == 1)
		{
			bodies[0].Get("Mass", varData.m_fWeightKg);
			bodies[0].Get("CenterOfMass", varData.m_vCenterOfMass);
			string center = TBD_EquipmentExportJson.Field(bodies[0], "CenterOfMass", "/physics/center_of_mass");
			varData.m_sPhysicsJson = TBD_EquipmentExportJson.Member(varData.m_sPhysicsJson, "center_of_mass", center);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void CollectComponentChain(BaseContainer root, notnull map<string, ref array<BaseContainer>> outComps)
	{
		TBD_EquipmentComponentGraph.CollectComponentChain(root, outComps, COMPONENT_DEPTH_CAP);
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (TBD_EquipmentComponentGraph.IsA(cls, suffix)) return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected void AssignToPlatform(string platformId, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleDeepPlatform plat = m_mPlatformMap.Get(platformId);
		if (!plat)
		{
			plat = new TBD_VehicleDeepPlatform();
			plat.m_sPlatformId = platformId;
			plat.m_sDisplayName = TBD_VehicleExportNaming.FormatPlatformDisplayName(platformId);
			plat.m_sVehicleDomain = varData.m_sVehicleDomain;
			plat.m_sPrimaryFaction = varData.m_sFaction;
			m_mPlatformMap.Insert(platformId, plat);
			m_aPlatforms.Insert(plat);
		}

		if (plat.m_sPrimaryFaction == "Unaffiliated" && varData.m_sFaction != "Unaffiliated")
			plat.m_sPrimaryFaction = varData.m_sFaction;

		plat.m_aVariants.Insert(varData);
	}
}
