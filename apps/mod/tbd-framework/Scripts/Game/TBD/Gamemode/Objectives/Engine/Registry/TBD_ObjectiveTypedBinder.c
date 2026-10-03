/**
 * @file TBD_ObjectiveTypedBinder.c
 * @brief Joins an `objectives[]` row onto its prepared objective and reports what it bound.
 *
 * Role: copies a typed row's identity, side, label, per-side framing, `lock` and `autoLose` onto
 * the objective, reports every disagreement with the zone by name, and logs the typed coverage
 * of the build.  Position: called by `TBD_ObjectiveRegistry.Prepare`, `LogPrepared` and `Build`;
 * reads `TBD_ObjectiveEntityReader`, `TBD_DeclaredFactions` and the objective's
 * `TBD_ObjectiveKindBehaviour`.
 * State: none; writes only the objective it is given.  Invariants: an objective without a row is
 * left untouched; the zone wins every kind disagreement, because it is what the runtime
 * enforces; `lock` and `autoLose` are carried and reported, never enforced (their meaning in the
 * source missions is inferred); what a side supplies beyond the row is the kind behaviour's
 * `SeedFromSide` (a hold zone that names no holder takes it).
 */

//! Typed-row binding and reporting for prepared objectives.
class TBD_ObjectiveTypedBinder
{
	//! Log the typed half of one objective's load line: its row, side role, `lock`, `autoLose` and
	//! both framings. Logs nothing for an untyped objective.
	static void LogTyped(notnull TBD_Objective objective)
	{
		if (!objective.m_bTyped)
			return;

		string sideRole = "attacks";
		if (objective.m_bSideDefends)
			sideRole = "defends";

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "objectiveTyped", string.Format("zone=%1 id='%2' type='%3' side='%4' (%5) lock=%6 autoLose='%7'",
			objective.m_sId,
			objective.m_sEntityId,
			objective.m_sTaskType,
			objective.m_sSide,
			sideRole,
			objective.m_bLocked,
			objective.m_sAutoLoseFaction));

		if (!objective.HasFraming())
			return;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "objectiveFraming", string.Format("zone=%1 attacker='%2' | '%3'",
			objective.m_sId, objective.m_sAttackerTitle, objective.m_sAttackerText));

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "objectiveFraming", string.Format("zone=%1 defender='%2' | '%3'",
			objective.m_sId, objective.m_sDefenderTitle, objective.m_sDefenderText));
	}

	//! Join the `objectives[]` row naming this objective's zone, if any, and validate it. An absent
	//! `type` keeps the side role of the zone's own kind.
	//! @param objective the objective being prepared
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	static void Bind(notnull TBD_Objective objective, string subject)
	{
		TBD_ObjectiveEntityStruct row = TBD_ObjectiveEntityReader.ForZone(objective.m_sId);
		if (!row)
			return;

		TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.For(objective.m_eKind);

		TBD_ObjectiveEntityReader.MarkClaimed(objective.m_sId);

		objective.m_bTyped = true;
		objective.m_sEntityId = row.id;
		objective.m_sTaskType = row.type;
		objective.m_sSide = row.side;
		objective.m_bLocked = row.lock;
		objective.m_sAutoLoseFaction = row.autoLose;

		// An absent `type` leaves the framing on the zone's own kind rather than silently reading
		// as an attacker task: a hold zone is held by its faction whatever the row forgot to say.
		if (row.type.IsEmpty())
			objective.m_bSideDefends = behaviour.SideDefendsByDefault();
		else
			objective.m_bSideDefends = TBD_ObjectiveEntityReader.IsDefenderFraming(row.type);

		ApplyFraming(objective, row.framing);
		ApplyTypedLabel(objective, row, subject);
		CheckTypedKind(objective, row, subject);
		CheckTypedSide(objective, row, subject);
		CheckAutoLose(objective, subject);
		behaviour.SeedFromSide(objective, subject);
	}

	//! Copy both framing halves onto the objective. Each half is copied on its own, because both
	//! keys are optional and the nested refs are always allocated.
	protected static void ApplyFraming(notnull TBD_Objective objective, TBD_ObjectiveFramingStruct framing)
	{
		if (!framing)
			return;

		if (framing.attacker)
		{
			objective.m_sAttackerTitle = framing.attacker.title;
			objective.m_sAttackerText = framing.attacker.text;
		}

		if (framing.defender)
		{
			objective.m_sDefenderTitle = framing.defender.title;
			objective.m_sDefenderText = framing.defender.text;
		}
	}

	//! Take `objectives[].label` as the display name when authored; a label replacing a different
	//! zone label is logged as a note.
	protected static void ApplyTypedLabel(notnull TBD_Objective objective, TBD_ObjectiveEntityStruct row, string subject)
	{
		if (!row || row.label.IsEmpty())
			return;

		if (objective.m_sLabel == row.label)
			return;

		if (!objective.m_sLabel.IsEmpty())
			TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "note", string.Format("objective '%1' takes its display name from objectives[].label ('%2'); the zone's own label ('%3') names the VOLUME and is not what a player is shown",
				subject, row.label, objective.m_sLabel));

		objective.m_sLabel = row.label;
	}

	//! Warn when the row's task type is missing, unknown, or disagrees with the zone's kind; the
	//! zone's kind is what runs. `defend` on a capture zone agrees.
	protected static void CheckTypedKind(notnull TBD_Objective objective, TBD_ObjectiveEntityStruct row, string subject)
	{
		if (!row || row.type.IsEmpty())
		{
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objectives[] row for zone '%1' names no `type`, which #/$defs/objective requires. The zone's own type decides what runs; the per-side framing falls back to that kind.",
				subject));
			return;
		}

		TBD_ObjectiveKindBehaviour declared = TBD_ObjectiveKindBehaviour.ForTaskType(row.type);

		if (declared.Kind() == TBD_EObjectiveKind.NONE)
		{
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objectives[] row for zone '%1' has type='%2', which is not one of capture|destroy|hold|defend. The zone's own type still decides what runs.",
				subject, row.type));
			return;
		}

		if (declared.Kind() == objective.m_eKind)
			return;

		TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objectives[] row for zone '%1' declares type='%2' but the zone is type='%3'. THE ZONE WINS -- it is what the runtime enforces. Fix one of the two, or a player is given a task that does not match the rules being applied to them.",
			subject, row.type, objective.m_Zone.m_sType));
	}

	//! Validate `objectives[].side`: a side naming no faction marks the objective's side invalid
	//! (framing reads NEUTRAL); a side differing from the zone's faction is a note, because the
	//! faction restricts ownership while the side is who the task is for.
	protected static void CheckTypedSide(notnull TBD_Objective objective, TBD_ObjectiveEntityStruct row, string subject)
	{
		if (!row || row.side.IsEmpty())
			return;

		if (!TBD_DeclaredFactions.Exists(row.side))
		{
			objective.m_bInvalidSide = true;
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' names side='%2', which is no factions[].key. Nobody matches it, so the per-side framing reads NEUTRAL to every player and the objective falls back to its label.",
				subject, row.side));
			return;
		}

		if (objective.m_sFaction.IsEmpty() || objective.m_sFaction == row.side)
			return;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "note", string.Format("objective '%1' names side='%2' while its zone names faction='%3'. Both are kept and they are not the same claim: the zone's faction is the ownership restriction, the objective's side is who the task is for.",
			subject, row.side, objective.m_sFaction));
	}

	//! Validate `objectives[].autoLose`: a key naming no faction is blanked; a valid one is logged
	//! as carried and not acted on.
	protected static void CheckAutoLose(notnull TBD_Objective objective, string subject)
	{
		if (objective.m_sAutoLoseFaction.IsEmpty())
			return;

		if (!TBD_DeclaredFactions.Exists(objective.m_sAutoLoseFaction))
		{
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' declares autoLose='%2', which is no factions[].key. Dropping it rather than carrying a dangling side.",
				subject, objective.m_sAutoLoseFaction));
			objective.m_sAutoLoseFaction = string.Empty;
			return;
		}

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "note", string.Format("objective '%1' declares autoLose='%2'. CARRIED AND REPORTED ONLY -- nothing in this build ends a round on it. WOG's `_AutoLose` semantics are INFERRED and the round-end authority is TBD_FrameworkManager, not this file.",
			subject, objective.m_sAutoLoseFaction));
	}

	//! Log what the typed pass produced, always, even at zero: rows, bound, framed, locked and
	//! `autoLose` counts, the variant-gated count, a warning when any objective authors `lock`, and
	//! every row that bound to nothing.
	//! @param prepared the registry's prepared objectives; may be null
	static void ReportCoverage(array<ref TBD_Objective> prepared)
	{
		int bound = 0;
		int framed = 0;
		int locked = 0;
		int losers = 0;

		if (prepared)
		{
			foreach (TBD_Objective objective : prepared)
			{
				if (!objective || !objective.m_bTyped)
					continue;

				bound++;

				if (objective.HasFraming())
					framed++;

				if (objective.m_bLocked)
					locked++;

				if (!objective.m_sAutoLoseFaction.IsEmpty())
					losers++;
			}
		}

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "typed", string.Format("rows=%1 bound=%2 framed=%3 locked=%4 autoLose=%5",
			TBD_ObjectiveEntityReader.Count(), bound, framed, locked, losers));

		if (TBD_ObjectiveEntityReader.GatedOutCount() > 0)
			TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "note", string.Format("%1 objectives[] row(s) were excluded by the active variant set. A row excluded because its variantId names no variants[] entry cannot be told apart from one that is simply deselected here -- that walk belongs to TBD_MissionLoader.",
				TBD_ObjectiveEntityReader.GatedOutCount()));

		if (locked > 0)
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("%1 objective(s) author `lock: true`. THIS BUILD DOES NOT IMPLEMENT LOCKING -- the field is parsed, counted and reported, never enforced, and the round runs as if every objective were live. WOG's `_Lock` semantics are INFERRED and the parameter says 'at round start', which cannot be honoured without an unlock the corpus gives no evidence for.",
				locked));

		TBD_ObjectiveEntityReader.ReportUnclaimed();
	}
}
