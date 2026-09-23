/**
 * TBD_StaticWeaponScanner.c
 *
 * Scans all loaded addons for static emplacements, crew-served mortars, tripod-mounted
 * heavy weapons, and standalone tripod mount frames.
 *
 * Categorizes and serializes to:
 *   - $profile:TBD_Export/equipment/statics/mortars.json
 *   - $profile:TBD_Export/equipment/statics/tripods.json
 *   - $profile:TBD_Export/equipment/statics/mounts.json
 *   - $profile:TBD_Export/equipment/statics/statics_all.json + statics_meta.json
 */

class TBD_StaticWeaponScanner
{
	protected static const string TAG = "[TBD][StaticExport]";
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;

	protected ref array<ref TBD_StaticWeaponInfo> m_aMortars = {};
	protected ref array<ref TBD_StaticWeaponInfo> m_aTripods = {};
	protected ref array<ref TBD_StaticWeaponInfo> m_aMounts = {};
	protected ref array<ref TBD_StaticWeaponInfo> m_aAllStatics = {};

	protected ref set<string> m_SeenResourceNames = new set<string>();
	protected string m_sCurrentAddonId;

	//------------------------------------------------------------------------------------------------
	void TBD_StaticWeaponScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run static weapons scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print(TAG + " Starting static & crew-served weapons scan...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		array<string> scanRoots = {
			"Prefabs/Weapons/Mortars",
			"Prefabs/Weapons/Tripods"
		};

		foreach (string relPath : scanRoots)
		{
			foreach (string guid : guids)
			{
				string addonId = GameProject.GetAddonID(guid);
				m_sCurrentAddonId = addonId;
				string rootPath = "$" + addonId + ":" + relPath;
				Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);
			}
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 Static weapons scan finished in %2 ms. Total statics: %3 (Mortars: %4, Tripods: %5, Mounts: %6)",
			TAG, elapsedMs, m_aAllStatics.Count(), m_aMortars.Count(), m_aTripods.Count(), m_aMounts.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllStatics.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aMortars.Clear();
		m_aTripods.Clear();
		m_aMounts.Clear();
		m_aAllStatics.Clear();
		m_SeenResourceNames.Clear();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		// Filter out preview prefabs and destruction/debris states
		if (path.Contains("/dst/") || path.Contains("_Damaged.et") || path.Contains("_Destroyed.et") || path.Contains("DeployablePreview"))
			return;

		bool isAbstract = (path.EndsWith("_base.et") || path.Contains("/base/"));
		if (isAbstract && !m_Config.m_bIncludeAbstract)
			return;

		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid())
			return;

		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return;

		BaseContainer root = obj.ToBaseContainer();
		if (!root)
			return;

		string rootClass = root.GetClassName();
		if (rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter" || rootClass == "Vehicle")
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, comps, COMPONENT_DEPTH_CAP);

		// Must be a Turret entity or carry a TurretComponent
		bool isTurret = (rootClass == "Turret" || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "TurretComponent"));
		if (!isTurret)
			return;

		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		TBD_StaticWeaponInfo info = new TBD_StaticWeaponInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		info.m_sDisplayName = TBD_StaticWeaponNaming.DisplayNameFor(comps, path);
		info.m_sDescription = TBD_StaticWeaponNaming.DescriptionFor(comps);
		info.m_sFamily = TBD_StaticWeaponNaming.ExtractFamily(path);
		info.m_sFaction = TBD_StaticWeaponNaming.ExtractFaction(comps, path);
		info.m_sAddonId = m_sCurrentAddonId;
		info.m_bIsAbstract = isAbstract;

		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRn = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(anc.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Introspect detailed specs
		TBD_StaticWeaponExtractor.ExtractSpecs(comps, info);

		// Categorize into mortars, tripods, or bare mounts
		if (path.Contains("/Mortars/") || info.m_sFamily == "2B14" || info.m_sFamily == "M252")
		{
			info.m_sCategory = "mortars";
			m_aMortars.Insert(info);
		}
		else if (info.m_bHasArmament && !info.m_sMountedWeaponTemplate.IsEmpty())
		{
			info.m_sCategory = "tripods";
			m_aTripods.Insert(info);
		}
		else
		{
			info.m_sCategory = "mounts";
			m_aMounts.Insert(info);
		}

		m_aAllStatics.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		WriteCatalog("mortars", m_aMortars, destDir);
		WriteCatalog("tripods", m_aTripods, destDir);
		WriteCatalog("mounts", m_aMounts, destDir);
		WriteMasterCatalog(destDir, elapsedMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteCatalog(string category, array<ref TBD_StaticWeaponInfo> list, string destDir)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "statics", category + ".json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "statics", category + "_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("%1 ERROR: Failed to open %2 for writing!", TAG, filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"" + category + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + list.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"items\": [\n", TAG);

		for (int i = 0; i < list.Count(); i++)
		{
			TBD_StaticWeaponInfo item = list[i];
			string itemJson = item.SerializeJson("    ");
			if (i < list.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";

			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		// Sidecar meta
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"category\": \"" + category + "\",\n";
			meta += "  \"totalCount\": " + list.Count().ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		Print(string.Format("%1 Wrote %2 items to %3", TAG, list.Count(), filePath), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMasterCatalog(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "statics", "statics_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "statics", "statics_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("%1 ERROR: Failed to open %2 for writing!", TAG, filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"statics_all\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllStatics.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"items\": [\n", TAG);

		for (int i = 0; i < m_aAllStatics.Count(); i++)
		{
			TBD_StaticWeaponInfo item = m_aAllStatics[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllStatics.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";

			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		// Sidecar meta
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"category\": \"statics_all\",\n";
			meta += "  \"totalCount\": " + m_aAllStatics.Count().ToString() + ",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"categories\": {\n";
			meta += "    \"mortars\": " + m_aMortars.Count().ToString() + ",\n";
			meta += "    \"tripods\": " + m_aTripods.Count().ToString() + ",\n";
			meta += "    \"mounts\": " + m_aMounts.Count().ToString() + "\n";
			meta += "  }\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		Print(string.Format("%1 Master statics catalog written: %2 total items across 3 categories.", TAG, m_aAllStatics.Count()), LogLevel.NORMAL);
	}
}
