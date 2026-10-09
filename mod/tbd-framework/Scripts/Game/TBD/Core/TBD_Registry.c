/**
 * @file TBD_Registry.c
 * @brief Alias to prefab resource resolution from the mod's registry file.
 *
 * Role: loads `Data/registry.json` (or a `$profile:` copy) once and resolves a registry alias to
 * its `"{GUID}path"` prefab.
 * Position: called by the spawn, loadout and mission code that names prefabs by alias, and by
 * `TBD_RegistryPocComponent`.
 * State: the alias map and the loaded flag, for the life of the script VM.
 * Invariants: the mod copy wins over the profile copy; an entry without alias or guid is skipped;
 * an unknown alias logs an ERROR and resolves to empty.
 */

//! One registry entry. JSON keys are the field names.
//! @contract registry.schema.json#/$defs/entry
class TBD_RegistryEntryStruct
{
	string alias; //!< JSON "alias": the name missions use
	string guid; //!< JSON "guid": the `"{GUID}path"` prefab resource
	string displayName; //!< JSON "displayName": the human-readable name
}

//! The registry document. JSON keys are the field names; other keys are ignored.
//! @contract registry.schema.json#/
class TBD_RegistryDocumentStruct
{
	string registryVersion; //!< JSON "registryVersion"
	ref array<ref TBD_RegistryEntryStruct> entries; //!< JSON "entries"
}

//! Alias -> prefab resource name resolution. Registry ships in mod Data/registry.json.
class TBD_Registry
{
	protected static ref map<string, ResourceName> s_AliasToResource; //!< alias -> prefab resource; filled by Load
	protected static bool s_Loaded; //!< true once Load succeeded

	protected static const string REGISTRY_PATH_MOD = "$TBD_Framework:Data/registry.json"; //!< the registry shipped with the mod
	protected static const string REGISTRY_PATH_PROFILE = "$profile:TBD_Registry.json"; //!< the fallback copy in the server profile

	//! Load the registry once, from the mod copy or else the profile copy.
	//! @return true when the registry is loaded; false with an ERROR when the file is missing,
	//! unreadable or unparsable
	static bool Load()
	{
		if (s_Loaded)
			return true;

		s_AliasToResource = new map<string, ResourceName>();

		string path = REGISTRY_PATH_MOD;
		if (!FileIO.FileExists(path) && FileIO.FileExists(REGISTRY_PATH_PROFILE))
			path = REGISTRY_PATH_PROFILE;

		if (!FileIO.FileExists(path))
		{
			Print("[TBD] Registry file missing (mod and profile). Run: cargo xtask setup server-profile", LogLevel.ERROR);
			return false;
		}

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromFile(path))
		{
			Print("[TBD] Failed to read registry.", LogLevel.ERROR);
			return false;
		}

		ref TBD_RegistryDocumentStruct doc = new TBD_RegistryDocumentStruct();
		if (!ctx.ReadValue("", doc) || !doc.entries)
		{
			Print("[TBD] Failed to parse registry.", LogLevel.ERROR);
			return false;
		}

		foreach (TBD_RegistryEntryStruct entry : doc.entries)
		{
			if (!entry || entry.alias.IsEmpty() || entry.guid.IsEmpty())
				continue;

			s_AliasToResource.Insert(entry.alias, entry.guid);
		}

		s_Loaded = true;
		Print(string.Format("[TBD] Registry loaded (%1 aliases).", s_AliasToResource.Count()));
		return true;
	}

	//! Resolve `alias`, loading the registry on first use.
	//! @param ok set true when the alias resolved
	//! @return the prefab resource, or empty (with an ERROR for an unknown alias)
	static ResourceName Resolve(string alias, out bool ok)
	{
		ok = false;
		if (!s_Loaded && !Load())
			return string.Empty;

		ResourceName res;
		if (!s_AliasToResource.Find(alias, res))
		{
			Print("[TBD] Unknown registry alias: " + alias, LogLevel.ERROR);
			return string.Empty;
		}

		ok = true;
		return res;
	}

	//! @return every loaded alias, loading the registry on first use
	static array<string> GetAllAliases()
	{
		array<string> aliases = {};
		if (!s_Loaded)
			Load();

		foreach (string alias, ResourceName res : s_AliasToResource)
		{
			aliases.Insert(alias);
		}

		return aliases;
	}
}
