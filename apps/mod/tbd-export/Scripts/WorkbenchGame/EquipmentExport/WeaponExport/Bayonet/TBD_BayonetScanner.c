//------------------------------------------------------------------------------------------------
// TBD_BayonetScanner.c
//
// Universal bayonets scanner across all loaded addons for Arma Reforger Workbench.
// Discovers, introspects, and exports all rifle bayonets and blade attachments to dedicated JSON
// catalogs and metadata sidecars at $profile:TBD_Export/equipment/bayonets/.
//------------------------------------------------------------------------------------------------

class TBD_BayonetScanner
{
	//! How deep into nested component arrays this scan reads. Passed to the shared graph walk.
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_BayonetInfo> m_aAllBayonets = {};
	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_BayonetScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal bayonets scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][BayonetExport] Starting universal bayonets scan across all loaded addons...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target paths containing bayonet attachments across addons
		array<string> scanPaths = {
			"Prefabs/Weapons/Attachments/Bayonets",
			"Prefabs/Weapons/Attachments/Bayonet",
			"Prefabs/Weapons/Attachments",
			"Prefabs/Weapons/Core"
		};

		foreach (string relPath : scanPaths)
		{
			foreach (string guid : guids)
			{
				string addonId = GameProject.GetAddonID(guid);
				string rootPath = "$" + addonId + ":" + relPath;
				Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);
			}
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("[TBD][BayonetExport] Universal scan finished in %1 ms. Total bayonets: %2.", elapsedMs, m_aAllBayonets.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllBayonets.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aAllBayonets.Clear();
		m_SeenResourceNames.Clear();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

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
		if (rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter" || rootClass == "Vehicle" || rootClass.EndsWith("Vehicle"))
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, comps, COMPONENT_DEPTH_CAP);

		// Complete weapons have WeaponComponent, they are not attachments
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WeaponComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "BaseWeaponComponent"))
			return;

		// Extract mounting relational keys directly from container data
		ref TBD_BayonetMountingInfo mounting = new TBD_BayonetMountingInfo();
		TBD_BayonetExtractor.ExtractMounting(comps, mounting);

		string lowerPath = path;
		lowerPath.ToLower();
		bool isBayonetPath = lowerPath.Contains("/attachments/") && (lowerPath.Contains("/bayonet/") || lowerPath.Contains("/bayonets/"));

		string attType = mounting.m_sAttachmentType;
		typename t;
		if (!attType.IsEmpty())
			t = attType.ToType();

		// 1. Strict non-bayonet attachment type rejection
		bool isNonBayonet = false;
		if (t)
		{
			if (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)
				|| t == AttachmentOptics || t.IsInherited(AttachmentOptics)
				|| t == AttachmentUnderBarrel || t.IsInherited(AttachmentUnderBarrel)
				|| t == AttachmentStock || t.IsInherited(AttachmentStock)
				|| t == AttachmentHandGuard || t.IsInherited(AttachmentHandGuard))
			{
				isNonBayonet = true;
			}
		}

		if (isNonBayonet)
			return;

		if (mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentMuzzle")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentOptics")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentUnderBarrel")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentStock")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentHandGuard"))
		{
			return;
		}

		// 2. Strict ground-truth qualification:
		// - Script typename inherits from AttachmentBayonet
		// - Container config inheritance reaches AttachmentBayonet
		// - Attachment type identifier contains Bayonet
		// - Has SCR_BayonetComponent or SCR_BayonetEffectComponent
		bool isBayonetType = (t && (t == AttachmentBayonet || t.IsInherited(AttachmentBayonet)));
		bool isBayonetContainer = mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentBayonet");
		bool hasBayonetTypeName = attType.Contains("Bayonet") || attType.Contains("bayonet");
		bool hasBayonetComponent = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_BayonetComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_BayonetEffectComponent");

		bool isBayonet = isBayonetType || isBayonetContainer || hasBayonetTypeName || hasBayonetComponent;

		// Fallback for abstract base prefabs in bayonet path
		if (!isBayonet && isBayonetPath && isAbstract)
			isBayonet = true;

		if (!isBayonet)
			return;

		string canonical = TBD_EquipmentResourceNames.NormalizePathSeparators(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		ref TBD_BayonetInfo info = new TBD_BayonetInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = TBD_EquipmentResourceNames.NormalizePathSeparators(path);
		info.m_bIsAbstract = isAbstract;

		// Ancestor linkage for variant_of
		BaseContainer ancestor = root.GetAncestor();
		if (ancestor)
		{
			string ancRn = TBD_EquipmentResourceNames.NormalizePathSeparators(ancestor.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Raw string extraction (zero mutation, zero fake fallbacks)
		info.m_sDisplayName = TBD_EquipmentDisplayAttributes.RawDisplayNameFor(comps);
		info.m_sDescription = TBD_EquipmentDisplayAttributes.RawDescriptionFor(comps);
		info.m_sIcon = TBD_EquipmentDisplayAttributes.RawIconFor(comps);

		// Mounting keys
		info.m_Mounting = mounting;

		// Combat & Handling attributes (extra obstruction length, melee damage)
		TBD_BayonetCombatExtractor.ExtractCombatHandling(comps, info.m_Combat);

		// Physical attributes
		TBD_BayonetExtractor.ExtractPhysical(comps, info.m_Physical);

		// Visuals (3D mesh)
		TBD_BayonetExtractor.ExtractVisuals(comps, info.m_Visuals);

		m_aAllBayonets.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "bayonets", "bayonets.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "bayonets", "bayonets_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][BayonetExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllBayonets.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"bayonets\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllBayonets.Count(); i++)
		{
			TBD_BayonetInfo item = m_aAllBayonets[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllBayonets.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";

			TBD_EquipmentExportJson.Write(f, itemJson, "[TBD]");
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", "[TBD]");
		f.Close();

		// Metadata sidecar
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"category\": \"bayonets\",\n";
			meta += "  \"totalCount\": " + m_aAllBayonets.Count().ToString() + ",\n";
			meta += "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][BayonetExport] Wrote %1 bayonets to %2", m_aAllBayonets.Count(), filePath), LogLevel.NORMAL);
	}
}
