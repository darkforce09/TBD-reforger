/**
 * @file TBD_MissionValidator.c
 * @brief One pass over a parsed mission document that reports every problem, then blocks on errors.
 *
 * Role: runs every mission check and keeps the last run's findings for `#tbd validate`.
 * Position: `TBD_MissionLoader.ParseMissionJson` calls `Run` after the variant filter; the checks
 * live in `TBD_MissionStructureChecks`, `TBD_MissionSlotChecks`, `TBD_MissionWinConditionChecks`
 * and `TBD_MissionUnconsumedKeyCheck`; `TBD_AdminCommands` reads `BuildReportLines`.
 * State: the last run's `TBD_MissionValidationFindings`, on the server; it outlives the rejected
 * document, and a run on a document with no `meta.id` names none, so an earlier world's verdict is
 * never shown as this one's.  Invariants: every check runs regardless of earlier failures; an ERROR
 * means the mission cannot be played (the document is discarded and the stage machine never leaves
 * LOADING), a WARNING that it can be played but may not end (the round runs).
 */

//! Mission validation entry point and the last run's verdict.
class TBD_MissionValidator
{
	protected static ref TBD_MissionValidationFindings s_LastFindings; //!< Findings of the last run; null before the first run.

	//! Validate a parsed mission document and log every finding.
	//! @param mission the document; null is one ERROR
	//! @return true when the run found no ERROR (warnings do not block)
	//! @authority server
	static bool Run(TBD_MissionDocumentStruct mission)
	{
		TBD_MissionValidationFindings findings = new TBD_MissionValidationFindings();
		s_LastFindings = findings;

		if (!mission)
		{
			findings.AddError("mission", "no document to validate (the JSON deserialised to null)");
			findings.Report();
			return false;
		}

		if (mission.meta)
			findings.SetSubjectMissionId(mission.meta.id);

		bool slotsRequired = TBD_MissionStructureChecks.CheckSchemaVersion(findings, mission);
		TBD_MissionStructureChecks.CheckMeta(findings, mission);

		map<string, bool> declaredFactions = new map<string, bool>();
		TBD_MissionStructureChecks.CheckFactions(findings, mission, declaredFactions);

		map<string, int> slotsPerFaction = new map<string, int>();
		TBD_MissionSlotChecks.CheckSlots(findings, mission, slotsRequired, declaredFactions, slotsPerFaction);

		TBD_MissionStructureChecks.CheckOrbatSlotParity(findings, mission);
		TBD_MissionStructureChecks.CheckFactionCoverage(findings, mission, slotsPerFaction);
		TBD_MissionWinConditionChecks.CheckWinConditions(findings, mission, slotsPerFaction);
		TBD_MissionStructureChecks.CheckZones(findings, mission, declaredFactions);
		TBD_MissionUnconsumedKeyCheck.CheckUnconsumedKeys(findings, mission);

		findings.Report();
		return findings.Passed();
	}

	//! Whether `Run` has been called in this process.
	//! @return true after the first run
	static bool HasRun()
	{
		return s_LastFindings != null;
	}

	//! Whether the last run found no blocking error.
	//! @return true when a run happened and found no error
	static bool Passed()
	{
		return s_LastFindings && s_LastFindings.Passed();
	}

	//! The number of blocking findings of the last run.
	//! @return the count; 0 before the first run
	static int GetErrorCount()
	{
		if (!s_LastFindings)
			return 0;
		return s_LastFindings.GetErrorCount();
	}

	//! The number of non-blocking findings of the last run.
	//! @return the count; 0 before the first run
	static int GetWarningCount()
	{
		if (!s_LastFindings)
			return 0;
		return s_LastFindings.GetWarningCount();
	}

	//! The blocking findings of the last run.
	//! @return the lines, or null before the first run
	static array<string> GetErrors()
	{
		if (!s_LastFindings)
			return null;
		return s_LastFindings.GetErrors();
	}

	//! The non-blocking findings of the last run.
	//! @return the lines, or null before the first run
	static array<string> GetWarnings()
	{
		if (!s_LastFindings)
			return null;
		return s_LastFindings.GetWarnings();
	}

	//! The last run's findings rendered for the in-game admin (`#tbd validate`), truncated for chat.
	//! @return the report lines; one line saying so before the first run
	static array<string> BuildReportLines()
	{
		if (!s_LastFindings)
		{
			array<string> lines = new array<string>();
			lines.Insert("TBD validate: no mission has been parsed yet.");
			return lines;
		}

		return s_LastFindings.BuildReportLines();
	}
}
