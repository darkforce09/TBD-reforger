/**
 * @file EMCP_WB_ModifyEntityObjectClassActions.c
 * @brief The setObjectClass action of the EMCP_WB_ModifyEntity Net API handler.
 *
 * Role: swaps the class of an object property or array member, as the class drop-down of the
 * World Editor does.  Position: called by EMCP_WB_ModifyEntity.GetResponse for "setObjectClass";
 * writes through WorldEditorAPI.ChangeObjectClass.
 * State: none.  Invariants: the target path is `propertyPath` joined to `propertyKey` with a
 * dot; the change runs inside one Begin/EndEntityAction pair; an empty key or class is answered
 * "error" before anything changes.
 */

//! The class-change action of `EMCP_WB_ModifyEntity`.
class EMCP_WB_ModifyEntityObjectClassActions
{
	//! Changes the class of the object at `req.propertyPath` + "." + `req.propertyKey` to
	//! `req.value` as one undoable action and answers "ok". Answers "error" when the key or the
	//! class is empty or ChangeObjectClass returns false.
	static void SetObjectClass(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		// Changes the class of an existing object property or array element (the dropdown in the editor).
		// propertyPath = component class name (e.g. "SCR_ScenarioFrameworkArea")
		// propertyKey  = property name of the object whose class is being changed
		// value        = new class name
		// The full path to the target is propertyPath + propertyKey.
		if (req.propertyKey == "" || req.value == "")
		{
			resp.status = "error";
			resp.message = "propertyKey and value (new class name) required for setObjectClass";
			return;
		}

		// Build path including propertyKey so ChangeObjectClass targets the correct object
		string fullPath = req.propertyPath;
		if (fullPath != "")
			fullPath += ".";
		fullPath += req.propertyKey;

		array<ref ContainerIdPathEntry> pathEntries = EMCP_WB_ModifyEntity.BuildPathEntries(fullPath);

		api.BeginEntityAction("Set object class via NetAPI");
		bool result = api.ChangeObjectClass(entSrc, pathEntries, req.value);
		api.EndEntityAction();

		if (result)
		{
			resp.status = "ok";
			resp.message = "Changed class of '" + req.propertyKey + "' to '" + req.value + "'";
		}
		else
		{
			resp.status = "error";
			resp.message = "ChangeObjectClass returned false -- check class name";
		}
	}
}
