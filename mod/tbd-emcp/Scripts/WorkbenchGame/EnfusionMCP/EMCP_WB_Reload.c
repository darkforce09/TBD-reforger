/**
 * @file EMCP_WB_Reload.c
 * @brief Net API handler that triggers a script compile or a plugin reload.
 *
 * Role: runs the Workbench menu actions that compile scripts and reload plugins.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Reload` here;
 * the enfusion-mcp `wb_reload` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: always answers "ok"; `message` joins one line per target
 * with " | "; script compilation tries the Script and Build menus of the ScriptEditor,
 * then the World Editor's Plugins menu, and stops at the first that succeeds.
 */

//! Request wire of `EMCP_WB_Reload`: the call's JSON body, decoded by Workbench.
class EMCP_WB_ReloadRequestWire : JsonApiStruct
{
	string target; //!< JSON "target": "scripts", "plugins" or "both"; empty is "scripts"

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_ReloadRequestWire()
	{
		RegV("target");
	}
}

//! Response wire of `EMCP_WB_Reload`: encoded as the call's JSON reply.
class EMCP_WB_ReloadResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": always "ok"
	string message; //!< JSON "message": human-readable outcome or error

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_ReloadResponseWire()
	{
		RegV("status");
		RegV("message");
	}
}

//! Net API handler `EMCP_WB_Reload`: script compile and plugin reload.
class EMCP_WB_Reload : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_ReloadRequestWire();
	}

	//! Triggers `target` (scripts, plugins or both; empty is scripts) and returns the
	//! response wire with each step's outcome in `message`. Never fails: a missing module is
	//! reported in `message`.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_ReloadRequestWire req = EMCP_WB_ReloadRequestWire.Cast(request);
		EMCP_WB_ReloadResponseWire resp = new EMCP_WB_ReloadResponseWire();

		string target = req.target;
		if (target == "")
			target = "scripts";

		array<string> results = {};

		if (target == "scripts" || target == "both")
		{
			ScriptEditor scriptEditor = Workbench.GetModule(ScriptEditor);
			if (scriptEditor)
			{
				// Try known menu paths for script compilation in ScriptEditor
				array<string> menuPath = {};
				bool compiled = false;

				// Try "Script, Compile" first
				menuPath.Insert("Script");
				menuPath.Insert("Compile");
				compiled = scriptEditor.ExecuteAction(menuPath);

				if (!compiled)
				{
					// Try "Build, Compile All"
					menuPath.Clear();
					menuPath.Insert("Build");
					menuPath.Insert("Compile All");
					compiled = scriptEditor.ExecuteAction(menuPath);
				}

				if (!compiled)
				{
					// Try "Script, Compile All"
					menuPath.Clear();
					menuPath.Insert("Script");
					menuPath.Insert("Compile All");
					compiled = scriptEditor.ExecuteAction(menuPath);
				}

				if (!compiled)
				{
					// Try via WorldEditor as fallback
					WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
					if (worldEditor)
					{
						menuPath.Clear();
						menuPath.Insert("Plugins");
						menuPath.Insert("Reload Scripts");
						compiled = worldEditor.ExecuteAction(menuPath);
					}
				}

				results.Insert("Scripts: compilation triggered (ExecuteAction=" + compiled.ToString() + ")");
			}
			else
			{
				results.Insert("Scripts: ScriptEditor module not available");
			}
		}

		if (target == "plugins" || target == "both")
		{
			ResourceManager resMgr = Workbench.GetModule(ResourceManager);
			if (resMgr)
			{
				array<string> menuPath = {};
				menuPath.Insert("Plugins");
				menuPath.Insert("Reload");
				bool result = resMgr.ExecuteAction(menuPath);
				results.Insert("Plugins: reload triggered (ExecuteAction=" + result.ToString() + ")");
			}
			else
			{
				results.Insert("Plugins: ResourceManager module not available");
			}
		}

		resp.status = "ok";
		string msg = "";
		for (int i = 0; i < results.Count(); i++)
		{
			if (i > 0)
				msg = msg + " | ";
			msg = msg + results[i];
		}
		resp.message = msg;

		return resp;
	}
}
