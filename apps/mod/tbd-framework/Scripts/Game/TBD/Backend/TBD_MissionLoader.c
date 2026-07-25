//! Minimal parsed mission header — expanded in Phase 1 as loader hardens.
//! @contract mission.schema.json#/$defs/meta (subset: id/name/terrain).
class TBD_MissionMetaStruct
{
	string id;
	string name;
	string terrain;
}

//! One playable faction from the mission `factions[]` array.
//! @contract mission.schema.json#/$defs/faction
class TBD_MissionFactionStruct
{
	string key;
	string displayName;
	string presetId;
}

//! Circle shape (metres, world XZ + radius) used by spawn/objective zones.
//! @contract mission.schema.json#/$defs/circle
class TBD_MissionCircleStruct
{
	float x;
	float z;
	float r;
}

//! Zone shape wrapper. Only `circle` is modelled in Phase 1 (polygon zones parse to null).
//! @contract mission.schema.json#/$defs/shape
class TBD_MissionShapeStruct
{
	ref TBD_MissionCircleStruct circle;
}

//! One entry from the mission `zones[]` array (spawn, objective, boundary, …).
//! @contract mission.schema.json#/$defs/zone
class TBD_MissionZoneStruct
{
	string id;
	string type;
	string faction;
	ref TBD_MissionShapeStruct shape;
}

//! One ORBAT role line (its fill count).
//! @contract mission.schema.json#/$defs/role
class TBD_MissionOrbatRoleStruct
{
	int count; //!< Number of slots to materialize for this role.
}

//! One ORBAT group (squad) and its roles.
//! @contract mission.schema.json#/$defs/group
class TBD_MissionOrbatGroupStruct
{
	ref array<ref TBD_MissionOrbatRoleStruct> roles;
}

//! One faction's ORBAT (its groups), keyed by faction in the orbat map.
//! @contract mission.schema.json#/$defs/orbatFaction
class TBD_MissionOrbatFactionStruct
{
	ref array<ref TBD_MissionOrbatGroupStruct> groups;
}

//! T-181.13 — how the round ends, straight from the mission JSON.
//! `endOn` values are the schema enum: time_limit | all_objectives_captured |
//! faction_eliminated | objective_destroyed | hold_expired. TBD one-life events evaluate
//! `faction_eliminated` today; the others are parsed and ignored until their slice lands, so
//! an authored mission never silently loses a condition it declared.
//! @contract mission.schema.json#/$defs/winConditions
class TBD_MissionWinConditionsStruct
{
	string mode;               //!< Free-form mode label from the editor.
	ref array<string> endOn;   //!< One or more end triggers.
}

//! Full mission document parsed from the backend — the canonical contract the loader
//! consumes. schemaVersion is the canonical STRING ("1.0"/"1.1"/"1.2"), distinct from the
//! website's integer editor/export version. Field names must equal the JSON keys.
//! @contract mission.schema.json#/
class TBD_MissionDocumentStruct
{
	string schemaVersion;                                      //!< Canonical contract version ("1.0"/"1.1"/"1.2").
	ref TBD_MissionMetaStruct meta;                            //!< Mission header.
	ref array<ref TBD_MissionFactionStruct> factions;         //!< Playable factions.
	ref array<ref TBD_MissionZoneStruct> zones;               //!< Spawn/objective/boundary zones.
	ref map<string, ref TBD_MissionOrbatFactionStruct> orbat; //!< ORBAT keyed by faction.
	ref array<ref TBD_MissionSlotStruct> slots;               //!< Flattened spawn slots (schema 1.1).
	ref TBD_MissionWinConditionsStruct winConditions;         //!< T-181.13 round-end triggers.
}

//! Loads Mission JSON from backend REST or $profile fallback.
//! @route GET /api/v1/missions/{id}/compiled (service-token tier; body = this canonical document, T-092.2).
class TBD_MissionLoader
{
	//! Hard cap on a profile mission file. A file over this would silently truncate in
	//! Read() and then fail JSON parse with a misleading error — reject it up front (T-130.4 F1-16).
	protected static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024;

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
			// B1 — uid-aware lookup: durable uid matches first, display id stays valid.
			if (slot && (slot.id == slotId || (!slot.uid.IsEmpty() && slot.uid == slotId)))
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
	//! T-181.13 — true when the mission declared this end trigger. Missions authored before
	//! winConditions existed simply have none, so callers get `false` and the round runs until
	//! an admin ends it rather than ending unexpectedly.
	static bool HasEndTrigger(string trigger)
	{
		if (!s_Mission || !s_Mission.winConditions || !s_Mission.winConditions.endOn)
			return false;

		foreach (string t : s_Mission.winConditions.endOn)
		{
			if (t == trigger)
				return true;
		}
		return false;
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
		{
			s_Loaded = true;
			LogLoaded("profile");
		}
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

		// Backend guards the game-server tier with X-Service-Token (middleware.RequireServiceToken),
		// not an Authorization bearer — same header the /ingest telemetry endpoints use.
		string token = TBD_BackendConfig.GetServerToken();
		ctx.SetHeaders(string.Format("X-Service-Token,%1,Accept,application/json", token));

		string path = string.Format("/api/v1/missions/%1/compiled", missionId);
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
		LogLoaded("backend");
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
			LogLoaded("profile");
		}
		else
		{
			TBD_Log.Error(TBD_Log.CH_MISSION, "load failed (REST + profile) — server stays in LOADING");
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
		{
			Print("[TBD] Could not open profile mission file: " + path, LogLevel.ERROR);
			return false;
		}

		int fileSize = handle.GetLength();
		if (fileSize > MISSION_FILE_MAX_BYTES)
		{
			Print(string.Format("[TBD] Profile mission file too large (%1 B > %2 B cap): %3 — refusing to parse a truncated read.",
				fileSize, MISSION_FILE_MAX_BYTES, path), LogLevel.ERROR);
			handle.Close();
			return false;
		}

		string data;
		handle.Read(data, MISSION_FILE_MAX_BYTES);
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

		// T-181.14 — one validation pass over the whole document. It reports EVERY problem
		// (never just the first) as its own [TBD][Validate] line, then blocks on errors. The
		// meta.id, slots-required, orbat-parity and duplicate-key checks that used to live in
		// this file moved into TBD_MissionValidator so a broken mission surfaces all of its
		// faults in a single reload instead of one per fix.
		//
		// Blocking here is the fail-fast: s_Valid stays false, so TBD_FrameworkManager never
		// leaves LOADING and nobody is stranded in a half-built lobby.
		if (!TBD_MissionValidator.Run(s_Mission))
		{
			s_Mission = null;
			return false;
		}

		s_Valid = true;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.14 — one structured line per successful load:
	//! `[TBD][Mission] loaded id=… name='…' slots=… source=…`.
	protected static void LogLoaded(string source)
	{
		int slotCount = 0;
		if (s_Mission.slots)
			slotCount = s_Mission.slots.Count();

		TBD_Log.MissionLoaded(s_Mission.meta.id, s_Mission.meta.name, slotCount, source);
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
