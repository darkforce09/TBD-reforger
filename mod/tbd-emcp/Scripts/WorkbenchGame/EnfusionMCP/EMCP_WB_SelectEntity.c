/**
 * @file EMCP_WB_SelectEntity.c
 * @brief Net API handler that deselects, clears and reads the World Editor selection.
 *
 * Role: edits and reads the entity selection through WorldEditorAPI.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_SelectEntity` here;
 * the enfusion-mcp `wb_entity_select` tool and `cargo xtask mcp wbcall` send them.
 * State: none; the selection is Workbench's.  Invariants: select clears the selection
 * and adds nothing, because the script API offers no call that adds an entity to it;
 * getSelected reports at most 100 entities.
 */

//! Request wire of `EMCP_WB_SelectEntity`: the call's JSON body, decoded by Workbench.
class EMCP_WB_SelectEntityRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": select, deselect, clear or getSelected
	string name; //!< JSON "name": the entity for select and deselect

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_SelectEntityRequestWire()
	{
		RegV("action");
		RegV("name");
	}
}

//! Response wire of `EMCP_WB_SelectEntity`: encoded as the call's JSON reply.
class EMCP_WB_SelectEntityResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string action; //!< JSON "action": echo of the request action
	int selectedCount; //!< JSON "selectedCount": selected entities after the action

	// Selected entity names for getSelected
	ref array<string> m_aSelectedNames; //!< selected names for getSelected; packed into "selectedEntities" by OnPack
	ref array<string> m_aSelectedClasses; //!< selected classes, parallel to m_aSelectedNames

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_SelectEntityResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("action");
		RegV("selectedCount");

		m_aSelectedNames = {};
		m_aSelectedClasses = {};
	}

	//! Writes the "selectedEntities" array of {name, className} objects when getSelected
	//! collected any; writes nothing otherwise.
	override void OnPack()
	{
		if (m_aSelectedNames.Count() > 0)
		{
			StartArray("selectedEntities");
			for (int i = 0; i < m_aSelectedNames.Count(); i++)
			{
				StartObject("");
				StoreString("name", m_aSelectedNames[i]);
				StoreString("className", m_aSelectedClasses[i]);
				EndObject();
			}
			EndArray();
		}
	}
}

//! Net API handler `EMCP_WB_SelectEntity`: selection edits and reads.
class EMCP_WB_SelectEntity : NetApiHandler
{
	//! Returns the first editor entity source named `name`, or null when no entity has
	//! that name.
	static IEntitySource FindEntityByName(WorldEditorAPI api, string name)
	{
		int count = api.GetEditorEntityCount();
		for (int i = 0; i < count; i++)
		{
			IEntitySource candidate = api.GetEditorEntity(i);
			if (candidate && candidate.GetName() == name)
				return candidate;
		}
		return null;
	}

	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_SelectEntityRequestWire();
	}

	//! Runs `action` (select, deselect, clear or getSelected) and returns the response wire
	//! with the selection count. Answers "error" when the World Editor, its API, a required
	//! `name` or the named entity is missing, or the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_SelectEntityRequestWire req = EMCP_WB_SelectEntityRequestWire.Cast(request);
		EMCP_WB_SelectEntityResponseWire resp = new EMCP_WB_SelectEntityResponseWire();
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

		if (req.action == "select")
		{
			if (req.name == "")
			{
				resp.status = "error";
				resp.message = "name parameter required for select action";
				return resp;
			}

			IEntitySource entSrc = FindEntityByName(api, req.name);
			if (!entSrc)
			{
				resp.status = "error";
				resp.message = "Entity not found: " + req.name;
				return resp;
			}

			// Clear existing selection and select via menu action approach
			// The API provides ClearEntitySelection and GetSelectedEntity but
			// not AddToEntitySelection directly. We use the following workaround:
			// Set the entity as the focused/selected entity via ExecuteAction
			api.ClearEntitySelection();

			// Workaround: Use SetVariableValue on the entity to trigger selection
			// or use the select-all-by-name approach via menu
			// In practice, selection can be achieved by centering on the entity
			// Best available approach: report the entity was found and suggest
			// using the GUI or ExecuteAction("Edit", "Select All") + filter
			resp.status = "ok";
			resp.selectedCount = api.GetSelectedEntitiesCount();
			resp.message = "Entity found: " + req.name + ". Note: Programmatic AddToEntitySelection not available in public API. Use EMCP_WB_ExecuteAction with Edit menu for selection.";
		}
		else if (req.action == "deselect")
		{
			if (req.name == "")
			{
				resp.status = "error";
				resp.message = "name parameter required for deselect action";
				return resp;
			}

			IEntitySource entSrc = FindEntityByName(api, req.name);
			if (!entSrc)
			{
				resp.status = "error";
				resp.message = "Entity not found: " + req.name;
				return resp;
			}

			api.RemoveFromEntitySelection(entSrc);
			resp.selectedCount = api.GetSelectedEntitiesCount();
			resp.status = "ok";
			resp.message = "Entity deselected: " + req.name;
		}
		else if (req.action == "clear")
		{
			api.ClearEntitySelection();
			resp.selectedCount = 0;
			resp.status = "ok";
			resp.message = "Selection cleared";
		}
		else if (req.action == "getSelected")
		{
			int selCount = api.GetSelectedEntitiesCount();
			resp.selectedCount = selCount;

			int maxReport = selCount;
			if (maxReport > 100)
				maxReport = 100; // Cap to prevent oversized responses

			for (int i = 0; i < maxReport; i++)
			{
				IEntitySource selSrc = api.GetSelectedEntity(i);
				if (selSrc)
				{
					resp.m_aSelectedNames.Insert(selSrc.GetName());
					resp.m_aSelectedClasses.Insert(selSrc.GetClassName());
				}
			}

			resp.status = "ok";
			resp.message = "Selected entities: " + selCount.ToString();
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: select, deselect, clear, getSelected";
		}

		return resp;
	}
}
