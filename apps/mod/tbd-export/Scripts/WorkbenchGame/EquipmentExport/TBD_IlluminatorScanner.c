//------------------------------------------------------------------------------------------------
// TBD_IlluminatorScanner.c
//
// Universal tactical lights, IR illuminators, and laser aiming modules scanner across
// all loaded addons for Arma Reforger Workbench.
// Discovers, introspects, and exports all weapon-mounted flashlights, IR illuminators,
// visible/IR lasers, and combo modules to dedicated JSON catalog and metadata sidecar
// at $profile:TBD_Export/equipment/illuminators/.
//------------------------------------------------------------------------------------------------

class TBD_IlluminatorScanner
{
	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_IlluminatorInfo> m_aAllIlluminators = {};
	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_IlluminatorScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal illuminators and pointers scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][IlluminatorExport] Starting universal tactical lights & pointers scan across all loaded addons...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target paths containing tactical lights, lasers, and illuminators across addons
		array<string> scanPaths = {
			"Prefabs/Weapons/Attachments/Flashlights",
			"Prefabs/Weapons/Attachments/Lasers",
			"Prefabs/Weapons/Attachments/Lights",
			"Prefabs/Weapons/Attachments/Illuminators",
			"Prefabs/Weapons/Attachments",
			"Prefabs/Items/Equipment/Flashlights",
			"Prefabs/Items/Equipment/Lasers",
			"Prefabs/Items/Equipment/Illuminators",
			"Prefabs/Items/Equipment",
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
		Print(string.Format("[TBD][IlluminatorExport] Universal scan finished in %1 ms. Total illuminators: %2.", elapsedMs, m_aAllIlluminators.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllIlluminators.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aAllIlluminators.Clear();
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
		TBD_IlluminatorExtractor.CollectComponentChain(root, comps);

		// Complete weapons have WeaponComponent, they are not attachments
		if (TBD_IlluminatorExtractor.HasCompSuffix(comps, "WeaponComponent") || TBD_IlluminatorExtractor.HasCompSuffix(comps, "BaseWeaponComponent"))
			return;

		// Extract mounting relational keys directly from container data
		ref TBD_IlluminatorMountingInfo mounting = new TBD_IlluminatorMountingInfo();
		TBD_IlluminatorExtractor.ExtractMounting(comps, mounting);

		string lowerPath = path;
		lowerPath.ToLower();

		string attType = mounting.m_sAttachmentType;
		typename t;
		if (!attType.IsEmpty())
			t = attType.ToType();

		// 1. Strict non-illuminator attachment type rejection
		bool isNonIlluminator = false;
		if (t)
		{
			if (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)
				|| t == AttachmentOptics || t.IsInherited(AttachmentOptics)
				|| t == AttachmentUnderBarrel || t.IsInherited(AttachmentUnderBarrel)
				|| t == AttachmentBayonet || t.IsInherited(AttachmentBayonet)
				|| t == AttachmentStock || t.IsInherited(AttachmentStock)
				|| t == AttachmentHandGuard || t.IsInherited(AttachmentHandGuard))
			{
				isNonIlluminator = true;
			}
		}

		if (isNonIlluminator)
			return;

		if (mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentMuzzle")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentOptics")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentUnderBarrel")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentBayonet")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentStock")
			|| mounting.m_aCompatibleAttachmentTypes.Contains("AttachmentHandGuard"))
		{
			return;
		}

		// 2. Strict ground-truth qualification:
		// - Possesses illumination component (SCR_FlashlightComponent, BaseLightComponent, LightComponent, etc.)
		// - Possesses laser component (SCR_LaserComponent, LaserComponent, SCR_LaserPointerComponent, etc.)
		// - Attachment type specifies flashlight / laser / illuminator / pointer
		// - Container ancestry specifies illumination or laser attachment type
		bool hasLightComp = TBD_IlluminatorExtractor.HasCompSuffix(comps, "SCR_FlashlightComponent")
			|| TBD_IlluminatorExtractor.HasCompSuffix(comps, "FlashlightComponent")
			|| TBD_IlluminatorExtractor.HasCompSuffix(comps, "BaseLightComponent")
			|| TBD_IlluminatorExtractor.HasCompSuffix(comps, "LightComponent")
			|| TBD_IlluminatorExtractor.HasCompSuffix(comps, "SCR_BaseInteractiveLightComponent");

		bool hasLaserComp = TBD_IlluminatorExtractor.HasCompSuffix(comps, "SCR_LaserComponent")
			|| TBD_IlluminatorExtractor.HasCompSuffix(comps, "LaserComponent")
			|| TBD_IlluminatorExtractor.HasCompSuffix(comps, "SCR_LaserPointerComponent");

		string lowerAttType = attType;
		lowerAttType.ToLower();

		bool hasIllumTypeName = lowerAttType.Contains("flashlight")
			|| lowerAttType.Contains("laser")
			|| lowerAttType.Contains("pointer")
			|| lowerAttType.Contains("illum")
			|| (lowerAttType.Contains("light") && !lowerAttType.Contains("lightmachinegun"));

		bool isIllumContainer = false;
		foreach (string cType : mounting.m_aCompatibleAttachmentTypes)
		{
			string lowerCType = cType;
			lowerCType.ToLower();
			if (lowerCType.Contains("flashlight")
				|| lowerCType.Contains("laser")
				|| lowerCType.Contains("pointer")
				|| lowerCType.Contains("illum")
				|| (lowerCType.Contains("light") && !lowerCType.Contains("lightmachinegun")))
			{
				isIllumContainer = true;
				break;
			}
		}

		bool isIlluminator = hasLightComp || hasLaserComp || hasIllumTypeName || isIllumContainer;

		// Fallback for abstract base prefabs in illuminator/flashlight/laser paths
		if (!isIlluminator && isAbstract && (lowerPath.Contains("/flashlight") || lowerPath.Contains("/laser") || lowerPath.Contains("/illuminator")))
			isIlluminator = true;

		if (!isIlluminator)
			return;

		string canonical = TBD_IlluminatorExtractor.ResolveCanonical(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		ref TBD_IlluminatorInfo info = new TBD_IlluminatorInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = TBD_IlluminatorExtractor.ResolveCanonical(path);
		info.m_bIsAbstract = isAbstract;

		// Ancestor linkage for variant_of
		BaseContainer ancestor = root.GetAncestor();
		if (ancestor)
		{
			string ancRn = TBD_IlluminatorExtractor.ResolveCanonical(ancestor.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Raw string extraction (zero mutation, zero fake fallbacks)
		info.m_sDisplayName = TBD_IlluminatorExtractor.RawDisplayNameFor(comps);
		info.m_sDescription = TBD_IlluminatorExtractor.RawDescriptionFor(comps);
		info.m_sIcon = TBD_IlluminatorExtractor.RawIconFor(comps);

		// Mounting keys
		info.m_Mounting = mounting;

		// Illumination & laser capabilities
		TBD_IlluminatorExtractor.ExtractIllumination(comps, info.m_Illumination, mounting.m_sAttachmentType);

		// Physical attributes
		TBD_IlluminatorExtractor.ExtractPhysical(comps, info.m_Physical);

		// Visuals (3D mesh)
		TBD_IlluminatorExtractor.ExtractVisuals(comps, info.m_Visuals);

		m_aAllIlluminators.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "illuminators", "illuminators.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "illuminators", "illuminators_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][IlluminatorExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllIlluminators.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"illuminators\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllIlluminators.Count(); i++)
		{
			TBD_IlluminatorInfo item = m_aAllIlluminators[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllIlluminators.Count() - 1)
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
			meta += "  \"category\": \"illuminators\",\n";
			meta += "  \"totalCount\": " + m_aAllIlluminators.Count().ToString() + ",\n";
			meta += "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][IlluminatorExport] Exported %1 illuminators to %2", m_aAllIlluminators.Count(), filePath), LogLevel.NORMAL);
	}
}
