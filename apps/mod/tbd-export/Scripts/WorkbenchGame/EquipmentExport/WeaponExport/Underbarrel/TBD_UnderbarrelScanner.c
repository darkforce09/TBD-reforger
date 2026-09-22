//------------------------------------------------------------------------------------------------
// TBD_UnderbarrelScanner.c
//
// Universal underbarrel devices scanner across all loaded addons for Arma Reforger Workbench.
// Discovers, introspects, and exports all underbarrel attachments, secondary weapon systems
// (grenade launchers like M203 and GP-25, grips, accessories) to dedicated JSON catalogs and
// metadata sidecars at $profile:TBD_Export/equipment/underbarrel/.
//------------------------------------------------------------------------------------------------

class TBD_UnderbarrelScanner
{
	//! How deep into nested component arrays this scan reads. Passed to the shared graph walk.
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_UnderbarrelInfo> m_aAllUnderbarrel = {};
	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_UnderbarrelScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal underbarrel devices scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][UnderbarrelExport] Starting universal underbarrel devices scan across all loaded addons...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target paths containing underbarrel attachments across addons
		array<string> scanPaths = {
			"Prefabs/Weapons/Attachments/Underbarrel",
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
		Print(string.Format("[TBD][UnderbarrelExport] Universal scan finished in %1 ms. Total underbarrel devices: %2.", elapsedMs, m_aAllUnderbarrel.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllUnderbarrel.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aAllUnderbarrel.Clear();
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

		// Extract mounting relational keys directly from container data
		ref TBD_UnderbarrelMountingInfo mounting = new TBD_UnderbarrelMountingInfo();
		TBD_UnderbarrelExtractor.ExtractMounting(comps, mounting);

		string lowerPath = path;
		lowerPath.ToLower();
		bool isUnderbarrelPath = lowerPath.Contains("/underbarrel/") || lowerPath.Contains("/underbarrel");

		string attType = mounting.m_sAttachmentType;
		typename t;
		if (!attType.IsEmpty())
			t = attType.ToType();

		// 1. Strict non-underbarrel attachment type rejection
		bool isNonUnderbarrel = false;
		if (t)
		{
			if (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)
				|| t == AttachmentOptics || t.IsInherited(AttachmentOptics)
				|| t == AttachmentBayonet || t.IsInherited(AttachmentBayonet)
				|| t == AttachmentStock || t.IsInherited(AttachmentStock)
				|| t == AttachmentHandGuard || t.IsInherited(AttachmentHandGuard))
			{
				isNonUnderbarrel = true;
			}
		}

		if (isNonUnderbarrel)
			return;

		if (mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentMuzzle")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentOptics")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentBayonet")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentStock")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentHandGuard"))
		{
			return;
		}

		// 2. Strict ground-truth qualification:
		// - Script typename inherits from AttachmentUnderBarrel
		// - Container config inheritance reaches AttachmentUnderBarrel
		// - Attachment type identifier contains UnderBarrel
		bool isUnderbarrelType = (t && (t == AttachmentUnderBarrel || t.IsInherited(AttachmentUnderBarrel)));
		bool isUnderbarrelContainer = mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentUnderBarrel");
		bool hasUnderbarrelTypeName = attType.Contains("UnderBarrel") || attType.Contains("Underbarrel") || attType.Contains("underbarrel");

		bool isUnderbarrel = isUnderbarrelType || isUnderbarrelContainer || hasUnderbarrelTypeName;

		// Fallback for abstract base templates or base prefabs in underbarrel path
		if (!isUnderbarrel && isUnderbarrelPath && (isAbstract || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MuzzleComponent")))
			isUnderbarrel = true;

		if (!isUnderbarrel)
			return;

		string canonical = TBD_EquipmentResourceNames.NormalizePathSeparators(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		ref TBD_UnderbarrelInfo info = new TBD_UnderbarrelInfo();
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

		// Launcher / secondary weapon system properties (muzzles, mag wells, chamber capacity, zeroing distances)
		TBD_UnderbarrelLauncherExtractor.ExtractLauncher(comps, info.m_Launcher);

		// Physical attributes
		TBD_UnderbarrelExtractor.ExtractPhysical(comps, info.m_Physical);

		// Visuals (3D mesh)
		TBD_UnderbarrelExtractor.ExtractVisuals(comps, info.m_Visuals);

		m_aAllUnderbarrel.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "underbarrel", "underbarrel.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "underbarrel", "underbarrel_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][UnderbarrelExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllUnderbarrel.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"underbarrel\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllUnderbarrel.Count(); i++)
		{
			TBD_UnderbarrelInfo item = m_aAllUnderbarrel[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllUnderbarrel.Count() - 1)
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
			meta += "  \"category\": \"underbarrel\",\n";
			meta += "  \"totalCount\": " + m_aAllUnderbarrel.Count().ToString() + ",\n";
			meta += "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][UnderbarrelExport] Wrote %1 underbarrel devices to %2", m_aAllUnderbarrel.Count(), filePath), LogLevel.NORMAL);
	}
}
