/**
 * @file EMCP_WB_EditorControl.c
 * @brief Net API handler that plays, stops, saves, undoes, redoes and opens resources.
 *
 * Role: drives the World Editor's mode switch, save, undo and redo, and resource opening.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_EditorControl` here;
 * the enfusion-mcp `wb_play`, `wb_stop`, `wb_save`, `wb_undo_redo`, `wb_open_resource` and `wb_projects` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: saveAs runs a plain Save; undo and redo run the Edit menu
 * actions; save, saveAs and openResource answer "ok" even when the call returns false,
 * with the result in `message`.
 */

//! Request wire of `EMCP_WB_EditorControl`: the call's JSON body, decoded by Workbench.
class EMCP_WB_EditorControlRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": play, stop, save, saveAs, undo, redo or openResource
	bool debugMode; //!< JSON "debugMode": play with the debug flag
	bool fullScreen; //!< JSON "fullScreen": play full screen
	string path; //!< JSON "path": resource path for openResource

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_EditorControlRequestWire()
	{
		RegV("action");
		RegV("debugMode");
		RegV("fullScreen");
		RegV("path");
	}
}

//! Response wire of `EMCP_WB_EditorControl`: encoded as the call's JSON reply.
class EMCP_WB_EditorControlResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string action; //!< JSON "action": echo of the request action
	string message; //!< JSON "message": human-readable outcome or error

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_EditorControlResponseWire()
	{
		RegV("status");
		RegV("action");
		RegV("message");
	}
}

//! Net API handler `EMCP_WB_EditorControl`: World Editor mode, save, undo and redo.
class EMCP_WB_EditorControl : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_EditorControlRequestWire();
	}

	//! Runs `action` (play, stop, save, saveAs, undo, redo or openResource) on the World
	//! Editor and returns the response wire. Answers "error" when the World Editor is missing,
	//! undo finds no API, openResource has no `path`, or the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_EditorControlRequestWire req = EMCP_WB_EditorControlRequestWire.Cast(request);
		EMCP_WB_EditorControlResponseWire resp = new EMCP_WB_EditorControlResponseWire();
		resp.action = req.action;

		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			resp.status = "error";
			resp.message = "WorldEditor module not available";
			return resp;
		}

		if (req.action == "play")
		{
			worldEditor.SwitchToGameMode(req.debugMode, req.fullScreen);
			resp.status = "ok";
			resp.message = "Switched to game mode";
		}
		else if (req.action == "stop")
		{
			worldEditor.SwitchToEditMode();
			resp.status = "ok";
			resp.message = "Switched to edit mode";
		}
		else if (req.action == "save")
		{
			bool saved = worldEditor.Save();
			resp.status = "ok";
			if (saved)
				resp.message = "World saved";
			else
				resp.message = "Save returned false (may already be up to date)";
		}
		else if (req.action == "saveAs")
		{
			// WorldEditor does not expose SaveAs directly; fall back to Save
			bool saved = worldEditor.Save();
			resp.status = "ok";
			resp.message = "SaveAs not available, used Save instead";
		}
		else if (req.action == "undo")
		{
			WorldEditorAPI api = worldEditor.GetApi();
			if (api)
			{
				// Undo is available via GameWorldEditor or WorldEditorIngame
				// Use ExecuteAction as a safe fallback
				array<string> menuPath = {};
				menuPath.Insert("Edit");
				menuPath.Insert("Undo");
				worldEditor.ExecuteAction(menuPath);
				resp.status = "ok";
				resp.message = "Undo executed";
			}
			else
			{
				resp.status = "error";
				resp.message = "WorldEditorAPI not available for undo";
			}
		}
		else if (req.action == "redo")
		{
			array<string> menuPath = {};
			menuPath.Insert("Edit");
			menuPath.Insert("Redo");
			worldEditor.ExecuteAction(menuPath);
			resp.status = "ok";
			resp.message = "Redo executed";
		}
		else if (req.action == "openResource")
		{
			if (req.path == "")
			{
				resp.status = "error";
				resp.message = "path parameter required for openResource action";
			}
			else
			{
				bool opened = worldEditor.SetOpenedResource(req.path);
				resp.status = "ok";
				if (opened)
					resp.message = "Opened resource: " + req.path;
				else
					resp.message = "SetOpenedResource returned false for: " + req.path;
			}
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: play, stop, save, saveAs, undo, redo, openResource";
		}

		return resp;
	}
}
