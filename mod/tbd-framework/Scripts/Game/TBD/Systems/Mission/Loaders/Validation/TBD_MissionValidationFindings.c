/**
 * @file TBD_MissionValidationFindings.c
 * @brief The errors and warnings of one validation run, their log report and their chat rendering.
 *
 * Role: the finding buckets every mission check writes into.  Position: created by
 * `TBD_MissionValidator.Run`, filled by the structure, slot, win-condition and unconsumed-key
 * checks, reported to the log at the end of the run, and kept by the validator for `#tbd validate`
 * (`TBD_AdminCommands`).
 * State: the two finding lists and the mission id they belong to, owned by one run on the server.
 * Invariants: every finding reads `<subject> -- <message>`; an ERROR blocks the load, a WARNING
 * does not; the lists outlive the rejected document, so a rejected load stays explainable.
 */

//! The findings of one validation run.
class TBD_MissionValidationFindings : Managed
{
	protected static const int CHAT_ERROR_LINES = 15;   //!< Error lines `#tbd validate` shows in chat before a "... and N more" tail.
	protected static const int CHAT_WARNING_LINES = 10; //!< Warning lines `#tbd validate` shows in chat before a tail.

	protected ref array<string> m_aErrors = new array<string>();   //!< Blocking findings, in check order.
	protected ref array<string> m_aWarnings = new array<string>(); //!< Non-blocking findings, in check order.
	protected string m_sSubjectMissionId;                           //!< `meta.id` of the document checked; empty when it has none.

	//! Name the mission these findings belong to.
	//! @param missionId the document's `meta.id`
	void SetSubjectMissionId(string missionId)
	{
		m_sSubjectMissionId = missionId;
	}

	//! Whether the run found no blocking error.
	//! @return true when there is no error
	bool Passed()
	{
		return m_aErrors.IsEmpty();
	}

	//! The number of blocking findings.
	//! @return the error count
	int GetErrorCount()
	{
		return m_aErrors.Count();
	}

	//! The number of non-blocking findings.
	//! @return the warning count
	int GetWarningCount()
	{
		return m_aWarnings.Count();
	}

	//! The blocking findings.
	//! @return the error lines
	array<string> GetErrors()
	{
		return m_aErrors;
	}

	//! The non-blocking findings.
	//! @return the warning lines
	array<string> GetWarnings()
	{
		return m_aWarnings;
	}

	//! Record a blocking finding.
	//! @param subject the offending slot, faction or field
	//! @param message what is wrong and its consequence
	void AddError(string subject, string message)
	{
		m_aErrors.Insert(subject + " -- " + message);
	}

	//! Record a non-blocking finding; it is still logged and shown to admins.
	//! @param subject the offending slot, faction or field
	//! @param message what is wrong and its consequence
	void AddWarning(string subject, string message)
	{
		m_aWarnings.Insert(subject + " -- " + message);
	}

	//! Log one `[TBD][Validate]` line per finding, then the verdict; a rejection ends with a banner
	//! so it cannot scroll past the operator.
	void Report()
	{
		foreach (string finding : m_aErrors)
			TBD_Log.Error(TBD_Log.CH_VALIDATE, "ERROR   " + finding);

		foreach (string finding : m_aWarnings)
			TBD_Log.Warn(TBD_Log.CH_VALIDATE, "WARNING " + finding);

		bool passed = m_aErrors.IsEmpty();
		TBD_Log.ValidationResult(passed, m_aErrors.Count(), m_aWarnings.Count());

		if (passed)
			return;

		TBD_Log.Banner(TBD_Log.CH_VALIDATE, string.Format(
			"MISSION REJECTED -- %1 error(s). The server stays in LOADING until the mission is fixed. Admins: '#tbd validate'.",
			m_aErrors.Count()), true);
	}

	//! The findings rendered for the in-game admin, truncated because chat is not a log window.
	//! @return the verdict line, the capped findings and, on a rejection, the LOADING notice
	array<string> BuildReportLines()
	{
		array<string> lines = new array<string>();

		string verdict = "FAILED";
		if (Passed())
			verdict = "PASSED";

		string subject = m_sSubjectMissionId;
		if (subject.IsEmpty())
			subject = "(no meta.id)";

		lines.Insert(string.Format("TBD validate [%1]: %2 -- %3 error(s), %4 warning(s).",
			subject, verdict, GetErrorCount(), GetWarningCount()));

		AppendCapped(lines, m_aErrors, "ERROR", CHAT_ERROR_LINES);
		AppendCapped(lines, m_aWarnings, "WARN", CHAT_WARNING_LINES);

		if (!Passed())
			lines.Insert("TBD validate: mission is REJECTED -- the server stays in LOADING until it is fixed.");

		return lines;
	}

	//! Copy at most `cap` findings into `lines`, then a "... and N more" tail.
	//! @param lines the output
	//! @param findings the findings to copy; null or empty copies nothing
	//! @param label "ERROR" or "WARN"
	//! @param cap the most findings to copy
	protected static void AppendCapped(array<string> lines, array<string> findings, string label, int cap)
	{
		if (!findings || findings.IsEmpty())
			return;

		foreach (int i, string finding : findings)
		{
			if (i >= cap)
			{
				lines.Insert(string.Format("  ... and %1 more %2 finding(s) -- see the server console.",
					findings.Count() - cap, label));
				return;
			}

			lines.Insert(string.Format("  %1 %2", label, finding));
		}
	}
}
