/**
 * @file EMCP_WB_ModifyEntityTransformActions.c
 * @brief The move and rotate actions of the EMCP_WB_ModifyEntity Net API handler.
 *
 * Role: writes an entity source's position and rotation variables.  Position: called by
 * EMCP_WB_ModifyEntity.GetResponse for the actions "move" and "rotate"; writes through
 * WorldEditorAPI.SetVariableValue.
 * State: none.  Invariants: each write runs inside one Begin/EndEntityAction pair, so Workbench
 * undoes it as one step; an entity with no runtime entity is answered "error" and left unchanged.
 */

//! The transform actions of `EMCP_WB_ModifyEntity`: move and rotate an entity source.
class EMCP_WB_ModifyEntityTransformActions
{
	//! Writes `req.value`, an "x y z" string in metres, to the entity's "coords" variable as one
	//! undoable action and answers "ok". Answers "error" and writes nothing when the entity has
	//! no runtime entity.
	static void Move(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		vector pos = EMCP_WB_ModifyEntity.ParseVectorString(req.value);
		IEntity ent = api.SourceToEntity(entSrc);
		if (!ent)
		{
			resp.status = "error";
			resp.message = "Cannot get runtime entity for transform update";
			return;
		}

		api.BeginEntityAction("Move entity via NetAPI");

		// Set position via SetVariableValue on the coords property
		BaseContainer entContainer = entSrc.ToBaseContainer();
		if (entContainer)
		{
			api.SetVariableValue(entContainer, null, "coords", req.value);
		}

		api.EndEntityAction();
		resp.status = "ok";
		resp.message = "Entity moved to " + req.value;
	}

	//! Parses `req.value` as "x y z" degrees and writes the parts to "angleX", "angleY" and
	//! "angleZ" as one undoable action, then answers "ok". Answers "error" and writes nothing when
	//! the entity has no runtime entity.
	static void Rotate(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		vector angles = EMCP_WB_ModifyEntity.ParseVectorString(req.value);
		IEntity ent = api.SourceToEntity(entSrc);
		if (!ent)
		{
			resp.status = "error";
			resp.message = "Cannot get runtime entity for rotation update";
			return;
		}

		api.BeginEntityAction("Rotate entity via NetAPI");

		BaseContainer entContainer = entSrc.ToBaseContainer();
		if (entContainer)
		{
			api.SetVariableValue(entContainer, null, "angleX", angles[0].ToString());
			api.SetVariableValue(entContainer, null, "angleY", angles[1].ToString());
			api.SetVariableValue(entContainer, null, "angleZ", angles[2].ToString());
		}

		api.EndEntityAction();
		resp.status = "ok";
		resp.message = "Entity rotated to " + req.value;
	}
}
