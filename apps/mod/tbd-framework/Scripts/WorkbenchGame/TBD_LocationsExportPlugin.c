/**
 * TBD_LocationsExportPlugin.c - T-152.6 Workbench locations export (Path A).
 *
 * Enumerates World/Locations/* composition entities over the full Everon AABB via
 * BaseWorld.QueryEntitiesByAABB (same spine as TBD_TerrainWorldExportPlugin.c).
 * Resolves display names from prefab basename (Morton.et → "Morton") — never the
 * generic "Location composition" class label.
 *
 * Writes:
 *   $profile:TBD_LocationsExport.json   (locations.schema.json array)
 *   $profile:TBD_LocationsExport_meta.json
 *
 * Run: Workbench > Plugins > TBD > "Export TBD Locations"
 * Then: node scripts/map-assets/copy-locations-export-profile.mjs TERRAIN=everon
 *   (or node scripts/map-assets/export-locations.mjs after staging raw-entities — Path B)
 */
[WorkbenchPluginAttribute(name: "Export TBD Locations", description: "Export World/Locations named places to JSON (T-152.6).", category: "TBD")]
class TBD_LocationsExportPlugin : WorkbenchPlugin
{
	protected static const float Y_MIN = -1000.0;
	protected static const float Y_MAX =  4000.0;
	protected static const float WORLD_SIZE = 12800.0;
	protected static const string OUT_JSON = "$profile:TBD_LocationsExport.json";
	protected static const string OUT_META = "$profile:TBD_LocationsExport_meta.json";
	protected static const string TAG = "[TBD][Locations]";

	protected ref array<IEntity> m_aHits;

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected BaseWorld ResolveWorld(WorldEditorAPI api)
	{
		int rootCount = api.GetEditorEntityCount();
		for (int i = 0; i < rootCount; i++)
		{
			IEntitySource s = api.GetEditorEntity(i);
			if (!s)
				continue;
			IEntity re = api.SourceToEntity(s);
			if (re)
			{
				BaseWorld w = re.GetWorld();
				if (w)
					return w;
			}
		}
		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected string ResolvePrefab(WorldEditorAPI api, IEntity e)
	{
		IEntitySource src = api.EntityToSource(e);
		if (!src)
			return "";
		BaseContainer anc = src.GetAncestor();
		if (!anc)
			return "";
		return anc.GetResourceName();
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
	protected bool IsDirectTownPrefab(string rn)
	{
		// Eden/{Name}.et — no subfolder between Eden/ and basename
		return rn.Contains("Prefabs/World/Locations/Eden/") && !rn.Contains("/Urban/") && !rn.Contains("/Natural/") && !rn.Contains("/Aquatic/");
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
	protected string SlugId(string terrainId, string name)
	{
		string s = name;
		s.ToLower();
		return terrainId + "-" + s;
	}

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		WorldEditorAPI api = Workbench.GetModule(WorldEditorAPI);
		if (!api)
		{
			Print(TAG + " WorldEditorAPI unavailable", LogLevel.ERROR);
			return;
		}

		BaseWorld world = ResolveWorld(api);
		if (!world)
		{
			Print(TAG + " no runtime world — open a populated Everon world first", LogLevel.ERROR);
			return;
		}

		m_aHits = {};
		vector mins = Vector(0, Y_MIN, 0);
		vector maxs = Vector(WORLD_SIZE, Y_MAX, WORLD_SIZE);
		world.QueryEntitiesByAABB(mins, maxs, CollectEntity);
		Print(TAG + string.Format(" AABB query hit %1 entities", m_aHits.Count()));

		FileHandle f = FileIO.OpenFile(OUT_JSON, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " cannot open " + OUT_JSON, LogLevel.ERROR);
			return;
		}

		int written = 0;
		TBD_ExportJson.Write(f, "[\n", TAG);
		bool first = true;

		foreach (IEntity e : m_aHits)
		{
			string rn = ResolvePrefab(api, e);
			if (!IsLocationPrefab(rn))
				continue;

			string base = BasenameFromResource(rn);
			if (base.IsEmpty())
				continue;

			bool keep = IsDirectTownPrefab(rn) || rn.Contains("StPhilippe_StPhilippe_01.et");
			if (!keep)
				continue;

			string name = DisplayNameFromBasename(base);
			if (name.IsEmpty() || name.Length() < 2)
				continue;
			if (name.Contains("Location composition"))
				continue;

			vector pos = e.GetOrigin();
			string id = SlugId("everon", name);
			string row = string.Format(
				"  {\"id\":\"%1\",\"name\":\"%2\",\"x\":%3,\"y\":%4,\"importance\":0.55}",
				TBD_ExportJson.Escape(id),
				TBD_ExportJson.Escape(name),
				pos[0].ToString(),
				pos[2].ToString());

			if (!first)
				TBD_ExportJson.Write(f, ",\n", TAG);
			first = false;
			TBD_ExportJson.Write(f, row, TAG);
			written++;
		}

		TBD_ExportJson.Write(f, "\n]\n", TAG);
		f.Close();

		string meta = string.Format(
			"{\"terrainId\":\"everon\",\"written\":%1,\"source\":\"TBD_LocationsExportPlugin\",\"exportedAt\":\"%2\"}\n",
			written,
			"2026-07-13T00:00:00Z");
		FileHandle fm = FileIO.OpenFile(OUT_META, FileMode.WRITE);
		if (fm)
		{
			TBD_ExportJson.Write(fm, meta, TAG);
			fm.Close();
		}

		Print(TAG + string.Format(" DONE — %1 location rows → %2", written, OUT_JSON));
	}
}
