/**
 * @file EMCP_WB_Clipboard.c
 * @brief Net API handler for copy, cut, paste and duplicate on the World Editor selection.
 *
 * Role: runs the World Editor clipboard actions and reports their boolean result.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Clipboard` here;
 * the enfusion-mcp `wb_clipboard` tool and `cargo xtask mcp wbcall` send them.
 * State: none; the clipboard itself is Workbench's.  Invariants: every action but an
 * unknown one answers "ok" with the API's boolean in `result`; an unknown action or a
 * missing World Editor answers "error".
 */

//! Request wire of `EMCP_WB_Clipboard`: the call's JSON body, decoded by Workbench.
class EMCP_WB_ClipboardRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": copy, cut, paste, pasteAtCursor, duplicate or hasCopied

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_ClipboardRequestWire()
	{
		RegV("action");
	}
}

//! Response wire of `EMCP_WB_Clipboard`: encoded as the call's JSON reply.
class EMCP_WB_ClipboardResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string action; //!< JSON "action": echo of the request action
	bool result; //!< JSON "result": the boolean the WorldEditorAPI call returned

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_ClipboardResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("action");
		RegV("result");
	}
}

//! Net API handler `EMCP_WB_Clipboard`: clipboard actions on the World Editor selection.
class EMCP_WB_Clipboard : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_ClipboardRequestWire();
	}

	//! Runs `action` (copy, cut, paste, pasteAtCursor, duplicate or hasCopied) through
	//! WorldEditorAPI and returns the response wire with the API's boolean in `result`.
	//! Answers "error" when the World Editor or its API is missing or the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_ClipboardRequestWire req = EMCP_WB_ClipboardRequestWire.Cast(request);
		EMCP_WB_ClipboardResponseWire resp = new EMCP_WB_ClipboardResponseWire();
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

		if (req.action == "copy")
		{
			resp.result = api.CopySelectedEntities();
			resp.status = "ok";
			if (resp.result)
				resp.message = "Selected entities copied";
			else
				resp.message = "CopySelectedEntities returned false (nothing selected?)";
		}
		else if (req.action == "cut")
		{
			resp.result = api.CutSelectedEntities();
			resp.status = "ok";
			if (resp.result)
				resp.message = "Selected entities cut";
			else
				resp.message = "CutSelectedEntities returned false (nothing selected?)";
		}
		else if (req.action == "paste")
		{
			resp.result = api.PasteEntities();
			resp.status = "ok";
			if (resp.result)
				resp.message = "Entities pasted at original position";
			else
				resp.message = "PasteEntities returned false (nothing copied?)";
		}
		else if (req.action == "pasteAtCursor")
		{
			resp.result = api.PasteEntitiesAtMouseCursorPos();
			resp.status = "ok";
			if (resp.result)
				resp.message = "Entities pasted at mouse cursor position";
			else
				resp.message = "PasteEntitiesAtMouseCursorPos returned false";
		}
		else if (req.action == "duplicate")
		{
			resp.result = api.DuplicateSelectedEntities();
			resp.status = "ok";
			if (resp.result)
				resp.message = "Selected entities duplicated";
			else
				resp.message = "DuplicateSelectedEntities returned false (nothing selected?)";
		}
		else if (req.action == "hasCopied")
		{
			resp.result = api.HasCopiedEntities();
			resp.status = "ok";
			if (resp.result)
				resp.message = "Clipboard has copied entities";
			else
				resp.message = "Clipboard is empty";
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: copy, cut, paste, pasteAtCursor, duplicate, hasCopied";
		}

		return resp;
	}
}
