//! Minimal parsed mission header — expanded in Phase 1 as loader hardens.
class TBD_MissionMetaStruct
{
	string id;
	string name;
	string terrain;
}

//! One playable faction from the mission `factions[]` array.
class TBD_MissionFactionStruct
{
	string key;
	string displayName;
	string presetId;
}

//! Circle shape (metres, world XZ + radius) used by spawn/objective zones.
class TBD_MissionCircleStruct
{
	float x;
	float z;
	float r;
}

//! Zone shape wrapper. Only `circle` is modelled in Phase 1 (polygon zones parse to null).
class TBD_MissionShapeStruct
{
	ref TBD_MissionCircleStruct circle;
}

//! One entry from the mission `zones[]` array (spawn, objective, boundary, …).
class TBD_MissionZoneStruct
{
	string id;
	string type;
	string faction;
	ref TBD_MissionShapeStruct shape;
}

class TBD_MissionOrbatRoleStruct
{
	int count;
}

class TBD_MissionOrbatGroupStruct
{
	ref array<ref TBD_MissionOrbatRoleStruct> roles;
}

class TBD_MissionOrbatFactionStruct
{
	ref array<ref TBD_MissionOrbatGroupStruct> groups;
}

class TBD_MissionDocumentStruct
{
	string schemaVersion;
	ref TBD_MissionMetaStruct meta;
	ref array<ref TBD_MissionFactionStruct> factions;
	ref array<ref TBD_MissionZoneStruct> zones;
	ref map<string, ref TBD_MissionOrbatFactionStruct> orbat;
	ref array<ref TBD_MissionSlotStruct> slots;
}

//! Loads Mission JSON from backend REST or $profile fallback.
class TBD_MissionLoader
{
	protected static ref TBD_MissionDocumentStruct s_Mission;
	protected static string s_RawJson;
	protected static bool s_Loaded;
	protected static bool s_Valid;
	protected static bool s_LoadInFlight;

	protected static ref RestCallback s_RestCallback;

	//------------------------------------------------------------------------------------------------
	static bool IsLoaded()
	{
		return s_Loaded;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsValid()
	{
		return s_Valid;
	}

	//------------------------------------------------------------------------------------------------
	//! Flattened slot instances (null until loaded + validated).
	static array<ref TBD_MissionSlotStruct> GetSlots()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.slots;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_MissionSlotStruct GetSlotById(string slotId)
	{
		array<ref TBD_MissionSlotStruct> slots = GetSlots();
		if (!slots || slotId.IsEmpty())
			return null;

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (slot && slot.id == slotId)
				return slot;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_MissionDocumentStruct GetMission()
	{
		return s_Mission;
	}

	//------------------------------------------------------------------------------------------------
	static string GetRawJson()
	{
		return s_RawJson;
	}

	//------------------------------------------------------------------------------------------------
	//! Playable factions parsed from the mission document (null until loaded).
	static array<ref TBD_MissionFactionStruct> GetFactions()
	{
		if (!s_Mission)
			return null;

		return s_Mission.factions;
	}

	//------------------------------------------------------------------------------------------------
	//! World-space spawn point for a faction key. Returns vector.Zero if no spawn zone exists.
	static vector GetSpawnZoneForFaction(string factionKey)
	{
		if (!s_Mission || !s_Mission.zones)
		{
			Print("[TBD] GetSpawnZoneForFaction: no mission loaded.", LogLevel.ERROR);
			return vector.Zero;
		}

		foreach (TBD_MissionZoneStruct zone : s_Mission.zones)
		{
			if (!zone || zone.type != "spawn" || zone.faction != factionKey)
				continue;

			if (!zone.shape || !zone.shape.circle)
				continue;

			float x = zone.shape.circle.x;
			float z = zone.shape.circle.z;
			return Vector(x, GetGame().GetWorld().GetSurfaceY(x, z), z);
		}

		Print("[TBD] No spawn zone for faction '" + factionKey + "'.", LogLevel.ERROR);
		return vector.Zero;
	}

	//------------------------------------------------------------------------------------------------
	//! Entry point: tries REST when backend config exists, else file only.
	static void BeginLoad()
	{
		if (s_Loaded || s_LoadInFlight)
			return;

		TBD_BackendConfig.Load();
		string missionId = TBD_BackendConfig.GetMissionId();
		if (missionId.IsEmpty())
		{
			Print("[TBD] missionId not configured — cannot load mission.", LogLevel.ERROR);
			return;
		}

		if (!TBD_BackendConfig.GetBackendUrl().IsEmpty() && !TBD_BackendConfig.GetServerToken().IsEmpty())
		{
			s_LoadInFlight = true;
			FetchFromBackend(missionId);
			return;
		}

		if (LoadFromProfileFile(missionId))
			s_Loaded = true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void FetchFromBackend(string missionId)
	{
		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			Print("[TBD] RestApi unavailable — trying profile fallback.", LogLevel.WARNING);
			s_LoadInFlight = false;
			if (LoadFromProfileFile(missionId))
				s_Loaded = true;
			return;
		}

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext ctx = rest.GetContext(baseUrl);
		if (!ctx)
		{
			Print("[TBD] RestContext failed for " + baseUrl, LogLevel.ERROR);
			s_LoadInFlight = false;
			if (LoadFromProfileFile(missionId))
				s_Loaded = true;
			return;
		}

		s_RestCallback = new RestCallback();
		s_RestCallback.SetOnSuccess(OnBackendFetchSuccess);
		s_RestCallback.SetOnError(OnBackendFetchError);

		string token = TBD_BackendConfig.GetServerToken();
		ctx.SetHeaders(string.Format("Authorization, Bearer %1,Accept,application/json", token));

		string path = string.Format("/api/missions/%1/compiled", missionId);
		Print("[TBD] Fetching mission " + missionId + " from " + baseUrl + path);
		ctx.GET(s_RestCallback, path);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnBackendFetchSuccess(RestCallback cb)
	{
		s_LoadInFlight = false;
		string data = cb.GetData();
		if (data.IsEmpty())
		{
			Print("[TBD] Backend returned empty mission body.", LogLevel.ERROR);
			TryProfileFallbackAfterRestFailure();
			return;
		}

		if (!ParseMissionJson(data))
		{
			TryProfileFallbackAfterRestFailure();
			return;
		}

		string missionId = TBD_BackendConfig.GetMissionId();
		CacheToProfile(missionId, data);
		s_Loaded = true;
		Print("[TBD] Mission loaded from backend: " + s_Mission.meta.name);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnBackendFetchError(RestCallback cb)
	{
		s_LoadInFlight = false;
		Print("[TBD] Backend mission fetch failed — trying profile fallback.", LogLevel.WARNING);
		TryProfileFallbackAfterRestFailure();
	}

	//------------------------------------------------------------------------------------------------
	protected static void TryProfileFallbackAfterRestFailure()
	{
		string missionId = TBD_BackendConfig.GetMissionId();
		if (LoadFromProfileFile(missionId))
		{
			s_Loaded = true;
			Print("[TBD] Mission loaded from profile fallback.");
		}
		else
		{
			Print("[TBD] Mission load failed (REST + profile). Server stays in LOADING.", LogLevel.ERROR);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static bool LoadFromProfileFile(string missionId)
	{
		string path = string.Format("$profile:missions/%1.json", missionId);
		if (!FileIO.FileExists(path))
		{
			Print("[TBD] Profile mission file missing: " + path, LogLevel.ERROR);
			return false;
		}

		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
			return false;

		string data;
		handle.Read(data, 8 * 1024 * 1024);
		handle.Close();

		return ParseMissionJson(data);
	}

	//------------------------------------------------------------------------------------------------
	protected static bool ParseMissionJson(string data)
	{
		s_RawJson = data;
		s_Valid = false;

		// Parse a JSON string: JsonLoadContext.LoadFromString (ImportFromString /
		// SCR_JsonLoadContext are both flagged obsolete by the engine).
		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(data))
		{
			Print("[TBD] Mission JSON parse failed.", LogLevel.ERROR);
			return false;
		}

		s_Mission = new TBD_MissionDocumentStruct();
		if (!ctx.ReadValue("", s_Mission))
		{
			Print("[TBD] Mission JSON schema mismatch (meta block).", LogLevel.ERROR);
			s_Mission = null;
			return false;
		}

		if (!s_Mission.meta || s_Mission.meta.id.IsEmpty())
		{
			Print("[TBD] Mission JSON missing meta.id.", LogLevel.ERROR);
			s_Mission = null;
			return false;
		}

		if (!ValidateMissionSlots())
		{
			s_Mission = null;
			return false;
		}

		s_Valid = true;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! schemaVersion 1.1 requires non-empty slots[] matching ORBAT instance count.
	protected static bool ValidateMissionSlots()
	{
		if (!s_Mission.schemaVersion || s_Mission.schemaVersion != "1.1")
			return true;

		if (!s_Mission.slots || s_Mission.slots.IsEmpty())
		{
			Print("[TBD] Mission schemaVersion 1.1 requires non-empty slots[].", LogLevel.ERROR);
			return false;
		}

		int expected = CountOrbatInstances();
		int actual = s_Mission.slots.Count();
		if (expected > 0 && actual != expected)
		{
			Print(string.Format("[TBD] Mission slots count mismatch: orbat=%1 slots=%2", expected, actual), LogLevel.ERROR);
			return false;
		}

		ref set<string> seen = new set<string>();
		foreach (TBD_MissionSlotStruct slot : s_Mission.slots)
		{
			if (!slot || slot.id.IsEmpty())
			{
				Print("[TBD] Mission slot missing id.", LogLevel.ERROR);
				return false;
			}

			if (seen.Contains(slot.id))
			{
				Print("[TBD] Mission duplicate slot id: " + slot.id, LogLevel.ERROR);
				return false;
			}

			seen.Insert(slot.id);
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static int CountOrbatInstances()
	{
		int total = 0;
		if (!s_Mission.orbat)
			return total;

		foreach (string factionKey, TBD_MissionOrbatFactionStruct faction : s_Mission.orbat)
		{
			if (!faction || !faction.groups)
				continue;

			foreach (TBD_MissionOrbatGroupStruct group : faction.groups)
			{
				if (!group || !group.roles)
					continue;

				foreach (TBD_MissionOrbatRoleStruct role : group.roles)
				{
					if (role)
						total += role.count;
				}
			}
		}

		return total;
	}

	//------------------------------------------------------------------------------------------------
	protected static void CacheToProfile(string missionId, string data)
	{
		string dir = "$profile:missions";
		if (!FileIO.MakeDirectory(dir))
		{
			// May already exist — not fatal.
		}

		string path = string.Format("%1/%2.json", dir, missionId);
		FileHandle handle = FileIO.OpenFile(path, FileMode.WRITE);
		if (!handle)
		{
			Print("[TBD] Could not cache mission to " + path, LogLevel.WARNING);
			return;
		}

		handle.Write(data);
		handle.Close();
		Print("[TBD] Cached mission to " + path);
	}
}
