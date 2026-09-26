/**
 * @file TBD_MissionSlotChecks.c
 * @brief Checks every slot: identity, faction, kit, spawn position, loadout, squad and heading.
 *
 * Role: the per-slot checks of mission validation and the per-faction slot census the win-condition
 * and coverage checks use.  Position: called by `TBD_MissionValidator.Run`; reads
 * `TBD_Registry.GetAllAliases` and the world bound box; writes into a
 * `TBD_MissionValidationFindings`.
 * State: none.  Invariants: an unplayable slot (no identity, no side, an unresolvable kit, off the
 * terrain, a malformed cargo row) is an ERROR, because the alternative is a player with no
 * character; a check that cannot be proven (registry empty, world box implausible) is skipped with
 * one WARNING for the document instead of one ERROR per slot.
 */

//! Static per-slot checks.
class TBD_MissionSlotChecks
{
	protected static const string KIT_PREFIX = "kit:";              //!< Registry alias prefix a slot kit carries (`$defs/slot/kit`).
	protected static const float BOUNDS_TOLERANCE_M = 1.0;          //!< Slack around the world box, metres, so a slot on the border passes.
	protected static const float BOUNDS_MIN_EXTENT_M = 100.0;       //!< Smallest world box extent, metres, trusted as a terrain.
	protected static const float BOUNDS_MAX_EXTENT_M = 200000.0;    //!< Largest world box extent, metres, trusted as a terrain.

	//! Check every slot: present, uniquely keyed, on a declared faction, with a resolvable kit,
	//! inside the terrain and carrying a sane loadout.
	//! @param findings receives the findings
	//! @param mission the document
	//! @param slotsRequired true when the schema version makes `slots[]` mandatory (ERROR when empty,
	//! else WARNING)
	//! @param declaredFactions the declared faction keys
	//! @param slotsPerFaction receives the slot count per faction key
	static void CheckSlots(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission, bool slotsRequired,
		map<string, bool> declaredFactions, map<string, int> slotsPerFaction)
	{
		array<ref TBD_MissionSlotStruct> slots = mission.slots;
		if (!slots || slots.IsEmpty())
		{
			if (slotsRequired)
			{
				findings.AddError("slots", string.Format(
					"schemaVersion %1 requires a non-empty slots[] -- no player can spawn", mission.schemaVersion));
			}
			else
			{
				findings.AddWarning("slots", "no slots[] -- a pre-1.1 document carries no spawn positions, so no player can spawn from it. Recompile the mission at schemaVersion 1.1 or later.");
			}

			return;
		}

		ref set<string> registryAliases = new set<string>();
		LoadRegistryAliases(findings, registryAliases);

		vector mins;
		vector maxs;
		bool boundsKnown = TryGetTerrainBounds(mins, maxs);
		if (!boundsKnown)
			findings.AddWarning("slots", "terrain bounds unavailable at validation time -- slot positions were NOT range-checked");

		// uid-else-id is the durable identity (TBD_MissionSlotStruct.Key), so that is the key that
		// must be unique. The display id is tracked separately because
		// TBD_MissionLoader.GetSlotById also matches on id -- two slots sharing an id make that
		// lookup ambiguous even when their keys differ. A duplicate uid is always a duplicate key
		// (Key() returns uid when set), so it needs no third set.
		ref set<string> seenKey = new set<string>();
		ref set<string> seenId = new set<string>();

		foreach (int i, TBD_MissionSlotStruct slot : slots)
		{
			string subject = string.Format("slots[%1]", i);

			if (!slot)
			{
				findings.AddError(subject, "null slot entry");
				continue;
			}

			if (slot.id.IsEmpty())
				findings.AddError(subject, "slot has no id");
			else
				subject = "slot:" + slot.Key();

			CheckSlotIdentity(findings, subject, slot, seenKey, seenId);
			CheckSlotFaction(findings, subject, slot, declaredFactions, slotsPerFaction);
			CheckSlotKit(findings, subject, slot, registryAliases);
			CheckSlotPosition(findings, subject, slot, boundsKnown, mins, maxs);
			CheckSlotLoadout(findings, subject, slot);

			if (slot.groupCallsign.IsEmpty())
				findings.AddWarning(subject, "no groupCallsign -- the slot has no squad to file under");

			if (slot.role.IsEmpty())
				findings.AddWarning(subject, "no role label");

			if (slot.headingDeg < 0 || slot.headingDeg > 360)
				findings.AddWarning(subject, string.Format("headingDeg=%1 is outside 0..360", slot.headingDeg));
		}
	}

	//! Duplicate slot keys, reported once per slot, most precise first. The key (uid, else id) is
	//! the durable identity; the display id is tracked too, because `TBD_MissionLoader.GetSlotById`
	//! also matches on id.
	//! @param findings receives the findings
	//! @param subject the slot's finding subject
	//! @param slot the slot
	//! @param seenKey the keys seen so far
	//! @param seenId the display ids seen so far
	protected static void CheckSlotIdentity(TBD_MissionValidationFindings findings, string subject, TBD_MissionSlotStruct slot,
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
			findings.AddError(subject, string.Format(
				"duplicate slot key '%1' (uid-else-id) -- two slots claim one identity, so claims, rosters and spawn points would collide",
				key));
			return;
		}

		if (idDuplicate)
		{
			findings.AddError(subject, string.Format(
				"duplicate slot id '%1' -- TBD_MissionLoader.GetSlotById matches id as well as uid, so lookups by this id are ambiguous",
				slot.id));
		}
	}

	//! The slot's faction must be set and declared. The census counts it even when undeclared.
	//! @param findings receives the findings
	//! @param subject the slot's finding subject
	//! @param slot the slot
	//! @param declaredFactions the declared faction keys; empty skips the declared test
	//! @param slotsPerFaction receives the slot count per faction key
	protected static void CheckSlotFaction(TBD_MissionValidationFindings findings, string subject, TBD_MissionSlotStruct slot,
		map<string, bool> declaredFactions, map<string, int> slotsPerFaction)
	{
		if (slot.faction.IsEmpty())
		{
			findings.AddError(subject, "no faction -- the slot cannot be assigned to a side");
			return;
		}

		// Counted even when undeclared: the slot still occupies that side for the
		// faction_eliminated arithmetic, and the undeclared-faction error below already
		// names the real problem.
		int count = 0;
		slotsPerFaction.Find(slot.faction, count);
		slotsPerFaction.Set(slot.faction, count + 1);

		// Skip when factions[] failed outright -- one error there beats one per slot here.
		if (declaredFactions.Count() == 0)
			return;

		if (!declaredFactions.Contains(slot.faction))
			findings.AddError(subject, string.Format("faction '%1' is not declared in factions[]", slot.faction));
	}

	//! The slot's kit must be a `kit:` alias the spawn registry resolves; `TBD_SpawnManager` treats
	//! an unresolvable kit as a permanent failure of that slot.
	//! @param findings receives the findings
	//! @param subject the slot's finding subject
	//! @param slot the slot
	//! @param registryAliases every registry alias; empty skips the resolution test
	protected static void CheckSlotKit(TBD_MissionValidationFindings findings, string subject, TBD_MissionSlotStruct slot, set<string> registryAliases)
	{
		if (slot.kit.IsEmpty())
		{
			findings.AddError(subject, "no kit alias -- TBD_SpawnManager has no prefab to spawn");
			return;
		}

		if (!slot.kit.StartsWith(KIT_PREFIX))
		{
			findings.AddError(subject, string.Format(
				"kit '%1' is not a kit: alias (mission.schema.json#/$defs/slot)", slot.kit));
			return;
		}

		// Registry unavailable -- LoadRegistryAliases already warned once for the document.
		// Staying quiet here beats emitting one unprovable error per slot.
		if (registryAliases.Count() == 0)
			return;

		if (!registryAliases.Contains(slot.kit))
		{
			findings.AddError(subject, string.Format(
				"kit alias '%1' does not resolve in the spawn registry -- TBD_SpawnManager would fail this slot permanently",
				slot.kit));
		}
	}

	//! The spawn position must land on the loaded terrain, within `BOUNDS_TOLERANCE_M`.
	//! @param findings receives the findings
	//! @param subject the slot's finding subject
	//! @param slot the slot
	//! @param boundsKnown false skips the check (see `TryGetTerrainBounds`)
	//! @param mins the world box minimum
	//! @param maxs the world box maximum
	protected static void CheckSlotPosition(TBD_MissionValidationFindings findings, string subject, TBD_MissionSlotStruct slot,
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

		findings.AddError(subject, string.Format(
			"spawn (%1, %2) is outside the loaded terrain (x %3..%4, z %5..%6) -- the player would drop into the void. Check that the mission terrain matches the loaded world.",
			slot.x, slot.z, mins[0], maxs[0], mins[2], maxs[2]));
	}

	//! The optional Arsenal loadout. The kit prefab stays authoritative, so an empty loadout is not
	//! fatal; a malformed cargo row blocks. The parse allocates `loadout` and `loadout.gear` even
	//! when absent, so gear presence is a count of refs; a `ref array<>` such as `cargo` is null
	//! when its key is absent, so a non-null empty `cargo` with no gear proves an authored empty
	//! loadout (a compiled mission never emits one) and warns.
	//! @param findings receives the findings
	//! @param subject the slot's finding subject
	//! @param slot the slot
	protected static void CheckSlotLoadout(TBD_MissionValidationFindings findings, string subject, TBD_MissionSlotStruct slot)
	{
		TBD_SlotLoadoutStruct loadout = slot.loadout;
		if (!loadout)
			return; // the parse always allocates it; the guard only prevents a null dereference

		int gearRefs = TBD_LoadoutInventoryUtil.CountGear(loadout.gear);

		// A non-null container means its key was authored; Count() says whether anything is in it.
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
				findings.AddWarning(subject, "loadout is present but carries neither gear nor cargo -- the slot falls back to the bare kit prefab");

			return;
		}

		if (cargoRows == 0)
			return;

		foreach (int c, TBD_SlotCargoStruct row : loadout.cargo)
		{
			string where = string.Format("%1 loadout.cargo[%2]", subject, c);

			if (!row)
			{
				findings.AddError(where, "null cargo row");
				continue;
			}

			if (row.container.IsEmpty())
				findings.AddError(where, "no container (expected vest / pants / jacket / backpack)");

			if (row.item.IsEmpty())
				findings.AddError(where, "no item ResourceName");

			if (row.qty < 1)
				findings.AddError(where, string.Format("qty=%1 -- mission.schema.json requires qty >= 1", row.qty));
		}
	}

	//! Fill `outAliases` with every registry alias. `TBD_Registry.GetAllAliases` loads the registry
	//! on demand. An empty registry is one WARNING: validation runs earlier in boot than the
	//! registry normally loads, and a missing registry fails loudly per slot in `TBD_SpawnManager`.
	//! @param findings receives the findings
	//! @param outAliases receives the aliases
	protected static void LoadRegistryAliases(TBD_MissionValidationFindings findings, set<string> outAliases)
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
			findings.AddWarning("registry", "no spawn-registry aliases available at validation time -- slot kit resolution was NOT verified");
	}

	//! The world box of the loaded terrain.
	//! @param mins receives the box minimum
	//! @param maxs receives the box maximum
	//! @return false when there is no world yet or the box extent is outside the plausible terrain
	//! window (still streaming, or an entity box)
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
}
