/**
 * @file TBD_ObjectiveEntityReader.c
 * @brief Reads the mission's top-level `objectives[]` rows and hands them out by `zoneId`.
 *
 * Role: the typed-objective pass: binds each `objectives[]` row (identity, side, label, per-side
 * framing, `lock`, `autoLose`, `variantId`) so `TBD_ObjectiveTypedBinder` can join it onto its
 * objective zone.  Position: called by `TBD_ObjectiveRegistry.Build`; reads the held mission
 * JSON through `TBD_MissionJsonPass`, gated by the loader's active variant set.
 * State: the rows and the claimed zone ids of the current world, cleared by
 * `TBD_ObjectiveRegistry.Clear` because statics outlive a world.  Invariants: a client holds no
 * mission text and reads no rows; the first row for a `zoneId` wins; every row that binds to no
 * objective zone is reported by name; a nested framing ref is always allocated, so its strings
 * are the presence test.
 */

//! One side's framing: task title and body text, both optional.
//! @contract mission.schema.json#/$defs/objectiveFraming
class TBD_ObjectiveFramingSideStruct
{
	string title; //!< `title`; empty when absent
	string text; //!< `text`; empty when absent
}

//! `objectives[].framing`: the attacker's and defender's readings of one objective. Both refs
//! are allocated whether or not the keys were authored.
//! @contract mission.schema.json#/$defs/objective/properties/framing
class TBD_ObjectiveFramingStruct
{
	ref TBD_ObjectiveFramingSideStruct attacker; //!< `attacker`
	ref TBD_ObjectiveFramingSideStruct defender; //!< `defender`
}

//! One `objectives[]` row. Member names equal the JSON keys, because `JsonLoadContext` binds by
//! member name and a key no member spells is silently absent.
//! @contract mission.schema.json#/$defs/objective
class TBD_ObjectiveEntityStruct
{
	string id; //!< `id`, the objective's own identity
	string type; //!< `type`: capture | destroy | hold | defend
	string side; //!< `side`: the faction the task is for
	string zoneId; //!< `zoneId`: the objective zone this row frames
	string label; //!< `label`: the objective's display name
	ref TBD_ObjectiveFramingStruct framing; //!< `framing`
	bool lock; //!< `lock`; absent and authored false both read false, the schema default
	string autoLose; //!< `autoLose`: the faction that loses if the objective falls
	string variantId; //!< `variantId`; empty = in every variant
}

//! The document root of the typed-objective pass; declares `objectives` only.
//! @contract mission.schema.json#/properties/objectives
class TBD_ObjectiveEntityDocStruct
{
	ref array<ref TBD_ObjectiveEntityStruct> objectives; //!< `objectives[]`
}

//! Reads `objectives[]` once per world and hands rows out by `zoneId`.
class TBD_ObjectiveEntityReader
{
	static const string CH = "ObjTyped"; //!< log channel of the typed pass
	static const string TASK_CAPTURE = "capture"; //!< `objectives[].type` task vocabulary, distinct from the zone types and end triggers
	static const string TASK_DESTROY = "destroy"; //!< `objectives[].type` value
	static const string TASK_HOLD    = "hold"; //!< `objectives[].type` value
	static const string TASK_DEFEND  = "defend"; //!< `objectives[].type` value: the defender side of a capture

	protected static ref array<ref TBD_ObjectiveEntityStruct> s_aRows; //!< rows that passed variant gating; null until `Read`
	protected static ref array<string> s_aClaimedZoneIds; //!< zone ids a prepared objective bound; the rest are reported
	protected static bool s_bRead; //!< `Read` ran since the last `Clear`
	protected static bool s_bOk; //!< the pass produced an `objectives[]` array
	protected static int s_iGatedOut; //!< rows excluded by the active variant set

	//! Drop every row; the next `Read` parses again.
	static void Clear()
	{
		s_aRows = null;
		s_aClaimedZoneIds = null;
		s_bRead = false;
		s_bOk = false;
		s_iGatedOut = 0;
	}

	//! Whether the pass produced an `objectives[]` array. False is not an error: a document
	//! without the key is valid.
	static bool IsOk()
	{
		return s_bOk;
	}

	//! How many rows survived variant gating.
	static int Count()
	{
		if (!s_aRows)
			return 0;

		return s_aRows.Count();
	}

	//! How many rows the active variant set excluded.
	static int GatedOutCount()
	{
		return s_iGatedOut;
	}

	//! Parse `objectives[]`. Idempotent: only the first call after a `Clear` does work. A row
	//! whose `variantId` is outside the loader's active set is excluded and counted.
	//! @return true when the document carries an `objectives[]` array
	//! @authority server
	static bool Read()
	{
		if (s_bRead)
			return s_bOk;

		s_bRead = true;
		s_bOk = false;
		s_iGatedOut = 0;
		s_aRows = new array<ref TBD_ObjectiveEntityStruct>();
		s_aClaimedZoneIds = new array<string>();

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
			return false;

		TBD_ObjectiveEntityDocStruct doc = new TBD_ObjectiveEntityDocStruct();
		if (!ctx.ReadValue("", doc))
			return false;

		if (!doc.objectives)
			return false;

		foreach (TBD_ObjectiveEntityStruct row : doc.objectives)
		{
			if (!row)
				continue;

			if (!TBD_MissionVariants.IsActive(row.variantId, TBD_MissionLoader.GetActiveVariantIds()))
			{
				s_iGatedOut++;
				continue;
			}

			s_aRows.Insert(row);
		}

		s_bOk = true;
		WarnDuplicateZoneIds();
		return true;
	}

	//! The row for one zone, joined by `zoneId` (the two arrays share no index).
	//! @return the first row naming `zoneId`, or null
	static TBD_ObjectiveEntityStruct ForZone(string zoneId)
	{
		if (!s_aRows || zoneId.IsEmpty())
			return null;

		foreach (TBD_ObjectiveEntityStruct row : s_aRows)
		{
			if (row && row.zoneId == zoneId)
				return row;
		}

		return null;
	}

	//! Record that a prepared objective took this zone's row.
	static void MarkClaimed(string zoneId)
	{
		if (!s_aClaimedZoneIds || zoneId.IsEmpty())
			return;

		if (s_aClaimedZoneIds.Find(zoneId) != -1)
			return;

		s_aClaimedZoneIds.Insert(zoneId);
	}

	//! Warn, by name, for every row that bound to no objective: one with no `zoneId`, or one
	//! naming a zone that is missing or not an objective zone.
	static void ReportUnclaimed()
	{
		if (!s_aRows)
			return;

		foreach (TBD_ObjectiveEntityStruct row : s_aRows)
		{
			if (!row)
				continue;

			if (row.zoneId.IsEmpty())
			{
				TBD_Log.Warn(CH, string.Format("objectives[] row id='%1' names no `zoneId`, so it has no geometry and nobody can ever be inside it. It is carried on the wire and does nothing. Point it at a `zones[].id` of an objective zone.",
					row.id));
				continue;
			}

			if (s_aClaimedZoneIds && s_aClaimedZoneIds.Find(row.zoneId) != -1)
				continue;

			TBD_Log.Warn(CH, string.Format("objectives[] row id='%1' names zoneId '%2', which no prepared objective zone matched -- either no zone carries that id, or that zone's type is not one of %3 / %4 / %5. The row is inert.",
				row.id, row.zoneId,
				TBD_ObjectiveRegistry.TYPE_CAPTURE, TBD_ObjectiveRegistry.TYPE_DESTROY, TBD_ObjectiveRegistry.TYPE_HOLD_UNTIL));
		}
	}

	//! Warn for every second row naming the same `zoneId`, which the schema forbids (one entity,
	//! two framings); the first row wins.
	protected static void WarnDuplicateZoneIds()
	{
		if (!s_aRows)
			return;

		array<string> seen = new array<string>();

		foreach (TBD_ObjectiveEntityStruct row : s_aRows)
		{
			if (!row || row.zoneId.IsEmpty())
				continue;

			if (seen.Find(row.zoneId) != -1)
			{
				TBD_Log.Warn(CH, string.Format("objectives[] carries more than one row for zoneId '%1' (this one is id='%2'). mission.schema.json forbids that shape: ONE ENTITY, TWO FRAMINGS, never two rows for one objective. The FIRST row wins; author the other side under `framing.attacker` / `framing.defender` on that row instead.",
					row.zoneId, row.id));
				continue;
			}

			seen.Insert(row.zoneId);
		}
	}

	//! The zone kind a task type belongs on; `defend` is the far side of a capture.
	//! @return the kind, or NONE for an unknown type
	static TBD_EObjectiveKind KindOfTaskType(string taskType)
	{
		if (taskType == TASK_CAPTURE || taskType == TASK_DEFEND)
			return TBD_EObjectiveKind.CAPTURE;

		if (taskType == TASK_DESTROY)
			return TBD_EObjectiveKind.DESTROY;

		if (taskType == TASK_HOLD)
			return TBD_EObjectiveKind.HOLD_UNTIL;

		return TBD_EObjectiveKind.NONE;
	}

	//! Whether the task type frames `side` as the defender (`hold`, `defend`).
	static bool IsDefenderFraming(string taskType)
	{
		return taskType == TASK_HOLD || taskType == TASK_DEFEND;
	}
}
