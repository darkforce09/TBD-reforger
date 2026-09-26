/**
 * @file TBD_MissionLoader.c
 * @brief Holds the mission document this world runs, parsed and validated, and answers every query
 * on it.
 *
 * Role: the parse of the verified artifact bytes into `TBD_MissionDocumentStruct`, the load
 * sequence around it, and the read API every system uses.  Position: `BeginLoad` is called by
 * `TBD_FrameworkManager.OnPostInit` on the server and hands off to `TBD_DeployedMission`, which
 * calls `LoadDocument` once the artifact's SHA-256 matches; the variant filter, the validator and
 * the world applier run inside the parse; nothing here fetches, caches or reads files.
 * State: the static document, its raw text, the loaded and valid flags and the active variant ids,
 * owned by the server and cleared by `BeginLoad` for every world.  Invariants: a document over
 * `MISSION_FILE_MAX_BYTES` is never parsed; a document that fails validation is discarded, so
 * `IsValid` stays false and the stage machine never leaves LOADING; the gated queries answer null
 * until a valid document is held.
 */

//! The mission document this world runs and the queries every system asks of it.
class TBD_MissionLoader
{
	//! Hard cap on a mission document, whichever way it arrives (a fetched artifact or the profile
	//! cache). The same ceiling is `x-tbd-missionFileMaxBytes` (8388608) on mission.schema.json,
	//! the `artifact_bytes` maximum of the RuntimeDeployment contract and the limit
	//! `validate_mission_document` enforces before a compile stores an artifact; all change together.
	static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024; //!< Bytes (8 MiB).

	protected static ref TBD_MissionDocumentStruct s_Mission; //!< The parsed document; null until a parse succeeds and after a rejected one.
	protected static string s_RawJson;                          //!< The exact text last handed to the parse, for second-pass readers.
	protected static bool s_Loaded;                             //!< True once `LoadDocument` succeeded this world.
	protected static bool s_Valid;                              //!< True once the document passed validation.
	protected static ref array<string> s_ActiveVariantIds;      //!< The active variant set; null when the document declares no `variants` key or nothing is loaded.

	//! Whether a mission document loaded this world.
	//! @return true once `LoadDocument` succeeded
	static bool IsLoaded()
	{
		return s_Loaded;
	}

	//! Whether the held document passed validation.
	//! @return true once the parse and validation succeeded
	static bool IsValid()
	{
		return s_Valid;
	}

	//! The flattened slot instances.
	//! @return `slots[]`, or null until a valid document is held
	static array<ref TBD_MissionSlotStruct> GetSlots()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.slots;
	}

	//! The slot whose display id or durable uid is `slotId`.
	//! @param slotId a slot id or uid; empty returns null
	//! @return the first matching slot, or null
	static TBD_MissionSlotStruct GetSlotById(string slotId)
	{
		array<ref TBD_MissionSlotStruct> slots = GetSlots();
		if (!slots || slotId.IsEmpty())
			return null;

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			// Uid-aware: a durable uid and a display id both match.
			if (slot && (slot.id == slotId || (!slot.uid.IsEmpty() && slot.uid == slotId)))
				return slot;
		}

		return null;
	}

	//! Whether `slot` leads its own squad; see `TBD_MissionOrbatQuery.IsSquadLeader`.
	//! @param slot the seat; null returns false
	//! @return true when `slot` is its squad's authored leader
	static bool IsSquadLeader(TBD_MissionSlotStruct slot)
	{
		return TBD_MissionOrbatQuery.IsSquadLeader(slot);
	}

	//! The held document, valid or not.
	//! @return the document, or null when none is held
	static TBD_MissionDocumentStruct GetMission()
	{
		return s_Mission;
	}

	//! The held document's `meta.id`. Not gated on validation.
	//! @return the id, or empty when no document or no `meta` is held
	static string GetMissionId()
	{
		TBD_MissionDocumentStruct doc = GetMission();
		if (!doc || !doc.meta)
			return string.Empty;

		return doc.meta.id;
	}

	//! The active variant ids the held document was filtered with. Readers that re-parse
	//! `GetRawJson` (objectives, editor triggers) gate their rows on this set.
	//! @return the ids, or null when no valid document is held or it declares no `variants` key
	static array<string> GetActiveVariantIds()
	{
		if (!s_Valid)
			return null;

		return s_ActiveVariantIds;
	}

	//! The exact mission text last parsed, for second-pass readers (`TBD_MissionJsonPass`).
	//! @return the text, or empty before a parse
	static string GetRawJson()
	{
		return s_RawJson;
	}

	//! Whether the held document declares `trigger` in `winConditions.endOn`. A mission with no
	//! win conditions answers false, so its round runs until an admin ends it.
	//! @param trigger an end trigger name
	//! @return true when declared
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

	//! The written orders for one faction. Keyed like `orbat`, so the server hands each side only
	//! its own orders.
	//! @param factionKey the faction key; empty returns null
	//! @return the briefing, or null when the mission authors none for that side (the block is optional)
	static TBD_MissionBriefingStruct GetBriefingForFaction(string factionKey)
	{
		if (!s_Mission || !s_Mission.briefings || factionKey.IsEmpty())
			return null;

		TBD_MissionBriefingStruct briefing;
		if (!s_Mission.briefings.Find(factionKey, briefing))
			return null;

		return briefing;
	}

	//! The playable factions. Not gated on validation.
	//! @return `factions[]`, or null when no document is held
	static array<ref TBD_MissionFactionStruct> GetFactions()
	{
		if (!s_Mission)
			return null;

		return s_Mission.factions;
	}

	//! The zones. Gated on validation, so a rejected document never confines players.
	//! @return `zones[]`, or null until a valid document is held
	static array<ref TBD_MissionZoneStruct> GetZones()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.zones;
	}

	//! The mission-placed world objects.
	//! @return `entities[]`, or null until a valid document is held
	static array<ref TBD_MissionEntityStruct> GetEntities()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.entities;
	}

	//! The vehicle crew roster. A mission without the key yields an empty array (the parse
	//! allocates it), so callers test for null (no mission) and for empty (no roster).
	//! @return `vehicles[]`, or null until a valid document is held
	static array<ref TBD_MissionVehicleStruct> GetVehicles()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.vehicles;
	}

	//! The mission policy block; always allocated after a parse, so callers read its fields.
	//! @return `settings`, or null until a valid document is held
	static TBD_MissionSettingsStruct GetSettings()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.settings;
	}

	//! World-space spawn point for a faction: the centre of its first `spawn` zone with a circle of
	//! positive radius, on the terrain surface. A polygon spawn zone is skipped, because placing on
	//! navigable ground inside a polygon is not a containment test and its centroid is not such a
	//! point. Reads the held document whether or not it passed validation.
	//! @param factionKey the faction key
	//! @return the spawn point, or `vector.Zero` with an ERROR when no document or no usable zone exists
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

			if (!zone.shape || !zone.shape.circle || zone.shape.circle.r <= 0)
				continue;

			float x = zone.shape.circle.x;
			float z = zone.shape.circle.z;
			return Vector(x, GetGame().GetWorld().GetSurfaceY(x, z), z);
		}

		Print("[TBD] No spawn zone for faction '" + factionKey + "'.", LogLevel.ERROR);
		return vector.Zero;
	}

	//! Forget the previous world's document (statics outlive a world inside one process), then run
	//! this world's boot sequence, `TBD_DeployedMission.Begin`, which ends in `LoadDocument`.
	//! @authority server
	static void BeginLoad()
	{
		// Statics outlive a world inside one process: a new world starts with no document.
		s_Mission = null;
		s_RawJson = string.Empty;
		s_Loaded = false;
		s_Valid = false;
		s_ActiveVariantIds = null;

		TBD_DeployedMission.Begin();
	}

	//! Parse and validate `data`, the verified bytes of the artifact this world runs.
	//! @param data the document text
	//! @param source where the bytes came from, for the loaded log line
	//! @return true when the mission loaded; false with an ERROR when it is over the cap, does not
	//! parse or fails validation
	//! @authority server
	static bool LoadDocument(string data, string source)
	{
		if (!IsMissionBodyWithinCap(data))
		{
			TBD_Log.Error(TBD_Log.CH_MISSION, string.Format("mission document too large (%1 B > %2 B cap) - refusing to parse.",
				data.Length(), MISSION_FILE_MAX_BYTES));
			return false;
		}

		if (!ParseMissionJson(data))
			return false;

		s_Loaded = true;
		LogLoaded(source);
		return true;
	}

	//! The document ceiling, checked before any parse. `string.Length()` counts bytes, the unit of
	//! `MISSION_FILE_MAX_BYTES`.
	//! @param data the document text
	//! @return true when `data` is within the cap
	protected static bool IsMissionBodyWithinCap(string data)
	{
		return data.Length() <= MISSION_FILE_MAX_BYTES;
	}

	//! Parse `data`, filter it to its active variant set, validate it, and on success apply it to
	//! the world and arm the readers and reports that need a valid document.
	//! @param data the document text
	//! @return true when the document is valid; on false the document and variant set are cleared
	//! @authority server
	protected static bool ParseMissionJson(string data)
	{
		s_RawJson = data;
		s_Valid = false;
		s_ActiveVariantIds = null;

		// JsonLoadContext.LoadFromString is the engine's current string parser.
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

		// The variant filter runs before anything, the validator included, rules on the document.
		s_ActiveVariantIds = TBD_MissionVariantFilter.Apply(s_Mission, data);

		// One validation pass reports every finding, then blocks on errors: s_Valid stays false,
		// so TBD_FrameworkManager never leaves LOADING and nobody is stranded in a half-built lobby.
		if (!TBD_MissionValidator.Run(s_Mission))
		{
			s_Mission = null;
			s_ActiveVariantIds = null;
			return false;
		}

		s_Valid = true;

		TBD_MissionWorldApplier.SpawnMissionEntities();
		TBD_MissionWorldApplier.ApplyMissionSettings();

		// Each reader is a no-op when the mission authors none of its keys.
		TBD_EnvironmentReader.Apply();
		TBD_GadgetFlags.Bind();
		TBD_MissionParams.Resolve();

		// A valid document is the earliest moment a results report means anything. Both arms are
		// idempotent, so a later world of the same process arming them again is harmless; the
		// identity link lets the results report join players on their Arma id.
		TBD_ResultsReporter.Arm();
		TBD_IdentityLink.Arm();

		return true;
	}

	//! One structured `[TBD][Mission] loaded id=... name='...' slots=... source=...` line per load.
	//! @param source where the bytes came from
	protected static void LogLoaded(string source)
	{
		int slotCount = 0;
		if (s_Mission.slots)
			slotCount = s_Mission.slots.Count();

		TBD_Log.MissionLoaded(s_Mission.meta.id, s_Mission.meta.name, slotCount, source);
	}
}
