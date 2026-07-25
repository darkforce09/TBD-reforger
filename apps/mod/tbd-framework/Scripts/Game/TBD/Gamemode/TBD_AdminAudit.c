//! T-181.11.2 — the admin audit trail.
//!
//! These are **powers, not conveniences**: under ONE LIFE an admin respawn hands a player back a
//! life the event says they had already spent, and a forced stage change moves the round for
//! everybody. A surface that can do that has to leave a record of who did what to whom, or the
//! only account of a contested call is somebody's memory.
//!
//! ── What is recorded ────────────────────────────────────────────────────────────────────────
//! Every attempt that reaches `TBD_AdminService`, including the ones that were **REFUSED**. A
//! security log that only lists successes cannot answer "did somebody try". Refusals are flagged
//! so the screen can ink them differently and so `TBD_Log.Warn` carries them at WARNING level.
//!
//! ── Where it lives ──────────────────────────────────────────────────────────────────────────
//! SERVER-side and static, so the chat commands (`TBD_AdminCommands`) and the admin screen
//! (`TBD_AdminScreen`, over RPC) share ONE trail rather than each keeping their own. The client
//! never writes to it — it only ever receives a copy inside the snapshot payload.
//!
//! Statics outlive a world inside one process (measured landmine in this codebase). That is
//! deliberate here: an admin who force-switches the mission and restarts the scenario is exactly
//! the event the next round's operator wants to still be able to read.
class TBD_AdminAuditEntry
{
	string m_sTime;   //!< server-local HH:MM:SS at the moment the action was attempted
	string m_sText;   //!< "Hicks(3) respawn Vasquez(7) -> DEPLOYED"
	bool m_bDenied;   //!< the attempt was refused (permission, or the operation said no)

	//------------------------------------------------------------------------------------------------
	void TBD_AdminAuditEntry(string time, string text, bool denied)
	{
		m_sTime = time;
		m_sText = text;
		m_bDenied = denied;
	}
}

//! @authority server — the trail is written on the server only; clients receive a copy.
class TBD_AdminAudit
{
	//! Log channel. A local constant rather than an edit to `TBD_Log`'s vocabulary — two slices
	//! adding a channel to that one enum block in the same wave is a merge conflict for no
	//! benefit, and `TBD_BriefingService.CH_BRIEFING` already set that precedent.
	static const string CH_ADMIN = "Admin";

	//! Bounded on purpose. A long event with a busy admin must not grow an unbounded array that is
	//! then serialised into a reliable-channel RPC every refresh.
	protected static const int MAX_ENTRIES = 60;

	//! How many entries the chat replay (`#tbd audit`) prints. Chat is not a log file.
	protected static const int CHAT_LINES = 12;

	protected static ref array<ref TBD_AdminAuditEntry> s_aEntries;

	//------------------------------------------------------------------------------------------------
	protected static void Ensure()
	{
		if (!s_aEntries)
			s_aEntries = {};
	}

	//------------------------------------------------------------------------------------------------
	//! Record one admin action attempt. `denied` covers both "not allowed" and "allowed but the
	//! operation refused"; either way the admin tried and the trail says so.
	//! @authority server
	static void Record(string text, bool denied)
	{
		Ensure();

		string time = Timestamp();
		s_aEntries.Insert(new TBD_AdminAuditEntry(time, text, denied));

		// Enfusion arrays remove BY INDEX (TBD_MOD_DESIGN.md §5). Index 0 is the oldest.
		while (s_aEntries.Count() > MAX_ENTRIES)
		{
			s_aEntries.Remove(0);
		}

		// The console is the durable copy — the in-memory ring dies with the process.
		if (denied)
			TBD_Log.Warn(CH_ADMIN, time + " " + text);
		else
			TBD_Log.Event(CH_ADMIN, time + " " + text);
	}

	//------------------------------------------------------------------------------------------------
	//! Console-only WARNING, no ring insert.
	//!
	//! For the REPEAT of a refusal that has already been recorded. The ring is bounded, so a client
	//! spamming refused requests would otherwise push every real action out of it — a log you can
	//! flush by attacking it is not an audit trail. The console copy is unbounded and greppable, so
	//! nothing is actually lost: every attempt still lands in `console.log`, only the first of each
	//! kind takes a slot on screen.
	//! @authority server
	static void Note(string text)
	{
		TBD_Log.Warn(CH_ADMIN, Timestamp() + " " + text);
	}

	//------------------------------------------------------------------------------------------------
	//! Oldest first. Never null.
	static array<ref TBD_AdminAuditEntry> GetEntries()
	{
		Ensure();
		return s_aEntries;
	}

	//------------------------------------------------------------------------------------------------
	static int GetCount()
	{
		Ensure();
		return s_aEntries.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! `#tbd audit` — the trail in chat, newest first, capped. The fallback surface for an event
	//! running before the menu preset is registered.
	static array<string> BuildReportLines()
	{
		Ensure();

		array<string> lines = new array<string>();

		if (s_aEntries.IsEmpty())
		{
			lines.Insert("TBD audit: no admin actions this session.");
			return lines;
		}

		lines.Insert(string.Format("TBD audit: %1 admin action(s) this session, newest first.", s_aEntries.Count()));

		int shown = 0;
		for (int i = s_aEntries.Count() - 1; i >= 0; i--)
		{
			if (shown >= CHAT_LINES)
			{
				lines.Insert(string.Format("… and %1 older (see the server console for the full trail).", i + 1));
				break;
			}

			TBD_AdminAuditEntry entry = s_aEntries[i];
			string mark = "  ";
			if (entry.m_bDenied)
				mark = "! ";

			lines.Insert(string.Format("%1%2  %3", mark, entry.m_sTime, entry.m_sText));
			shown++;
		}

		return lines;
	}

	//------------------------------------------------------------------------------------------------
	//! Server-local wall clock, `HH:MM:SS`. `System.GetHourMinuteSecond` is a native engine call and
	//! so resolves through no script index — probed against the compiler, which is the only oracle
	//! for a native symbol (SLICE_WORKFLOW.md §Sources).
	//!
	//! `int.ToString(2)` is the width argument; whether it pads with zeros or spaces is a
	//! presentation detail nothing here depends on.
	static string Timestamp()
	{
		int hour;
		int minute;
		int second;
		System.GetHourMinuteSecond(hour, minute, second);

		return string.Format("%1:%2:%3", hour.ToString(2), minute.ToString(2), second.ToString(2));
	}
}
