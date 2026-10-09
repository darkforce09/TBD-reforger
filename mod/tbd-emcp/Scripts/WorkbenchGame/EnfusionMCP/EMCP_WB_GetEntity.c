/**
 * @file EMCP_WB_GetEntity.c
 * @brief Net API handler that returns one entity's details, found by name or index.
 *
 * Role: reads an entity source's name, class, layer, transform, variables and
 * components into EMCP_WB_GetEntityResponseWire.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_GetEntity` here;
 * the enfusion-mcp `wb_entity_inspect` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: `name` wins over `index`; at most 50 variables are
 * reported, `varCount` gives the full count; an entity with no runtime entity reports
 * "0 0 0" for position and rotation.
 */

//! Request wire of `EMCP_WB_GetEntity`: the call's JSON body, decoded by Workbench.
class EMCP_WB_GetEntityRequestWire : JsonApiStruct
{
	string name; //!< JSON "name": entity name; wins over index
	int index; //!< JSON "index": editor entity index; default -1 (unset)

	//! Registers each field as the JSON key of the same name; `index` defaults to -1 (unset).
	void EMCP_WB_GetEntityRequestWire()
	{
		RegV("name");
		RegV("index");
		index = -1;
	}
}

//! Net API handler `EMCP_WB_GetEntity`: one entity's details.
class EMCP_WB_GetEntity : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_GetEntityRequestWire();
	}

	//! Finds the entity by `name`, or by `index` when the name is empty, and returns its
	//! details in an EMCP_WB_GetEntityResponseWire. Answers "error" when the World Editor or
	//! its API is missing, when neither selector is given, or when no entity matches.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_GetEntityRequestWire req = EMCP_WB_GetEntityRequestWire.Cast(request);
		EMCP_WB_GetEntityResponseWire resp = new EMCP_WB_GetEntityResponseWire();

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
			resp.message = "WorldEditorAPI not available";
			return resp;
		}

		// Find entity by name or index
		IEntitySource entSrc = null;

		if (req.name != "")
		{
			// Search by name
			int count = api.GetEditorEntityCount();
			for (int i = 0; i < count; i++)
			{
				IEntitySource candidate = api.GetEditorEntity(i);
				if (candidate && candidate.GetName() == req.name)
				{
					entSrc = candidate;
					break;
				}
			}

			if (!entSrc)
			{
				resp.status = "error";
				resp.message = "Entity not found with name: " + req.name;
				return resp;
			}
		}
		else if (req.index >= 0)
		{
			int count = api.GetEditorEntityCount();
			if (req.index >= count)
			{
				resp.status = "error";
				resp.message = "Index " + req.index.ToString() + " out of range (count: " + count.ToString() + ")";
				return resp;
			}
			entSrc = api.GetEditorEntity(req.index);
			if (!entSrc)
			{
				resp.status = "error";
				resp.message = "Entity at index " + req.index.ToString() + " is null";
				return resp;
			}
		}
		else
		{
			resp.status = "error";
			resp.message = "Provide either name or index (>= 0)";
			return resp;
		}

		// Populate response
		resp.name = entSrc.GetName();
		resp.className = entSrc.GetClassName();
		resp.componentCount = entSrc.GetComponentCount();
		resp.layerID = entSrc.GetLayerID();
		resp.subScene = entSrc.GetSubScene();

		// Get transform from runtime entity
		IEntity ent = api.SourceToEntity(entSrc);
		if (ent)
		{
			vector pos = ent.GetOrigin();
			resp.position = pos[0].ToString() + " " + pos[1].ToString() + " " + pos[2].ToString();

			vector angles = ent.GetAngles();
			resp.rotation = angles[0].ToString() + " " + angles[1].ToString() + " " + angles[2].ToString();
		}
		else
		{
			resp.position = "0 0 0";
			resp.rotation = "0 0 0";
		}

		// Collect variables/properties
		int numVars = entSrc.GetNumVars();
		resp.varCount = numVars;
		int maxVars = numVars;
		if (maxVars > 50)
			maxVars = 50; // Cap to prevent oversized responses

		for (int v = 0; v < maxVars; v++)
		{
			string varName = entSrc.GetVarName(v);
			string varVal;
			entSrc.GetDefaultAsString(varName, varVal);
			resp.m_aVarNames.Insert(varName);
			resp.m_aVarValues.Insert(varVal);
		}

		// Collect components
		for (int c = 0; c < resp.componentCount; c++)
		{
			IEntityComponentSource compSrc = entSrc.GetComponent(c);
			if (compSrc)
				resp.m_aComponentClasses.Insert(compSrc.GetClassName());
			else
				resp.m_aComponentClasses.Insert("null");
		}

		resp.status = "ok";
		resp.message = "Entity details retrieved";
		return resp;
	}
}
