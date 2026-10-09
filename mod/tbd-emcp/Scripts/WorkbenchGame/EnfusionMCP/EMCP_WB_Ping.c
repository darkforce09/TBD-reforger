/**
 * @file EMCP_WB_Ping.c
 * @brief Net API handler that answers the bridge health check with the editor mode.
 *
 * Role: proves the Net API bridge is loaded and reports edit or game mode.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Ping` here;
 * the enfusion-mcp `wb_connect` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: always answers "ok"; `mode` is "edit", "game" or
 * "no_world_editor"; `cargo xtask mod dev-bootstrap` requires this file to exist.
 */

//! Request wire of `EMCP_WB_Ping`: the call's JSON body, decoded by Workbench.
class EMCP_WB_PingRequestWire : JsonApiStruct
{
	//! Takes no parameters.
	void EMCP_WB_PingRequestWire()
	{
		// No request parameters needed for ping
	}
}

//! Response wire of `EMCP_WB_Ping`: encoded as the call's JSON reply.
class EMCP_WB_PingResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string mode; //!< JSON "mode": "edit", "game" or "no_world_editor"
	string message; //!< JSON "message": human-readable outcome or error

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_PingResponseWire()
	{
		RegV("status");
		RegV("mode");
		RegV("message");
	}
}

//! Net API handler `EMCP_WB_Ping`: the bridge health check.
class EMCP_WB_Ping : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_PingRequestWire();
	}

	//! Returns the response wire with status "ok" and the current editor mode. Never fails.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_PingResponseWire resp = new EMCP_WB_PingResponseWire();

		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			resp.status = "ok";
			resp.mode = "no_world_editor";
			resp.message = "EnfusionMCP Workbench bridge active (no WorldEditor module)";
			return resp;
		}

		WorldEditorAPI api = worldEditor.GetApi();
		if (api)
		{
			resp.status = "ok";
			resp.mode = "edit";
			resp.message = "EnfusionMCP Workbench bridge active";
		}
		else
		{
			resp.status = "ok";
			resp.mode = "game";
			resp.message = "EnfusionMCP Workbench bridge active (game mode)";
		}

		return resp;
	}
}
