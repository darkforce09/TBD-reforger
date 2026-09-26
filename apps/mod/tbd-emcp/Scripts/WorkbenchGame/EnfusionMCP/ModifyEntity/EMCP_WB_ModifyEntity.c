/**
 * @file EMCP_WB_ModifyEntity.c
 * @brief Net API handler that moves, renames, reparents and edits the properties of an entity.
 *
 * Role: validates the call, finds the named entity source and dispatches `action` to
 * the action class of its family; holds the lookups those classes share.
 * Position: Workbench's Net API dispatches each call whose APIFunc is
 * `EMCP_WB_ModifyEntity` here; the enfusion-mcp `wb_entity_modify` tool and
 * `cargo xtask mcp wbcall` send them. The work runs in
 * EMCP_WB_ModifyEntityTransformActions, EMCP_WB_ModifyEntityHierarchyActions,
 * EMCP_WB_ModifyEntityPropertyActions, EMCP_WB_ModifyEntityArrayMemberActions and
 * EMCP_WB_ModifyEntityObjectClassActions.
 * State: none.  Invariants: an empty `name`, a missing World Editor or API, or an
 * unknown entity is answered "error" before any action runs; an unknown action is
 * answered "error" with the list of valid actions; `entityName` always carries the
 * entity's name as found.
 */

//! Request wire of `EMCP_WB_ModifyEntity`: the call's JSON body, decoded by Workbench.
class EMCP_WB_ModifyEntityRequestWire : JsonApiStruct
{
	string name; //!< JSON "name": the entity to modify
	string action; //!< JSON "action": one of the twelve action names
	string value; //!< JSON "value": the action's value: "x y z", a name, a property value or a class
	string propertyPath; //!< JSON "propertyPath": a component class name or a dot-separated container path
	string propertyKey; //!< JSON "propertyKey": the property or array name
	int    memberIndex; //!< JSON "memberIndex": array member index; negative appends for addArrayItem

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_ModifyEntityRequestWire()
	{
		RegV("name");
		RegV("action");
		RegV("value");
		RegV("propertyPath");
		RegV("propertyKey");
		RegV("memberIndex");
	}
}

//! Response wire of `EMCP_WB_ModifyEntity`: encoded as the call's JSON reply.
class EMCP_WB_ModifyEntityResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome, error, or the value read
	string entityName; //!< JSON "entityName": the entity's name as found
	string action; //!< JSON "action": echo of the request action

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_ModifyEntityResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("entityName");
		RegV("action");
	}
}

//! Net API handler `EMCP_WB_ModifyEntity`: entity transform, hierarchy and property edits.
class EMCP_WB_ModifyEntity : NetApiHandler
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

	//! Builds a container path from the dot-separated `propertyPath`, one entry per part.
	//! Returns null for an empty path, which targets the entity itself.
	static array<ref ContainerIdPathEntry> BuildPathEntries(string propertyPath)
	{
		if (propertyPath == "")
			return null;

		array<ref ContainerIdPathEntry> pathEntries = {};
		array<string> pathParts = {};
		propertyPath.Split(".", pathParts, true);
		for (int p = 0; p < pathParts.Count(); p++)
		{
			pathEntries.Insert(new ContainerIdPathEntry(pathParts[p]));
		}
		return pathEntries;
	}

	//! Returns the first component of `entSrc` whose class name is `className`, or null when
	//! none has that class.
	static IEntityComponentSource FindComponentByClassName(IEntitySource entSrc, string className)
	{
		int count = entSrc.GetComponentCount();
		for (int i = 0; i < count; i++)
		{
			IEntityComponentSource candidate = entSrc.GetComponent(i);
			if (candidate && candidate.GetClassName() == className)
				return candidate;
		}
		return null;
	}

	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_ModifyEntityRequestWire();
	}

	//! Finds the entity named `name` and runs `action` on it through the action class of its
	//! family, then returns the response wire. Answers "error" when `name` is empty, when
	//! the World Editor, its API or the entity is missing, or when the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_ModifyEntityRequestWire req = EMCP_WB_ModifyEntityRequestWire.Cast(request);
		EMCP_WB_ModifyEntityResponseWire resp = new EMCP_WB_ModifyEntityResponseWire();
		resp.action = req.action;

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

		resp.entityName = entSrc.GetName();

		if (req.action == "move")
			EMCP_WB_ModifyEntityTransformActions.Move(api, entSrc, req, resp);
		else if (req.action == "rotate")
			EMCP_WB_ModifyEntityTransformActions.Rotate(api, entSrc, req, resp);
		else if (req.action == "rename")
			EMCP_WB_ModifyEntityHierarchyActions.Rename(api, entSrc, req, resp);
		else if (req.action == "reparent")
			EMCP_WB_ModifyEntityHierarchyActions.Reparent(api, entSrc, req, resp);
		else if (req.action == "setProperty")
			EMCP_WB_ModifyEntityPropertyActions.SetProperty(api, entSrc, req, resp);
		else if (req.action == "clearProperty")
			EMCP_WB_ModifyEntityPropertyActions.ClearProperty(api, entSrc, req, resp);
		else if (req.action == "getProperty")
			EMCP_WB_ModifyEntityPropertyActions.GetProperty(api, entSrc, req, resp);
		else if (req.action == "listProperties")
			EMCP_WB_ModifyEntityPropertyActions.ListProperties(api, entSrc, req, resp);
		else if (req.action == "listArrayItems")
			EMCP_WB_ModifyEntityArrayMemberActions.ListArrayItems(api, entSrc, req, resp);
		else if (req.action == "addArrayItem")
			EMCP_WB_ModifyEntityArrayMemberActions.AddArrayItem(api, entSrc, req, resp);
		else if (req.action == "removeArrayItem")
			EMCP_WB_ModifyEntityArrayMemberActions.RemoveArrayItem(api, entSrc, req, resp);
		else if (req.action == "setObjectClass")
			EMCP_WB_ModifyEntityObjectClassActions.SetObjectClass(api, entSrc, req, resp);
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: move, rotate, rename, reparent, setProperty, clearProperty, getProperty, listProperties, listArrayItems, addArrayItem, removeArrayItem, setObjectClass";
		}

		return resp;
	}
}
