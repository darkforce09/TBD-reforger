/**
 * @file EMCP_WB_Localization.c
 * @brief Net API handler that inserts, deletes, modifies and counts localization rows.
 *
 * Role: edits the string table open in the LocalizationEditor module.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Localization` here;
 * the enfusion-mcp `wb_localization` tool and `cargo xtask mcp wbcall` send them.
 * State: none; the table is Workbench's.  Invariants: every edit runs inside one
 * BeginModify/EndModify pair; modify finds the row by its "Id" and the property by
 * name, answering "error" when either is missing.
 */

//! Request wire of `EMCP_WB_Localization`: the call's JSON body, decoded by Workbench.
class EMCP_WB_LocalizationRequestWire : JsonApiStruct
{
	string action; //!< JSON "action": insert, delete, modify or getTable
	string itemId; //!< JSON "itemId": the row Id
	string property; //!< JSON "property": the row property for modify
	string value; //!< JSON "value": the new property value for modify

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_LocalizationRequestWire()
	{
		RegV("action");
		RegV("itemId");
		RegV("property");
		RegV("value");
	}
}

//! Response wire of `EMCP_WB_Localization`: encoded as the call's JSON reply.
class EMCP_WB_LocalizationResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string action; //!< JSON "action": echo of the request action
	string itemId; //!< JSON "itemId": echo of the request row Id
	int tableItemCount; //!< JSON "tableItemCount": rows in the table for getTable

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_LocalizationResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("action");
		RegV("itemId");
		RegV("tableItemCount");
	}
}

//! Net API handler `EMCP_WB_Localization`: string table edits.
class EMCP_WB_Localization : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_LocalizationRequestWire();
	}

	//! Runs `action` (insert, delete, modify or getTable) on the open string table and
	//! returns the response wire. Answers "error" when the LocalizationEditor, a required
	//! parameter, the table, the row or the property is missing, or the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_LocalizationRequestWire req = EMCP_WB_LocalizationRequestWire.Cast(request);
		EMCP_WB_LocalizationResponseWire resp = new EMCP_WB_LocalizationResponseWire();
		resp.action = req.action;
		resp.itemId = req.itemId;

		LocalizationEditor locEditor = Workbench.GetModule(LocalizationEditor);
		if (!locEditor)
		{
			resp.status = "error";
			resp.message = "LocalizationEditor module not available";
			return resp;
		}

		if (req.action == "insert")
		{
			if (req.itemId == "")
			{
				resp.status = "error";
				resp.message = "itemId parameter required for insert action";
				return resp;
			}

			locEditor.BeginModify("Insert item via NetAPI");
			BaseContainer newItem = locEditor.InsertItem(req.itemId, true, true);
			locEditor.EndModify();

			if (newItem)
			{
				resp.status = "ok";
				resp.message = "Localization item inserted: " + req.itemId;
			}
			else
			{
				resp.status = "error";
				resp.message = "InsertItem returned null for: " + req.itemId;
			}
		}
		else if (req.action == "delete")
		{
			if (req.itemId == "")
			{
				resp.status = "error";
				resp.message = "itemId parameter required for delete action";
				return resp;
			}

			locEditor.BeginModify("Delete item via NetAPI");
			locEditor.DeleteItem(req.itemId);
			locEditor.EndModify();

			resp.status = "ok";
			resp.message = "Localization item deleted: " + req.itemId;
		}
		else if (req.action == "modify")
		{
			if (req.itemId == "" || req.property == "")
			{
				resp.status = "error";
				resp.message = "itemId and property parameters required for modify action";
				return resp;
			}

			// Get the string table to find the item container
			BaseContainer table = locEditor.GetTable();
			if (!table)
			{
				resp.status = "error";
				resp.message = "Could not get string table";
				return resp;
			}

			// Find the item in the table by iterating children
			int childCount = table.GetNumChildren();
			BaseContainer itemContainer = null;
			for (int i = 0; i < childCount; i++)
			{
				BaseContainer child = table.GetChild(i);
				if (!child)
					continue;

				string childId;
				if (child.Get("Id", childId) && childId == req.itemId)
				{
					itemContainer = child;
					break;
				}
			}

			if (!itemContainer)
			{
				resp.status = "error";
				resp.message = "Localization item not found: " + req.itemId;
				return resp;
			}

			// Find the variable index for the property
			int varIdx = itemContainer.GetVarIndex(req.property);
			if (varIdx < 0)
			{
				resp.status = "error";
				resp.message = "Property not found: " + req.property;
				return resp;
			}

			locEditor.BeginModify("Modify property via NetAPI");
			locEditor.ModifyProperty(itemContainer, varIdx, req.value);
			locEditor.EndModify();

			resp.status = "ok";
			resp.message = "Property '" + req.property + "' set to '" + req.value + "' on item: " + req.itemId;
		}
		else if (req.action == "getTable")
		{
			BaseContainer table = locEditor.GetTable();
			if (table)
			{
				resp.tableItemCount = table.GetNumChildren();
				resp.status = "ok";
				resp.message = "String table has " + resp.tableItemCount.ToString() + " items";
			}
			else
			{
				resp.status = "error";
				resp.message = "Could not get string table (no localization file loaded?)";
			}
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: insert, delete, modify, getTable";
		}

		return resp;
	}
}
