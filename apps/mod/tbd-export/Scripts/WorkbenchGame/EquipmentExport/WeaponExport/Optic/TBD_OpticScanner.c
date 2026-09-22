//------------------------------------------------------------------------------------------------
// TBD_OpticScanner.c
//
// Universal optics and sights scanner across all loaded addons for Arma Reforger Workbench.
// Discovers, introspects, and exports all optical devices (collimators, reflex sights,
// holographic sights, telescopic scopes, variable power optics, launcher sights, and backup sights)
// to dedicated JSON catalog and metadata sidecar at $profile:TBD_Export/equipment/optics/.
//------------------------------------------------------------------------------------------------

class TBD_OpticScanner
{
	//! How deep into nested component arrays this scan reads. Passed to the shared graph walk.
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_OpticInfo> m_aAllOptics = {};
	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_OpticScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal optics scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][OpticExport] Starting universal optics & sights scan across all loaded addons...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target paths containing optics and sights attachments across addons
		array<string> scanPaths = {
			"Prefabs/Weapons/Attachments/Optics",
			"Prefabs/Weapons/Attachments/Sights",
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
		Print(string.Format("[TBD][OpticExport] Universal scan finished in %1 ms. Total optics: %2.", elapsedMs, m_aAllOptics.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllOptics.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aAllOptics.Clear();
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
		ref TBD_OpticMountingInfo mounting = new TBD_OpticMountingInfo();
		TBD_OpticExtractor.ExtractMounting(comps, mounting);

		string lowerPath = path;
		lowerPath.ToLower();

		string attType = mounting.m_sAttachmentType;
		typename t;
		if (!attType.IsEmpty())
			t = attType.ToType();

		// Explicit non-optic attachment type rejection
		bool isNonOptic = false;
		if (t)
		{
			if (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)
				|| t == AttachmentUnderBarrel || t.IsInherited(AttachmentUnderBarrel)
				|| t == AttachmentBayonet || t.IsInherited(AttachmentBayonet)
				|| t == AttachmentStock || t.IsInherited(AttachmentStock)
				|| t == AttachmentHandGuard || t.IsInherited(AttachmentHandGuard))
			{
				isNonOptic = true;
			}
		}

		if (isNonOptic)
			return;

		if (mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentMuzzle")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentUnderBarrel")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentBayonet")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentStock")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentHandGuard"))
		{
			return;
		}

		// Verify genuine optic device identity:
		// 1. Script typename inherits from AttachmentOptics
		// 2. Container config inheritance reaches AttachmentOptics
		// 3. Attachment type identifier describes an optic attachment
		bool isOpticType = (t && (t == AttachmentOptics || t.IsInherited(AttachmentOptics)));
		bool isOpticContainer = mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentOptics");
		bool hasOpticTypeName = attType.Contains("AttachmentOptics") || attType.Contains("Optic");

		bool isOptic = isOpticType || isOpticContainer || hasOpticTypeName;

		// Fallback for abstract base prefabs or base templates without explicit attachment type
		if (!isOptic && isAbstract && (lowerPath.Contains("/optics/") || lowerPath.Contains("/sights/")))
			isOptic = true;

		if (!isOptic)
			return;

		string canonical = TBD_EquipmentResourceNames.NormalizePathSeparators(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		ref TBD_OpticInfo info = new TBD_OpticInfo();
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

		// Optics & sights parameters
		TBD_OpticExtractor.ExtractSights(comps, info.m_Sights);

		// Physical attributes
		TBD_OpticExtractor.ExtractPhysical(comps, info.m_Physical);

		// Visuals (3D mesh)
		TBD_OpticExtractor.ExtractVisuals(comps, info.m_Visuals);

		m_aAllOptics.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "optics", "optics.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "optics", "optics_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][OpticExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllOptics.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"optics\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllOptics.Count(); i++)
		{
			TBD_OpticInfo item = m_aAllOptics[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllOptics.Count() - 1)
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
			meta += "  \"category\": \"optics\",\n";
			meta += "  \"totalCount\": " + m_aAllOptics.Count().ToString() + ",\n";
			meta += "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][OpticExport] Wrote %1 optics to %2", m_aAllOptics.Count(), filePath), LogLevel.NORMAL);
	}
}
