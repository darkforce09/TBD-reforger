/**
 * TBD_TerrainWorldExportPlugin.c - T-090.3.0 Workbench world-export feasibility spike (subregion only).
 *
 * Proves the engine can enumerate placed world objects (buildings/trees/props) over a ~512 m Everon
 * subregion, read each prefab ResourceName + transform + bounds, and write a raw-entities JSONL the
 * spike verifiers consume (K1/K1b). Full-map export stays blocked until T-090.3 (P1).
 *
 * KEY FINDING (why this is not the terrain-plugin flat loop): WorldEditorAPI.GetEditorEntityCount()/
 * GetEditorEntity(i) only returns TOP-LEVEL editor entities (EditorEntityIterator "Skips all entities
 * that are not top-level") — on Everon that is ~10 roots (world, Eden, SCR_MapEntity1, ...), NOT the
 * 1,235,873 placed objects wb_state reports. The placed objects are reached via a runtime spatial query:
 *   BaseWorld.QueryEntitiesByAABB(mins, maxs, callback)   (api_search-confirmed @ S1)
 * The BaseWorld handle comes from any top-level runtime entity's GetWorld(). Each hit resolves:
 *   - transform: IEntity.GetOrigin() + IEntity.GetAngles()        (EMCP handlers + api_search)
 *   - bounds:    IEntity.GetWorldBounds(out min, out max)         (api_search @ S1 — real world AABB → S2)
 *   - prefab:    WorldEditorAPI.EntityToSource(e).GetAncestor().GetResourceName()  (EMCP Prefabs getAncestor)
 *
 * One run, no manual bbox guessing: scan a 5x5 grid of 512 m cells across the map, auto-pick the cell
 * with the most building-ish prefabs, then export that cell. Writes to $profile:
 *   TBD_WorldExport_subregion.jsonl  (one entity per line)
 *   TBD_WorldExport_meta.json        (chosen bbox, counts, scan grid, obb availability)
 *
 * Run: Workbench > Plugins > TBD > "Export TBD World Subregion"
 *   (or NetAPI wb_execute_action menuPath "Plugins,TBD,Export TBD World Subregion").
 */

[WorkbenchPluginAttribute(name: "Export TBD World Subregion", description: "Spatial-query the densest building 512 m cell; write raw-entities JSONL (prefab + transform + world-AABB).", category: "TBD")]
class TBD_TerrainWorldExportPlugin : WorkbenchPlugin
{
	protected static const float CELL_HALF = 256.0;   // 512 m cell
	protected static const float Y_MIN     = -1000.0; // AABB vertical span (covers Everon -204..375 m)
	protected static const float Y_MAX     = 2000.0;

	protected static const string OUT_JSONL = "$profile:TBD_WorldExport_subregion.jsonl";
	protected static const string OUT_META  = "$profile:TBD_WorldExport_meta.json";

	protected ref array<IEntity> m_aHits;

	//------------------------------------------------------------------------------------------------
	//! QueryEntitiesByAABB callback — collect every entity touched by the box. Return true to continue.
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected bool LooksLikeBuilding(string rn)
	{
		return rn.Contains("House") || rn.Contains("Village") || rn.Contains("Building")
			|| rn.Contains("Residential") || rn.Contains("Barn") || rn.Contains("Shed")
			|| rn.Contains("Church") || rn.Contains("Hangar") || rn.Contains("Garage") || rn.Contains("Shop");
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
	//! Query one cell (centre cx,cz); fills m_aHits; returns total count, out building-ish count.
	protected int QueryCell(BaseWorld world, WorldEditorAPI api, float cx, float cz, out int buildingish)
	{
		m_aHits = {};
		vector mins = Vector(cx - CELL_HALF, Y_MIN, cz - CELL_HALF);
		vector maxs = Vector(cx + CELL_HALF, Y_MAX, cz + CELL_HALF);
		world.QueryEntitiesByAABB(mins, maxs, CollectEntity);

		buildingish = 0;
		foreach (IEntity e : m_aHits)
		{
			if (LooksLikeBuilding(ResolvePrefab(api, e)))
				buildingish++;
		}
		return m_aHits.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected string JsonEscape(string s)
	{
		s.Replace("\\", "\\\\");
		s.Replace("\"", "\\\"");
		return s;
	}

	//------------------------------------------------------------------------------------------------
	//! Export the cell centred on (cx,cz): re-query, write JSONL rows for entities whose ORIGIN is inside.
	protected void ExportCell(BaseWorld world, WorldEditorAPI api, float cx, float cz, string scanSummary)
	{
		int b;
		int hit = QueryCell(world, api, cx, cz, b);
		float minX = cx - CELL_HALF;
		float minZ = cz - CELL_HALF;
		float maxX = cx + CELL_HALF;
		float maxZ = cz + CELL_HALF;
		Print(string.Format("[TBD][World] export cell centre (%1,%2) hit %3 (buildingish=%4)", cx, cz, hit, b));

		FileHandle f = FileIO.OpenFile(OUT_JSONL, FileMode.WRITE);
		if (!f)
		{
			Print("[TBD][World] cannot open " + OUT_JSONL, LogLevel.ERROR);
			return;
		}

		int written = 0;
		int withPrefab = 0;
		string buf = "";
		foreach (IEntity e : m_aHits)
		{
			vector pos = e.GetOrigin();
			if (pos[0] < minX || pos[0] > maxX || pos[2] < minZ || pos[2] > maxZ)
				continue;

			vector ang = e.GetAngles();
			vector bmin;
			vector bmax;
			e.GetWorldBounds(bmin, bmax);
			float hx = (bmax[0] - bmin[0]) * 0.5;
			float hy = (bmax[1] - bmin[1]) * 0.5;
			float hz = (bmax[2] - bmin[2]) * 0.5;

			string rn = ResolvePrefab(api, e);
			if (rn != "")
				withPrefab++;

			string row = "{";
			row += "\"resourceName\":\"" + JsonEscape(rn) + "\",";
			row += "\"className\":\"" + JsonEscape(e.ClassName()) + "\",";
			row += "\"x\":" + pos[0].ToString() + ",";
			row += "\"y\":" + pos[1].ToString() + ",";
			row += "\"z\":" + pos[2].ToString() + ",";
			row += "\"yawDeg\":" + ang[0].ToString() + ",";
			row += "\"pitchDeg\":" + ang[1].ToString() + ",";
			row += "\"rollDeg\":" + ang[2].ToString() + ",";
			row += "\"halfExtentsM\":[" + hx.ToString() + "," + hy.ToString() + "," + hz.ToString() + "]";
			row += "}\n";
			buf += row;
			written++;
			if (buf.Length() > 8000)
			{
				f.Write(buf);
				buf = "";
			}
		}
		if (buf.Length() > 0)
			f.Write(buf);
		f.Close();
		Print(string.Format("[TBD][World] wrote %1 rows (withPrefab=%2) to %3", written, withPrefab, OUT_JSONL));

		FileHandle mh = FileIO.OpenFile(OUT_META, FileMode.WRITE);
		if (mh)
		{
			string mj = "{\n";
			mj += "  \"subregionBBoxM\": [" + minX.ToString() + "," + minZ.ToString() + "," + maxX.ToString() + "," + maxZ.ToString() + "],\n";
			mj += "  \"cellCentreM\": [" + cx.ToString() + "," + cz.ToString() + "],\n";
			mj += "  \"aabbHitCount\": " + hit.ToString() + ",\n";
			mj += "  \"keptCount\": " + written.ToString() + ",\n";
			mj += "  \"withPrefab\": " + withPrefab.ToString() + ",\n";
			mj += "  \"buildingish\": " + b.ToString() + ",\n";
			mj += "  \"obbApiAvailable\": true,\n";
			mj += "  \"obbModel\": \"world-aabb via IEntity.GetWorldBounds; yaw via GetAngles\",\n";
			mj += "  \"anglesOrderNote\": \"GetAngles() vector component order measured in S6\",\n";
			mj += "  \"scanGrid\": [" + scanSummary + "]\n";
			mj += "}\n";
			mh.Write(mj);
			mh.Close();
		}
		Print("[TBD][World] DONE export");
	}

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		WorldEditor we = Workbench.GetModule(WorldEditor);
		if (!we)
		{
			Print("[TBD][World] WorldEditor module not available", LogLevel.ERROR);
			return;
		}
		WorldEditorAPI api = we.GetApi();
		if (!api)
		{
			Print("[TBD][World] WorldEditorAPI not available", LogLevel.ERROR);
			return;
		}

		BaseWorld world = ResolveWorld(api);
		if (!world)
		{
			Print("[TBD][World] could not resolve BaseWorld from top-level entities", LogLevel.ERROR);
			return;
		}

		// Scan a 5x5 grid of 512 m cells across the 12.8 km map; pick the densest building cell.
		array<float> centers = { 1280.0, 3840.0, 6400.0, 8960.0, 11520.0 };
		float bestCx = centers[2];
		float bestCz = centers[2];
		int bestB = -1;
		string scanSummary = "";
		bool first = true;
		Print("[TBD][World][SCAN] 5x5 grid (centreX, centreZ -> total / buildingish)");
		for (int iz = 0; iz < centers.Count(); iz++)
		{
			for (int ix = 0; ix < centers.Count(); ix++)
			{
				float cx = centers[ix];
				float cz = centers[iz];
				int b;
				int total = QueryCell(world, api, cx, cz, b);
				Print(string.Format("[TBD][World][SCAN] (%1,%2) -> %3 / %4", cx, cz, total, b));
				if (!first)
					scanSummary += ",";
				scanSummary += string.Format("{\"cx\":%1,\"cz\":%2,\"total\":%3,\"buildingish\":%4}", cx, cz, total, b);
				first = false;
				if (b > bestB)
				{
					bestB = b;
					bestCx = cx;
					bestCz = cz;
				}
			}
		}
		Print(string.Format("[TBD][World][SCAN] best cell (%1,%2) buildingish=%3", bestCx, bestCz, bestB));

		ExportCell(world, api, bestCx, bestCz, scanSummary);
	}
}
