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
//! ══ T-181.37 — THE RULE THAT DECIDES WHICH ═══════════════════════════════════════════════════
//! **ERROR when the mission cannot be PLAYED. WARNING when it can be played but not ENDED.**
//!
//! An ERROR is not "more serious", it is a DIFFERENT OUTAGE: the document is discarded, the stage
//! machine never leaves LOADING, and nobody can even join. For an unfillable slot or an
//! unresolvable kit that is the right trade — the alternative is a player with no character. For a
//! mission whose declared end trigger can never fire, it is strictly worse than the bug: the round
//! would merely have run long and been ended by an admin, and instead the event does not happen at
//! all. So every end-trigger REACHABILITY finding below is a WARNING.
//!
//! The one apparent exception proves the rule. `faction_eliminated` with fewer than two sides
//! holding slots stays an ERROR, because that document has no opposition — it is not a round that
//! ends late, it is not a PvP event at all, and its win condition is the place that fact is
//! provable.
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

	//! mission.schema.json#/$defs/winConditions/properties/endOn enum — the two triggers with no
	//! owning constant anywhere else in the mod.
	//!
	//! T-181.37 — the other three (`all_objectives_captured`, `objective_destroyed`,
	//! `hold_expired`) USED to be spelled out here too and are not any more: they are read from
	//! `TBD_ObjectiveRegistry.TRIGGER_*`, which is the code that EVALUATES them. A validator
	//! keeping its own copy of the vocabulary would go on validating against a list the evaluator
	//! had moved off — which is precisely the class of bug this check exists to catch, so it must
	//! not be the shape of the check itself.
	//!
	//! These two stay because there is nothing to ask. `TBD_FrameworkManager` spells both as bare
	//! literals at the point of use — `"time_limit"` in `ArmRoundClock`, `"faction_eliminated"` in
	//! `ArmFactionEliminated` — so there is no constant to reference. Closing that seam means
	//! adding `TRIGGER_TIME_LIMIT` / `TRIGGER_FACTION_ELIMINATED` next to `TBD_MissionFlow.CH_FLOW`
	//! and using them in both places; `Gamemode/**` belongs to another slice's lane, so it is
	//! reported rather than edited.
	protected static const string TRIGGER_TIME_LIMIT         = "time_limit";
	protected static const string TRIGGER_FACTION_ELIMINATED = "faction_eliminated";

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
	//!
	//! T-181.13.1 — truth in comments: the `!mission.meta` guard below CANNOT fire for a document
	//! that simply omits `meta`, because `meta` is a `ref <class>` and `JsonLoadContext` allocates
	//! it regardless (see CheckSlotLoadout for the measurement). It is deliberately left in place —
	//! unlike the loadout case it costs nothing and is not misleading, and the per-field emptiness
	//! checks below are the CONTENT tests that actually catch an absent meta block: `meta.id` empty
	//! is already a blocking ERROR, so a mission with no header is still rejected, with three
	//! specific findings instead of one general one. That is a better report, not a worse one.
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
	//! base character, so a loadout that carries nothing is not fatal; a malformed cargo row is a
	//! schema violation and blocks.
	//!
	//! ══ T-181.13.1 — MAJOR BUG FIX: this function used to test non-null ════════════════════════
	//! `slot.loadout` (TBD_MissionSlotStruct) and `loadout.gear` (TBD_SlotLoadoutStruct) are
	//! `ref <class>` fields, and `JsonLoadContext` ALLOCATES a nested `ref <class>` even when the
	//! JSON key is ABSENT — see the landmine block on `TBD_MissionShapeStruct` in
	//! TBD_MissionLoader.c, and `TBD_SpawnManager.HasAuthoredLoadout`, which fixed the identical
	//! bug in the slot-body census at T-181.32. So `if (!loadout) return;` never returned and
	//! `if (loadout.gear)` was always true. Consequences, all measured on live world boots:
	//!   * every slot that authored NO loadout at all was reported as "loadout.gear is present but
	//!     every gear ref is empty". `golden-missions/bridgehead-at-levie.json`, whose 18 slots
	//!     carry no `loadout` key, booted to `mission result=PASS errors=0 warnings=18` — all 18
	//!     of them this one warning;
	//!   * the `else if (!hasCargo)` branch under it was UNREACHABLE dead code;
	//!   * across the four goldens 27 warnings were emitted and 2 were correct.
	//!
	//! ── What is actually observable (measured 2026-07-25, instrumented boot) ────────────────────
	//! A temporary `[TBDPROBE]` dump over `s_Mission.slots` immediately after `ctx.ReadValue`:
	//!
	//!   bridgehead-at-levie   — 0 of 18 slots author `loadout`
	//!       every slot:  loadoutNull=0  gearNull=0  gearRefs=0  cargoNull=1
	//!   empty-warning-fields  — 4 of 7 author `loadout`; two of those author `"cargo": []`
	//!       the two authoring `cargo`:   cargoNull=0  cargoCount=0
	//!       the other five:              cargoNull=1
	//!
	//! Two facts follow, and the SECOND ONE IS NEW to this program:
	//!   1. a `ref <class>` field is allocated whether or not the key was authored (third
	//!      independent confirmation);
	//!   2. **a `ref array<>` field is NOT.** `loadout.cargo` came back non-null on exactly the
	//!      slots whose JSON authored a `cargo` key and null on every slot that did not — both
	//!      polarities in one run, which is its own negative control. A container's NON-NULLNESS is
	//!      therefore a genuine presence test, even though its emptiness is not.
	//!
	//! ── The rule this function now follows ──────────────────────────────────────────────────────
	//! GEAR PRESENCE IS UNOBSERVABLE. An absent `gear` key and an authored `gear: {}` both parse to
	//! a non-null struct holding ten empty strings, and there is no scalar sentinel to hang a
	//! presence test on. So this never claims gear is "present" again — it COUNTS REFS, which is
	//! the same content test `TBD_SpawnManager.HasAuthoredLoadout` settled on.
	//!
	//! A loadout with zero gear refs and no `cargo` key is therefore indistinguishable from having
	//! no loadout at all, AND behaves identically (the slot falls back to the bare kit prefab). It
	//! gets no warning — that is the normal, legal shape of every loadout-less slot in every
	//! mission. The website compiler cannot even emit an empty block: `mod_slot_loadout` in
	//! `crates/map-engine-core/src/mission/flatten.rs` returns `None` when gear and cargo are both
	//! empty, skips `gear` when all ten fields are empty, and skips `cargo` when the vec is empty.
	//!
	//! The one authored-but-empty case that IS provable is a `cargo` key that parsed to zero rows
	//! while gear carries nothing: non-null `cargo` proves the block was authored, and nothing is in
	//! it. That is precisely what the old dead branch was written to say, so the message survives —
	//! and it is now reachable. It cannot fire on a compiled mission, so on a real mission it means
	//! the JSON was hand-edited.
	//!
	//! If fact 2 ever stops holding, `bridgehead-at-levie` lights up again with 18 warnings and the
	//! `.world-boot-warning-baseline` ratchet fails the wave gate. That is the intended backstop.
	protected static void CheckSlotLoadout(string subject, TBD_MissionSlotStruct slot)
	{
		TBD_SlotLoadoutStruct loadout = slot.loadout;
		if (!loadout)
			return;      // cannot fire today (see above); kept because a null deref would be worse.

		int gearRefs = CountGearRefs(loadout.gear);

		// CONTENT, not non-null — but note the asymmetry proved above: for a CONTAINER, non-null
		// does mean "the key was authored". Count() is what says whether anything is in it.
		bool cargoAuthored = false;
		int cargoRows = 0;
		if (loadout.cargo)
		{
			cargoAuthored = true;
			cargoRows = loadout.cargo.Count();
		}

		if (gearRefs == 0 && cargoRows == 0)
		{
			// Only provable when `cargo` was authored. Without it there is nothing to distinguish
			// this slot from one that never had a loadout, so saying anything would be noise.
			if (cargoAuthored)
				AddWarning(subject, "loadout is present but carries neither gear nor cargo — the slot falls back to the bare kit prefab");

			return;
		}

		if (cargoRows == 0)
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
	//!
	//! Null-safe and CONTENT-based on purpose: `gear` is a `ref <class>`, so it is non-null even on a
	//! slot whose JSON never mentioned it (see CheckSlotLoadout). This count is the only thing that
	//! separates an authored gear block from an allocated-empty one, so it is the only test callers
	//! are allowed to use. The null guard is defence against a future caller, not a live case.
	protected static int CountGearRefs(TBD_SlotGearStruct gear)
	{
		if (!gear)
			return 0;

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
	//! End triggers must be schema values — and each one must have, IN THIS DOCUMENT, the thing it
	//! needs in order to ever fire.
	//!
	//! ══ T-181.37 — WHY SCHEMA MEMBERSHIP WAS NEVER ENOUGH ═══════════════════════════════════════
	//! `IsKnownEndTrigger` asks whether a trigger is a legal enum value. Every golden mission passes
	//! that and always did. It says nothing about whether the mission CONTAINS what the trigger
	//! watches, and all five triggers have their own way of being inert:
	//!
	//!   faction_eliminated       needs two sides that actually hold slots
	//!   time_limit               needs `flow.timeLimitSeconds` — the trigger is the "whether",
	//!                            the duration is the "how long", and either alone does nothing
	//!   all_objectives_captured  needs at least one `objective_capture` zone
	//!   objective_destroyed      needs at least one `objective_destroy` zone
	//!   hold_expired             needs at least one `objective_hold_until` zone
	//!
	//! A mission declaring `all_objectives_captured` with no capture zone is exactly as broken as
	//! one declaring an unimplemented trigger: it validates clean, boots clean, and runs forever.
	//! That is what these checks close, at LOAD, by name, with the missing piece named.
	//!
	//! ══ WHY THE RUNTIME WARNINGS ARE NOT A SUBSTITUTE ═══════════════════════════════════════════
	//! Two other places already say some of this, and both say it too late to be a gate:
	//!   * `TBD_ObjectiveRegistry.ReportTriggerCoverage` runs when the objective registry BUILDS,
	//!     which needs `TBD_ObjectivesComponent` to be on the game-mode prefab — and a component on
	//!     a prefab is exactly the thing that gets dropped silently (recorded landmine). It also
	//!     never runs at all for a document this validator has already rejected.
	//!   * `TBD_FrameworkManager.ArmRoundClock` reports both halves of the `time_limit` pair, but it
	//!     hangs off the LIVE transition. `world-boot.sh --mission=` boots with ZERO players and
	//!     never leaves LOADING, so nothing in a wave gate has ever observed it.
	//! Validation runs on every load on every host, and its findings are the ones `#tbd validate`
	//! replays to an admin in game. These belong here as well, not instead.
	//!
	//! ══ WHAT THIS DELIBERATELY DOES NOT CLAIM ═══════════════════════════════════════════════════
	//! This is a DOCUMENT check. It proves the mission carries the piece each trigger watches; it
	//! cannot prove that piece will resolve at runtime. `objective_destroyed` is the live example:
	//! `last-stand-at-montfort.json` authors a perfectly good `objective_destroy` zone, so this
	//! passes it — and the objective still goes INERT at LIVE because the mod does not place the
	//! document's `entities[]` and `comp:ammo_cache` is not in `Data/registry.json`.
	//! `TBD_ObjectiveRegistry.ArmDestroyTargets` owns that verdict and states it in full. Restating
	//! a build limitation here would hardcode, in the validator, a fact that becomes false the day
	//! an entity-placement slice lands — the same drift this check exists to prevent.
	//!
	//! T-181.13.1 — CONTENT, not non-null, here too. `mission.winConditions` is a `ref <class>`, so
	//! `JsonLoadContext` allocates it whether or not the document authored the key (same landmine as
	//! `slot.loadout` above): the old `if (!conditions)` guard could NEVER fire, so a mission that
	//! declared no win conditions at all got two vaguer findings (empty mode + empty endOn) instead
	//! of the one precise "absent" line this check was written to give. Both fields carrying nothing
	//! is the observable form of "no block".
	protected static void CheckWinConditions(TBD_MissionDocumentStruct mission, map<string, int> slotsPerFaction)
	{
		TBD_MissionWinConditionsStruct conditions = mission.winConditions;

		bool hasMode = false;
		bool hasEndOn = false;
		if (conditions)
		{
			hasMode = !conditions.mode.IsEmpty();
			if (conditions.endOn)
				hasEndOn = !conditions.endOn.IsEmpty();
		}

		if (!hasMode && !hasEndOn)
		{
			AddWarning("winConditions", "absent or empty — the round has no end trigger and runs until an admin ends it");
			return;
		}

		if (!hasMode)
			AddWarning("winConditions.mode", "empty — the round has no declared mode label");

		if (!hasEndOn)
		{
			AddWarning("winConditions.endOn", "empty — the round has no end trigger and runs until an admin ends it");
			return;
		}

		// Computed once, before the loop, because the faction_eliminated check needs it and the
		// census is the same for every trigger.
		int sidesWithSlots = 0;
		foreach (string factionKey, int count : slotsPerFaction)
		{
			if (count > 0)
				sidesWithSlots++;
		}

		int declared = 0;
		int reachable = 0;

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

			declared++;
			if (CheckTriggerReachable(subject, trigger, mission, sidesWithSlots))
				reachable++;
		}

		// The roll-up, and the only finding that reads the mission as a whole. Each unreachable
		// trigger above is survivable on its own — the round still ends on one of the others. All of
		// them unreachable means the round has NO way to end, which is the same state as an empty
		// `endOn` and is warned about in the same words, so an operator recognises it.
		if (declared > 0 && reachable == 0)
		{
			AddWarning("winConditions.endOn", string.Format(
				"NONE of the %1 declared end trigger(s) can fire in this mission — the round has no end trigger and runs until an admin ends it. Each one is reported above with the piece it is missing.",
				declared));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Can THIS trigger fire in THIS document? Reports the specific missing piece when it cannot.
	//! Returns TRUE when the document carries everything the trigger needs.
	protected static bool CheckTriggerReachable(string subject, string trigger,
		TBD_MissionDocumentStruct mission, int sidesWithSlots)
	{
		if (trigger == TRIGGER_FACTION_ELIMINATED)
			return CheckFactionEliminatedReachable(subject, mission, sidesWithSlots);

		if (trigger == TRIGGER_TIME_LIMIT)
			return CheckTimeLimitReachable(subject, mission);

		string zoneType;
		TBD_EObjectiveKind kind = ObjectiveKindFor(trigger, zoneType);

		// Cannot fire today: IsKnownEndTrigger admits exactly five values and the four above are
		// all handled. Kept because the alternative — assuming the fifth branch is total — is how a
		// future sixth trigger would be silently reported as reachable.
		if (kind == TBD_EObjectiveKind.NONE)
			return true;

		return CheckObjectiveTriggerReachable(subject, trigger, zoneType, kind, mission);
	}

	//------------------------------------------------------------------------------------------------
	//! `faction_eliminated` — two sides must actually hold slots.
	//!
	//! TBD_FrameworkManager.TickWinConditions already refuses to end a round with fewer than two
	//! CONTESTING factions, so a one-sided mission does not actually end at kickoff — it just
	//! never ends, which looks identical to a broken win condition from the server console.
	//! Catching it here turns that mystery into an authoring error before anyone joins.
	//!
	//! This is the ONE end-trigger finding that blocks, and the class header says why: a document
	//! with no second side is not a round that ends late, it is not a PvP event at all.
	protected static bool CheckFactionEliminatedReachable(string subject,
		TBD_MissionDocumentStruct mission, int sidesWithSlots)
	{
		// A document with no slots has no sides to eliminate; CheckSlots already said so, and
		// repeating it here would bury that finding under a second one saying the same thing. Not
		// reachable either, so the roll-up above is told the truth.
		if (!mission.slots || mission.slots.IsEmpty())
			return false;

		if (sidesWithSlots < 2)
		{
			AddError(subject, string.Format(
				"declares faction_eliminated but only %1 faction(s) actually have slots — no second side can ever be eliminated, so the round can never resolve",
				sidesWithSlots));
			return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! `time_limit` — the trigger says the round MAY end on the clock; `flow.timeLimitSeconds` says
	//! how long. Either half alone does nothing, and TBD_FrameworkManager.ArmRoundClock refuses to
	//! guess the other.
	//!
	//! The resolution rule is NOT reimplemented here. `TBD_MissionFlow.ResolveSeconds` is the one
	//! place that decides what absent / negative / zero mean for all three flow durations, it is
	//! pure and it logs nothing, so this asks it and branches on the `source` label it returns. A
	//! second copy of "0 means an explicit no-limit" is exactly the drift this slice is about.
	//!
	//! Only the half this slice owns is reported: a DECLARED trigger that cannot fire. The mirror
	//! image — a duration authored with no `time_limit` trigger — is T-181.38's finding and is
	//! reported by `ArmRoundClock`; it is not a trigger this mission cannot end on.
	protected static bool CheckTimeLimitReachable(string subject, TBD_MissionDocumentStruct mission)
	{
		// `flow` is a `ref <class>`, so JsonLoadContext allocates it even for a document with no
		// `flow` key and this guard cannot fire. Kept only so a null deref is impossible; ABSENT is
		// the value that actually carries "not authored" into ResolveSeconds.
		int raw = TBD_MissionFlowStruct.ABSENT;
		if (mission.flow)
			raw = mission.flow.timeLimitSeconds;

		string source;
		int seconds = TBD_MissionFlow.ResolveSeconds(raw, source);

		if (source == TBD_MissionFlow.SRC_DEFAULT)
		{
			AddWarning(subject, "declares 'time_limit' but flow.timeLimitSeconds is not authored — TBD_FrameworkManager.ArmRoundClock has no duration to arm, so this round CANNOT end on time. Author flow.timeLimitSeconds, or drop the trigger.");
			return false;
		}

		if (source == TBD_MissionFlow.SRC_INVALID)
		{
			AddWarning(subject, string.Format(
				"declares 'time_limit' but flow.timeLimitSeconds=%1 is negative (mission.schema.json requires >= 0) — the clock is not armed, so this round CANNOT end on time.",
				raw));
			return false;
		}

		if (seconds == 0)
		{
			AddWarning(subject, "declares 'time_limit' but flow.timeLimitSeconds=0, which is an explicit NO LIMIT — the clock is deliberately not armed, so this trigger will never end the round. Author a real duration, or drop the trigger.");
			return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! `all_objectives_captured` / `objective_destroyed` / `hold_expired` — each needs at least one
	//! zone of its own kind, carrying geometry something can actually be inside.
	protected static bool CheckObjectiveTriggerReachable(string subject, string trigger, string zoneType,
		TBD_EObjectiveKind kind, TBD_MissionDocumentStruct mission)
	{
		int placeable;
		int total = CountObjectiveZones(mission, kind, placeable);

		if (total == 0)
		{
			AddWarning(subject, string.Format(
				"declares '%1' but this mission has no '%2' zone for it to watch — that trigger can NEVER fire. Add one to zones[], or drop the trigger.",
				trigger, zoneType));
			return false;
		}

		if (placeable == 0)
		{
			// Not reachable from a schema-valid document — `$defs/circle` puts `exclusiveMinimum: 0`
			// on `r` and `$defs/polygon` requires three vertices — so this fires only on a
			// hand-edited or hand-generated file. Same status, and same value, as the spawn-zone
			// shape warning in CheckZones: the compiler is not the only thing that writes these.
			AddWarning(subject, string.Format(
				"declares '%1' and this mission has %2 '%3' zone(s), but none of them carries a usable shape (no circle with r > 0 and no polygon) — nothing can ever be inside them, so that trigger can NEVER fire.",
				trigger, total, zoneType));
			return false;
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! How many `kind` objective zones the document declares, and — through `outPlaceable` — how
	//! many of those carry geometry.
	//!
	//! A zone's kind is decided by `TBD_ObjectiveRegistry.KindOf`, the same call the objective
	//! registry itself uses, so this file never spells an objective zone-type string and a renamed
	//! or retired type cannot leave it counting something nothing produces.
	//!
	//! CONTENT, not non-null: `shape` and `shape.circle` are `ref <class>` fields that
	//! JsonLoadContext allocates whether or not the key was authored (recorded landmine — a polygon-
	//! only zone comes back with a non-null circle at x=0 z=0 r=0). `circle.r > 0` is the scalar
	//! sentinel and `polygon.Count() > 0` is the container count; `if (shape.circle)` is always true
	//! and would count a shapeless zone as placeable.
	protected static int CountObjectiveZones(TBD_MissionDocumentStruct mission, TBD_EObjectiveKind kind,
		out int outPlaceable)
	{
		outPlaceable = 0;

		if (!mission.zones)
			return 0;

		int total = 0;

		foreach (TBD_MissionZoneStruct zone : mission.zones)
		{
			if (!zone)
				continue;

			if (TBD_ObjectiveRegistry.KindOf(zone.type) != kind)
				continue;

			total++;

			if (!zone.shape)
				continue;

			if (zone.shape.circle && zone.shape.circle.r > 0)
				outPlaceable++;
			else if (zone.shape.polygon && zone.shape.polygon.Count() > 0)
				outPlaceable++;
		}

		return total;
	}

	//------------------------------------------------------------------------------------------------
	//! Which objective kind a `winConditions.endOn` trigger needs at least one of, and the schema
	//! zone type to name in the finding. `TBD_EObjectiveKind.NONE` means this module does not drive
	//! that trigger.
	//!
	//! ══ THE ONE PAIRING THIS FILE OWNS — AND WHERE IT BELONGS ═══════════════════════════════════
	//! Everything else is asked of the authority: the trigger names are
	//! `TBD_ObjectiveRegistry.TRIGGER_*`, the zone-type names are `TBD_ObjectiveRegistry.TYPE_*`,
	//! and a zone's kind comes from `TBD_ObjectiveRegistry.KindOf`. So neither a renamed trigger nor
	//! a renamed zone type can leave this check quietly validating against a vocabulary nothing
	//! produces. The PAIRING between the two is the one fact with no accessor to ask, and it is a
	//! second copy of the pairing `TBD_ObjectiveRegistry.ReportTriggerCoverage` already makes.
	//!
	//! Its right home is beside `KindOf()` in `Objectives/TBD_ObjectiveRegistry.c`, as
	//!     static TBD_EObjectiveKind KindForTrigger(string trigger, out string zoneType)
	//! with `ReportTriggerCoverage` driven from it too, at which point neither file holds a copy.
	//! `Objectives/**` belongs to another slice's lane (wave_plan.tsv: T-181.39), so it is reported
	//! rather than edited — and it lives in ONE function here, not sprinkled through the checks, so
	//! moving it is a delete and a call.
	protected static TBD_EObjectiveKind ObjectiveKindFor(string trigger, out string zoneType)
	{
		if (trigger == TBD_ObjectiveRegistry.TRIGGER_ALL_CAPTURED)
		{
			zoneType = TBD_ObjectiveRegistry.TYPE_CAPTURE;
			return TBD_EObjectiveKind.CAPTURE;
		}

		if (trigger == TBD_ObjectiveRegistry.TRIGGER_DESTROYED)
		{
			zoneType = TBD_ObjectiveRegistry.TYPE_DESTROY;
			return TBD_EObjectiveKind.DESTROY;
		}

		if (trigger == TBD_ObjectiveRegistry.TRIGGER_HOLD_EXPIRED)
		{
			zoneType = TBD_ObjectiveRegistry.TYPE_HOLD_UNTIL;
			return TBD_EObjectiveKind.HOLD_UNTIL;
		}

		zoneType = string.Empty;
		return TBD_EObjectiveKind.NONE;
	}

	//------------------------------------------------------------------------------------------------
	//! mission.schema.json#/$defs/winConditions endOn enum membership.
	//!
	//! Schema legality ONLY — see CheckWinConditions for why that was never sufficient on its own.
	//! The three objective triggers are read from the module that evaluates them rather than
	//! re-spelled here; see the constants block at the top of the class.
	protected static bool IsKnownEndTrigger(string trigger)
	{
		return trigger == TRIGGER_TIME_LIMIT
			|| trigger == TRIGGER_FACTION_ELIMINATED
			|| trigger == TBD_ObjectiveRegistry.TRIGGER_ALL_CAPTURED
			|| trigger == TBD_ObjectiveRegistry.TRIGGER_DESTROYED
			|| trigger == TBD_ObjectiveRegistry.TRIGGER_HOLD_EXPIRED;
	}

	//------------------------------------------------------------------------------------------------
	//! Zones are light-touch: only a faction reference that does not exist is fatal. A spawn zone
	//! the loader cannot place from is a warning.
	//!
	//! T-181.18 — this comment used to claim polygon shapes were "deliberately not modelled yet".
	//! They are modelled now. `GetSpawnZoneForFaction` still places from a CIRCLE, so the warning
	//! below is specifically "no circle with a real radius", which a polygon-only spawn zone also
	//! trips — correctly, since the loader cannot place from it.
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

			// CONTENT, not non-null. `JsonLoadContext` allocates a nested `ref` field whether or
			// not the JSON key was present, so `zone.shape.circle` is ALWAYS non-null and the old
			// `!zone.shape.circle` test could NEVER fire — this warning was unreachable, and a
			// polygon-only spawn zone sailed through it into GetSpawnZoneForFaction, which would
			// have placed the faction at the map corner (0,0). See the landmine on
			// TBD_MissionShapeStruct in TBD_MissionLoader.c.
			if (zone.type == ZONE_SPAWN && (!zone.shape || !zone.shape.circle || zone.shape.circle.r <= 0))
				AddWarning(subject, "spawn zone has no circle with a usable radius — TBD_MissionLoader.GetSpawnZoneForFaction cannot place from it");
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
