/**
 * @file EMCP_WB_GetEntityResponseWire.c
 * @brief Response wire of the EMCP_WB_GetEntity Net API handler.
 *
 * Role: carries one entity's details as the call's JSON reply.  Position: filled by
 * EMCP_WB_GetEntity.GetResponse; encoded by Workbench's Net API.
 * State: the collected variable and component lists, owned by one call.
 * Invariants: OnPack always writes "properties" and "components", empty when nothing was
 * collected; `m_aVarValues` stays parallel to `m_aVarNames`.
 */

//! Response wire of `EMCP_WB_GetEntity`: encoded as the call's JSON reply.
class EMCP_WB_GetEntityResponseWire : JsonApiStruct
{
	string status; //!< JSON "status": "ok" or "error"
	string message; //!< JSON "message": human-readable outcome or error
	string name; //!< JSON "name": entity name
	string className; //!< JSON "className": entity class
	string position; //!< JSON "position": runtime origin, "x y z" in metres
	string rotation; //!< JSON "rotation": runtime angles, "x y z" in degrees
	int componentCount; //!< JSON "componentCount": components on the entity source
	int layerID; //!< JSON "layerID": the entity's layer
	int subScene; //!< JSON "subScene": the entity's sub-scene
	int varCount; //!< JSON "varCount": variables on the entity source, before the 50-entry cap

	// Properties collected before OnPack
	ref array<string> m_aVarNames; //!< variable names, at most 50; packed into "properties" by OnPack
	ref array<string> m_aVarValues; //!< variable default values as strings, parallel to m_aVarNames
	ref array<string> m_aComponentClasses; //!< component class names; packed into "components" by OnPack

	//! Registers each scalar field as the JSON key of the same name.
	void EMCP_WB_GetEntityResponseWire()
	{
		RegV("status");
		RegV("message");
		RegV("name");
		RegV("className");
		RegV("position");
		RegV("rotation");
		RegV("componentCount");
		RegV("layerID");
		RegV("subScene");
		RegV("varCount");

		m_aVarNames = {};
		m_aVarValues = {};
		m_aComponentClasses = {};
	}

	//! Writes the "properties" array of {name, value} objects and the "components" array
	//! of {className, index} objects.
	override void OnPack()
	{
		// Pack properties array
		StartArray("properties");
		for (int i = 0; i < m_aVarNames.Count(); i++)
		{
			StartObject("");
			StoreString("name", m_aVarNames[i]);
			StoreString("value", m_aVarValues[i]);
			EndObject();
		}
		EndArray();

		// Pack components array
		StartArray("components");
		for (int i = 0; i < m_aComponentClasses.Count(); i++)
		{
			StartObject("");
			StoreString("className", m_aComponentClasses[i]);
			StoreInteger("index", i);
			EndObject();
		}
		EndArray();
	}
}
