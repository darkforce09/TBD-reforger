//------------------------------------------------------------------------------------------------
// TBD_HandguardScanner.c
//
// Universal handguards, rail systems, and foregrips scanner across all loaded addons for Arma Reforger Workbench.
// Discovers, introspects, and exports all handguard devices to dedicated JSON catalogs and metadata
// sidecars at $profile:TBD_Export/equipment/handguards/.
//------------------------------------------------------------------------------------------------

class TBD_HandguardScanner
{
	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_HandguardInfo> m_aAllHandguards = {};
	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_HandguardScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal handguards scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][HandguardExport] Starting universal handguards & foregrips scan across all loaded addons...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target paths containing handguards and foregrips across addons
		array<string> scanPaths = {
			"Prefabs/Weapons/Attachments/Handguards",
			"Prefabs/Weapons/Attachments/Handguard",
			"Prefabs/Weapons/Attachments/Grips",
			"Prefabs/Weapons/Attachments/Grip",
			"Prefabs/Weapons/Attachments/Foregrips",
			"Prefabs/Weapons/Attachments/Foregrip",
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
		Print(string.Format("[TBD][HandguardExport] Universal scan finished in %1 ms. Total handguards & foregrips: %2.", elapsedMs, m_aAllHandguards.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllHandguards.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aAllHandguards.Clear();
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
		TBD_HandguardExtractor.CollectComponentChain(root, comps);

		// Complete weapons have WeaponComponent, they are not attachments
		if (TBD_HandguardExtractor.HasCompSuffix(comps, "WeaponComponent") || TBD_HandguardExtractor.HasCompSuffix(comps, "BaseWeaponComponent"))
			return;

		// Extract mounting relational keys directly from container data
		ref TBD_HandguardMountingInfo mounting = new TBD_HandguardMountingInfo();
		TBD_HandguardExtractor.ExtractMounting(comps, mounting);

		string lowerPath = path;
		lowerPath.ToLower();
		bool isHandguardPath = lowerPath.Contains("/handguard/") || lowerPath.Contains("/handguards/") || lowerPath.Contains("/grip/") || lowerPath.Contains("/grips/") || lowerPath.Contains("/foregrip/") || lowerPath.Contains("/foregrips/");

		string attType = mounting.m_sAttachmentType;
		typename t;
		if (!attType.IsEmpty())
			t = attType.ToType();

		// 1. Strict non-handguard attachment type rejection
		bool isNonHandguard = false;
		if (t)
		{
			if (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)
				|| t == AttachmentOptics || t.IsInherited(AttachmentOptics)
				|| t == AttachmentBayonet || t.IsInherited(AttachmentBayonet)
				|| t == AttachmentStock || t.IsInherited(AttachmentStock))
			{
				isNonHandguard = true;
			}
		}

		if (isNonHandguard)
			return;

		if (mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentMuzzle")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentOptics")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentBayonet")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentStock"))
		{
			return;
		}

		// 2. Strict ground-truth qualification:
		// - Script typename inherits from AttachmentHandGuard
		// - Container config inheritance reaches AttachmentHandGuard
		// - Attachment type identifier contains HandGuard / Foregrip
		bool isHandguardType = (t && (t == AttachmentHandGuard || t.IsInherited(AttachmentHandGuard)));
		bool isHandguardContainer = mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentHandGuard");
		bool hasHandguardTypeName = attType.Contains("HandGuard") || attType.Contains("Handguard") || attType.Contains("handguard") || attType.Contains("Foregrip") || attType.Contains("foregrip");

		bool isHandguard = isHandguardType || isHandguardContainer || hasHandguardTypeName;

		// Fallback for abstract base prefabs in handguard path
		if (!isHandguard && isHandguardPath && isAbstract)
			isHandguard = true;

		if (!isHandguard)
			return;

		string canonical = TBD_HandguardExtractor.ResolveCanonical(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		ref TBD_HandguardInfo info = new TBD_HandguardInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = TBD_HandguardExtractor.ResolveCanonical(path);
		info.m_bIsAbstract = isAbstract;

		// Ancestor linkage for variant_of
		BaseContainer ancestor = root.GetAncestor();
		if (ancestor)
		{
			string ancRn = TBD_HandguardExtractor.ResolveCanonical(ancestor.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Raw string extraction (zero mutation, zero fake fallbacks)
		info.m_sDisplayName = TBD_HandguardExtractor.RawDisplayNameFor(comps);
		info.m_sDescription = TBD_HandguardExtractor.RawDescriptionFor(comps);
		info.m_sIcon = TBD_HandguardExtractor.RawIconFor(comps);

		// Mounting keys
		info.m_Mounting = mounting;

		// Nested child attachment slots (modular rails)
		TBD_HandguardExtractor.ExtractNestedAttachmentSlots(comps, info.m_aNestedSlots);

		// Handling & Recoil modifiers (SCR_WeaponAttachmentAttributes)
		TBD_HandguardExtractor.ExtractHandlingModifiers(comps, info.m_Handling);

		// Physical attributes
		TBD_HandguardExtractor.ExtractPhysical(comps, info.m_Physical);

		// Visuals (3D model mesh)
		TBD_HandguardExtractor.ExtractVisuals(comps, info.m_Visuals);

		m_aAllHandguards.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "handguards", "handguards.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "handguards", "handguards_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][HandguardExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllHandguards.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"handguards\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllHandguards.Count(); i++)
		{
			TBD_HandguardInfo item = m_aAllHandguards[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllHandguards.Count() - 1)
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
			meta += "  \"category\": \"handguards\",\n";
			meta += "  \"totalCount\": " + m_aAllHandguards.Count().ToString() + ",\n";
			meta += "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][HandguardExport] Wrote %1 handguards & foregrips to %2", m_aAllHandguards.Count(), filePath), LogLevel.NORMAL);
	}
}
