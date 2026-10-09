/**
 * @file EMCP_WB_ListEntities.c
 * @brief Net API handler that lists editor entities, paginated and filtered by name.
 *
 * Role: pages through the editor's entity sources with an optional case-insensitive
 * name filter.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_ListEntities` here;
 * the enfusion-mcp `wb_entity_list` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: `totalCount` counts every match, `returnedCount` the page;
 * a non-positive limit is 50 and a negative offset is 0; an entity with no runtime
 * entity reports position "0 0 0".
 */

//! Request wire of `EMCP_WB_ListEntities`: the call's JSON body, decoded by Workbench.
class EMCP_WB_ListEntitiesRequestWire : JsonApiStruct
{
	int offset; //!< JSON "offset": matches to skip; negative is 0
	int limit; //!< JSON "limit": page size; 0 or less is 50
	string nameFilter; //!< JSON "nameFilter": case-insensitive substring; empty matches all

	//! Registers each field as the JSON key of the same name.
	void EMCP_WB_ListEntitiesRequestWire()
	{
		RegV("offset");
		RegV("limit");
		RegV("nameFilter");
	}
}

//! Response wire of `EMCP_WB_ListEntities`: encoded as the call's JSON reply.
class EMCP_WB_ListEntitiesResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	int totalCount; //!< JSON "totalCount": entities matching the filter
	int returnedCount; //!< JSON "returnedCount": entities in this page
	int offset; //!< JSON "offset": the offset applied

	// Entity data collected before OnPack
	ref array<string> m_aNames; //!< entity names of the page; packed into "entities" by OnPack
	ref array<string> m_aClassNames; //!< entity class names, parallel to m_aNames
	ref array<string> m_aPositions; //!< entity positions, "x y z" in metres, parallel to m_aNames

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_ListEntitiesResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("totalCount");
		RegV("returnedCount");
		RegV("offset");

		m_aNames = {};
		m_aClassNames = {};
		m_aPositions = {};
	}

	//! Writes the "entities" array of {name, className, position} objects for the page.
	override void OnPack()
	{
		StartArray("entities");
		for (int i = 0; i < m_aNames.Count(); i++)
		{
			StartObject("");
			StoreString("name", m_aNames[i]);
			StoreString("className", m_aClassNames[i]);
			StoreString("position", m_aPositions[i]);
			EndObject();
		}
		EndArray();
	}
}

//! Net API handler `EMCP_WB_ListEntities`: paginated entity listing.
class EMCP_WB_ListEntities : NetApiHandler
{
	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_ListEntitiesRequestWire();
	}

	//! Collects the page of entities matching `nameFilter` from `offset` up to `limit` and
	//! returns the response wire. Answers "error" when the World Editor or its API is
	//! missing.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_ListEntitiesRequestWire req = EMCP_WB_ListEntitiesRequestWire.Cast(request);
		EMCP_WB_ListEntitiesResponseWire resp = new EMCP_WB_ListEntitiesResponseWire();

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

		int entityCount = api.GetEditorEntityCount();
		int pageLimit = req.limit;
		if (pageLimit <= 0)
			pageLimit = 50;

		int pageOffset = req.offset;
		if (pageOffset < 0)
			pageOffset = 0;

		string filter = req.nameFilter;
		filter.ToLower();

		// Collect matching entities with pagination
		int matched = 0;
		int skipped = 0;
		resp.totalCount = 0;

		for (int i = 0; i < entityCount; i++)
		{
			IEntitySource entSrc = api.GetEditorEntity(i);
			if (!entSrc)
				continue;

			string entName = entSrc.GetName();

			// Apply name filter
			if (filter != "")
			{
				string lowerName = entName;
				lowerName.ToLower();
				if (lowerName.IndexOf(filter) < 0)
					continue;
			}

			resp.totalCount++;

			// Pagination: skip until offset
			if (skipped < pageOffset)
			{
				skipped++;
				continue;
			}

			// Pagination: stop at limit
			if (matched >= pageLimit)
				continue;

			string className = entSrc.GetClassName();

			// Get position from the runtime entity
			string posStr = "0 0 0";
			IEntity ent = api.SourceToEntity(entSrc);
			if (ent)
			{
				vector pos = ent.GetOrigin();
				posStr = pos[0].ToString() + " " + pos[1].ToString() + " " + pos[2].ToString();
			}

			resp.m_aNames.Insert(entName);
			resp.m_aClassNames.Insert(className);
			resp.m_aPositions.Insert(posStr);
			matched++;
		}

		resp.returnedCount = matched;
		resp.offset = pageOffset;
		resp.status = "ok";
		resp.message = "Listed " + matched.ToString() + " of " + resp.totalCount.ToString() + " entities";

		return resp;
	}
}
