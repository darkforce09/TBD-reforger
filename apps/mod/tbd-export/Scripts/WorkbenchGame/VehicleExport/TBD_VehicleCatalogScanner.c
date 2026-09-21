/**
 * TBD_VehicleCatalogScanner.c
 *
 * Universal scanner that discovers, classifies, and catalogs all vehicle platforms
 * and their variants across all loaded addons using pure engine introspection.
 */

class TBD_VehicleCatalogScanner
{
	protected static const string TAG = "[TBD][VehicleCatalogScanner]";
	protected static const int ANCESTOR_CAP = 16;
	protected static const int COMPONENT_DEPTH_CAP = 6;

	// Structural asset folders to skip early for performance
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

	ref array<ref TBD_VehiclePlatformInfo> m_aPlatforms = {};
	ref map<string, ref TBD_VehiclePlatformInfo> m_mPlatformMap = new map<string, ref TBD_VehiclePlatformInfo>();
	ref array<ref TBD_VehicleVariantInfo> m_aAllVariants = {};

	protected int m_iSeen = 0;
	protected int m_iSkippedNonVehicle = 0;
	protected string m_sCurrentAddonId;

	//------------------------------------------------------------------------------------------------
	//! Execute catalog discovery across all loaded addons.
	bool ScanAllAddons()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Vehicle Catalog Scan...", LogLevel.NORMAL);

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			m_sCurrentAddonId = addonId;

			// Scan main vehicle directory
			string rootPath = "$" + addonId + ":Prefabs/Vehicles";
			Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);

			// Also scan MP assets and scenario vehicles
			Workbench.SearchResources(OnResourceFound, extEt, null, "$" + addonId + ":Prefabs/MP", true);
			Workbench.SearchResources(OnResourceFound, extEt, null, "$" + addonId + ":Prefabs/Scenarios", true);
		}

		// Fallback global search if needed
		if (m_aAllVariants.IsEmpty())
		{
			m_sCurrentAddonId = string.Empty;
			Workbench.SearchResources(OnResourceFound, extEt, null, string.Empty, true);
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 Scan completed in %2 ms: %3 platforms, %4 total vehicle variants discovered.",
			TAG, elapsedMs, m_aPlatforms.Count(), m_aAllVariants.Count()), LogLevel.NORMAL);

		return !m_aAllVariants.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnResourceFound(ResourceName resName, string filePath = "")
	{
		m_iSeen++;
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		if (!path.Contains("Prefabs/"))
			return;

		foreach (string deny : DENY_HARD)
		{
			if (path.Contains(deny))
				return;
		}

		if (path.EndsWith("_dst.et") || path.Contains("_wreck_") || path.Contains("_Wreck"))
			return;

		string addonId = m_sCurrentAddonId;
		if (addonId.IsEmpty() && path.StartsWith("$"))
		{
			int colon = path.IndexOf(":");
			if (colon > 1)
				addonId = path.Substring(1, colon - 1);
		}
		if (addonId.IsEmpty())
			addonId = "ArmaReforger";

		ProcessCandidate(resName, path, addonId);
	}

	//------------------------------------------------------------------------------------------------
	protected void ProcessCandidate(string resName, string filePath, string addonId)
	{
		foreach (TBD_VehicleVariantInfo ex : m_aAllVariants)
		{
			if (ex.m_sResourceName == resName)
				return;
		}

		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid())
			return;

		BaseResourceObject resObj = res.GetResource();
		if (!resObj)
			return;

		BaseContainer root = resObj.ToBaseContainer();
		if (!root)
			return;

		string rootClass = root.GetClassName();

		// Hard exclusion: characters
		if (rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter")
		{
			m_iSkippedNonVehicle++;
			return;
		}

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		CollectComponentChain(root, comps);

		if (HasCompSuffix(comps, "CharacterControllerComponent"))
		{
			m_iSkippedNonVehicle++;
			return;
		}

		// Vehicle detection: must possess simulation or vehicle controller component
		bool isVehicle = (rootClass == "Vehicle" || rootClass.EndsWith("Vehicle")
			|| HasCompSuffix(comps, "VehicleWheeledSimulation")
			|| HasCompSuffix(comps, "VehicleHelicopterSimulation")
			|| HasCompSuffix(comps, "VehicleBoatSimulation")
			|| HasCompSuffix(comps, "VehicleTrackedSimulation")
			|| HasCompSuffix(comps, "VehiclePlaneSimulation")
			|| HasCompSuffix(comps, "VehicleFixedWingSimulation")
			|| HasCompSuffix(comps, "SCR_CarControllerComponent")
			|| HasCompSuffix(comps, "VehicleControllerComponent")
			|| HasCompSuffix(comps, "HelicopterControllerComponent"));

		if (!isVehicle)
		{
			m_iSkippedNonVehicle++;
			return;
		}

		// Must have compartment manager for crew/passengers
		if (!HasCompSuffix(comps, "CompartmentManagerComponent"))
		{
			m_iSkippedNonVehicle++;
			return;
		}

		// Build variant record
		TBD_VehicleVariantInfo varInfo = new TBD_VehicleVariantInfo();
		varInfo.m_sResourceName = resName;
		varInfo.m_sFilePath = filePath;
		varInfo.m_sAddonId = addonId;

		int lastSlash = filePath.LastIndexOf("/");
		if (lastSlash != -1)
		{
			string slug = filePath.Substring(lastSlash + 1, filePath.Length() - lastSlash - 1);
			slug.Replace(".et", "");
			varInfo.m_sId = slug;
		}
		else
		{
			varInfo.m_sId = "Vehicle";
		}

		varInfo.m_bIsAbstract = varInfo.m_sId.EndsWith("_Base") || varInfo.m_sId.EndsWith("_base");

		BaseContainer anc = root.GetAncestor();
		if (anc)
			varInfo.m_sParentPrefab = anc.GetResourceName();

		// Component-driven introspection pipeline
		TBD_VehicleCatalogClassifier.ExtractIdentity(comps, varInfo);
		TBD_VehicleCatalogClassifier.ExtractSpecs(comps, varInfo);
		TBD_VehicleCatalogClassifier.ExtractSeating(comps, varInfo);
		TBD_VehicleCatalogClassifier.ExtractArmament(comps, varInfo);
		TBD_VehicleCatalogClassifier.ExtractDomainAndRole(comps, varInfo);

		// Dynamic platform resolution via taxonomy and inheritance
		string platformId = TBD_VehicleCatalogClassifier.ResolvePlatformId(filePath, resName, root);
		varInfo.m_sPlatform = platformId;

		AssignToPlatform(platformId, varInfo);
		m_aAllVariants.Insert(varInfo);
	}

	//------------------------------------------------------------------------------------------------
	protected void CollectComponentChain(BaseContainer root, notnull map<string, ref array<BaseContainer>> outComps)
	{
		BaseContainer cur = root;
		int hops = 0;
		while (cur && hops < ANCESTOR_CAP)
		{
			CollectComponentsRec(cur, outComps, 0);
			cur = cur.GetAncestor();
			hops++;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void CollectComponentsRec(BaseContainer holder, notnull map<string, ref array<BaseContainer>> outComps, int depth)
	{
		if (depth > COMPONENT_DEPTH_CAP)
			return;

		BaseContainerList comps = holder.GetObjectArray("components");
		if (!comps)
			return;

		for (int i = 0, n = comps.Count(); i < n; i++)
		{
			BaseContainer comp = comps.Get(i);
			if (!comp)
				continue;

			string cls = comp.GetClassName();
			array<BaseContainer> bucket = outComps.Get(cls);
			if (!bucket)
			{
				bucket = {};
				outComps.Insert(cls, bucket);
			}
			bucket.Insert(comp);

			CollectComponentsRec(comp, outComps, depth + 1);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith(suffix))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected void AssignToPlatform(string platformId, TBD_VehicleVariantInfo varInfo)
	{
		TBD_VehiclePlatformInfo plat = m_mPlatformMap.Get(platformId);
		if (!plat)
		{
			plat = new TBD_VehiclePlatformInfo();
			plat.m_sPlatformId = platformId;
			plat.m_sDisplayName = TBD_VehicleExportNaming.FormatPlatformDisplayName(platformId);
			plat.m_sVehicleDomain = varInfo.m_sVehicleDomain;
			plat.m_sPrimaryFaction = varInfo.m_sFaction;
			m_mPlatformMap.Insert(platformId, plat);
			m_aPlatforms.Insert(plat);
		}

		if (plat.m_sPrimaryFaction == "Unaffiliated" && varInfo.m_sFaction != "Unaffiliated")
			plat.m_sPrimaryFaction = varInfo.m_sFaction;

		plat.m_aVariants.Insert(varInfo);
	}

	//------------------------------------------------------------------------------------------------
	string SerializeAllToJson()
	{
		string json = "{\n";
		json += "  \"generatedAt\": \"" + TBD_VehicleExportJson.IsoNowUtc() + "\",\n";
		json += "  \"totalPlatforms\": " + m_aPlatforms.Count().ToString() + ",\n";
		json += "  \"totalVariants\": " + m_aAllVariants.Count().ToString() + ",\n";
		json += "  \"platforms\": [\n";

		for (int i = 0, n = m_aPlatforms.Count(); i < n; i++)
		{
			json += m_aPlatforms[i].SerializeToJson();
			if (i < n - 1) json += ",";
			json += "\n";
		}

		json += "  ]\n";
		json += "}\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	string SerializeSummaryToJson()
	{
		string json = "{\n";
		json += "  \"generatedAt\": \"" + TBD_VehicleExportJson.IsoNowUtc() + "\",\n";
		json += "  \"totalPlatforms\": " + m_aPlatforms.Count().ToString() + ",\n";
		json += "  \"totalVariants\": " + m_aAllVariants.Count().ToString() + ",\n";
		json += "  \"summary\": [\n";

		for (int i = 0, n = m_aPlatforms.Count(); i < n; i++)
		{
			TBD_VehiclePlatformInfo plat = m_aPlatforms[i];
			json += "    {\n";
			json += "      \"platformId\": \"" + plat.m_sPlatformId + "\",\n";
			json += "      \"displayName\": \"" + TBD_VehicleExportJson.Escape(plat.m_sDisplayName) + "\",\n";
			json += "      \"domain\": \"" + plat.m_sVehicleDomain + "\",\n";
			json += "      \"primaryFaction\": \"" + plat.m_sPrimaryFaction + "\",\n";
			json += "      \"variantCount\": " + plat.m_aVariants.Count().ToString() + "\n";
			json += "    }";
			if (i < n - 1) json += ",";
			json += "\n";
		}

		json += "  ]\n";
		json += "}\n";
		return json;
	}
}
