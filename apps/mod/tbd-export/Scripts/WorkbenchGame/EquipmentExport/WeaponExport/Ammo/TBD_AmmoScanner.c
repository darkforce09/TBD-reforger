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

	// The rows this scan has classified so far, handed to the writer when the scan finishes.
	protected ref TBD_AmmoCatalog m_Catalog = new TBD_AmmoCatalog();

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

		Print(string.Format("%1 Initial discovery complete: %2 magazines, %3 projectiles found.", TAG, m_Catalog.MagazineCount(), m_Catalog.ProjectileCount()), LogLevel.NORMAL);

		// Transitive Projectile Resolution: resolve every projectile linked in ammo_resources
		ResolveTransitiveProjectiles();

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 Universal scan finished in %2 ms. Total magazines: %3, Total projectiles: %4.",
			TAG, elapsedMs, m_Catalog.MagazineCount(), m_Catalog.ProjectileCount()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		TBD_AmmoCatalogWriter.WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs, m_Catalog);

		return m_Catalog.MagazineCount() + m_Catalog.ProjectileCount();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_Catalog.Clear();

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
		mag.m_sDisplayName = TBD_AmmoNaming.DisplayNameFor(comps, path);
		mag.m_sDescription = TBD_AmmoNaming.DescriptionFor(comps);
		mag.m_sIcon = TBD_AmmoNaming.IconFor(comps);
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
		TBD_AmmoMagazineExtractor.ExtractMagazineWells(comps, mag.m_aMagazineWells);

		// Capacity & Style
		TBD_AmmoMagazineExtractor.ExtractCapacity(comps, mag.m_Capacity, path);

		// Caliber
		TBD_AmmoMagazineExtractor.ExtractCaliber(comps, mag.m_Caliber, path);

		// AmmoConfig & Projectile Links
		string confPath;
		TBD_AmmoMagazineExtractor.ExtractAmmoConfig(comps, mag.m_aAmmoResources, confPath);

		// Tracers
		TBD_AmmoTracerExtractor.ExtractTracers(comps, mag.m_aAmmoResources, mag.m_Capacity.m_iRoundCapacity, mag.m_Tracers, mag.m_sDisplayName);

		// Physical
		TBD_AmmoMagazineExtractor.ExtractPhysical(comps, mag.m_Capacity.m_iRoundCapacity, mag.m_Capacity.m_fWeightPerRoundKg, mag.m_Physical);

		// Categorize & Family
		mag.m_sCategory = TBD_AmmoMagazineExtractor.CategorizeMagazine(mag.m_aMagazineWells, mag.m_Capacity.m_iRoundCapacity, path);
		mag.m_sFamily = TBD_AmmoNaming.ExtractFamily(path, mag.m_sCategory);

		// Insert into category bucket
		m_Catalog.InsertMagazine(mag.m_sCategory, mag);
	}

	//------------------------------------------------------------------------------------------------
	protected void ProcessProjectile(string canonical, string path, BaseContainer root, map<string, ref array<BaseContainer>> comps, bool isAbstract)
	{
		m_SeenProjResourceNames.Insert(canonical);

		TBD_ProjectileInfo proj = new TBD_ProjectileInfo();
		proj.m_sResourceName = canonical;
		proj.m_sFilePath = path;
		proj.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		proj.m_sDisplayName = TBD_AmmoNaming.DisplayNameFor(comps, path);
		proj.m_sDescription = TBD_AmmoNaming.DescriptionFor(comps);
		proj.m_sAddonId = m_sCurrentAddonId;
		proj.m_bIsAbstract = isAbstract;

		// Ballistics
		TBD_AmmoProjectileExtractor.ExtractBallistics(comps, proj.m_Ballistics);

		// Warhead & Damage
		TBD_AmmoProjectileExtractor.ExtractWarhead(comps, proj.m_Warhead);

		// Tracer status
		TBD_AmmoTracerExtractor.ExtractTracerInfo(root, comps, proj.m_Tracer);

		// Visuals
		TBD_AmmoProjectileExtractor.ExtractVisuals(root, proj.m_Visuals);

		// Caliber & Bullet Type
		TBD_MagazineCaliberInfo calInfo = new TBD_MagazineCaliberInfo();
		TBD_AmmoMagazineExtractor.ExtractCaliber(comps, calInfo, path);
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
		proj.m_sCategory = TBD_AmmoProjectileExtractor.CategorizeProjectile(path, proj.m_sCaliber, proj.m_Warhead.m_bIsExplosive);

		// Insert into category bucket
		m_Catalog.InsertProjectile(proj.m_sCategory, proj);
	}

	//------------------------------------------------------------------------------------------------
	//! Guarantee zero dangling foreign keys: resolve and index every projectile referenced by magazines.
	protected void ResolveTransitiveProjectiles()
	{
		array<string> missing = {};
		foreach (TBD_MagazineInfo mag : m_Catalog.m_aAllMags)
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

}
