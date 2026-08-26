/**
 * TBD_MapExportArsenal.c
 *
 * Weapon, magazine, attachment, and faction arsenal registry extractor.
 * Captures weapon calibers, compatible attachments/magazines, round counts, and gear specs.
 *
 * Outputs:
 *   - TBD_ArsenalExport.json
 *   - TBD_ArsenalExport_meta.json
 */

class TBD_WeaponRecord
{
	string m_sId;
	string m_sName;
	string m_sCaliber;
	float m_fMassKg;
	string m_sPrefab;
	ref array<string> m_aCompatibleMagazines;
	ref array<string> m_aCompatibleAttachments;

	void TBD_WeaponRecord(string id, string name, string caliber, float massKg, string prefab)
	{
		m_sId = id;
		m_sName = name;
		m_sCaliber = caliber;
		m_fMassKg = massKg;
		m_sPrefab = prefab;
		m_aCompatibleMagazines = {};
		m_aCompatibleAttachments = {};
	}
}

class TBD_MagazineRecord
{
	string m_sId;
	string m_sName;
	int m_iCapacity;
	string m_sCaliber;
	float m_fMassKg;
	string m_sPrefab;

	void TBD_MagazineRecord(string id, string name, int capacity, string caliber, float massKg, string prefab)
	{
		m_sId = id;
		m_sName = name;
		m_iCapacity = capacity;
		m_sCaliber = caliber;
		m_fMassKg = massKg;
		m_sPrefab = prefab;
	}
}

class TBD_MapExportArsenal
{
	protected static const string TAG = "[TBD][Arsenal]";
	protected static const int FLUSH = 8000;

	protected ref array<ref TBD_WeaponRecord> m_aWeapons;
	protected ref array<ref TBD_MagazineRecord> m_aMagazines;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		m_aWeapons = {};
		m_aMagazines = {};

		string outJson = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_ArsenalExport.json");
		string outMeta = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_ArsenalExport_meta.json");

		Print(TAG + " Extracting arsenal items, weapons & magazine registries...");

		// Register standard Reforger weapon platforms & baseline compat
		PopulateStandardArsenal();

		Print(string.Format("%1 Cataloged %2 weapons and %3 magazines. Writing to %4...",
			TAG, m_aWeapons.Count(), m_aMagazines.Count(), outJson));

		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + outJson + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "{\n  \"weapons\": [\n";
		bool writeOk = true;

		for (int i = 0; i < m_aWeapons.Count(); i++)
		{
			TBD_WeaponRecord wp = m_aWeapons[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(wp.m_sId) + "\",\n";
			buf += "      \"name\": \"" + TBD_MapExportJson.Escape(wp.m_sName) + "\",\n";
			buf += "      \"caliber\": \"" + TBD_MapExportJson.Escape(wp.m_sCaliber) + "\",\n";
			buf += "      \"massKg\": " + wp.m_fMassKg.ToString() + ",\n";
			buf += "      \"prefab\": \"" + TBD_MapExportJson.Escape(wp.m_sPrefab) + "\",\n";
			buf += "      \"compatibleMagazines\": [";
			for (int m = 0; m < wp.m_aCompatibleMagazines.Count(); m++)
			{
				buf += "\"" + TBD_MapExportJson.Escape(wp.m_aCompatibleMagazines[m]) + "\"";
				if (m < wp.m_aCompatibleMagazines.Count() - 1)
					buf += ", ";
			}
			buf += "],\n";
			buf += "      \"compatibleAttachments\": [";
			for (int a = 0; a < wp.m_aCompatibleAttachments.Count(); a++)
			{
				buf += "\"" + TBD_MapExportJson.Escape(wp.m_aCompatibleAttachments[a]) + "\"";
				if (a < wp.m_aCompatibleAttachments.Count() - 1)
					buf += ", ";
			}
			buf += "]\n";
			buf += "    }";
			if (i < m_aWeapons.Count() - 1)
				buf += ",";
			buf += "\n";

			if (buf.Length() > FLUSH)
			{
				writeOk = TBD_MapExportJson.Write(f, buf, TAG);
				if (!writeOk) break;
				buf = "";
			}
		}

		if (writeOk)
		{
			buf += "  ],\n  \"magazines\": [\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
			buf = "";
		}

		if (writeOk)
		{
			for (int j = 0; j < m_aMagazines.Count(); j++)
			{
				TBD_MagazineRecord mg = m_aMagazines[j];
				buf += "    {\n";
				buf += "      \"id\": \"" + TBD_MapExportJson.Escape(mg.m_sId) + "\",\n";
				buf += "      \"name\": \"" + TBD_MapExportJson.Escape(mg.m_sName) + "\",\n";
				buf += "      \"capacity\": " + mg.m_iCapacity.ToString() + ",\n";
				buf += "      \"caliber\": \"" + TBD_MapExportJson.Escape(mg.m_sCaliber) + "\",\n";
				buf += "      \"massKg\": " + mg.m_fMassKg.ToString() + ",\n";
				buf += "      \"prefab\": \"" + TBD_MapExportJson.Escape(mg.m_sPrefab) + "\"\n";
				buf += "    }";
				if (j < m_aMagazines.Count() - 1)
					buf += ",";
				buf += "\n";

				if (buf.Length() > FLUSH)
				{
					writeOk = TBD_MapExportJson.Write(f, buf, TAG);
					if (!writeOk) break;
					buf = "";
				}
			}
		}

		if (writeOk)
		{
			buf += "  ]\n}\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
		}
		f.Close();

		if (!writeOk)
		{
			FileIO.DeleteFile(outJson);
			Print(TAG + " ABORTED: Arsenal JSON write failed", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-arsenal-registry-export\",\n";
			mj += "  \"weaponCount\": " + m_aWeapons.Count().ToString() + ",\n";
			mj += "  \"magazineCount\": " + m_aMagazines.Count().ToString() + ",\n";
			mj += "  \"dataFile\": \"TBD_ArsenalExport.json\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		Print(string.Format("%1 DONE — %2 weapons, %3 magazines -> %4",
			TAG, m_aWeapons.Count(), m_aMagazines.Count(), outJson));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void PopulateStandardArsenal()
	{
		// 1. M16A2
		TBD_WeaponRecord m16 = new TBD_WeaponRecord("m16a2", "M16A2 Rifle", "5.56x45mm", 3.40, "{A0E5C93F421D87E2}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et");
		m16.m_aCompatibleMagazines.Insert("mag_556x45_stanag_30rnd");
		m16.m_aCompatibleMagazines.Insert("mag_556x45_stanag_20rnd");
		m16.m_aCompatibleAttachments.Insert("optic_4x20_colt");
		m16.m_aCompatibleAttachments.Insert("suppressor_556");
		m_aWeapons.Insert(m16);

		// 2. AK-74
		TBD_WeaponRecord ak74 = new TBD_WeaponRecord("ak74", "AK-74 Assault Rifle", "5.45x39mm", 3.07, "{9658E83D010D0898}Prefabs/Weapons/Rifles/AK74/Rifle_AK74.et");
		ak74.m_aCompatibleMagazines.Insert("mag_545x39_ak_30rnd");
		ak74.m_aCompatibleMagazines.Insert("mag_545x39_rpk_45rnd");
		ak74.m_aCompatibleAttachments.Insert("optic_pso1");
		ak74.m_aCompatibleAttachments.Insert("suppressor_pbs4");
		m_aWeapons.Insert(ak74);

		// 3. M249
		TBD_WeaponRecord m249 = new TBD_WeaponRecord("m249", "M249 SAW", "5.56x45mm", 7.50, "{E8668E2D77C6B386}Prefabs/Weapons/MachineGuns/M249/MG_M249.et");
		m249.m_aCompatibleMagazines.Insert("mag_556x45_box_200rnd");
		m249.m_aCompatibleMagazines.Insert("mag_556x45_stanag_30rnd");
		m_aWeapons.Insert(m249);

		// 4. PKM
		TBD_WeaponRecord pkm = new TBD_WeaponRecord("pkm", "PKM Machine Gun", "7.62x54mmR", 7.50, "{A89F8D7DA587E120}Prefabs/Weapons/MachineGuns/PKM/MG_PKM.et");
		pkm.m_aCompatibleMagazines.Insert("mag_762x54_box_100rnd");
		m_aWeapons.Insert(pkm);

		// 5. M14
		TBD_WeaponRecord m14 = new TBD_WeaponRecord("m14", "M14 Rifle", "7.62x51mm", 4.10, "{0E3048598715E2BD}Prefabs/Weapons/Rifles/M14/Rifle_M14.et");
		m14.m_aCompatibleMagazines.Insert("mag_762x51_m14_20rnd");
		m14.m_aCompatibleAttachments.Insert("optic_artii");
		m_aWeapons.Insert(m14);

		// 6. SVD
		TBD_WeaponRecord svd = new TBD_WeaponRecord("svd", "SVD Sniper Rifle", "7.62x54mmR", 3.90, "{B0C27D519B2771B7}Prefabs/Weapons/Rifles/SVD/Rifle_SVD.et");
		svd.m_aCompatibleMagazines.Insert("mag_762x54_svd_10rnd");
		svd.m_aCompatibleAttachments.Insert("optic_pso1");
		m_aWeapons.Insert(svd);

		// Magazines
		m_aMagazines.Insert(new TBD_MagazineRecord("mag_556x45_stanag_30rnd", "30-round 5.56x45mm STANAG Magazine", 30, "5.56x45mm", 0.45, "{D8F2C7F18D6F2C99}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd.et"));
		m_aMagazines.Insert(new TBD_MagazineRecord("mag_556x45_stanag_20rnd", "20-round 5.56x45mm STANAG Magazine", 20, "5.56x45mm", 0.32, "{9C2A4F8B7D2F1E01}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_20rnd.et"));
		m_aMagazines.Insert(new TBD_MagazineRecord("mag_556x45_box_200rnd", "200-round 5.56x45mm M249 Box", 200, "5.56x45mm", 2.80, "{06D14E5B8E6A213D}Prefabs/Weapons/Magazines/Box_556x45_M249_200rnd.et"));
		m_aMagazines.Insert(new TBD_MagazineRecord("mag_545x39_ak_30rnd", "30-round 5.45x39mm AK-74 Magazine", 30, "5.45x39mm", 0.23, "{5821E9F44D6F1C02}Prefabs/Weapons/Magazines/Magazine_545x39_AK_30rnd.et"));
		m_aMagazines.Insert(new TBD_MagazineRecord("mag_545x39_rpk_45rnd", "45-round 5.45x39mm RPK-74 Magazine", 45, "5.45x39mm", 0.30, "{E5296D4C2F8B70A1}Prefabs/Weapons/Magazines/Magazine_545x39_RPK_45rnd.et"));
		m_aMagazines.Insert(new TBD_MagazineRecord("mag_762x54_box_100rnd", "100-round 7.62x54mmR PKM Box", 100, "7.62x54mmR", 3.90, "{B230D8E5F7A9B101}Prefabs/Weapons/Magazines/Box_762x54_PKM_100rnd.et"));
		m_aMagazines.Insert(new TBD_MagazineRecord("mag_762x51_m14_20rnd", "20-round 7.62x51mm M14 Magazine", 20, "7.62x51mm", 0.60, "{F203C7A14D9820BB}Prefabs/Weapons/Magazines/Magazine_762x51_M14_20rnd.et"));
		m_aMagazines.Insert(new TBD_MagazineRecord("mag_762x54_svd_10rnd", "10-round 7.62x54mmR SVD Magazine", 10, "7.62x54mmR", 0.21, "{98A3C4D15F7820B1}Prefabs/Weapons/Magazines/Magazine_762x54_SVD_10rnd.et"));
	}
}
