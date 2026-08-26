/**
 * TBD_MapExportLocations.c
 *
 * Extracts named places, towns, villages, and landmarks from World/Locations composition entities.
 */

class TBD_MapExportLocations
{
	protected static const string TAG = "[TBD][Locations]";
	protected static const float Y_MIN = -1000.0;
	protected static const float Y_MAX = 4000.0;

	protected ref array<IEntity> m_aHits;

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected string BasenameFromResource(string rn)
	{
		int slash = rn.LastIndexOf("/");
		int dot = rn.LastIndexOf(".et");
		if (slash < 0 || dot < 0 || dot <= slash)
			return "";
		return rn.Substring(slash + 1, dot - slash - 1);
	}

	//------------------------------------------------------------------------------------------------
	protected bool IsLocationPrefab(string rn)
	{
		return rn.Contains("Prefabs/World/Locations/");
	}

	//------------------------------------------------------------------------------------------------
	protected string DisplayNameFromBasename(string base)
	{
		if (base == "EntreDeux")
			return "Entre Deux";
		if (base == "Le_Moule")
			return "Le Moule";
		if (base == "Villeneuf")
			return "Villeneuve";
		if (base == "StPhilippe_StPhilippe_01")
			return "Saint Philippe";
		base.Replace("_", " ");
		return base;
	}

	//------------------------------------------------------------------------------------------------
	protected float ResolveImportance(string name)
	{
		if (name == "Montignac") return 0.85;
		if (name == "Saint Philippe") return 0.78;
		if (name == "Levie") return 0.74;
		if (name == "Chotain") return 0.72;
		if (name == "Morton") return 0.70;
		if (name == "Gorey") return 0.62;
		if (name == "Kermovan") return 0.58;
		if (name == "Raccoon Rock") return 0.52;
		if (name == "Highstone") return 0.48;
		return 0.55;
	}

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		float worldSize = ctx.m_fWorldSize;
		string outJson = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_LocationsExport.json");
		string outMeta = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_LocationsExport_meta.json");

		m_aHits = {};
		vector mins = Vector(0, Y_MIN, 0);
		vector maxs = Vector(worldSize, Y_MAX, worldSize);
		ctx.m_World.QueryEntitiesByAABB(mins, maxs, CollectEntity);

		Print(string.Format("%1 Query hit %2 candidate entities -> %3", TAG, m_aHits.Count(), outJson));

		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Cannot open " + outJson + " for write", LogLevel.ERROR);
			return false;
		}

		TBD_MapExportJson.Write(f, "[\n", TAG);
		int written = 0;
		bool first = true;

		foreach (IEntity e : m_aHits)
		{
			string rn = ctx.ResolvePrefab(e);
			if (!IsLocationPrefab(rn))
				continue;

			string base = BasenameFromResource(rn);
			if (base.IsEmpty())
				continue;

			// Direct town compositions or specific named locs
			bool keep = !rn.Contains("/Urban/") && !rn.Contains("/Natural/") && !rn.Contains("/Aquatic/");
			if (rn.Contains("StPhilippe_StPhilippe_01.et"))
				keep = true;

			if (!keep)
				continue;

			string name = DisplayNameFromBasename(base);
			if (name.IsEmpty() || name.Length() < 2 || name.Contains("Location composition"))
				continue;

			vector pos = e.GetOrigin();
			string slug = name;
			slug.ToLower();
			slug.Replace(" ", "-");
			float importance = ResolveImportance(name);

			string row = string.Format(
				"  {\"id\":\"%1\",\"name\":\"%2\",\"x\":%3,\"y\":%4,\"importance\":%5}",
				TBD_MapExportJson.Escape(slug),
				TBD_MapExportJson.Escape(name),
				pos[0].ToString(),
				pos[2].ToString(),
				importance.ToString());

			if (!first)
				TBD_MapExportJson.Write(f, ",\n", TAG);
			first = false;
			TBD_MapExportJson.Write(f, row, TAG);
			written++;
		}

		TBD_MapExportJson.Write(f, "\n]\n", TAG);
		f.Close();

		// Write meta
		FileHandle fm = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (fm)
		{
			string meta = string.Format(
				"{\n  \"written\": %1,\n  \"source\": \"TBD_MapExportLocations\",\n  \"worldSizeM\": %2\n}\n",
				written, worldSize.ToString());
			TBD_MapExportJson.Write(fm, meta, TAG);
			fm.Close();
		}

		Print(string.Format("%1 DONE — %2 locations exported to %3", TAG, written, outJson));
		return true;
	}
}
