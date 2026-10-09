/**
 * @file EMCP_WB_Resources.c
 * @brief Net API handler that registers, rebuilds and opens resource files.
 *
 * Role: runs the ResourceManager module's register, rebuild and open calls on a path.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Resources` here;
 * the enfusion-mcp `wb_resources` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: register and open answer "ok" with the call's result in
 * `message`; rebuild is fire-and-forget; an empty path answers "error".
 */

//! Request wire of `EMCP_WB_Resources`: the call's JSON body, decoded by Workbench.
class EMCP_WB_ResourcesRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": register, rebuild or open
	string path; //!< JSON "path": the resource file path
	bool buildRuntime; //!< JSON "buildRuntime": register also builds the runtime resource

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_ResourcesRequestWire()
	{
		RegV("action");
		RegV("path");
		RegV("buildRuntime");
	}
}

//! Response wire of `EMCP_WB_Resources`: encoded as the call's JSON reply.
class EMCP_WB_ResourcesResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string action; //!< JSON "action": echo of the request action
	string path; //!< JSON "path": echo of the request path

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_ResourcesResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("action");
		RegV("path");
	}
}

//! Net API handler `EMCP_WB_Resources`: resource file operations.
class EMCP_WB_Resources : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_ResourcesRequestWire();
	}

	//! Runs `action` (register, rebuild or open) on `path` and returns the response wire.
	//! Answers "error" when `path` is empty, the ResourceManager is missing, or the action
	//! is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_ResourcesRequestWire req = EMCP_WB_ResourcesRequestWire.Cast(request);
		EMCP_WB_ResourcesResponseWire resp = new EMCP_WB_ResourcesResponseWire();
		resp.action = req.action;
		resp.path = req.path;

		if (req.path == "")
		{
			resp.status = "error";
			resp.message = "path parameter required";
			return resp;
		}

		ResourceManager resMgr = Workbench.GetModule(ResourceManager);
		if (!resMgr)
		{
			resp.status = "error";
			resp.message = "ResourceManager module not available";
			return resp;
		}

		if (req.action == "register")
		{
			bool result = resMgr.RegisterResourceFile(req.path, req.buildRuntime);
			resp.status = "ok";
			if (result)
				resp.message = "Resource registered: " + req.path;
			else
				resp.message = "RegisterResourceFile returned false for: " + req.path;
		}
		else if (req.action == "rebuild")
		{
			resMgr.RebuildResourceFile(req.path, "", false);
			resp.status = "ok";
			resp.message = "Rebuild initiated for: " + req.path;
		}
		else if (req.action == "open")
		{
			bool result = resMgr.SetOpenedResource(req.path);
			resp.status = "ok";
			if (result)
				resp.message = "Opened resource: " + req.path;
			else
				resp.message = "SetOpenedResource returned false for: " + req.path;
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: register, rebuild, open";
		}

		return resp;
	}
}
