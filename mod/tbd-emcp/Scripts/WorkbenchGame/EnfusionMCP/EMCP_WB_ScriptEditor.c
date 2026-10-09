/**
 * @file EMCP_WB_ScriptEditor.c
 * @brief Net API handler that reads and edits the open script line by line or whole.
 *
 * Role: runs the ScriptEditor module's file, line and open calls, plus getAllText,
 * which returns every line of the open file.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_ScriptEditor` here;
 * the enfusion-mcp `wb_script_editor` tool and `cargo xtask mcp wbcall` send them.
 * State: none; the open file is Workbench's.  Invariants: getAllText reads lines 1 to
 * the line count and ends each with a newline, an unreadable line as an empty one;
 * setLine, insertLine and removeLine answer "ok" without checking the result.
 */

//! Request wire of `EMCP_WB_ScriptEditor`: the call's JSON body, decoded by Workbench.
class EMCP_WB_ScriptEditorRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": the script editor action
	int line; //!< JSON "line": line number for the line actions; default -1
	string text; //!< JSON "text": line text for setLine and insertLine
	string path; //!< JSON "path": file for openFile

	//! Registers each field as the JSON key of the same name; `line` defaults to -1.
	void EMCP_WB_ScriptEditorRequestWire()
	{
		RegV("action");
		RegV("line");
		RegV("text");
		RegV("path");
		line = -1;
	}
}

//! Response wire of `EMCP_WB_ScriptEditor`: encoded as the call's JSON reply.
class EMCP_WB_ScriptEditorResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string action; //!< JSON "action": echo of the request action
	string currentFile; //!< JSON "currentFile": the open file for getCurrentFile
	int currentLine; //!< JSON "currentLine": the cursor line for getCurrentFile
	int linesCount; //!< JSON "linesCount": lines in the open file
	string lineText; //!< JSON "lineText": the line for getLine; the whole file for getAllText
	string text; //!< JSON "text": the whole open file for getAllText

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_ScriptEditorResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("action");
		RegV("currentFile");
		RegV("currentLine");
		RegV("linesCount");
		RegV("lineText");
		RegV("text");
	}
}

//! Net API handler `EMCP_WB_ScriptEditor`: script editor reads and edits.
class EMCP_WB_ScriptEditor : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_ScriptEditorRequestWire();
	}

	//! Runs `action` (getCurrentFile, getLine, setLine, insertLine, removeLine,
	//! getLinesCount, getAllText or openFile) and returns the response wire. Answers "error"
	//! when the ScriptEditor is missing, getLine fails, openFile has no `path`, or the action
	//! is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_ScriptEditorRequestWire req = EMCP_WB_ScriptEditorRequestWire.Cast(request);
		EMCP_WB_ScriptEditorResponseWire resp = new EMCP_WB_ScriptEditorResponseWire();
		resp.action = req.action;

		ScriptEditor scriptEditor = Workbench.GetModule(ScriptEditor);
		if (!scriptEditor)
		{
			resp.status = "error";
			resp.message = "ScriptEditor module not available";
			return resp;
		}

		if (req.action == "getCurrentFile")
		{
			string filename;
			bool result = scriptEditor.GetCurrentFile(filename);
			if (result)
			{
				resp.currentFile = filename;
				resp.currentLine = scriptEditor.GetCurrentLine();
				resp.linesCount = scriptEditor.GetLinesCount();
				resp.status = "ok";
				resp.message = "Current file: " + filename;
			}
			else
			{
				resp.status = "ok";
				resp.message = "No file currently open in script editor";
			}
		}
		else if (req.action == "getLine")
		{
			string lineText;
			bool result = scriptEditor.GetLineText(lineText, req.line);
			if (result)
			{
				resp.lineText = lineText;
				resp.status = "ok";
				resp.message = "Line " + req.line.ToString() + " retrieved";
			}
			else
			{
				resp.status = "error";
				resp.message = "GetLineText returned false for line " + req.line.ToString();
			}
		}
		else if (req.action == "setLine")
		{
			scriptEditor.SetLineText(req.text, req.line);
			resp.status = "ok";
			resp.message = "Line " + req.line.ToString() + " set";
		}
		else if (req.action == "insertLine")
		{
			scriptEditor.InsertLine(req.text, req.line);
			resp.status = "ok";
			resp.message = "Line inserted at " + req.line.ToString();
		}
		else if (req.action == "removeLine")
		{
			scriptEditor.RemoveLine(req.line);
			resp.status = "ok";
			resp.message = "Line " + req.line.ToString() + " removed";
		}
		else if (req.action == "getLinesCount")
		{
			resp.linesCount = scriptEditor.GetLinesCount();
			resp.status = "ok";
			resp.message = "Lines count: " + resp.linesCount.ToString();
		}
		else if (req.action == "getAllText")
		{
			int count = scriptEditor.GetLinesCount();
			resp.linesCount = count;
			string all;
			for (int i = 1; i <= count; i++)
			{
				string lineText;
				if (scriptEditor.GetLineText(lineText, i))
					all += lineText;
				all += "\n";
			}
			resp.text = all;
			resp.lineText = all;
			resp.status = "ok";
			resp.message = "Dumped " + count.ToString() + " lines";
		}
		else if (req.action == "openFile")
		{
			if (req.path == "")
			{
				resp.status = "error";
				resp.message = "path parameter required for openFile action";
				return resp;
			}

			bool result = scriptEditor.SetOpenedResource(req.path);
			resp.status = "ok";
			if (result)
				resp.message = "Opened file: " + req.path;
			else
				resp.message = "SetOpenedResource returned false for: " + req.path;
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: getCurrentFile, getLine, setLine, insertLine, removeLine, getLinesCount, getAllText, openFile";
		}

		return resp;
	}
}
