/**
 * @file EMCP_WB_Terrain.c
 * @brief Net API handler that reports the terrain height at a point and the terrain bounds.
 *
 * Role: reads terrain height through WorldEditorAPI and bounds through WorldEditor.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Terrain` here;
 * the enfusion-mcp `wb_terrain` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: getHeight needs WorldEditorAPI (edit mode); getBounds
 * works in either mode; coordinates travel as strings and parse with ToFloat.
 */

//! Request wire of `EMCP_WB_Terrain`: the call's JSON body, decoded by Workbench.
class EMCP_WB_TerrainRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": getHeight or getBounds
	string x; //!< JSON "x": world x in metres, as a string
	string z; //!< JSON "z": world z in metres, as a string

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_TerrainRequestWire()
	{
		RegV("action");
		RegV("x");
		RegV("z");
	}
}

//! Response wire of `EMCP_WB_Terrain`: encoded as the call's JSON reply.
class EMCP_WB_TerrainResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string action; //!< JSON "action": echo of the request action
	float height; //!< JSON "height": terrain surface y at (x, z), in metres
	string boundsMin; //!< JSON "boundsMin": terrain minimum corner, "x y z" in metres
	string boundsMax; //!< JSON "boundsMax": terrain maximum corner, "x y z" in metres

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_TerrainResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("action");
		RegV("height");
		RegV("boundsMin");
		RegV("boundsMax");
	}
}

//! Net API handler `EMCP_WB_Terrain`: terrain height and bounds.
class EMCP_WB_Terrain : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_TerrainRequestWire();
	}

	//! Runs `action` (getHeight or getBounds) and returns the response wire. Answers
	//! "error" when the World Editor or, for getHeight, its API is missing, when there are
	//! no terrain bounds, or when the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_TerrainRequestWire req = EMCP_WB_TerrainRequestWire.Cast(request);
		EMCP_WB_TerrainResponseWire resp = new EMCP_WB_TerrainResponseWire();
		resp.action = req.action;

		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			resp.status = "error";
			resp.message = "WorldEditor module not available";
			return resp;
		}

		if (req.action == "getHeight")
		{
			WorldEditorAPI api = worldEditor.GetApi();
			if (!api)
			{
				resp.status = "error";
				resp.message = "WorldEditorAPI not available";
				return resp;
			}

			float fx = req.x.ToFloat();
			float fz = req.z.ToFloat();
			float surfaceY = api.GetTerrainSurfaceY(fx, fz);
			resp.height = surfaceY;
			resp.status = "ok";
			resp.message = "Terrain height at (" + fx.ToString() + ", " + fz.ToString() + "): " + surfaceY.ToString();
		}
		else if (req.action == "getBounds")
		{
			vector boundsMinVec, boundsMaxVec;
			bool result = worldEditor.GetTerrainBounds(boundsMinVec, boundsMaxVec);

			if (result)
			{
				resp.boundsMin = boundsMinVec[0].ToString() + " " + boundsMinVec[1].ToString() + " " + boundsMinVec[2].ToString();
				resp.boundsMax = boundsMaxVec[0].ToString() + " " + boundsMaxVec[1].ToString() + " " + boundsMaxVec[2].ToString();
				resp.status = "ok";
				resp.message = "Terrain bounds retrieved";
			}
			else
			{
				resp.status = "error";
				resp.message = "GetTerrainBounds returned false (no terrain loaded?)";
			}
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: getHeight, getBounds";
		}

		return resp;
	}
}
