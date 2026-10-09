/**
 * @file TBD_MissionStructureChecks.c
 * @brief Checks the document's version, header, factions, zones, ORBAT parity and faction coverage.
 *
 * Role: the document-shape checks of mission validation.  Position: called by
 * `TBD_MissionValidator.Run`; writes into a `TBD_MissionValidationFindings`.
 * State: none.  Invariants: every check runs to completion and reports every finding it can prove;
 * a nested object is tested on content, never on a null reference, because the parse allocates it
 * even when its key is absent; a finding already reported elsewhere is not repeated.
 */

//! Static document-shape checks.
class TBD_MissionStructureChecks
{
	protected static const string SCHEMA_1_0 = "1.0"; //!< Schema version without `slots[]`.
	protected static const string SCHEMA_1_1 = "1.1"; //!< Schema version that makes `slots[]` mandatory.
	protected static const string SCHEMA_1_2 = "1.2"; //!< Adds the optional per-slot y.
	protected static const string SCHEMA_1_3 = "1.3"; //!< Adds optional per-slot identity and group `leaderSlotId`.
	protected static const string ZONE_SPAWN = "spawn"; //!< The zone type `TBD_MissionLoader.GetSpawnZoneForFaction` places from.

	//! `schemaVersion` must be one this build understands; an unknown version may carry fields this
	//! build silently drops.
	//! @param findings receives the findings
	//! @param mission the document
	//! @return true when the version makes `slots[]` mandatory (1.1 and later)
	static bool CheckSchemaVersion(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission)
	{
		string version = mission.schemaVersion;

		if (version.IsEmpty())
		{
			findings.AddError("schemaVersion", "missing -- this build understands 1.0, 1.1, 1.2 and 1.3");
			return false;
		}

		if (version == SCHEMA_1_1 || version == SCHEMA_1_2 || version == SCHEMA_1_3)
			return true;

		if (version == SCHEMA_1_0)
			return false;

		findings.AddError("schemaVersion", string.Format(
			"'%1' is not recognised -- this build understands 1.0, 1.1, 1.2 and 1.3. A newer document may carry fields this server silently drops.",
			version));
		return false;
	}

	//! The mission header. The mod loads published missions only, which always carry a content-hash
	//! `meta.id`, so an empty id is an ERROR; the parse allocates `meta` even when absent, so the
	//! per-field emptiness tests are what catch a missing header.
	//! @param findings receives the findings
	//! @param mission the document
	static void CheckMeta(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission)
	{
		if (!mission.meta)
		{
			findings.AddError("meta", "missing meta block");
			return;
		}

		if (mission.meta.id.IsEmpty())
			findings.AddError("meta.id", "missing -- the mod only loads published missions, which always carry a content-hash id");

		if (mission.meta.name.IsEmpty())
			findings.AddWarning("meta.name", "empty -- the mission browser will show a blank row");

		if (mission.meta.terrain.IsEmpty())
			findings.AddWarning("meta.terrain", "empty -- admin mission switching cannot route this mission to a scenario");
	}

	//! At least one playable faction, each with a unique key.
	//! @param findings receives the findings
	//! @param mission the document
	//! @param declared receives every faction key that parsed, which later checks use to decide
	//! whether a referenced faction exists
	static void CheckFactions(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission, map<string, bool> declared)
	{
		array<ref TBD_MissionFactionStruct> factions = mission.factions;
		if (!factions || factions.IsEmpty())
		{
			findings.AddError("factions", "no playable faction declared -- nobody can pick a side");
			return;
		}

		foreach (int i, TBD_MissionFactionStruct faction : factions)
		{
			string subject = string.Format("factions[%1]", i);

			if (!faction)
			{
				findings.AddError(subject, "null faction entry");
				continue;
			}

			if (faction.key.IsEmpty())
			{
				findings.AddError(subject, "faction has no key -- slots cannot reference it");
				continue;
			}

			subject = "faction:" + faction.key;

			if (declared.Contains(faction.key))
			{
				findings.AddError(subject, "declared more than once -- faction lookups would be ambiguous");
				continue;
			}

			declared.Insert(faction.key, true);

			if (faction.displayName.IsEmpty())
				findings.AddWarning(subject, "no displayName -- the lobby will show the raw key");

			if (faction.presetId.IsEmpty())
				findings.AddWarning(subject, "no presetId -- the faction has no registry preset to build from");
		}
	}

	//! `slots[]` must materialize exactly the instance count the ORBAT declares; a mismatch means the
	//! compiled document is out of step with its own ORBAT. An empty `slots[]` is reported by
	//! `TBD_MissionSlotChecks` and not repeated.
	//! @param findings receives the findings
	//! @param mission the document
	static void CheckOrbatSlotParity(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission)
	{
		int expected = CountOrbatInstances(mission);
		if (expected <= 0)
			return;

		int actual = 0;
		if (mission.slots)
			actual = mission.slots.Count();

		// An empty slots[] is already reported by CheckSlots -- do not say it twice.
		if (actual == 0)
			return;

		if (actual != expected)
		{
			findings.AddError("orbat", string.Format(
				"orbat declares %1 slot instance(s) but slots[] carries %2 -- the compiled document is out of step with its ORBAT",
				expected, actual));
		}
	}

	//! The total `role.count` across every group of every faction in the ORBAT.
	//! @param mission the document
	//! @return the instance count; 0 when there is no ORBAT
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

	//! A declared faction with no slots is a side nobody can play: a WARNING, because an author may
	//! stage a side for a later mission.
	//! @param findings receives the findings
	//! @param mission the document
	//! @param slotsPerFaction the slot census from `TBD_MissionSlotChecks.CheckSlots`
	static void CheckFactionCoverage(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission, map<string, int> slotsPerFaction)
	{
		if (!mission.factions)
			return;

		// Nothing to say when the document carries no slots at all -- already reported once.
		if (!mission.slots || mission.slots.IsEmpty())
			return;

		foreach (TBD_MissionFactionStruct faction : mission.factions)
		{
			if (!faction || faction.key.IsEmpty())
				continue;

			int count = 0;
			slotsPerFaction.Find(faction.key, count);
			if (count == 0)
				findings.AddWarning("faction:" + faction.key, "declared but has no slots -- nobody can play this side");
		}
	}

	//! Zones: a faction reference that does not exist is an ERROR; a spawn zone without a circle of
	//! positive radius is a WARNING, because `TBD_MissionLoader.GetSpawnZoneForFaction` places from a
	//! circle only (a polygon-only spawn zone trips it too).
	//! @param findings receives the findings
	//! @param mission the document
	//! @param declaredFactions the declared faction keys
	static void CheckZones(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission, map<string, bool> declaredFactions)
	{
		if (!mission.zones || mission.zones.IsEmpty())
		{
			findings.AddWarning("zones", "no zones declared -- factions have no spawn-zone fallback position");
			return;
		}

		foreach (int i, TBD_MissionZoneStruct zone : mission.zones)
		{
			string subject = string.Format("zones[%1]", i);

			if (!zone)
			{
				findings.AddError(subject, "null zone entry");
				continue;
			}

			if (zone.id.IsEmpty())
				findings.AddWarning(subject, "zone has no id");
			else
				subject = "zone:" + zone.id;

			if (zone.type.IsEmpty())
				findings.AddWarning(subject, "zone has no type");

			if (!zone.faction.IsEmpty() && declaredFactions.Count() > 0 && !declaredFactions.Contains(zone.faction))
				findings.AddError(subject, string.Format("faction '%1' is not declared in factions[]", zone.faction));

			// Content, not non-null: the parse allocates `shape.circle` even for a polygon-only zone.
			if (zone.type == ZONE_SPAWN && (!zone.shape || !zone.shape.circle || zone.shape.circle.r <= 0))
				findings.AddWarning(subject, "spawn zone has no circle with a usable radius -- TBD_MissionLoader.GetSpawnZoneForFaction cannot place from it");
		}
	}
}
