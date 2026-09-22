//------------------------------------------------------------------------------------------------
// TBD_AmmoScanner.c
//
// Universal ammunition & magazines scanner across all loaded addons for Arma Reforger Workbench.
// Categorizes, introspects, and exports all magazines and projectiles
// (rifle mags, MG boxes/belts, pistol mags, autocannon belts, 40mm grenades, rockets, ballistics)
// to dedicated JSON catalogs and unified master catalogs in $profile:TBD_Export/equipment/ammunition/.
//------------------------------------------------------------------------------------------------

class TBD_AmmoScanner
{
	//! How deep into nested component arrays this scan reads. Passed to the shared graph walk.
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected static const string TAG = "[TBD][AmmoExport]";

	protected ref TBD_EquipmentExportConfig m_Config;

	// Magazine category buckets
	protected ref array<ref TBD_MagazineInfo> m_aRifleMags = {};
	protected ref array<ref TBD_MagazineInfo> m_aMgMags = {};
	protected ref array<ref TBD_MagazineInfo> m_aHandgunMags = {};
	protected ref array<ref TBD_MagazineInfo> m_aHeavyMags = {};
	protected ref array<ref TBD_MagazineInfo> m_aGrenadeMags = {};
	protected ref array<ref TBD_MagazineInfo> m_aRocketMags = {};
	protected ref array<ref TBD_MagazineInfo> m_aAllMags = {};

	// Projectile category buckets
	protected ref array<ref TBD_ProjectileInfo> m_aBulletProjectiles = {};
	protected ref array<ref TBD_ProjectileInfo> m_aHeavyProjectiles = {};
	protected ref array<ref TBD_ProjectileInfo> m_aGrenadeProjectiles = {};
	protected ref array<ref TBD_ProjectileInfo> m_aRocketProjectiles = {};
	protected ref array<ref TBD_ProjectileInfo> m_aMortarProjectiles = {};
	protected ref array<ref TBD_ProjectileInfo> m_aAllProjectiles = {};

	// Deduplication sets
	protected ref set<string> m_SeenMagResourceNames = new set<string>();
	protected ref set<string> m_SeenProjResourceNames = new set<string>();

	protected string m_sCurrentAddonId;

	//------------------------------------------------------------------------------------------------
	void TBD_AmmoScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal multi-category ammunition & magazines scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print(TAG + " Starting universal ammunition & magazines scan...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target search directories across all addons
		array<string> relPaths = {
			"Prefabs/Weapons/Magazines",
			"Prefabs/Weapons/Ammo",
			"Prefabs/Weapons/Warheads",
			"Prefabs/Weapons/Projectiles"
		};

		foreach (string rel : relPaths)
		{
			foreach (string guid : guids)
			{
				string addonId = GameProject.GetAddonID(guid);
				m_sCurrentAddonId = addonId;
				string rootPath = "$" + addonId + ":" + rel;
				Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);
			}
		}

		Print(string.Format("%1 Initial discovery complete: %2 magazines, %3 projectiles found.", TAG, m_aAllMags.Count(), m_aAllProjectiles.Count()), LogLevel.NORMAL);

		// Transitive Projectile Resolution: resolve every projectile linked in ammo_resources
		ResolveTransitiveProjectiles();

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 Universal scan finished in %2 ms. Total magazines: %3, Total projectiles: %4.",
			TAG, elapsedMs, m_aAllMags.Count(), m_aAllProjectiles.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllMags.Count() + m_aAllProjectiles.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aRifleMags.Clear();
		m_aMgMags.Clear();
		m_aHandgunMags.Clear();
		m_aHeavyMags.Clear();
		m_aGrenadeMags.Clear();
		m_aRocketMags.Clear();
		m_aAllMags.Clear();

		m_aBulletProjectiles.Clear();
		m_aHeavyProjectiles.Clear();
		m_aGrenadeProjectiles.Clear();
		m_aRocketProjectiles.Clear();
		m_aMortarProjectiles.Clear();
		m_aAllProjectiles.Clear();

		m_SeenMagResourceNames.Clear();
		m_SeenProjResourceNames.Clear();
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

		bool hasMag = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MagazineComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "BaseMagazineComponent");
		bool hasMove = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MoveComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ShellMoveComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ProjectileMoveComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MissileMoveComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "GrenadeMoveComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "FlareMoveComponent");
		bool isProj = (rootClass == "Projectile" || rootClass == "TracerProjectile" || rootClass.EndsWith("Projectile"));

		if (!hasMag && !hasMove && !isProj)
			return;

		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// 1. Process as Magazine if it has MagazineComponent
		if (hasMag && !m_SeenMagResourceNames.Contains(canonical))
		{
			ProcessMagazine(canonical, path, root, comps, isAbstract);
		}

		// 2. Process as Projectile if it has MoveComponent or is Projectile entity
		if ((hasMove || isProj) && !m_SeenProjResourceNames.Contains(canonical))
		{
			ProcessProjectile(canonical, path, root, comps, isAbstract);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void ProcessMagazine(string canonical, string path, BaseContainer root, map<string, ref array<BaseContainer>> comps, bool isAbstract)
	{
		m_SeenMagResourceNames.Insert(canonical);

		TBD_MagazineInfo mag = new TBD_MagazineInfo();
		mag.m_sResourceName = canonical;
		mag.m_sFilePath = path;
		mag.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		mag.m_sDisplayName = TBD_AmmoExtractor.DisplayNameFor(comps, path);
		mag.m_sDescription = TBD_AmmoExtractor.DescriptionFor(comps);
		mag.m_sIcon = TBD_AmmoExtractor.IconFor(comps);
		mag.m_sAddonId = m_sCurrentAddonId;
		mag.m_bIsAbstract = isAbstract;

		// Ancestor / Variant Of
		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRn = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(anc.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				mag.m_sVariantOf = ancRn;
		}

		// Relational Foreign Keys: Magazine Wells
		TBD_AmmoExtractor.ExtractMagazineWells(comps, mag.m_aMagazineWells);

		// Capacity & Style
		TBD_AmmoExtractor.ExtractCapacity(comps, mag.m_Capacity, path);

		// Caliber
		TBD_AmmoExtractor.ExtractCaliber(comps, mag.m_Caliber, path);

		// AmmoConfig & Projectile Links
		string confPath;
		TBD_AmmoExtractor.ExtractAmmoConfig(comps, mag.m_aAmmoResources, confPath);

		// Tracers
		TBD_AmmoExtractor.ExtractTracers(comps, mag.m_aAmmoResources, mag.m_Capacity.m_iRoundCapacity, mag.m_Tracers, mag.m_sDisplayName);

		// Physical
		TBD_AmmoExtractor.ExtractPhysical(comps, mag.m_Capacity.m_iRoundCapacity, mag.m_Capacity.m_fWeightPerRoundKg, mag.m_Physical);

		// Categorize & Family
		mag.m_sCategory = TBD_AmmoExtractor.CategorizeMagazine(mag.m_aMagazineWells, mag.m_Capacity.m_iRoundCapacity, path);
		mag.m_sFamily = TBD_AmmoExtractor.ExtractFamily(path, mag.m_sCategory);

		// Insert into category bucket
		InsertCategoryMagazine(mag.m_sCategory, mag);
		m_aAllMags.Insert(mag);
	}

	//------------------------------------------------------------------------------------------------
	protected void InsertCategoryMagazine(string category, TBD_MagazineInfo mag)
	{
		if (category == "magazines_rifle") m_aRifleMags.Insert(mag);
		else if (category == "magazines_mg") m_aMgMags.Insert(mag);
		else if (category == "magazines_handgun") m_aHandgunMags.Insert(mag);
		else if (category == "magazines_heavy") m_aHeavyMags.Insert(mag);
		else if (category == "magazines_grenades") m_aGrenadeMags.Insert(mag);
		else if (category == "magazines_rockets") m_aRocketMags.Insert(mag);
		else m_aRifleMags.Insert(mag);
	}

	//------------------------------------------------------------------------------------------------
	protected void ProcessProjectile(string canonical, string path, BaseContainer root, map<string, ref array<BaseContainer>> comps, bool isAbstract)
	{
		m_SeenProjResourceNames.Insert(canonical);

		TBD_ProjectileInfo proj = new TBD_ProjectileInfo();
		proj.m_sResourceName = canonical;
		proj.m_sFilePath = path;
		proj.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		proj.m_sDisplayName = TBD_AmmoExtractor.DisplayNameFor(comps, path);
		proj.m_sDescription = TBD_AmmoExtractor.DescriptionFor(comps);
		proj.m_sAddonId = m_sCurrentAddonId;
		proj.m_bIsAbstract = isAbstract;

		// Ballistics
		TBD_AmmoExtractor.ExtractBallistics(comps, proj.m_Ballistics);

		// Warhead & Damage
		TBD_AmmoExtractor.ExtractWarhead(comps, proj.m_Warhead);

		// Tracer status
		TBD_AmmoExtractor.ExtractTracerInfo(root, comps, proj.m_Tracer);

		// Visuals
		TBD_AmmoExtractor.ExtractVisuals(root, proj.m_Visuals);

		// Caliber & Bullet Type
		TBD_MagazineCaliberInfo calInfo = new TBD_MagazineCaliberInfo();
		TBD_AmmoExtractor.ExtractCaliber(comps, calInfo, path);
		proj.m_sCaliber = calInfo.m_sCaliberName;
		proj.m_sBulletType = calInfo.m_sAmmoType;

		if (proj.m_Tracer.m_bIsTracer && !proj.m_sBulletType.Contains("Tracer"))
		{
			if (proj.m_sBulletType == "Ball" || proj.m_sBulletType.IsEmpty())
				proj.m_sBulletType = "Tracer";
			else
				proj.m_sBulletType = proj.m_sBulletType + "Tracer";
		}

		// Categorize
		proj.m_sCategory = TBD_AmmoExtractor.CategorizeProjectile(path, proj.m_sCaliber, proj.m_Warhead.m_bIsExplosive);

		// Insert into category bucket
		InsertCategoryProjectile(proj.m_sCategory, proj);
		m_aAllProjectiles.Insert(proj);
	}

	//------------------------------------------------------------------------------------------------
	protected void InsertCategoryProjectile(string category, TBD_ProjectileInfo proj)
	{
		if (category == "projectiles_bullets") m_aBulletProjectiles.Insert(proj);
		else if (category == "projectiles_heavy") m_aHeavyProjectiles.Insert(proj);
		else if (category == "projectiles_grenades") m_aGrenadeProjectiles.Insert(proj);
		else if (category == "projectiles_rockets") m_aRocketProjectiles.Insert(proj);
		else if (category == "projectiles_mortar") m_aMortarProjectiles.Insert(proj);
		else m_aBulletProjectiles.Insert(proj);
	}

	//------------------------------------------------------------------------------------------------
	//! Guarantee zero dangling foreign keys: resolve and index every projectile referenced by magazines.
	protected void ResolveTransitiveProjectiles()
	{
		array<string> missing = {};
		foreach (TBD_MagazineInfo mag : m_aAllMags)
		{
			foreach (string res : mag.m_aAmmoResources)
			{
				if (!m_SeenProjResourceNames.Contains(res) && missing.Find(res) == -1)
					missing.Insert(res);
			}
		}

		if (missing.IsEmpty())
			return;

		Print(string.Format("%1 Resolving %2 transitive projectiles referenced by magazines...", TAG, missing.Count()), LogLevel.NORMAL);

		foreach (string resName : missing)
		{
			Resource r = Resource.Load(resName);
			if (!r || !r.IsValid())
				continue;

			BaseResourceObject bro = r.GetResource();
			if (!bro)
				continue;

			BaseContainer root = bro.ToBaseContainer();
			if (!root)
				continue;

			string path = root.GetResourceName();
			if (path.IsEmpty())
				path = resName;

			map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
			TBD_EquipmentComponentGraph.CollectComponentChain(root, comps, COMPONENT_DEPTH_CAP);

			bool isAbs = (path.EndsWith("_base.et") || path.Contains("/base/"));
			ProcessProjectile(resName, path, root, comps, isAbs);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Write all magazine catalogs, projectile catalogs, and unified master catalog to disk.
	void WriteAllFiles(string destDir, int elapsedMs)
	{
		// 1. Magazine Catalogs
		WriteMagazineCategoryCatalog("magazines_rifle", m_aRifleMags, destDir);
		WriteMagazineCategoryCatalog("magazines_mg", m_aMgMags, destDir);
		WriteMagazineCategoryCatalog("magazines_handgun", m_aHandgunMags, destDir);
		WriteMagazineCategoryCatalog("magazines_heavy", m_aHeavyMags, destDir);
		WriteMagazineCategoryCatalog("magazines_grenades", m_aGrenadeMags, destDir);
		WriteMagazineCategoryCatalog("magazines_rockets", m_aRocketMags, destDir);
		WriteMasterMagazinesCatalog(destDir);

		// 2. Projectile Catalogs
		WriteProjectileCategoryCatalog("projectiles_bullets", m_aBulletProjectiles, destDir);
		WriteProjectileCategoryCatalog("projectiles_heavy", m_aHeavyProjectiles, destDir);
		WriteProjectileCategoryCatalog("projectiles_grenades", m_aGrenadeProjectiles, destDir);
		WriteProjectileCategoryCatalog("projectiles_rockets", m_aRocketProjectiles, destDir);
		WriteProjectileCategoryCatalog("projectiles_mortar", m_aMortarProjectiles, destDir);
		WriteMasterProjectilesCatalog(destDir);

		// 3. Unified Master Catalog
		WriteUnifiedMasterCatalog(destDir, elapsedMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMagazineCategoryCatalog(string category, array<ref TBD_MagazineInfo> list, string destDir)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/magazines", category + ".json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/magazines", category + "_meta.json");

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
		TBD_EquipmentExportJson.Write(f, "  \"magazines\": [\n", TAG);

		for (int i = 0; i < list.Count(); i++)
		{
			TBD_MagazineInfo item = list[i];
			string itemJson = item.SerializeJson("    ");
			if (i < list.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";

			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		// Metadata sidecar
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

		Print(string.Format("%1 Wrote %2 magazines to %3", TAG, list.Count(), filePath), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMasterMagazinesCatalog(string destDir)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/magazines", "magazines_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/magazines", "magazines_all_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f) return;

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"magazines_all\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllMags.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"magazines\": [\n", TAG);

		for (int i = 0; i < m_aAllMags.Count(); i++)
		{
			string itemJson = m_aAllMags[i].SerializeJson("    ");
			if (i < m_aAllMags.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";
			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"category\": \"magazines_all\",\n";
			meta += "  \"totalCount\": " + m_aAllMags.Count().ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"categories\": {\n";
			meta += "    \"magazines_rifle\": " + m_aRifleMags.Count().ToString() + ",\n";
			meta += "    \"magazines_mg\": " + m_aMgMags.Count().ToString() + ",\n";
			meta += "    \"magazines_handgun\": " + m_aHandgunMags.Count().ToString() + ",\n";
			meta += "    \"magazines_heavy\": " + m_aHeavyMags.Count().ToString() + ",\n";
			meta += "    \"magazines_grenades\": " + m_aGrenadeMags.Count().ToString() + ",\n";
			meta += "    \"magazines_rockets\": " + m_aRocketMags.Count().ToString() + "\n";
			meta += "  }\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteProjectileCategoryCatalog(string category, array<ref TBD_ProjectileInfo> list, string destDir)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/projectiles", category + ".json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/projectiles", category + "_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f) return;

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"" + category + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + list.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"projectiles\": [\n", TAG);

		for (int i = 0; i < list.Count(); i++)
		{
			string itemJson = list[i].SerializeJson("    ");
			if (i < list.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";
			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

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

		Print(string.Format("%1 Wrote %2 projectiles to %3", TAG, list.Count(), filePath), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMasterProjectilesCatalog(string destDir)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/projectiles", "projectiles_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/projectiles", "projectiles_all_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f) return;

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"projectiles_all\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllProjectiles.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"projectiles\": [\n", TAG);

		for (int i = 0; i < m_aAllProjectiles.Count(); i++)
		{
			string itemJson = m_aAllProjectiles[i].SerializeJson("    ");
			if (i < m_aAllProjectiles.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";
			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"category\": \"projectiles_all\",\n";
			meta += "  \"totalCount\": " + m_aAllProjectiles.Count().ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"categories\": {\n";
			meta += "    \"projectiles_bullets\": " + m_aBulletProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_heavy\": " + m_aHeavyProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_grenades\": " + m_aGrenadeProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_rockets\": " + m_aRocketProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_mortar\": " + m_aMortarProjectiles.Count().ToString() + "\n";
			meta += "  }\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteUnifiedMasterCatalog(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition", "ammunition_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition", "ammunition_all_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f) return;

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"ammunition_all\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalMagazines\": " + m_aAllMags.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalProjectiles\": " + m_aAllProjectiles.Count().ToString() + ",\n", TAG);

		// Magazines
		TBD_EquipmentExportJson.Write(f, "  \"magazines\": [\n", TAG);
		for (int m = 0; m < m_aAllMags.Count(); m++)
		{
			string mJson = m_aAllMags[m].SerializeJson("    ");
			if (m < m_aAllMags.Count() - 1)
				mJson += ",\n";
			else
				mJson += "\n";
			TBD_EquipmentExportJson.Write(f, mJson, TAG);
		}
		TBD_EquipmentExportJson.Write(f, "  ],\n", TAG);

		// Projectiles
		TBD_EquipmentExportJson.Write(f, "  \"projectiles\": [\n", TAG);
		for (int p = 0; p < m_aAllProjectiles.Count(); p++)
		{
			string pJson = m_aAllProjectiles[p].SerializeJson("    ");
			if (p < m_aAllProjectiles.Count() - 1)
				pJson += ",\n";
			else
				pJson += "\n";
			TBD_EquipmentExportJson.Write(f, pJson, TAG);
		}
		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		// Metadata sidecar
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"category\": \"ammunition_all\",\n";
			meta += "  \"totalMagazines\": " + m_aAllMags.Count().ToString() + ",\n";
			meta += "  \"totalProjectiles\": " + m_aAllProjectiles.Count().ToString() + ",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"magazineCategories\": {\n";
			meta += "    \"magazines_rifle\": " + m_aRifleMags.Count().ToString() + ",\n";
			meta += "    \"magazines_mg\": " + m_aMgMags.Count().ToString() + ",\n";
			meta += "    \"magazines_handgun\": " + m_aHandgunMags.Count().ToString() + ",\n";
			meta += "    \"magazines_heavy\": " + m_aHeavyMags.Count().ToString() + ",\n";
			meta += "    \"magazines_grenades\": " + m_aGrenadeMags.Count().ToString() + ",\n";
			meta += "    \"magazines_rockets\": " + m_aRocketMags.Count().ToString() + "\n";
			meta += "  },\n";
			meta += "  \"projectileCategories\": {\n";
			meta += "    \"projectiles_bullets\": " + m_aBulletProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_heavy\": " + m_aHeavyProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_grenades\": " + m_aGrenadeProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_rockets\": " + m_aRocketProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_mortar\": " + m_aMortarProjectiles.Count().ToString() + "\n";
			meta += "  }\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		Print(string.Format("%1 Master ammunition catalog written: %2 magazines, %3 projectiles to %4",
			TAG, m_aAllMags.Count(), m_aAllProjectiles.Count(), filePath), LogLevel.NORMAL);
	}
}
