//------------------------------------------------------------------------------------------------
// TBD_StockScanner.c
//
// Universal weapon buttstocks and stock assemblies scanner across all loaded addons for Arma Reforger Workbench.
// Discovers, introspects, and exports all buttstock devices to dedicated JSON catalogs and metadata
// sidecars at $profile:TBD_Export/equipment/stocks/.
//------------------------------------------------------------------------------------------------

class TBD_StockScanner
{
	//! How deep into nested component arrays this scan reads. Passed to the shared graph walk.
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_StockInfo> m_aAllStocks = {};
	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_StockScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal buttstocks scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][StockExport] Starting universal buttstocks scan across all loaded addons...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target paths containing buttstock attachments across addons
		array<string> scanPaths = {
			"Prefabs/Weapons/Attachments/Stocks",
			"Prefabs/Weapons/Attachments/Stock",
			"Prefabs/Weapons/Attachments/Buttstocks",
			"Prefabs/Weapons/Attachments/Buttstock",
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
		Print(string.Format("[TBD][StockExport] Universal scan finished in %1 ms. Total buttstocks: %2.", elapsedMs, m_aAllStocks.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllStocks.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aAllStocks.Clear();
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
		ref TBD_StockMountingInfo mounting = new TBD_StockMountingInfo();
		TBD_StockMountingExtractor.ExtractMounting(comps, mounting);

		string lowerPath = path;
		lowerPath.ToLower();
		bool isStockPath = lowerPath.Contains("/stock/") || lowerPath.Contains("/stocks/") || lowerPath.Contains("/buttstock/") || lowerPath.Contains("/buttstocks/");

		string attType = mounting.m_sAttachmentType;
		typename t;
		if (!attType.IsEmpty())
			t = attType.ToType();

		// 1. Strict non-stock attachment type rejection
		bool isNonStock = false;
		if (t)
		{
			if (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)
				|| t == AttachmentOptics || t.IsInherited(AttachmentOptics)
				|| t == AttachmentUnderBarrel || t.IsInherited(AttachmentUnderBarrel)
				|| t == AttachmentBayonet || t.IsInherited(AttachmentBayonet)
				|| t == AttachmentHandGuard || t.IsInherited(AttachmentHandGuard))
			{
				isNonStock = true;
			}
		}

		if (isNonStock)
			return;

		if (mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentMuzzle")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentOptics")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentUnderBarrel")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentBayonet")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentHandGuard"))
		{
			return;
		}

		// 2. Strict ground-truth qualification:
		// - Script typename inherits from AttachmentStock
		// - Container config inheritance reaches AttachmentStock
		// - Attachment type identifier contains Stock or Buttstock
		bool isStockType = (t && (t == AttachmentStock || t.IsInherited(AttachmentStock)));
		bool isStockContainer = mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentStock");
		bool hasStockTypeName = attType.Contains("Stock") || attType.Contains("stock") || attType.Contains("Buttstock") || attType.Contains("buttstock");

		bool isStock = isStockType || isStockContainer || hasStockTypeName;

		// Fallback for abstract base prefabs in stock path
		if (!isStock && isStockPath && isAbstract)
			isStock = true;

		if (!isStock)
			return;

		string canonical = TBD_EquipmentResourceNames.NormalizePathSeparators(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		ref TBD_StockInfo info = new TBD_StockInfo();
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
		info.m_sDisplayName = TBD_StockNaming.RawDisplayNameFor(comps);
		info.m_sDescription = TBD_StockNaming.RawDescriptionFor(comps);
		info.m_sIcon = TBD_StockNaming.RawIconFor(comps);

		// Mounting keys
		info.m_Mounting = mounting;

		// Nested child attachment slots (e.g. cheek pads, cheek risers, sling swivels)
		TBD_StockMountingExtractor.ExtractNestedAttachmentSlots(comps, info.m_aNestedSlots);

		// Handling & Recoil modifiers (SCR_WeaponAttachmentAttributes)
		TBD_StockExtractor.ExtractHandlingModifiers(comps, info.m_Handling);

		// Physical attributes
		TBD_StockExtractor.ExtractPhysical(comps, info.m_Physical);

		// Visuals (3D model mesh)
		TBD_StockExtractor.ExtractVisuals(comps, info.m_Visuals);

		m_aAllStocks.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "stocks", "stocks.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "stocks", "stocks_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][StockExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllStocks.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"stocks\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllStocks.Count(); i++)
		{
			TBD_StockInfo item = m_aAllStocks[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllStocks.Count() - 1)
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
			meta += "  \"category\": \"stocks\",\n";
			meta += "  \"totalCount\": " + m_aAllStocks.Count().ToString() + ",\n";
			meta += "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][StockExport] Wrote %1 buttstocks to %2", m_aAllStocks.Count(), filePath), LogLevel.NORMAL);
	}
}
