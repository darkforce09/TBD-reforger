/**
 * @file EMCP_WB_Components.c
 * @brief Net API handler that adds, removes and lists the components of an entity.
 *
 * Role: edits and reads the component list of a named entity source.
 * Position: Workbench's Net API dispatches each call whose APIFunc is `EMCP_WB_Components` here;
 * the enfusion-mcp `wb_component` tool and `cargo xtask mcp wbcall` send them.
 * State: none.  Invariants: add and remove each run inside one Begin/EndEntityAction
 * pair; remove prefers `componentIndex` when it is in range, else the first component
 * of class `componentClass`; the "components" array is written only when non-empty.
 */

//! Request wire of `EMCP_WB_Components`: the call's JSON body, decoded by Workbench.
class EMCP_WB_ComponentsRequestWire : JsonApiStruct
{
	string entityName; //!< JSON "entityName": name of the target entity
	string action; //!< JSON "action": add, remove or list
	string componentClass; //!< JSON "componentClass": class to add, or class to remove when no index is given
	int componentIndex; //!< JSON "componentIndex": index to remove; default -1 (use componentClass)

	//! Registers each field as the JSON key of the same name; `componentIndex` defaults to
	//! -1 (no index).
	void EMCP_WB_ComponentsRequestWire()
	{
		RegV("entityName");
		RegV("action");
		RegV("componentClass");
		RegV("componentIndex");
		componentIndex = -1;
	}
}

//! Response wire of `EMCP_WB_Components`: encoded as the call's JSON reply.
class EMCP_WB_ComponentsResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string entityName; //!< JSON "entityName": echo of the request entity name
	string action; //!< JSON "action": echo of the request action
	int componentCount; //!< JSON "componentCount": the entity's component count after the action

	// Component data for list action
	ref array<string> m_aComponentClasses; //!< class names for the list action; packed into "components" by OnPack
	ref array<int> m_aComponentIndices; //!< component indices, parallel to m_aComponentClasses

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_ComponentsResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("entityName");
		RegV("action");
		RegV("componentCount");

		m_aComponentClasses = {};
		m_aComponentIndices = {};
	}

	//! Writes the "components" array of {className, index} objects when the list action
	//! collected any; writes nothing otherwise.
	override void OnPack()
	{
		if (m_aComponentClasses.Count() > 0)
		{
			StartArray("components");
			for (int i = 0; i < m_aComponentClasses.Count(); i++)
			{
				StartObject("");
				StoreString("className", m_aComponentClasses[i]);
				StoreInteger("index", m_aComponentIndices[i]);
				EndObject();
			}
			EndArray();
		}
	}
}

//! Net API handler `EMCP_WB_Components`: add, remove and list an entity's components.
class EMCP_WB_Components : NetApiHandler
{
	//! Returns the first editor entity source named `name`, or null when no entity has
	//! that name.
	static IEntitySource FindEntityByName(WorldEditorAPI api, string name)
	{
		int count = api.GetEditorEntityCount();
		for (int i = 0; i < count; i++)
		{
			IEntitySource candidate = api.GetEditorEntity(i);
			if (candidate && candidate.GetName() == name)
				return candidate;
		}
		return null;
	}

	//! Returns a new request wire for Workbench to fill from the call's JSON.
	override JsonApiStruct GetRequest()
	{
		return new EMCP_WB_ComponentsRequestWire();
	}

	//! Runs `action` (add, remove or list) on the entity named `entityName` and returns the
	//! response wire, with `componentCount` after the change. Answers "error" when a
	//! parameter, the World Editor, its API, the entity or the component is missing, when the
	//! API refuses, or when the action is unknown.
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		EMCP_WB_ComponentsRequestWire req = EMCP_WB_ComponentsRequestWire.Cast(request);
		EMCP_WB_ComponentsResponseWire resp = new EMCP_WB_ComponentsResponseWire();
		resp.action = req.action;
		resp.entityName = req.entityName;

		if (req.entityName == "")
		{
			resp.status = "error";
			resp.message = "entityName parameter required";
			return resp;
		}

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

		IEntitySource entSrc = FindEntityByName(api, req.entityName);
		if (!entSrc)
		{
			resp.status = "error";
			resp.message = "Entity not found: " + req.entityName;
			return resp;
		}

		if (req.action == "add")
		{
			if (req.componentClass == "")
			{
				resp.status = "error";
				resp.message = "componentClass parameter required for add action";
				return resp;
			}

			api.BeginEntityAction("Add component via NetAPI");
			IEntityComponentSource newComp = api.CreateComponent(entSrc, req.componentClass);
			api.EndEntityAction();

			if (newComp)
			{
				resp.componentCount = entSrc.GetComponentCount();
				resp.status = "ok";
				resp.message = "Component added: " + req.componentClass;
			}
			else
			{
				resp.status = "error";
				resp.message = "CreateComponent returned null for class: " + req.componentClass;
			}
		}
		else if (req.action == "remove")
		{
			int compCount = entSrc.GetComponentCount();

			// Find component by class name or index
			IEntityComponentSource targetComp = null;

			if (req.componentIndex >= 0 && req.componentIndex < compCount)
			{
				targetComp = entSrc.GetComponent(req.componentIndex);
			}
			else if (req.componentClass != "")
			{
				for (int i = 0; i < compCount; i++)
				{
					IEntityComponentSource comp = entSrc.GetComponent(i);
					if (comp && comp.GetClassName() == req.componentClass)
					{
						targetComp = comp;
						break;
					}
				}
			}

			if (!targetComp)
			{
				resp.status = "error";
				resp.message = "Component not found. Specify componentClass or componentIndex.";
				return resp;
			}

			api.BeginEntityAction("Remove component via NetAPI");
			bool deleted = api.DeleteComponent(entSrc, targetComp);
			api.EndEntityAction();

			if (deleted)
			{
				resp.componentCount = entSrc.GetComponentCount();
				resp.status = "ok";
				resp.message = "Component removed";
			}
			else
			{
				resp.status = "error";
				resp.message = "DeleteComponent returned false";
			}
		}
		else if (req.action == "list")
		{
			int compCount = entSrc.GetComponentCount();
			resp.componentCount = compCount;

			for (int i = 0; i < compCount; i++)
			{
				IEntityComponentSource comp = entSrc.GetComponent(i);
				if (comp)
				{
					resp.m_aComponentClasses.Insert(comp.GetClassName());
					resp.m_aComponentIndices.Insert(i);
				}
				else
				{
					resp.m_aComponentClasses.Insert("null");
					resp.m_aComponentIndices.Insert(i);
				}
			}

			resp.status = "ok";
			resp.message = "Components listed: " + compCount.ToString();
		}
		else
		{
			resp.status = "error";
			resp.message = "Unknown action: " + req.action + ". Valid: add, remove, list";
		}

		return resp;
	}
}
