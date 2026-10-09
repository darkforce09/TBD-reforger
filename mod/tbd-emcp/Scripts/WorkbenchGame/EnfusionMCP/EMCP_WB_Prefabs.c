/**
 * @file EMCP_WB_Prefabs.c
 * @brief Net API handler that creates a prefab from an entity, saves it and reads its ancestor.
 *
 * Role: runs the prefab (entity template) operations of WorldEditorAPI on a named entity.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Prefabs` here;
 * the enfusion-mcp `wb_prefabs` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: createTemplate runs inside one Begin/EndEntityAction pair;
 * getAncestor answers "ok" with an empty `ancestorPath` for an entity that is not a
 * prefab instance.
 */

//! Request wire of `EMCP_WB_Prefabs`: the call's JSON body, decoded by Workbench.
class EMCP_WB_PrefabsRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": createTemplate, save or getAncestor
	string entityName; //!< JSON "entityName": the entity to act on
	string templatePath; //!< JSON "templatePath": the new prefab path for createTemplate

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_PrefabsRequestWire()
	{
		RegV("action");
		RegV("entityName");
		RegV("templatePath");
	}
}

//! Response wire of `EMCP_WB_Prefabs`: encoded as the call's JSON reply.
class EMCP_WB_PrefabsResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string action; //!< JSON "action": echo of the request action
	string entityName; //!< JSON "entityName": echo of the request entity name
	string ancestorPath; //!< JSON "ancestorPath": the ancestor prefab resource for getAncestor; empty for none

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_PrefabsResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("action");
		RegV("entityName");
		RegV("ancestorPath");
	}
}

//! Net API handler `EMCP_WB_Prefabs`: prefab creation, save and ancestry.
class EMCP_WB_Prefabs : NetApiHandler
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
		return new EMCP_WB_PrefabsRequestWire();
	}

	//! Runs `action` (createTemplate, save or getAncestor) on the entity named `entityName`
	//! and returns the response wire. Answers "error" when a parameter, the World Editor, its
	//! API or the entity is missing, when the API returns false, or the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_PrefabsRequestWire req = EMCP_WB_PrefabsRequestWire.Cast(request);
		EMCP_WB_PrefabsResponseWire resp = new EMCP_WB_PrefabsResponseWire();
		resp.action = req.action;
		resp.entityName = req.entityName;

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

		if (req.action == "createTemplate")
		{
			if (req.entityName == "" || req.templatePath == "")
			{
				resp.status = "error";
				resp.message = "entityName and templatePath required for createTemplate";
				return resp;
			}

			IEntitySource entSrc = FindEntityByName(api, req.entityName);
			if (!entSrc)
			{
				resp.status = "error";
				resp.message = "Entity not found: " + req.entityName;
				return resp;
			}

			api.BeginEntityAction("Create template via NetAPI");
			bool result = api.CreateEntityTemplate(entSrc, req.templatePath);
			api.EndEntityAction();

			if (result)
			{
				resp.status = "ok";
				resp.message = "Template created at: " + req.templatePath;
			}
			else
			{
				resp.status = "error";
				resp.message = "CreateEntityTemplate returned false";
			}
		}
		else if (req.action == "save")
		{
			if (req.entityName == "")
			{
				resp.status = "error";
				resp.message = "entityName required for save action";
				return resp;
			}

			IEntitySource entSrc = FindEntityByName(api, req.entityName);
			if (!entSrc)
			{
				resp.status = "error";
				resp.message = "Entity not found: " + req.entityName;
				return resp;
			}

			bool result = api.SaveEntityTemplate(entSrc);
			if (result)
			{
				resp.status = "ok";
				resp.message = "Entity template saved for: " + req.entityName;
			}
			else
			{
				resp.status = "error";
				resp.message = "SaveEntityTemplate returned false (entity may not be a template instance)";
			}
		}
		else if (req.action == "getAncestor")
		{
			if (req.entityName == "")
			{
				resp.status = "error";
				resp.message = "entityName required for getAncestor action";
				return resp;
			}

			IEntitySource entSrc = FindEntityByName(api, req.entityName);
			if (!entSrc)
			{
				resp.status = "error";
				resp.message = "Entity not found: " + req.entityName;
				return resp;
			}

			BaseContainer ancestor = entSrc.GetAncestor();
			if (ancestor)
			{
				resp.ancestorPath = ancestor.GetResourceName();
				resp.status = "ok";
				resp.message = "Ancestor prefab: " + resp.ancestorPath;
			}
			else
			{
				resp.ancestorPath = "";
				resp.status = "ok";
				resp.message = "Entity has no ancestor (not a prefab instance)";
			}
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: createTemplate, save, getAncestor";
		}

		return resp;
	}
}
