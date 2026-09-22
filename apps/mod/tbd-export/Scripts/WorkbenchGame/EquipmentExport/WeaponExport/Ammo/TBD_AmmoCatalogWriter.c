//------------------------------------------------------------------------------------------------
// TBD_AmmoCatalogWriter.c
//
// Serializes an ammunition catalog to disk: one file per magazine caliber, one per projectile
// category, a rollup for each half, and the unified ammunition_all catalog.
//
// It holds no state. Every catalog it writes comes from the TBD_AmmoCatalog handed to it, so what
// lands in a file is decided by the scanner that filled that catalog and is visible at the call
// site.
//
// Each catalog is paired with a _meta.json sidecar carrying the row count, the per-category
// breakdown, and the UTC generation timestamp. A file whose handle fails to open is skipped rather
// than half-written.
//------------------------------------------------------------------------------------------------

class TBD_AmmoCatalogWriter
{
	//! Log prefix, identical to the scanner's so both halves of one export read as a single run.
	protected static const string TAG = "[TBD][AmmoExport]";

	//------------------------------------------------------------------------------------------------
	//! Write all magazine catalogs, projectile catalogs, and unified master catalog to disk.
	static void WriteAllFiles(string destDir, int elapsedMs, notnull TBD_AmmoCatalog catalog)
	{
		// 1. Magazine Catalogs
		WriteMagazineCategoryCatalog("magazines_rifle", catalog.m_aRifleMags, destDir);
		WriteMagazineCategoryCatalog("magazines_mg", catalog.m_aMgMags, destDir);
		WriteMagazineCategoryCatalog("magazines_handgun", catalog.m_aHandgunMags, destDir);
		WriteMagazineCategoryCatalog("magazines_heavy", catalog.m_aHeavyMags, destDir);
		WriteMagazineCategoryCatalog("magazines_grenades", catalog.m_aGrenadeMags, destDir);
		WriteMagazineCategoryCatalog("magazines_rockets", catalog.m_aRocketMags, destDir);
		WriteMasterMagazinesCatalog(destDir, catalog);

		// 2. Projectile Catalogs
		WriteProjectileCategoryCatalog("projectiles_bullets", catalog.m_aBulletProjectiles, destDir);
		WriteProjectileCategoryCatalog("projectiles_heavy", catalog.m_aHeavyProjectiles, destDir);
		WriteProjectileCategoryCatalog("projectiles_grenades", catalog.m_aGrenadeProjectiles, destDir);
		WriteProjectileCategoryCatalog("projectiles_rockets", catalog.m_aRocketProjectiles, destDir);
		WriteProjectileCategoryCatalog("projectiles_mortar", catalog.m_aMortarProjectiles, destDir);
		WriteMasterProjectilesCatalog(destDir, catalog);

		// 3. Unified Master Catalog
		WriteUnifiedMasterCatalog(destDir, elapsedMs, catalog);
	}

	//------------------------------------------------------------------------------------------------
	protected static void WriteMagazineCategoryCatalog(string category, array<ref TBD_MagazineInfo> list, string destDir)
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
	protected static void WriteMasterMagazinesCatalog(string destDir, notnull TBD_AmmoCatalog catalog)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/magazines", "magazines_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/magazines", "magazines_all_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f) return;

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"magazines_all\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + catalog.m_aAllMags.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"magazines\": [\n", TAG);

		for (int i = 0; i < catalog.m_aAllMags.Count(); i++)
		{
			string itemJson = catalog.m_aAllMags[i].SerializeJson("    ");
			if (i < catalog.m_aAllMags.Count() - 1)
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
			meta += "  \"totalCount\": " + catalog.m_aAllMags.Count().ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"categories\": {\n";
			meta += "    \"magazines_rifle\": " + catalog.m_aRifleMags.Count().ToString() + ",\n";
			meta += "    \"magazines_mg\": " + catalog.m_aMgMags.Count().ToString() + ",\n";
			meta += "    \"magazines_handgun\": " + catalog.m_aHandgunMags.Count().ToString() + ",\n";
			meta += "    \"magazines_heavy\": " + catalog.m_aHeavyMags.Count().ToString() + ",\n";
			meta += "    \"magazines_grenades\": " + catalog.m_aGrenadeMags.Count().ToString() + ",\n";
			meta += "    \"magazines_rockets\": " + catalog.m_aRocketMags.Count().ToString() + "\n";
			meta += "  }\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void WriteProjectileCategoryCatalog(string category, array<ref TBD_ProjectileInfo> list, string destDir)
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
	protected static void WriteMasterProjectilesCatalog(string destDir, notnull TBD_AmmoCatalog catalog)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/projectiles", "projectiles_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition/projectiles", "projectiles_all_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f) return;

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"projectiles_all\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + catalog.m_aAllProjectiles.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"projectiles\": [\n", TAG);

		for (int i = 0; i < catalog.m_aAllProjectiles.Count(); i++)
		{
			string itemJson = catalog.m_aAllProjectiles[i].SerializeJson("    ");
			if (i < catalog.m_aAllProjectiles.Count() - 1)
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
			meta += "  \"totalCount\": " + catalog.m_aAllProjectiles.Count().ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"categories\": {\n";
			meta += "    \"projectiles_bullets\": " + catalog.m_aBulletProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_heavy\": " + catalog.m_aHeavyProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_grenades\": " + catalog.m_aGrenadeProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_rockets\": " + catalog.m_aRocketProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_mortar\": " + catalog.m_aMortarProjectiles.Count().ToString() + "\n";
			meta += "  }\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void WriteUnifiedMasterCatalog(string destDir, int elapsedMs, notnull TBD_AmmoCatalog catalog)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition", "ammunition_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "ammunition", "ammunition_all_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f) return;

		TBD_EquipmentExportJson.Write(f, "{\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"ammunition_all\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalMagazines\": " + catalog.m_aAllMags.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalProjectiles\": " + catalog.m_aAllProjectiles.Count().ToString() + ",\n", TAG);

		// Magazines
		TBD_EquipmentExportJson.Write(f, "  \"magazines\": [\n", TAG);
		for (int m = 0; m < catalog.m_aAllMags.Count(); m++)
		{
			string mJson = catalog.m_aAllMags[m].SerializeJson("    ");
			if (m < catalog.m_aAllMags.Count() - 1)
				mJson += ",\n";
			else
				mJson += "\n";
			TBD_EquipmentExportJson.Write(f, mJson, TAG);
		}
		TBD_EquipmentExportJson.Write(f, "  ],\n", TAG);

		// Projectiles
		TBD_EquipmentExportJson.Write(f, "  \"projectiles\": [\n", TAG);
		for (int p = 0; p < catalog.m_aAllProjectiles.Count(); p++)
		{
			string pJson = catalog.m_aAllProjectiles[p].SerializeJson("    ");
			if (p < catalog.m_aAllProjectiles.Count() - 1)
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
			meta += "  \"totalMagazines\": " + catalog.m_aAllMags.Count().ToString() + ",\n";
			meta += "  \"totalProjectiles\": " + catalog.m_aAllProjectiles.Count().ToString() + ",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"magazineCategories\": {\n";
			meta += "    \"magazines_rifle\": " + catalog.m_aRifleMags.Count().ToString() + ",\n";
			meta += "    \"magazines_mg\": " + catalog.m_aMgMags.Count().ToString() + ",\n";
			meta += "    \"magazines_handgun\": " + catalog.m_aHandgunMags.Count().ToString() + ",\n";
			meta += "    \"magazines_heavy\": " + catalog.m_aHeavyMags.Count().ToString() + ",\n";
			meta += "    \"magazines_grenades\": " + catalog.m_aGrenadeMags.Count().ToString() + ",\n";
			meta += "    \"magazines_rockets\": " + catalog.m_aRocketMags.Count().ToString() + "\n";
			meta += "  },\n";
			meta += "  \"projectileCategories\": {\n";
			meta += "    \"projectiles_bullets\": " + catalog.m_aBulletProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_heavy\": " + catalog.m_aHeavyProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_grenades\": " + catalog.m_aGrenadeProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_rockets\": " + catalog.m_aRocketProjectiles.Count().ToString() + ",\n";
			meta += "    \"projectiles_mortar\": " + catalog.m_aMortarProjectiles.Count().ToString() + "\n";
			meta += "  }\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		Print(string.Format("%1 Master ammunition catalog written: %2 magazines, %3 projectiles to %4",
			TAG, catalog.m_aAllMags.Count(), catalog.m_aAllProjectiles.Count(), filePath), LogLevel.NORMAL);
	}
}
