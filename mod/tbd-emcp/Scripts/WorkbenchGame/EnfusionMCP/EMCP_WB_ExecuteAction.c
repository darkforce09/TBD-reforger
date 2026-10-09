/**
 * @file EMCP_WB_ExecuteAction.c
 * @brief Net API handler that runs a World Editor menu action by its menu path.
 *
 * Role: turns a comma-separated menu path such as "Edit,Select All" into a
 * WorldEditor.ExecuteAction call.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_ExecuteAction` here;
 * the enfusion-mcp `wb_execute_action` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: the reply is "ok" whenever the call is made, with its
 * boolean in `message`; an empty path, a missing World Editor or a path with no parts
 * answers "error".
 */

//! Request wire of `EMCP_WB_ExecuteAction`: the call's JSON body, decoded by Workbench.
class EMCP_WB_ExecuteActionRequestWire : JsonApiStruct
{
	string menuPath; //!< JSON "menuPath": comma-separated menu path, e.g. "Edit,Select All"

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_ExecuteActionRequestWire()
	{
		RegV("menuPath");
	}
}

//! Response wire of `EMCP_WB_ExecuteAction`: encoded as the call's JSON reply.
class EMCP_WB_ExecuteActionResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string menuPath; //!< JSON "menuPath": echo of the request menu path
	string message; //!< JSON "message": human-readable outcome or error

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_ExecuteActionResponseWire()
	{
		RegV("status");
		RegV("menuPath");
		RegV("message");
	}
}

//! Net API handler `EMCP_WB_ExecuteAction`: run a World Editor menu action.
class EMCP_WB_ExecuteAction : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_ExecuteActionRequestWire();
	}

	//! Splits `menuPath` on commas, runs the menu action it names and returns the response
	//! wire. Answers "error" when `menuPath` is empty or has no parts or the World Editor is
	//! missing.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_ExecuteActionRequestWire req = EMCP_WB_ExecuteActionRequestWire.Cast(request);
		EMCP_WB_ExecuteActionResponseWire resp = new EMCP_WB_ExecuteActionResponseWire();
		resp.menuPath = req.menuPath;

		if (req.menuPath == "")
		{
			resp.status = "error";
			resp.message = "menuPath parameter required (comma-separated, e.g. 'Edit,Select All')";
			return resp;
		}

		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			resp.status = "error";
			resp.message = "WorldEditor module not available";
			return resp;
		}

		// Split menuPath on commas
		array<string> parts = {};
		string remaining = req.menuPath;
		int commaIdx = remaining.IndexOf(",");
		while (commaIdx >= 0)
		{
			string part = remaining.Substring(0, commaIdx);
			part.Trim();
			parts.Insert(part);
			remaining = remaining.Substring(commaIdx + 1, remaining.Length() - commaIdx - 1);
			commaIdx = remaining.IndexOf(",");
		}
		remaining.Trim();
		if (remaining.Length() > 0)
			parts.Insert(remaining);

		if (parts.Count() == 0)
		{
			resp.status = "error";
			resp.message = "menuPath resolved to empty array";
			return resp;
		}

		bool result = worldEditor.ExecuteAction(parts);
		resp.status = "ok";
		if (result)
			resp.message = "Action executed successfully";
		else
			resp.message = "ExecuteAction returned false (action may not exist or is unavailable)";

		return resp;
	}
}
