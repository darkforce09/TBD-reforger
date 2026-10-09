/**
 * @file EMCP_WB_ModifyEntityPropertyActions.c
 * @brief The property actions of the EMCP_WB_ModifyEntity Net API handler.
 *
 * Role: sets, clears, reads and lists the variables of an entity source or of one of its
 * components.  Position: called by EMCP_WB_ModifyEntity.GetResponse for "setProperty",
 * "clearProperty", "getProperty" and "listProperties"; reads and writes through
 * WorldEditorAPI and IEntityComponentSource.
 * State: none.  Invariants: setProperty and clearProperty take `propertyPath` as a dot-separated
 * container path; getProperty and listProperties take it as a component class name; an empty
 * path targets the entity itself. A missing key or component is answered "error".
 */

//! The property actions of `EMCP_WB_ModifyEntity`: set, clear, read and list variables.
class EMCP_WB_ModifyEntityPropertyActions
{
	//! Sets `req.propertyKey` to `req.value` on the entity, or on the container at the
	//! dot-separated `req.propertyPath`, outside an entity action. Answers "ok" when
	//! SetVariableValue succeeds, "error" when the key is empty or the write is refused.
	static void SetProperty(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		if (req.propertyKey == "")
		{
			resp.status = "error";
			resp.message = "propertyKey parameter required for setProperty";
			return;
		}

		array<ref ContainerIdPathEntry> pathEntries = EMCP_WB_ModifyEntity.BuildPathEntries(req.propertyPath);

		bool result = api.SetVariableValue(entSrc, pathEntries, req.propertyKey, req.value);

		if (result)
		{
			resp.status = "ok";
			resp.message = "Property '" + req.propertyKey + "' set to '" + req.value + "'";
		}
		else
		{
			resp.status = "error";
			resp.message = "SetVariableValue returned false for key: " + req.propertyKey;
		}
	}

	//! Clears `req.propertyKey` back to its inherited value on the entity, or on the container at
	//! the dot-separated `req.propertyPath`, outside an entity action. Answers "ok" when
	//! ClearVariableValue succeeds, "error" when the key is empty or the clear is refused.
	static void ClearProperty(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		if (req.propertyKey == "")
		{
			resp.status = "error";
			resp.message = "propertyKey parameter required for clearProperty";
			return;
		}

		array<ref ContainerIdPathEntry> pathEntries = EMCP_WB_ModifyEntity.BuildPathEntries(req.propertyPath);

		bool result = api.ClearVariableValue(entSrc, pathEntries, req.propertyKey);

		if (result)
		{
			resp.status = "ok";
			resp.message = "Property '" + req.propertyKey + "' cleared";
		}
		else
		{
			resp.status = "error";
			resp.message = "ClearVariableValue returned false for key: " + req.propertyKey;
		}
	}

	//! Reads `req.propertyKey` as a string into `resp.message`, from the component whose class
	//! name is `req.propertyPath`, or from the entity when the path is empty, and answers "ok".
	//! Answers "error" when the key is empty or no component has that class name.
	static void GetProperty(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		if (req.propertyKey == "")
		{
			resp.status = "error";
			resp.message = "propertyKey parameter required for getProperty";
			return;
		}

		// WorldEditorAPI has no GetVariableValue -- use IEntityComponentSource.Get() instead.
		IEntityComponentSource compSrc = null;
		if (req.propertyPath != "")
		{
			compSrc = EMCP_WB_ModifyEntity.FindComponentByClassName(entSrc, req.propertyPath);
			if (!compSrc)
			{
				resp.status = "error";
				resp.message = "Component not found: " + req.propertyPath;
				return;
			}
		}

		string val;
		if (compSrc)
			compSrc.Get(req.propertyKey, val);
		else
			entSrc.Get(req.propertyKey, val);

		resp.status = "ok";
		resp.message = val;
	}

	//! Writes the variable names, comma-separated, into `resp.message`: those of the component
	//! whose class name is `req.propertyPath`, or of the entity when the path is empty. Answers
	//! "error" when no component has that class name.
	static void ListProperties(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		string result = "";

		if (req.propertyPath != "")
		{
			IEntityComponentSource compSrc = EMCP_WB_ModifyEntity.FindComponentByClassName(entSrc, req.propertyPath);
			if (!compSrc)
			{
				resp.status = "error";
				resp.message = "Component not found: " + req.propertyPath;
				return;
			}
			int numVars = compSrc.GetNumVars();
			for (int v = 0; v < numVars; v++)
			{
				if (result != "") result += ", ";
				result += compSrc.GetVarName(v);
			}
		}
		else
		{
			int numVars = entSrc.GetNumVars();
			for (int v = 0; v < numVars; v++)
			{
				if (result != "") result += ", ";
				result += entSrc.GetVarName(v);
			}
		}

		resp.status = "ok";
		resp.message = result;
	}
}
