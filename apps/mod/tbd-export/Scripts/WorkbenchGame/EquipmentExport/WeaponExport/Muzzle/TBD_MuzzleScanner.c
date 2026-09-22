//------------------------------------------------------------------------------------------------
// TBD_MuzzleScanner.c
//
// Universal muzzle devices scanner across all loaded addons for Arma Reforger Workbench.
// Discovers, introspects, and exports all muzzle devices (sound suppressors, silencers,
// flash hiders, muzzle brakes, and compensators) to dedicated JSON catalogs and metadata sidecars
// at $profile:TBD_Export/equipment/muzzles/.
//------------------------------------------------------------------------------------------------

class TBD_MuzzleScanner
{
	//! How deep into nested component arrays this scan reads. Passed to the shared graph walk.
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_MuzzleInfo> m_aAllMuzzles = {};
	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_MuzzleScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal muzzle devices scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][MuzzleExport] Starting universal muzzle devices scan across all loaded addons...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target paths containing muzzle attachments across addons
		array<string> scanPaths = {
			"Prefabs/Weapons/Attachments/Muzzle",
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
		Print(string.Format("[TBD][MuzzleExport] Universal scan finished in %1 ms. Total muzzle devices: %2.", elapsedMs, m_aAllMuzzles.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllMuzzles.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aAllMuzzles.Clear();
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
		ref TBD_MuzzleMountingInfo mounting = new TBD_MuzzleMountingInfo();
		TBD_MuzzleExtractor.ExtractMounting(comps, mounting);

		string lowerPath = path;
		lowerPath.ToLower();
		bool isMuzzlePath = lowerPath.Contains("/muzzle/") || lowerPath.Contains("/muzzles/");
		bool hasSuppressorComp = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SuppressorComponent");

		string attType = mounting.m_sAttachmentType;
		typename t;
		if (!attType.IsEmpty())
			t = attType.ToType();

		// Explicit non-muzzle attachment type rejection
		bool isNonMuzzle = false;
		if (t)
		{
			if (t == AttachmentUnderBarrel || t.IsInherited(AttachmentUnderBarrel)
				|| t == AttachmentOptics || t.IsInherited(AttachmentOptics)
				|| t == AttachmentBayonet || t.IsInherited(AttachmentBayonet)
				|| t == AttachmentStock || t.IsInherited(AttachmentStock)
				|| t == AttachmentHandGuard || t.IsInherited(AttachmentHandGuard))
			{
				isNonMuzzle = true;
			}
		}

		if (isNonMuzzle)
			return;

		if (mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentUnderBarrel")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentOptics")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentBayonet")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentStock")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentHandGuard"))
		{
			return;
		}

		// Verify genuine muzzle device identity:
		// 1. Script typename inherits from AttachmentMuzzle
		// 2. Container config inheritance reaches AttachmentMuzzle
		// 3. Attachment type identifier describes a muzzle device
		bool isMuzzleType = (t && (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)));
		bool isMuzzleContainer = mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentMuzzle");
		bool hasMuzzleTypeName = attType.Contains("Muzzle") || attType.Contains("Suppressor") || attType.Contains("FlashHider") || attType.Contains("Compensator")
			|| attType.Contains("muzzle") || attType.Contains("suppressor") || attType.Contains("flashhider") || attType.Contains("compensator");

		bool isMuzzle = isMuzzleType || isMuzzleContainer || hasMuzzleTypeName;

		// Fallback for abstract base prefabs or base templates without explicit attachment type
		if (!isMuzzle && isMuzzlePath && (hasSuppressorComp || isAbstract))
			isMuzzle = true;

		if (!isMuzzle)
			return;

		string canonical = TBD_EquipmentResourceNames.NormalizePathSeparators(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		ref TBD_MuzzleInfo info = new TBD_MuzzleInfo();
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

		// Acoustics & ballistics modifiers
		TBD_MuzzleExtractor.ExtractModifiers(comps, info.m_Modifiers, mounting.m_sAttachmentType, path);

		// Physical attributes
		TBD_MuzzleExtractor.ExtractPhysical(comps, info.m_Physical);

		// Visuals (3D mesh)
		TBD_MuzzleExtractor.ExtractVisuals(comps, info.m_Visuals);

		m_aAllMuzzles.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "muzzles", "muzzles.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "muzzles", "muzzles_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][MuzzleExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllMuzzles.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"muzzles\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllMuzzles.Count(); i++)
		{
			TBD_MuzzleInfo item = m_aAllMuzzles[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllMuzzles.Count() - 1)
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
			meta += "  \"category\": \"muzzles\",\n";
			meta += "  \"totalCount\": " + m_aAllMuzzles.Count().ToString() + ",\n";
			meta += "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][MuzzleExport] Wrote %1 muzzle devices to %2", m_aAllMuzzles.Count(), filePath), LogLevel.NORMAL);
	}
}
