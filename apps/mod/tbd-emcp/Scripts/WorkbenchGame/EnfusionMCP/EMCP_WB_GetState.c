/**
 * @file EMCP_WB_GetState.c
 * @brief Net API handler that returns a snapshot of the World Editor state.
 *
 * Role: reports the editor mode, entity and selection counts, sub-scene, prefab edit
 * mode, terrain bounds and the selected entity names.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_GetState` here;
 * the enfusion-mcp `wb_state` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: always answers "ok"; `mode` is "no_world_editor",
 * "game" (no WorldEditorAPI) or "edit"; at most 50 selected names are reported.
 */

//! Request wire of `EMCP_WB_GetState`: the call's JSON body, decoded by Workbench.
class EMCP_WB_GetStateRequestWire : JsonApiStruct
{
	//! Takes no parameters.
	void EMCP_WB_GetStateRequestWire()
	{
		// No request parameters
	}
}

//! Response wire of `EMCP_WB_GetState`: encoded as the call's JSON reply.
class EMCP_WB_GetStateResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string mode; //!< JSON "mode": "edit", "game" or "no_world_editor"
	int entityCount; //!< JSON "entityCount": editor entities in the world
	int selectedCount; //!< JSON "selectedCount": selected entities
	int currentSubScene; //!< JSON "currentSubScene": the active sub-scene (layer) index
	bool isPrefabEditMode; //!< JSON "isPrefabEditMode": the editor is editing a prefab
	string boundsMin; //!< JSON "boundsMin": terrain minimum corner, "x y z" in metres
	string boundsMax; //!< JSON "boundsMax": terrain maximum corner, "x y z" in metres

	// Selected entity names
	ref array<string> m_aSelectedNames; //!< selected entity names, at most 50; packed into "selectedNames" by OnPack

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_GetStateResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("mode");
		RegV("entityCount");
		RegV("selectedCount");
		RegV("currentSubScene");
		RegV("isPrefabEditMode");
		RegV("boundsMin");
		RegV("boundsMax");

		m_aSelectedNames = {};
	}

	//! Writes the "selectedNames" array of strings, empty when nothing is selected.
	override void OnPack()
	{
		StartArray("selectedNames");
		for (int i = 0; i < m_aSelectedNames.Count(); i++)
		{
			StoreString("", m_aSelectedNames[i]);
		}
		EndArray();
	}
}

//! Net API handler `EMCP_WB_GetState`: the World Editor state snapshot.
class EMCP_WB_GetState : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_GetStateRequestWire();
	}

	//! Fills a state snapshot for the current mode and returns it. Never fails: a missing
	//! World Editor or API is reported through `mode` with status "ok".
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_GetStateResponseWire resp = new EMCP_WB_GetStateResponseWire();

		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			resp.status = "ok";
			resp.mode = "no_world_editor";
			resp.message = "WorldEditor module not loaded";
			return resp;
		}

		WorldEditorAPI api = worldEditor.GetApi();
		if (!api)
		{
			resp.status = "ok";
			resp.mode = "game";
			resp.message = "In game mode (WorldEditorAPI not available)";

			// Still get terrain bounds from WorldEditor
			vector bMin, bMax;
			if (worldEditor.GetTerrainBounds(bMin, bMax))
			{
				resp.boundsMin = bMin[0].ToString() + " " + bMin[1].ToString() + " " + bMin[2].ToString();
				resp.boundsMax = bMax[0].ToString() + " " + bMax[1].ToString() + " " + bMax[2].ToString();
			}

			return resp;
		}

		// Edit mode - collect full state
		resp.mode = "edit";
		resp.entityCount = api.GetEditorEntityCount();
		resp.selectedCount = api.GetSelectedEntitiesCount();
		resp.currentSubScene = api.GetCurrentSubScene();
		resp.isPrefabEditMode = worldEditor.IsPrefabEditMode();

		// Terrain bounds
		vector bMin, bMax;
		if (worldEditor.GetTerrainBounds(bMin, bMax))
		{
			resp.boundsMin = bMin[0].ToString() + " " + bMin[1].ToString() + " " + bMin[2].ToString();
			resp.boundsMax = bMax[0].ToString() + " " + bMax[1].ToString() + " " + bMax[2].ToString();
		}

		// Selected entity names (cap at 50)
		int maxSel = resp.selectedCount;
		if (maxSel > 50)
			maxSel = 50;

		for (int i = 0; i < maxSel; i++)
		{
			IEntitySource selSrc = api.GetSelectedEntity(i);
			if (selSrc)
				resp.m_aSelectedNames.Insert(selSrc.GetName());
			else
				resp.m_aSelectedNames.Insert("");
		}

		resp.status = "ok";
		resp.message = "State snapshot: " + resp.entityCount.ToString() + " entities, " + resp.selectedCount.ToString() + " selected";

		return resp;
	}
}
