/**
 * @file EMCP_WB_Layers.c
 * @brief Net API handler that lists layers, reports the active one and an entity's layer.
 *
 * Role: derives layer information from the layer IDs of the editor's entity sources.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Layers` here;
 * the enfusion-mcp `wb_layers` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: list reports only layers that hold at least one entity;
 * the actions are list, getActive and getEntityLayer, and any other action answers
 * "error"; `subScene` and `visible` are decoded but read by no action.
 */

//! Request wire of `EMCP_WB_Layers`: the call's JSON body, decoded by Workbench.
class EMCP_WB_LayersRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": list, getActive or getEntityLayer
	int subScene; //!< JSON "subScene": default -1; read by no action
	string entityName; //!< JSON "entityName": entity for getEntityLayer
	bool visible; //!< JSON "visible": read by no action

	//! Registers each field as the JSON key of the same name; `subScene` defaults to -1.
	void EMCP_WB_LayersRequestWire()
	{
		RegV("action");
		RegV("subScene");
		RegV("entityName");
		RegV("visible");
		subScene = -1;
	}
}

//! Response wire of `EMCP_WB_Layers`: encoded as the call's JSON reply.
class EMCP_WB_LayersResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string action; //!< JSON "action": echo of the request action
	int currentSubScene; //!< JSON "currentSubScene": the active sub-scene index
	int layerID; //!< JSON "layerID": the entity's layer for getEntityLayer

	// Layer data collected for list
	ref array<int> m_aLayerIDs; //!< layer IDs for the list action; packed into "layers" by OnPack
	ref array<int> m_aEntityCounts; //!< entity count per layer, parallel to m_aLayerIDs

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_LayersResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("action");
		RegV("currentSubScene");
		RegV("layerID");

		m_aLayerIDs = {};
		m_aEntityCounts = {};
	}

	//! Writes the "layers" array of {layerID, entityCount} objects when the list action
	//! collected any; writes nothing otherwise.
	override void OnPack()
	{
		if (m_aLayerIDs.Count() > 0)
		{
			StartArray("layers");
			for (int i = 0; i < m_aLayerIDs.Count(); i++)
			{
				StartObject("");
				StoreInteger("layerID", m_aLayerIDs[i]);
				StoreInteger("entityCount", m_aEntityCounts[i]);
				EndObject();
			}
			EndArray();
		}
	}
}

//! Net API handler `EMCP_WB_Layers`: layer listing and lookup.
class EMCP_WB_Layers : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_LayersRequestWire();
	}

	//! Runs `action` (list, getActive or getEntityLayer) and returns the response wire with
	//! the current sub-scene. Answers "error" when the World Editor, its API or the named
	//! entity is missing, or the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_LayersRequestWire req = EMCP_WB_LayersRequestWire.Cast(request);
		EMCP_WB_LayersResponseWire resp = new EMCP_WB_LayersResponseWire();
		resp.action = req.action;

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

		resp.currentSubScene = api.GetCurrentSubScene();

		if (req.action == "list")
		{
			// Enumerate layers by scanning all entities and collecting unique layer IDs
			int entityCount = api.GetEditorEntityCount();
			map<int, int> layerCounts = new map<int, int>();

			for (int i = 0; i < entityCount; i++)
			{
				IEntitySource entSrc = api.GetEditorEntity(i);
				if (!entSrc)
					continue;

				int lid = entSrc.GetLayerID();
				if (layerCounts.Contains(lid))
				{
					int current = layerCounts.Get(lid);
					layerCounts.Set(lid, current + 1);
				}
				else
				{
					layerCounts.Set(lid, 1);
				}
			}

			// Output collected layers
			for (int k = 0; k < layerCounts.Count(); k++)
			{
				int layerKey = layerCounts.GetKey(k);
				int layerCount = layerCounts.GetElement(k);
				resp.m_aLayerIDs.Insert(layerKey);
				resp.m_aEntityCounts.Insert(layerCount);
			}

			resp.status = "ok";
			resp.message = "Found " + layerCounts.Count().ToString() + " layers across " + entityCount.ToString() + " entities";
		}
		else if (req.action == "getActive")
		{
			resp.currentSubScene = api.GetCurrentSubScene();
			resp.status = "ok";
			resp.message = "Current sub-scene: " + resp.currentSubScene.ToString();
		}
		else if (req.action == "getEntityLayer")
		{
			if (req.entityName == "")
			{
				resp.status = "error";
				resp.message = "entityName parameter required for getEntityLayer";
				return resp;
			}

			int count = api.GetEditorEntityCount();
			bool found = false;
			for (int i = 0; i < count; i++)
			{
				IEntitySource entSrc = api.GetEditorEntity(i);
				if (entSrc && entSrc.GetName() == req.entityName)
				{
					resp.layerID = entSrc.GetLayerID();
					found = true;
					break;
				}
			}

			if (found)
			{
				resp.status = "ok";
				resp.message = "Entity '" + req.entityName + "' is on layer " + resp.layerID.ToString();
			}
			else
			{
				resp.status = "error";
				resp.message = "Entity not found: " + req.entityName;
			}
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: list, getActive, getEntityLayer";
		}

		return resp;
	}
}
