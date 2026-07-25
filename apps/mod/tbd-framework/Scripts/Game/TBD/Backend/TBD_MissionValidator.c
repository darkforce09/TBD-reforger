//! One-pass, fail-fast validation of a compiled mission document (T-181.14).
//!
//! Runs immediately after TBD_MissionLoader deserialises the document and reports **every**
//! problem it finds — never stopping at the first — so an author fixes a broken mission in one
//! edit instead of five reload cycles. Each finding is its own `[TBD][Validate]` line naming the
//! offending slot / faction / field, followed by one overall verdict line.
//!
//! Two severities, and the difference is the whole point of the slice:
//!   * **ERROR** blocks. TBD_MissionLoader discards the document, `IsValid()` stays false, and
//!     TBD_FrameworkManager never leaves LOADING — so a malformed mission can never half-load
//!     and strand players in a lobby with unfillable slots.
//!   * **WARNING** proceeds loudly. The round runs; the finding still lands in the console and
//!     in `#tbd validate`.
//!
//! The two-bucket shape (collect critical errors + warnings, then print a verdict) mirrors
//! CRF's `CRF_MissionValidatorManager`. The checks themselves are ours and share no code: CRF
//! validates world entities placed in Workbench, TBD validates a JSON document fetched at
//! runtime from the website compiler.
//!
//! Admin surface: `#tbd validate` (TBD_AdminCommands) replays the findings in game, because a
//! rejected mission is otherwise invisible from inside the server — the stage machine simply
//! never leaves LOADING and nothing on screen says why.
//!
//! @contract mission.schema.json#/
class TBD_MissionValidator
{
	//! Schema versions this build understands (mission.schema.json#/properties/schemaVersion).
	protected static const string SCHEMA_1_0 = "1.0";
	protected static const string SCHEMA_1_1 = "1.1";
	protected static const string SCHEMA_1_2 = "1.2";

	//! mission.schema.json#/$defs/winConditions/properties/endOn enum.
	protected static const string TRIGGER_TIME_LIMIT              = "time_limit";
	protected static const string TRIGGER_ALL_OBJECTIVES_CAPTURED = "all_objectives_captured";
	protected static const string TRIGGER_FACTION_ELIMINATED      = "faction_eliminated";
	protected static const string TRIGGER_OBJECTIVE_DESTROYED     = "objective_destroyed";
	protected static const string TRIGGER_HOLD_EXPIRED            = "hold_expired";

	//! Registry alias prefix a slot kit must carry (mission.schema.json#/$defs/slot/kit).
	protected static const string KIT_PREFIX = "kit:";

	//! Zone type whose circle the loader actually consumes (GetSpawnZoneForFaction).
	protected static const string ZONE_SPAWN = "spawn";

	//! Slack (metres) around the world box before a slot counts as off-terrain. A slot sitting
	//! exactly on the border is legal; float noise should not reject it.
	protected static const float BOUNDS_TOLERANCE_M = 1.0;

	//! Sanity window for BaseWorld.GetBoundBox. A box outside this is not a playable terrain
	//! (world still streaming in, or an entity AABB), so the position check is skipped with a
	//! warning rather than rejecting a perfectly good mission on a bad box.
	protected static const float BOUNDS_MIN_EXTENT_M = 100.0;
	protected static const float BOUNDS_MAX_EXTENT_M = 200000.0;

	//! Chat is not a log window — cap what `#tbd validate` dumps and count the remainder.
	protected static const int CHAT_ERROR_LINES   = 15;
	protected static const int CHAT_WARNING_LINES = 10;

	//! Findings survive the caller nulling the document, so `#tbd validate` still works after a
	//! rejected load. Allocated in Reset(), never at declaration.
	protected static ref array<string> s_aErrors;
	protected static ref array<string> s_aWarnings;
	protected static bool s_bHasRun;

	//! Which mission these findings belong to. Statics outlive a world inside one process
	//! (measured landmine), so a mission that never parses at all would otherwise leave the
	//! PREVIOUS world's verdict on screen with nothing to say so.
	protected static string s_sSubjectMissionId;

	//------------------------------------------------------------------------------------------------
	//! Validate a parsed mission document. Returns TRUE when it carries no ERRORS (warnings do
	//! not block). Every check runs regardless of earlier failures — that is the contract.
	//! @authority server — only the server parses mission documents.
	static bool Run(TBD_MissionDocumentStruct mission)
	{
		Reset();
		s_bHasRun = true;

		if (!mission)
		{
			AddError("mission", "no document to validate (the JSON deserialised to null)");
			Report();
			return false;
		}

		if (mission.meta)
			s_sSubjectMissionId = mission.meta.id;

		bool slotsRequired = CheckSchemaVersion(mission);
		CheckMeta(mission);

		ref map<string, bool> declaredFactions = new map<string, bool>();
		CheckFactions(mission, declaredFactions);

		ref map<string, int> slotsPerFaction = new map<string, int>();
		CheckSlots(mission, slotsRequired, declaredFactions, slotsPerFaction);

		CheckOrbatSlotParity(mission);
		CheckFactionCoverage(mission, slotsPerFaction);
		CheckWinConditions(mission, slotsPerFaction);
		CheckZones(mission, declaredFactions);

		Report();
		return s_aErrors.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! True once Run() has been called at least once this session.
	static bool HasRun()
	{
		return s_bHasRun;
	}

	//------------------------------------------------------------------------------------------------
	//! True when the last run found no blocking errors.
	static bool Passed()
	{
		return s_bHasRun && s_aErrors && s_aErrors.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	static int GetErrorCount()
	{
		if (!s_aErrors)
			return 0;
		return s_aErrors.Count();
	}

	//------------------------------------------------------------------------------------------------
	static int GetWarningCount()
	{
		if (!s_aWarnings)
			return 0;
		return s_aWarnings.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! Blocking findings from the last run (null before the first run).
	static array<string> GetErrors()
	{
		return s_aErrors;
	}

	//------------------------------------------------------------------------------------------------
	//! Non-blocking findings from the last run (null before the first run).
	static array<string> GetWarnings()
	{
		return s_aWarnings;
	}

	//------------------------------------------------------------------------------------------------
	//! Findings rendered for the in-game admin (`#tbd validate`). Truncated: chat is not a log.
	static array<string> BuildReportLines()
	{
		array<string> lines = new array<string>();

		if (!s_bHasRun)
		{
			lines.Insert("TBD validate: no mission has been parsed yet.");
			return lines;
		}

		string verdict = "FAILED";
		if (Passed())
			verdict = "PASSED";

		string subject = s_sSubjectMissionId;
		if (subject.IsEmpty())
			subject = "(no meta.id)";

		lines.Insert(string.Format("TBD validate [%1]: %2 — %3 error(s), %4 warning(s).",
			subject, verdict, GetErrorCount(), GetWarningCount()));

		AppendCapped(lines, s_aErrors, "ERROR", CHAT_ERROR_LINES);
		AppendCapped(lines, s_aWarnings, "WARN", CHAT_WARNING_LINES);

		if (!Passed())
			lines.Insert("TBD validate: mission is REJECTED — the server stays in LOADING until it is fixed.");

		return lines;
	}

	//------------------------------------------------------------------------------------------------
	//! Copy at most `cap` findings into `lines`, then a "… and N more" tail.
	protected static void AppendCapped(array<string> lines, array<string> findings, string label, int cap)
	{
		if (!findings || findings.IsEmpty())
			return;

		foreach (int i, string finding : findings)
		{
			if (i >= cap)
			{
				lines.Insert(string.Format("  … and %1 more %2 finding(s) — see the server console.",
					findings.Count() - cap, label));
				return;
			}

			lines.Insert(string.Format("  %1 %2", label, finding));
		}
	}

	//------------------------------------------------------------------------------------
	// CHECKS
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! schemaVersion must be one this build understands. Returns TRUE when the version makes
	//! `slots[]` mandatory (1.1 added it; 1.2 only added the optional per-slot y).
	protected static bool CheckSchemaVersion(TBD_MissionDocumentStruct mission)
	{
		string version = mission.schemaVersion;

		if (version.IsEmpty())
		{
			AddError("schemaVersion", "missing — this build understands 1.0, 1.1 and 1.2");
			return false;
		}

		if (version == SCHEMA_1_1 || version == SCHEMA_1_2)
			return true;

		if (version == SCHEMA_1_0)
			return false;

		AddError("schemaVersion", string.Format(
			"'%1' is not recognised — this build understands 1.0, 1.1 and 1.2. A newer document may carry fields this server silently drops.",
			version));
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Mission header. meta.id is optional in the schema only for pre-publish hand-written
	//! drafts, which never reach this loader — the mod loads PUBLISHED missions, which always
	//! carry the content-hash id assigned at publish time (T-122 M11).
	protected static void CheckMeta(TBD_MissionDocumentStruct mission)
	{
		if (!mission.meta)
		{
			AddError("meta", "missing meta block");
			return;
		}

		if (mission.meta.id.IsEmpty())
			AddError("meta.id", "missing — the mod only loads published missions, which always carry a content-hash id");

		if (mission.meta.name.IsEmpty())
			AddWarning("meta.name", "empty — the mission browser will show a blank row");

		if (mission.meta.terrain.IsEmpty())
			AddWarning("meta.terrain", "empty — admin mission switching cannot route this mission to a scenario");
	}

	//------------------------------------------------------------------------------------------------
	//! At least one playable faction, unique keys. Fills `declared` with every key that parsed,
	//! which every later check uses to decide whether a referenced faction actually exists.
	protected static void CheckFactions(TBD_MissionDocumentStruct mission, map<string, bool> declared)
	{
		array<ref TBD_MissionFactionStruct> factions = mission.factions;
		if (!factions || factions.IsEmpty())
		{
			AddError("factions", "no playable faction declared — nobody can pick a side");
			return;
		}

		foreach (int i, TBD_MissionFactionStruct faction : factions)
		{
			string subject = string.Format("factions[%1]", i);

			if (!faction)
			{
				AddError(subject, "null faction entry");
				continue;
			}

			if (faction.key.IsEmpty())
			{
				AddError(subject, "faction has no key — slots cannot reference it");
				continue;
			}

			subject = "faction:" + faction.key;

			if (declared.Contains(faction.key))
			{
				AddError(subject, "declared more than once — faction lookups would be ambiguous");
				continue;
			}

			declared.Insert(faction.key, true);

			if (faction.displayName.IsEmpty())
				AddWarning(subject, "no displayName — the lobby will show the raw key");

			if (faction.presetId.IsEmpty())
				AddWarning(subject, "no presetId — the faction has no registry preset to build from");
		}

		if (declared.Count() == 1)
			AddWarning("factions", "only one faction is declared; mission.schema.json expects at least two");
	}

	//------------------------------------------------------------------------------------------------
	//! Every slot: present, uniquely keyed, on a declared faction, with a resolvable kit, inside
	//! the terrain, and carrying a sane loadout. `slotsPerFaction` accumulates the per-side slot
	//! census the win-condition check needs.
	protected static void CheckSlots(TBD_MissionDocumentStruct mission, bool slotsRequired,
		map<string, bool> declaredFactions, map<string, int> slotsPerFaction)
	{
		array<ref TBD_MissionSlotStruct> slots = mission.slots;
		if (!slots || slots.IsEmpty())
		{
			if (slotsRequired)
			{
				AddError("slots", string.Format(
					"schemaVersion %1 requires a non-empty slots[] — no player can spawn", mission.schemaVersion));
			}
			else
			{
				AddWarning("slots", "no slots[] — a pre-1.1 document carries no spawn positions, so no player can spawn from it. Recompile the mission at schemaVersion 1.1 or later.");
			}

			return;
		}

		ref set<string> registryAliases = new set<string>();
		LoadRegistryAliases(registryAliases);

		vector mins;
		vector maxs;
		bool boundsKnown = TryGetTerrainBounds(mins, maxs);
		if (!boundsKnown)
			AddWarning("slots", "terrain bounds unavailable at validation time — slot positions were NOT range-checked");

		// uid-else-id is the durable identity (TBD_MissionSlotStruct.Key), so that is the key that
		// must be unique. The display id is tracked separately because
		// TBD_MissionLoader.GetSlotById also matches on id — two slots sharing an id make that
		// lookup ambiguous even when their keys differ. A duplicate uid is always a duplicate key
		// (Key() returns uid when set), so it needs no third set.
		ref set<string> seenKey = new set<string>();
		ref set<string> seenId = new set<string>();

		foreach (int i, TBD_MissionSlotStruct slot : slots)
		{
			string subject = string.Format("slots[%1]", i);

			if (!slot)
			{
				AddError(subject, "null slot entry");
				continue;
			}

			if (slot.id.IsEmpty())
				AddError(subject, "slot has no id");
			else
				subject = "slot:" + slot.Key();

			CheckSlotIdentity(subject, slot, seenKey, seenId);
			CheckSlotFaction(subject, slot, declaredFactions, slotsPerFaction);
			CheckSlotKit(subject, slot, registryAliases);
			CheckSlotPosition(subject, slot, boundsKnown, mins, maxs);
			CheckSlotLoadout(subject, slot);

			if (slot.groupCallsign.IsEmpty())
				AddWarning(subject, "no groupCallsign — the slot has no squad to file under");

			if (slot.role.IsEmpty())
				AddWarning(subject, "no role label");

			if (slot.headingDeg < 0 || slot.headingDeg > 360)
				AddWarning(subject, string.Format("headingDeg=%1 is outside 0..360", slot.headingDeg));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Duplicate slot keys. Reported once per slot, most-precise finding first.
	protected static void CheckSlotIdentity(string subject, TBD_MissionSlotStruct slot,
		set<string> seenKey, set<string> seenId)
	{
		string key = slot.Key();

		bool keyDuplicate = false;
		if (!key.IsEmpty())
		{
			if (seenKey.Contains(key))
				keyDuplicate = true;
			else
				seenKey.Insert(key);
		}

		bool idDuplicate = false;
		if (!slot.id.IsEmpty())
		{
			if (seenId.Contains(slot.id))
				idDuplicate = true;
			else
				seenId.Insert(slot.id);
		}

		if (keyDuplicate)
		{
			AddError(subject, string.Format(
				"duplicate slot key '%1' (uid-else-id) — two slots claim one identity, so claims, rosters and spawn points would collide",
				key));
			return;
		}

		if (idDuplicate)
		{
			AddError(subject, string.Format(
				"duplicate slot id '%1' — TBD_MissionLoader.GetSlotById matches id as well as uid, so lookups by this id are ambiguous",
				slot.id));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Slot faction must exist and must be one of the declared factions.
	protected static void CheckSlotFaction(string subject, TBD_MissionSlotStruct slot,
		map<string, bool> declaredFactions, map<string, int> slotsPerFaction)
	{
		if (slot.faction.IsEmpty())
		{
			AddError(subject, "no faction — the slot cannot be assigned to a side");
			return;
		}

		// Counted even when undeclared: the slot still occupies that side for the
		// faction_eliminated arithmetic, and the undeclared-faction error below already
		// names the real problem.
		int count = 0;
		slotsPerFaction.Find(slot.faction, count);
		slotsPerFaction.Set(slot.faction, count + 1);

		// Skip when factions[] failed outright — one error there beats one per slot here.
		if (declaredFactions.Count() == 0)
			return;

		if (!declaredFactions.Contains(slot.faction))
			AddError(subject, string.Format("faction '%1' is not declared in factions[]", slot.faction));
	}

	//------------------------------------------------------------------------------------------------
	//! Slot kit must be a `kit:` alias the spawn registry can resolve. TBD_SpawnManager treats an
	//! unresolvable kit as a permanent per-slot failure (no body at all), so catching it here is
	//! the difference between an authoring error and a player with no character.
	protected static void CheckSlotKit(string subject, TBD_MissionSlotStruct slot, set<string> registryAliases)
	{
		if (slot.kit.IsEmpty())
		{
			AddError(subject, "no kit alias — TBD_SpawnManager has no prefab to spawn");
			return;
		}

		if (!slot.kit.StartsWith(KIT_PREFIX))
		{
			AddError(subject, string.Format(
				"kit '%1' is not a kit: alias (mission.schema.json#/$defs/slot)", slot.kit));
			return;
		}

		// Registry unavailable — LoadRegistryAliases already warned once for the document.
		// Staying quiet here beats emitting one unprovable error per slot.
		if (registryAliases.Count() == 0)
			return;

		if (!registryAliases.Contains(slot.kit))
		{
			AddError(subject, string.Format(
				"kit alias '%1' does not resolve in the spawn registry — TBD_SpawnManager would fail this slot permanently",
				slot.kit));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn position must land on the loaded terrain. Skipped entirely when the world box is
	//! not trustworthy (see TryGetTerrainBounds) — a wrong box must not reject a good mission.
	protected static void CheckSlotPosition(string subject, TBD_MissionSlotStruct slot,
		bool boundsKnown, vector mins, vector maxs)
	{
		if (!boundsKnown)
			return;

		float minX = mins[0] - BOUNDS_TOLERANCE_M;
		float maxX = maxs[0] + BOUNDS_TOLERANCE_M;
		float minZ = mins[2] - BOUNDS_TOLERANCE_M;
		float maxZ = maxs[2] + BOUNDS_TOLERANCE_M;

		if (slot.x >= minX && slot.x <= maxX && slot.z >= minZ && slot.z <= maxZ)
			return;

		AddError(subject, string.Format(
			"spawn (%1, %2) is outside the loaded terrain (x %3..%4, z %5..%6) — the player would drop into the void. Check that the mission terrain matches the loaded world.",
			slot.x, slot.z, mins[0], maxs[0], mins[2], maxs[2]));
	}

	//------------------------------------------------------------------------------------------------
	//! Optional per-slot Arsenal loadout (T-068.11). The kit prefab stays authoritative for the
	//! base character, so an empty gear block is loud but not fatal; a malformed cargo row is a
	//! schema violation and blocks.
	protected static void CheckSlotLoadout(string subject, TBD_MissionSlotStruct slot)
	{
		TBD_SlotLoadoutStruct loadout = slot.loadout;
		if (!loadout)
			return;

		bool hasCargo = loadout.cargo && !loadout.cargo.IsEmpty();

		if (loadout.gear)
		{
			if (CountGearRefs(loadout.gear) == 0)
				AddWarning(subject, "loadout.gear is present but every gear ref is empty — the slot falls back to the bare kit prefab");
		}
		else if (!hasCargo)
		{
			AddWarning(subject, "loadout is present but carries neither gear nor cargo");
		}

		if (!hasCargo)
			return;

		foreach (int c, TBD_SlotCargoStruct row : loadout.cargo)
		{
			string where = string.Format("%1 loadout.cargo[%2]", subject, c);

			if (!row)
			{
				AddError(where, "null cargo row");
				continue;
			}

			if (row.container.IsEmpty())
				AddError(where, "no container (expected vest / pants / jacket / backpack)");

			if (row.item.IsEmpty())
				AddError(where, "no item ResourceName");

			if (row.qty < 1)
				AddError(where, string.Format("qty=%1 — mission.schema.json requires qty >= 1", row.qty));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! How many of the ten fixed gear slots actually carry a ResourceName.
	protected static int CountGearRefs(TBD_SlotGearStruct gear)
	{
		int refs = 0;

		if (!gear.primary.IsEmpty())
			refs++;
		if (!gear.optic.IsEmpty())
			refs++;
		if (!gear.magazine.IsEmpty())
			refs++;
		if (!gear.uniform.IsEmpty())
			refs++;
		if (!gear.vest.IsEmpty())
			refs++;
		if (!gear.helmet.IsEmpty())
			refs++;
		if (!gear.pants.IsEmpty())
			refs++;
		if (!gear.boots.IsEmpty())
			refs++;
		if (!gear.handwear.IsEmpty())
			refs++;
		if (!gear.backpack.IsEmpty())
			refs++;

		return refs;
	}

	//------------------------------------------------------------------------------------------------
	//! slots[] must materialise exactly the instance count the ORBAT declares. A mismatch means
	//! the compiled document is out of step with its own ORBAT — slots would be missing or
	//! orphaned. (Preserved from the loader's original ValidateMissionSlots.)
	protected static void CheckOrbatSlotParity(TBD_MissionDocumentStruct mission)
	{
		int expected = CountOrbatInstances(mission);
		if (expected <= 0)
			return;

		int actual = 0;
		if (mission.slots)
			actual = mission.slots.Count();

		// An empty slots[] is already reported by CheckSlots — do not say it twice.
		if (actual == 0)
			return;

		if (actual != expected)
		{
			AddError("orbat", string.Format(
				"orbat declares %1 slot instance(s) but slots[] carries %2 — the compiled document is out of step with its ORBAT",
				expected, actual));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Total role.count across every faction/group in the ORBAT.
	protected static int CountOrbatInstances(TBD_MissionDocumentStruct mission)
	{
		int total = 0;
		if (!mission.orbat)
			return total;

		foreach (string factionKey, TBD_MissionOrbatFactionStruct faction : mission.orbat)
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
	//! A declared faction with no slots is a side nobody can play. Loud, not fatal — an author
	//! may legitimately be staging a third side for a later mission.
	protected static void CheckFactionCoverage(TBD_MissionDocumentStruct mission, map<string, int> slotsPerFaction)
	{
		if (!mission.factions)
			return;

		// Nothing to say when the document carries no slots at all — already reported once.
		if (!mission.slots || mission.slots.IsEmpty())
			return;

		foreach (TBD_MissionFactionStruct faction : mission.factions)
		{
			if (!faction || faction.key.IsEmpty())
				continue;

			int count = 0;
			slotsPerFaction.Find(faction.key, count);
			if (count == 0)
				AddWarning("faction:" + faction.key, "declared but has no slots — nobody can play this side");
		}
	}

	//------------------------------------------------------------------------------------------------
	//! End triggers must be schema values, and `faction_eliminated` needs a real contest.
	//!
	//! TBD_FrameworkManager.TickWinConditions already refuses to end a round with fewer than two
	//! CONTESTING factions, so a one-sided mission does not actually end at kickoff — it just
	//! never ends, which looks identical to a broken win condition from the server console.
	//! Catching it here turns that mystery into an authoring error before anyone joins.
	protected static void CheckWinConditions(TBD_MissionDocumentStruct mission, map<string, int> slotsPerFaction)
	{
		TBD_MissionWinConditionsStruct conditions = mission.winConditions;
		if (!conditions)
		{
			AddWarning("winConditions", "absent — the round has no end trigger and runs until an admin ends it");
			return;
		}

		if (conditions.mode.IsEmpty())
			AddWarning("winConditions.mode", "empty — the round has no declared mode label");

		if (!conditions.endOn || conditions.endOn.IsEmpty())
		{
			AddWarning("winConditions.endOn", "empty — the round has no end trigger and runs until an admin ends it");
			return;
		}

		bool eliminationDeclared = false;
		foreach (int i, string trigger : conditions.endOn)
		{
			string subject = string.Format("winConditions.endOn[%1]", i);

			if (trigger.IsEmpty())
			{
				AddError(subject, "empty end trigger");
				continue;
			}

			if (!IsKnownEndTrigger(trigger))
			{
				AddError(subject, string.Format(
					"'%1' is not a mission.schema.json end trigger (time_limit, all_objectives_captured, faction_eliminated, objective_destroyed, hold_expired)",
					trigger));
				continue;
			}

			if (trigger == TRIGGER_FACTION_ELIMINATED)
				eliminationDeclared = true;
		}

		if (!eliminationDeclared)
			return;

		// A document with no slots has no sides to eliminate; CheckSlots already said so.
		if (!mission.slots || mission.slots.IsEmpty())
			return;

		int sidesWithSlots = 0;
		foreach (string factionKey, int count : slotsPerFaction)
		{
			if (count > 0)
				sidesWithSlots++;
		}

		if (sidesWithSlots < 2)
		{
			AddError("winConditions.endOn", string.Format(
				"declares faction_eliminated but only %1 faction(s) actually have slots — no second side can ever be eliminated, so the round can never resolve",
				sidesWithSlots));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! mission.schema.json#/$defs/winConditions endOn enum membership.
	protected static bool IsKnownEndTrigger(string trigger)
	{
		return trigger == TRIGGER_TIME_LIMIT
			|| trigger == TRIGGER_ALL_OBJECTIVES_CAPTURED
			|| trigger == TRIGGER_FACTION_ELIMINATED
			|| trigger == TRIGGER_OBJECTIVE_DESTROYED
			|| trigger == TRIGGER_HOLD_EXPIRED;
	}

	//------------------------------------------------------------------------------------------------
	//! Zones are light-touch: only a faction reference that does not exist is fatal. A spawn zone
	//! without a circle is a warning because polygon shapes are deliberately not modelled yet
	//! (TBD_MissionShapeStruct parses circle only).
	protected static void CheckZones(TBD_MissionDocumentStruct mission, map<string, bool> declaredFactions)
	{
		if (!mission.zones || mission.zones.IsEmpty())
		{
			AddWarning("zones", "no zones declared — factions have no spawn-zone fallback position");
			return;
		}

		foreach (int i, TBD_MissionZoneStruct zone : mission.zones)
		{
			string subject = string.Format("zones[%1]", i);

			if (!zone)
			{
				AddError(subject, "null zone entry");
				continue;
			}

			if (zone.id.IsEmpty())
				AddWarning(subject, "zone has no id");
			else
				subject = "zone:" + zone.id;

			if (zone.type.IsEmpty())
				AddWarning(subject, "zone has no type");

			if (!zone.faction.IsEmpty() && declaredFactions.Count() > 0 && !declaredFactions.Contains(zone.faction))
				AddError(subject, string.Format("faction '%1' is not declared in factions[]", zone.faction));

			if (zone.type == ZONE_SPAWN && (!zone.shape || !zone.shape.circle))
				AddWarning(subject, "spawn zone has no circle shape — TBD_MissionLoader.GetSpawnZoneForFaction cannot use it");
		}
	}

	//------------------------------------------------------------------------------------
	// HELPERS
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Fill `outAliases` with every registry alias. TBD_Registry.GetAllAliases loads the registry
	//! on demand, so this runs before TBD_FrameworkManager's own Load() and is idempotent.
	//!
	//! An empty registry is a WARNING, not an error: this runs earlier in boot than the registry
	//! normally loads, and a genuinely missing registry already fails loudly per slot inside
	//! TBD_SpawnManager. Rejecting every slot of a good mission over a load-order quirk would be
	//! worse than the problem.
	protected static void LoadRegistryAliases(set<string> outAliases)
	{
		array<string> all = TBD_Registry.GetAllAliases();
		if (all)
		{
			foreach (string alias : all)
			{
				if (!alias.IsEmpty())
					outAliases.Insert(alias);
			}
		}

		if (outAliases.Count() == 0)
			AddWarning("registry", "no spawn-registry aliases available at validation time — slot kit resolution was NOT verified");
	}

	//------------------------------------------------------------------------------------------------
	//! World box of the loaded terrain. Returns FALSE (and the caller skips the position check)
	//! when there is no world yet or the box is not a plausible terrain extent.
	protected static bool TryGetTerrainBounds(out vector mins, out vector maxs)
	{
		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return false;

		world.GetBoundBox(mins, maxs);

		float extentX = maxs[0] - mins[0];
		float extentZ = maxs[2] - mins[2];

		if (extentX < BOUNDS_MIN_EXTENT_M || extentZ < BOUNDS_MIN_EXTENT_M)
			return false;

		if (extentX > BOUNDS_MAX_EXTENT_M || extentZ > BOUNDS_MAX_EXTENT_M)
			return false;

		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void Reset()
	{
		s_aErrors = new array<string>();
		s_aWarnings = new array<string>();
		s_sSubjectMissionId = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! A blocking finding. `subject` names the offending slot / faction / field.
	protected static void AddError(string subject, string message)
	{
		s_aErrors.Insert(subject + " — " + message);
	}

	//------------------------------------------------------------------------------------------------
	//! A non-blocking finding. Still printed, still shown to admins.
	protected static void AddWarning(string subject, string message)
	{
		s_aWarnings.Insert(subject + " — " + message);
	}

	//------------------------------------------------------------------------------------------------
	//! One `[TBD][Validate]` line per finding, then the overall verdict. A failure ends with a
	//! banner because a rejected mission must not scroll past the operator.
	protected static void Report()
	{
		foreach (string finding : s_aErrors)
			TBD_Log.Error(TBD_Log.CH_VALIDATE, "ERROR   " + finding);

		foreach (string finding : s_aWarnings)
			TBD_Log.Warn(TBD_Log.CH_VALIDATE, "WARNING " + finding);

		bool passed = s_aErrors.IsEmpty();
		TBD_Log.ValidationResult(passed, s_aErrors.Count(), s_aWarnings.Count());

		if (passed)
			return;

		TBD_Log.Banner(TBD_Log.CH_VALIDATE, string.Format(
			"MISSION REJECTED — %1 error(s). The server stays in LOADING until the mission is fixed. Admins: '#tbd validate'.",
			s_aErrors.Count()), true);
	}
}
