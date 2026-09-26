/**
 * @file EMCP_WB_CreateEntity.c
 * @brief Net API handler that creates an entity from a prefab at a position and rotation.
 *
 * Role: instantiates a prefab in the open world through WorldEditorAPI.CreateEntity.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_CreateEntity` here;
 * the enfusion-mcp `wb_entity_create` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: creation runs inside one Begin/EndEntityAction pair; a
 * negative layer becomes layer 0; a requested name the API did not apply is set by
 * RenameEntity; a null result is answered "error" and the action is still closed.
 */

//! Request wire of `EMCP_WB_CreateEntity`: the call's JSON body, decoded by Workbench.
class EMCP_WB_CreateEntityRequestWire : JsonApiStruct
{
	string prefab; //!< JSON "prefab": prefab resource name, "{GUID}path.et"
	string position; //!< JSON "position": "x y z" in metres; empty is the origin
	string rotation; //!< JSON "rotation": "x y z" angles in degrees; empty is no rotation
	string name; //!< JSON "name": entity name; empty lets Workbench generate one
	int layerID; //!< JSON "layerID": target layer; default -1 (layer 0)

	//! Registers each field as the JSON key of the same name; `layerID` defaults to -1
	//! (layer 0).
	void EMCP_WB_CreateEntityRequestWire()
	{
		RegV("prefab");
		RegV("position");
		RegV("rotation");
		RegV("name");
		RegV("layerID");
		layerID = -1;
	}
}

//! Response wire of `EMCP_WB_CreateEntity`: encoded as the call's JSON reply.
class EMCP_WB_CreateEntityResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string entityName; //!< JSON "entityName": name of the created entity
	string entityClass; //!< JSON "entityClass": class of the created entity
	string position; //!< JSON "position": the parsed position, "x y z" in metres

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_CreateEntityResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("entityName");
		RegV("entityClass");
		RegV("position");
	}
}

//! Net API handler `EMCP_WB_CreateEntity`: create an entity from a prefab.
class EMCP_WB_CreateEntity : NetApiHandler
{
	//! Parses an "x y z" string into a vector. Returns "0 0 0" for an empty string and
	//! for one with fewer than three space-separated parts.
	static vector ParseVectorString(string str)
	{
		vector result = "0 0 0";
		if (str == "")
			return result;

		array<string> parts = {};
		str.Split(" ", parts, true);
		if (parts.Count() >= 3)
		{
			result[0] = parts[0].ToFloat();
			result[1] = parts[1].ToFloat();
			result[2] = parts[2].ToFloat();
		}
		return result;
	}

	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_CreateEntityRequestWire();
	}

	//! Creates an entity from `prefab` at `position` with `rotation` on `layerID` and
	//! returns the response wire naming it. Answers "error" when `prefab` is empty, when the
	//! World Editor or its API is missing, or when CreateEntity returns null.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_CreateEntityRequestWire req = EMCP_WB_CreateEntityRequestWire.Cast(request);
		EMCP_WB_CreateEntityResponseWire resp = new EMCP_WB_CreateEntityResponseWire();

		if (req.prefab == "")
		{
			resp.status = "error";
			resp.message = "prefab parameter required (resource path, e.g. '{GUID}Prefabs/Entity.et')";
			return resp;
		}

		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			resp.status = "error";
			resp.message = "WorldEditor module not available";
			return resp;
		}

		WorldEditorAPI api = worldEditor.GetApi();
		if (!api)
		{
			resp.status = "error";
			resp.message = "WorldEditorAPI not available (in game mode?)";
			return resp;
		}

		vector pos = ParseVectorString(req.position);
		vector rot = ParseVectorString(req.rotation);

		// Default to layer 0 if not specified
		int targetLayer = req.layerID;
		if (targetLayer < 0)
			targetLayer = 0;

		// Entity name defaults to empty (auto-generated)
		string entityName = req.name;

		api.BeginEntityAction("CC: Create entity from prefab");

		// CreateEntity(prefab, name, layerID, parent, position, angles)
		IEntitySource entSrc = api.CreateEntity(req.prefab, entityName, targetLayer, null, pos, rot);

		if (!entSrc)
		{
			api.EndEntityAction();
			resp.status = "error";
			resp.message = "CreateEntity returned null. Check prefab path: " + req.prefab;
			return resp;
		}

		// If a name was requested but not set during creation, rename
		if (entityName != "" && entSrc.GetName() != entityName)
		{
			api.RenameEntity(entSrc, entityName);
		}

		api.EndEntityAction();

		resp.entityName = entSrc.GetName();
		resp.entityClass = entSrc.GetClassName();
		resp.position = pos[0].ToString() + " " + pos[1].ToString() + " " + pos[2].ToString();
		resp.status = "ok";
		resp.message = "Entity created from prefab: " + req.prefab;

		return resp;
	}
}
