/**
 * EMCP_WB_TbdBlueprint.c - TBD building-blueprint pipeline driver (Phase B agent loop).
 *
 * Menu ExecuteAction is unreliable in this Workbench build, so the extraction pipeline is driven
 * over the Net API instead (APIFunc = this class name; the agent calls it directly over TCP with
 * the enfusion-mcp wire codec).
 *
 * Actions:
 *   recon   -- dump the real child tree + components of the first instance matching `filter`
 *             (TBD_BlueprintRecon; writes prefabs/debug/<slug>_children.json)
 *   extract -- trace-scan blueprint (TBD_BuildingTraceExtract; prefabs/buildings/<slug>.json)
 *   parity  -- engine LOS oracle pairs (prefabs/debug/<slug>_parity.json; `maxEntities` = samples)
 *   dump    -- raw voxel scan, zero interpretation (TBD_BuildingVoxelDump;
 *             prefabs/dumps/<slug>_voxels.jsonl; consumed by `cargo xtask map blueprint-from-voxels`)
 *   world-parity -- T-090.12.4 cell-scoped LOS oracle (TBD_WorldTraceParity; `cx`, `cy`, `seed`,
 *             `maxEntities` = samples; writes prefabs/debug/world_parity_<cx>_<cy>.json, replayed by
 *             `cargo xtask map world-los --pairs`). A stale, uncompiled handler answers
 *             "unknown action" -- that is the detection.
 */

class EMCP_WB_TbdBlueprintRequest : JsonApiStruct
{
	string action;
	string filter;
	int maxEntities;
	// "world-parity" action: chunk cell + RNG seed (T-090.12.4).
	int cx;
	int cy;
	int seed;
	// "probe" action: LOCAL-frame segment endpoints.
	float ax;
	float ay;
	float az;
	float bx;
	float by;
	float bz;

	void EMCP_WB_TbdBlueprintRequest()
	{
		RegV("action");
		RegV("filter");
		RegV("maxEntities");
		RegV("cx");
		RegV("cy");
		RegV("seed");
		RegV("ax");
		RegV("ay");
		RegV("az");
		RegV("bx");
		RegV("by");
		RegV("bz");
		maxEntities = 512;
		cx = -1;
		cy = -1;
		seed = 1;
	}
}

class EMCP_WB_TbdBlueprintResponse : JsonApiStruct
{
	string status;
	string action;
	string message;

	void EMCP_WB_TbdBlueprintResponse()
	{
		RegV("status");
		RegV("action");
		RegV("message");
	}
}

class EMCP_WB_TbdBlueprint : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_TbdBlueprintRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_TbdBlueprintRequest req = EMCP_WB_TbdBlueprintRequest.Cast(request);
		EMCP_WB_TbdBlueprintResponse resp = new EMCP_WB_TbdBlueprintResponse();
		resp.action = req.action;

		string filter = req.filter;
		if (filter == "")
			filter = "FarmHouse_E_1L01";

		string result = "";
		if (req.action == "recon")
			result = TBD_BlueprintRecon.Execute(filter, req.maxEntities);
		else if (req.action == "extract")
			result = TBD_BuildingTraceExtract.Execute(filter);
		else if (req.action == "parity")
			result = TBD_BuildingTraceExtract.ExecuteParity(filter, req.maxEntities);
		else if (req.action == "probe")
			result = TBD_BuildingTraceExtract.ExecuteProbe(filter,
				Vector(req.ax, req.ay, req.az), Vector(req.bx, req.by, req.bz));
		else if (req.action == "dump")
			result = TBD_BuildingVoxelDump.Execute(filter);
		else if (req.action == "world-parity")
			result = TBD_WorldTraceParity.Execute(req.cx, req.cy, req.maxEntities, req.seed);

		if (result != "")
		{
			resp.message = result;
			if (result.StartsWith("OK"))
				resp.status = "ok";
			else
				resp.status = "error";
			return resp;
		}

		resp.status = "error";
		resp.message = "unknown action '" + req.action + "' (expected: recon | extract | parity | probe | dump | world-parity)";
		return resp;
	}
}
