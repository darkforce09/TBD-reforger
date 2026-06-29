class TBD_RegistryEntryStruct
{
	string alias;
	string guid;
	string displayName;
}

class TBD_RegistryDocumentStruct
{
	string registryVersion;
	ref array<ref TBD_RegistryEntryStruct> entries;
}

//! Alias → prefab resource name resolution. Registry ships in mod Data/registry.json.
class TBD_Registry
{
	protected static ref map<string, ResourceName> s_AliasToResource;
	protected static bool s_Loaded;

	protected static const string REGISTRY_PATH_MOD = "$TBD_Framework:Data/registry.json";
	protected static const string REGISTRY_PATH_PROFILE = "$profile:TBD_Registry.json";

	//------------------------------------------------------------------------------------------------
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
			Print("[TBD] Registry file missing (mod and profile). Run scripts/mod/setup-server-profile.sh", LogLevel.ERROR);
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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
