/**
 * TBD_BlueprintReconPlugin.c
 *
 * Phase-B recon: dumps the REAL child-entity tree of one building instance so the extractor's
 * detection rules come from observed data, not name guesses. For the first world instance whose
 * prefab resource contains the configured filter, walks the runtime hierarchy to full depth and
 * records per entity: name, class, prefab resource, world/root-relative position, bounds size,
 * and the component class list (IEntitySource level -- the EMCP_WB_Components idiom).
 *
 * Output: $profile:TBD_Export/<map>/prefabs/debug/<slug>_children.json
 * Menu:   Workbench > Plugins > TBD > "Recon Building Children"
 * Remote: NetApiHandler `EMCP_WB_TbdBlueprint` action "recon" (agent loop -- menu ExecuteAction
 *         is unreliable in this Workbench build, the Net API path is not).
 */

//! Shared recon core -- callable from the menu plugin AND the Net API handler.
class TBD_BlueprintRecon
{
	protected static const string TAG = "[TBD][Recon]";

	string m_sPrefabFilter;
	int m_iMaxEntities;

	protected ref array<IEntity> m_aHits;
	protected int m_iRecorded;
	protected string m_sJson;

	//------------------------------------------------------------------------------------------------
	//! Runs the dump; returns a one-line summary ("OK <count> <path>" or "ERROR: ...").
	static string Execute(string prefabFilter, int maxEntities)
	{
		TBD_BlueprintRecon r = new TBD_BlueprintRecon();
		r.m_sPrefabFilter = prefabFilter;
		r.m_iMaxEntities = maxEntities;
		return r.RunDump();
	}

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	string RunDump()
	{
		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
			return "ERROR: context init failed";

		IEntity root = FindTarget(ctx);
		if (!root)
			return "ERROR: no instance matching '" + m_sPrefabFilter + "' in the open world";

		string resName = ctx.ResolvePrefab(root);
		string slug = TBD_BuildingArchitectExtractor.DerivePrefabSlug(resName);
		Print(TAG + " target: " + slug + " @ " + root.GetOrigin().ToString() + " (" + resName + ")");

		m_iRecorded = 0;
		m_sJson = "{\n";
		m_sJson += "  \"prefabFilter\": \"" + TBD_MapExportJson.Escape(m_sPrefabFilter) + "\",\n";
		m_sJson += "  \"resourceName\": \"" + TBD_MapExportJson.Escape(resName) + "\",\n";
		m_sJson += "  \"slug\": \"" + TBD_MapExportJson.Escape(slug) + "\",\n";
		vector rootPos = root.GetOrigin();
		vector rootAng = root.GetAngles();
		m_sJson += "  \"rootWorldPos\": [" + rootPos[0].ToString() + "," + rootPos[1].ToString() + "," + rootPos[2].ToString() + "],\n";
		m_sJson += "  \"rootAngles\": [" + rootAng[0].ToString() + "," + rootAng[1].ToString() + "," + rootAng[2].ToString() + "],\n";
		vector bMin, bMax;
		root.GetBounds(bMin, bMax);
		m_sJson += "  \"rootBoundsMin\": [" + bMin[0].ToString() + "," + bMin[1].ToString() + "," + bMin[2].ToString() + "],\n";
		m_sJson += "  \"rootBoundsMax\": [" + bMax[0].ToString() + "," + bMax[1].ToString() + "," + bMax[2].ToString() + "],\n";
		m_sJson += "  \"rootComponents\": " + ComponentsJson(root) + ",\n";
		m_sJson += "  \"children\": [\n";

		bool first = true;
		IEntity child = root.GetChildren();
		while (child)
		{
			DumpRecursive(root, child, 1, first);
			first = false;
			child = child.GetSibling();
		}

		m_sJson += "\n  ]\n}\n";

		string mapName = ctx.GetMapName(null);
		TBD_MapExportConfig cfg = new TBD_MapExportConfig();
		string outPath = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "prefabs/debug", slug + "_children.json");
		FileHandle f = FileIO.OpenFile(outPath, FileMode.WRITE);
		if (!f)
			return "ERROR: cannot open " + outPath;
		TBD_MapExportJson.Write(f, m_sJson, TAG);
		f.Close();

		string summary = string.Format("OK %1 entities -> %2", m_iRecorded, outPath);
		Print(TAG + " DONE -- " + summary);
		return summary;
	}

	//------------------------------------------------------------------------------------------------
	//! First world entity whose resolved prefab contains the filter.
	protected IEntity FindTarget(TBD_MapExportContext ctx)
	{
		float worldSize = ctx.m_fWorldSize;
		float cellM = 512.0;
		int cells = Math.Ceil(worldSize / cellM);

		for (int iz = 0; iz < cells; iz++)
		{
			for (int ix = 0; ix < cells; ix++)
			{
				m_aHits = {};
				vector mins = Vector(ix * cellM, -1000.0, iz * cellM);
				vector maxs = Vector(ix * cellM + cellM, 2000.0, iz * cellM + cellM);
				ctx.m_World.QueryEntitiesByAABB(mins, maxs, CollectEntity);
				foreach (IEntity e : m_aHits)
				{
					string rn = ctx.ResolvePrefab(e);
					if (!rn.IsEmpty() && rn.Contains(m_sPrefabFilter))
						return e;
				}
			}
		}
		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected void DumpRecursive(IEntity root, IEntity ent, int depth, bool first)
	{
		if (!ent || m_iRecorded >= m_iMaxEntities)
			return;

		if (!first)
			m_sJson += ",\n";

		vector rel = ent.GetOrigin() - root.GetOrigin();
		vector wpos = ent.GetOrigin();
		vector ang = ent.GetAngles();
		vector cMin, cMax;
		ent.GetBounds(cMin, cMax);

		string resName = TBD_MapExportContext.GetEntityResourceName(ent);

		m_sJson += "    {";
		m_sJson += "\"depth\":" + depth.ToString() + ",";
		m_sJson += "\"name\":\"" + TBD_MapExportJson.Escape(ent.GetName()) + "\",";
		m_sJson += "\"class\":\"" + TBD_MapExportJson.Escape(ent.ClassName()) + "\",";
		m_sJson += "\"resource\":\"" + TBD_MapExportJson.Escape(resName) + "\",";
		m_sJson += "\"relPos\":[" + rel[0].ToString() + "," + rel[1].ToString() + "," + rel[2].ToString() + "],";
		m_sJson += "\"worldPos\":[" + wpos[0].ToString() + "," + wpos[1].ToString() + "," + wpos[2].ToString() + "],";
		m_sJson += "\"yawDeg\":" + ang[1].ToString() + ",";
		m_sJson += "\"size\":[" + (cMax[0] - cMin[0]).ToString() + "," + (cMax[1] - cMin[1]).ToString() + "," + (cMax[2] - cMin[2]).ToString() + "],";
		m_sJson += "\"boundsMinY\":" + cMin[1].ToString() + ",";
		m_sJson += "\"components\":" + ComponentsJson(ent);
		m_sJson += "}";
		m_iRecorded++;

		IEntity child = ent.GetChildren();
		while (child)
		{
			DumpRecursive(root, child, depth + 1, false);
			child = child.GetSibling();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Component class list via the entity's SOURCE (the EMCP_WB_Components idiom).
	protected string ComponentsJson(IEntity ent)
	{
		string json = "[";
		WorldEditor we = Workbench.GetModule(WorldEditor);
		if (we)
		{
			WorldEditorAPI api = we.GetApi();
			if (api)
			{
				IEntitySource src = api.EntityToSource(ent);
				if (src)
				{
					int n = src.GetComponentCount();
					for (int i = 0; i < n; i++)
					{
						IEntityComponentSource comp = src.GetComponent(i);
						if (!comp)
							continue;
						if (json != "[")
							json += ",";
						json += "\"" + TBD_MapExportJson.Escape(comp.GetClassName()) + "\"";
					}
				}
			}
		}
		json += "]";
		return json;
	}
}

[WorkbenchPluginAttribute(
	name: "Recon Building Children",
	description: "Dump the real child tree + components of the first building instance matching the prefab filter.",
	category: "TBD"
)]
class TBD_BlueprintReconPlugin : WorkbenchPlugin
{
	[Attribute("FarmHouse_E_1L01", UIWidgets.EditBox, desc: "Prefab resource substring to match")]
	string m_sPrefabFilter;

	[Attribute("512", UIWidgets.EditBox, desc: "Max entities to record")]
	int m_iMaxEntities;

	override void Run()
	{
		Print("[TBD][Recon] " + TBD_BlueprintRecon.Execute(m_sPrefabFilter, m_iMaxEntities));
	}
}
