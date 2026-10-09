/**
 * @file EMCP_WB_ModifyEntityArrayMemberActions.c
 * @brief The object-array actions of the EMCP_WB_ModifyEntity Net API handler.
 *
 * Role: lists, adds and removes the members of an array-of-objects property.  Position: called by
 * EMCP_WB_ModifyEntity.GetResponse for "listArrayItems", "addArrayItem" and "removeArrayItem";
 * reads BaseContainerList and writes through WorldEditorAPI.
 * State: none.  Invariants: `propertyPath` names a component class, or is empty for the entity;
 * adding and removing use the matched component as the top-level container with no path, because
 * the API refuses component arrays reached through a path from the entity; a path that names no
 * component is used as a dot-separated container path from the entity.
 */

//! The object-array actions of `EMCP_WB_ModifyEntity`: list, add and remove array members.
class EMCP_WB_ModifyEntityArrayMemberActions
{
	//! Writes "[index:className, ...] (n items)" into `resp.message` for the object array
	//! `req.propertyKey` of the component named by `req.propertyPath`, or of the entity. Answers
	//! "ok" with an empty list when the key holds no object array; "error" when the key is empty
	//! or no component has that class name.
	static void ListArrayItems(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		// Reads an array-of-objects property and returns each item's class name and index.
		// propertyPath = component class name (or "" for entity level)
		// propertyKey  = array property name (e.g. "Slots", "m_aTriggerActions")
		if (req.propertyKey == "")
		{
			resp.status = "error";
			resp.message = "propertyKey (array name) required for listArrayItems";
			return;
		}

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

		BaseContainerList itemList = null;
		if (compSrc)
			itemList = compSrc.GetObjectArray(req.propertyKey);
		else
			itemList = entSrc.GetObjectArray(req.propertyKey);

		if (!itemList)
		{
			resp.status = "ok";
			resp.message = "[] (empty or not an object array)";
			return;
		}

		string listResult = "";
		int itemCount = itemList.Count();
		for (int li = 0; li < itemCount; li++)
		{
			BaseContainer item = itemList.Get(li);
			string className = "";
			if (item)
				className = item.GetClassName();
			else
				className = "(null)";
			if (listResult != "") listResult += ", ";
			listResult += li.ToString() + ":" + className;
		}

		resp.status = "ok";
		resp.message = "[" + listResult + "] (" + itemCount.ToString() + " items)";
	}

	//! Creates a member of class `req.value` in the object array `req.propertyKey` at
	//! `req.memberIndex` (a negative index appends) as one undoable action. Answers "error" when
	//! the key or the class is empty or CreateObjectArrayVariableMember returns false.
	static void AddArrayItem(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		// Creates a new element in an array-of-objects property (the + button in the editor).
		// propertyPath = component class name (or "" for entity level)
		// propertyKey  = array property name (e.g. "m_aTriggerActions")
		// value        = class name of the new item (e.g. "SCR_ScenarioFrameworkActionSpawnObjects")
		// memberIndex  = index to insert at (-1 = append at end)
		if (req.propertyKey == "" || req.value == "")
		{
			resp.status = "error";
			resp.message = "propertyKey (array name) and value (item class name) required for addArrayItem";
			return;
		}

		// Use component as topLevel if propertyPath is a component class name.
		// NOTE: CreateObjectArrayVariableMember requires the component as topLevel with null path --
		// passing the entity with a path entry returns false for component arrays.
		BaseContainer addTopLevel = entSrc;
		array<ref ContainerIdPathEntry> pathEntries = null;
		if (req.propertyPath != "")
		{
			IEntityComponentSource addC = EMCP_WB_ModifyEntity.FindComponentByClassName(entSrc, req.propertyPath);
			if (addC)
				addTopLevel = addC;
			// If not found as component, fall back to path entries
			if (addTopLevel == entSrc)
				pathEntries = EMCP_WB_ModifyEntity.BuildPathEntries(req.propertyPath);
		}

		int insertIdx = req.memberIndex;
		if (insertIdx < 0)
			insertIdx = -1; // will be treated as append by the API

		api.BeginEntityAction("Add array item via NetAPI");
		bool result = api.CreateObjectArrayVariableMember(addTopLevel, pathEntries, req.propertyKey, req.value, insertIdx);
		api.EndEntityAction();

		if (result)
		{
			resp.status = "ok";
			resp.message = "Added '" + req.value + "' to '" + req.propertyKey + "' at index " + insertIdx;
		}
		else
		{
			resp.status = "error";
			resp.message = "CreateObjectArrayVariableMember returned false -- check class name and property key";
		}
	}

	//! Removes the member at the 0-based `req.memberIndex` from the object array
	//! `req.propertyKey` as one undoable action. Answers "error" when the key is empty or
	//! RemoveObjectArrayVariableMember returns false.
	static void RemoveArrayItem(WorldEditorAPI api, IEntitySource entSrc, EMCP_WB_ModifyEntityRequestWire req, EMCP_WB_ModifyEntityResponseWire resp)
	{
		// Removes an element from an array-of-objects property by index.
		// propertyPath = component class name (or "" for entity level)
		// propertyKey  = array property name
		// memberIndex  = 0-based index to remove
		if (req.propertyKey == "")
		{
			resp.status = "error";
			resp.message = "propertyKey (array name) required for removeArrayItem";
			return;
		}

		// Use component as topLevel if propertyPath is a component class name.
		// NOTE: RemoveObjectArrayVariableMember requires the component as topLevel with null path --
		// passing the entity with a path entry returns false for component arrays.
		BaseContainer removeTopLevel = entSrc;
		array<ref ContainerIdPathEntry> removePathEntries = null;
		if (req.propertyPath != "")
		{
			IEntityComponentSource removeC = EMCP_WB_ModifyEntity.FindComponentByClassName(entSrc, req.propertyPath);
			if (removeC)
				removeTopLevel = removeC;
			if (removeTopLevel == entSrc)
				removePathEntries = EMCP_WB_ModifyEntity.BuildPathEntries(req.propertyPath);
		}

		api.BeginEntityAction("Remove array item via NetAPI");
		bool result = api.RemoveObjectArrayVariableMember(removeTopLevel, removePathEntries, req.propertyKey, req.memberIndex);
		api.EndEntityAction();

		if (result)
		{
			resp.status = "ok";
			resp.message = "Removed index " + req.memberIndex + " from '" + req.propertyKey + "'";
		}
		else
		{
			resp.status = "error";
			resp.message = "RemoveObjectArrayVariableMember returned false -- check index and property key";
		}
	}
}
