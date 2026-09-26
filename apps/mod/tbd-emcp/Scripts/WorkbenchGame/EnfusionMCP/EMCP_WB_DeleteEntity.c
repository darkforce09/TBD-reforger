/**
 * @file EMCP_WB_DeleteEntity.c
 * @brief Net API handler that deletes an entity found by name.
 *
 * Role: removes one named entity source from the open world.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_DeleteEntity` here;
 * the enfusion-mcp `wb_entity_delete` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: the delete runs inside one Begin/EndEntityAction pair; the
 * name and class are read before deletion so the reply can report them.
 */

//! Request wire of `EMCP_WB_DeleteEntity`: the call's JSON body, decoded by Workbench.
class EMCP_WB_DeleteEntityRequestWire : JsonApiStruct
{
	string name; //!< JSON "name": name of the entity to delete

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_DeleteEntityRequestWire()
	{
		RegV("name");
	}
}

//! Response wire of `EMCP_WB_DeleteEntity`: encoded as the call's JSON reply.
class EMCP_WB_DeleteEntityResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string deletedName; //!< JSON "deletedName": name of the deleted entity
	string deletedClass; //!< JSON "deletedClass": class of the deleted entity

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_DeleteEntityResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("deletedName");
		RegV("deletedClass");
	}
}

//! Net API handler `EMCP_WB_DeleteEntity`: delete an entity by name.
class EMCP_WB_DeleteEntity : NetApiHandler
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
		return new EMCP_WB_DeleteEntityRequestWire();
	}

	//! Deletes the entity named `name` through WorldEditorAPI.DeleteEntity and returns the
	//! response wire with its name and class. Answers "error" when `name` is empty, the World
	//! Editor, its API or the entity is missing, or DeleteEntity returns false.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_DeleteEntityRequestWire req = EMCP_WB_DeleteEntityRequestWire.Cast(request);
		EMCP_WB_DeleteEntityResponseWire resp = new EMCP_WB_DeleteEntityResponseWire();

		if (req.name == "")
		{
			resp.status = "error";
			resp.message = "name parameter required";
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
			resp.message = "WorldEditorAPI not available";
			return resp;
		}

		IEntitySource entSrc = FindEntityByName(api, req.name);
		if (!entSrc)
		{
			resp.status = "error";
			resp.message = "Entity not found: " + req.name;
			return resp;
		}

		resp.deletedName = entSrc.GetName();
		resp.deletedClass = entSrc.GetClassName();

		// Delete entity using the WorldEditorAPI action system
		// The DeleteEntity method exists in the 84-method API surface
		api.BeginEntityAction("CC: Delete entity");
		bool deleted = api.DeleteEntity(entSrc);
		api.EndEntityAction();

		if (deleted)
		{
			resp.status = "ok";
			resp.message = "Entity deleted: " + resp.deletedName;
		}
		else
		{
			resp.status = "error";
			resp.message = "DeleteEntity returned false for: " + resp.deletedName;
		}

		return resp;
	}
}
