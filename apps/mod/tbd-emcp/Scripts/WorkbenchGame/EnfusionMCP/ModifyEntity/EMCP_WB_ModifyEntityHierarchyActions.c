/**
 * @file EMCP_WB_ModifyEntityHierarchyActions.c
 * @brief The rename and reparent actions of the EMCP_WB_ModifyEntity Net API handler.
 *
 * Role: changes an entity source's name and parent in the World Editor hierarchy.
 * Position: called by EMCP_WB_ModifyEntity.GetResponse for the actions "rename" and "reparent"; writes
 * through WorldEditorAPI.RenameEntity and WorldEditorAPI.ParentEntity.
 * State: none.  Invariants: each change runs inside one Begin/EndEntityAction pair; an empty
 * `value` or an unknown parent name is answered "error" before anything changes.
 */

//! The hierarchy actions of `EMCP_WB_ModifyEntity`: rename an entity and reparent it.
class EMCP_WB_ModifyEntityHierarchyActions
{
	//! Renames the entity to `req.value` as one undoable action. Answers "ok" when
	//! RenameEntity accepts the name, "error" when `req.value` is empty or the name is refused.
	static void Rename(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		if (req.value == "")
		{
			resp.status = "error";
			resp.message = "value parameter required for rename (new name)";
			return;
		}

		api.BeginEntityAction("Rename entity via NetAPI");
		bool renamed = api.RenameEntity(entSrc, req.value);
		api.EndEntityAction();

		if (renamed)
		{
			resp.status = "ok";
			resp.message = "Entity renamed to: " + req.value;
		}
		else
		{
			resp.status = "error";
			resp.message = "RenameEntity returned false";
		}
	}

	//! Makes the entity named `req.value` the parent of this one through ParentEntity with its
	//! transform flag set, as one undoable action, and answers "ok". Answers "error" when
	//! `req.value` is empty or names no entity.
	static void Reparent(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		if (req.value == "")
		{
			resp.status = "error";
			resp.message = "value parameter required for reparent (parent entity name)";
			return;
		}

		IEntitySource parentSrc = EMCP_WB_ModifyEntity.FindEntityByName(api, req.value);
		if (!parentSrc)
		{
			resp.status = "error";
			resp.message = "Parent entity not found: " + req.value;
			return;
		}

		api.BeginEntityAction("Reparent entity via NetAPI");
		api.ParentEntity(parentSrc, entSrc, true);
		api.EndEntityAction();

		resp.status = "ok";
		resp.message = "Entity reparented to: " + req.value;
	}
}
