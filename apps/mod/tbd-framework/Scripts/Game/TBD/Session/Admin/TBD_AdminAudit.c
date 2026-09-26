/**
 * @file TBD_AdminAudit.c
 * @brief The admin audit trail: a bounded ring of every admin attempt, refusals included.
 *
 * Role: records who did what to whom through an admin power, and replays it.  Position: TBD_AdminService, TBD_AdminSubcommands, TBD_AdminCommands, TBD_MissionDeploymentRelay and the
 * spawning and fleet scripts write it; TBD_AdminSnapshotService ships its newest entries to the
 * admin screen and `#tbd audit` prints it in chat.
 * State: the static entry ring on the server; it outlives a world inside one process, so a
 * restarted round can still read what the last one did.  Invariants: the ring holds at most
 * MAX_ENTRIES entries, of which unauthorised-access refusals hold at most
 * MAX_UNAUTHORISED_ENTRIES, so an attacker cannot flush real actions out; every entry is also
 * logged on the `Admin` channel, refusals at WARNING.
 */

//! One audit entry.
class TBD_AdminAuditEntry
{
	string m_sTime;   //!< server-local HH:MM:SS at the moment the action was attempted
	string m_sText;   //!< "Hicks(3) respawn Vasquez(7) -> DEPLOYED"
	bool m_bDenied;   //!< the attempt was refused (permission, or the operation said no)

	bool m_bUnauthorised; //!< a non-admin touched an admin surface; a subset of m_bDenied that has its own ring budget; not sent to clients

	//! Build an entry; m_bUnauthorised starts false.
	void TBD_AdminAuditEntry(string time, string text, bool denied)
	{
		m_sTime = time;
		m_sText = text;
		m_bDenied = denied;
	}
}

//! The admin audit trail. Written on the server only; clients receive a copy in the snapshot.
//! @authority server
class TBD_AdminAudit
{
	static const string CH_ADMIN = "Admin"; //!< TBD_Log channel of every audit line
	protected static const int MAX_ENTRIES = 60; //!< ring size; the oldest entry is dropped past it
	protected static const int CHAT_LINES = 12; //!< entries `#tbd audit` prints
	protected static const int MAX_UNAUTHORISED_ENTRIES = 12; //!< ring slots unauthorised refusals may hold; equal to CHAT_LINES so a full `#tbd audit` page can be all real actions
	protected static ref array<ref TBD_AdminAuditEntry> s_aEntries; //!< the ring, oldest first; created on first use

	//! Create the ring on first use.
	protected static void Ensure()
	{
		if (!s_aEntries)
			s_aEntries = {};
	}

	//! Record one admin action attempt.
	//! @param text the audit line
	//! @param denied true for "not allowed" and for "allowed but the operation refused"
	//! @authority server
	static void Record(string text, bool denied)
	{
		RecordEntry(text, denied, false);
	}

	//! Record an unauthorised-access refusal against its own budget: the oldest such entry is
	//! evicted first, so these roll among themselves and never displace other entries.
	//! @param text the audit line
	//! @authority server
	static void RecordUnauthorised(string text)
	{
		RecordEntry(text, true, true);
	}

	//! Append an entry, trim the ring, and log it (WARNING when denied).
	//! @param text the audit line
	//! @param denied the attempt was refused
	//! @param unauthorised the refusal was for a non-admin; evicts the oldest such entry first
	//! @authority server
	protected static void RecordEntry(string text, bool denied, bool unauthorised)
	{
		Ensure();

		if (unauthorised)
			EvictOldestUnauthorised();

		string time = Timestamp();
		TBD_AdminAuditEntry entry = new TBD_AdminAuditEntry(time, text, denied);
		entry.m_bUnauthorised = unauthorised;
		s_aEntries.Insert(entry);

		// Enfusion arrays remove by index; index 0 is the oldest.
		while (s_aEntries.Count() > MAX_ENTRIES)
		{
			s_aEntries.Remove(0);
		}

		// The console is the durable copy -- the in-memory ring dies with the process.
		if (denied)
			TBD_Log.Warn(CH_ADMIN, time + " " + text);
		else
			TBD_Log.Event(CH_ADMIN, time + " " + text);
	}

	//! Drop the oldest unauthorised entries until inserting one more stays within budget. A loop, so
	//! the bound holds whatever shape the ring is in.
	//! @authority server
	protected static void EvictOldestUnauthorised()
	{
		while (CountUnauthorised() >= MAX_UNAUTHORISED_ENTRIES)
		{
			int oldest = -1;
			for (int i = 0; i < s_aEntries.Count(); i++)
			{
				if (s_aEntries[i].m_bUnauthorised)
				{
					oldest = i;
					break;
				}
			}

			// Nothing to evict. Cannot happen while the count is at budget, but a loop that can
			// only exit by finding something is a hang, so it exits here instead.
			if (oldest == -1)
				return;

			s_aEntries.Remove(oldest);
		}
	}

	//! @return how many entries in the ring are unauthorised refusals
	//! @authority server
	protected static int CountUnauthorised()
	{
		int count = 0;
		for (int i = 0; i < s_aEntries.Count(); i++)
		{
			if (s_aEntries[i].m_bUnauthorised)
				count++;
		}

		return count;
	}

	//! Log a WARNING on the `Admin` channel without a ring entry, for the repeat of a refusal whose
	//! first occurrence already took a slot.
	//! @param text the line to log
	//! @authority server
	static void Note(string text)
	{
		TBD_Log.Warn(CH_ADMIN, Timestamp() + " " + text);
	}

	//! @return the ring, oldest first; never null
	static array<ref TBD_AdminAuditEntry> GetEntries()
	{
		Ensure();
		return s_aEntries;
	}

	//! @return how many entries the ring holds
	static int GetCount()
	{
		Ensure();
		return s_aEntries.Count();
	}

	//! The `#tbd audit` reply: a header and the newest CHAT_LINES entries, newest first, refusals
	//! marked `!`.
	//! @return the chat lines; one line when the trail is empty
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
				lines.Insert(string.Format("... and %1 older (see the server console for the full trail).", i + 1));
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

	//! Server-local wall clock from System.GetHourMinuteSecond, each part written with width 2.
	//! @return `HH:MM:SS`
	static string Timestamp()
	{
		int hour;
		int minute;
		int second;
		System.GetHourMinuteSecond(hour, minute, second);

		return string.Format("%1:%2:%3", hour.ToString(2), minute.ToString(2), second.ToString(2));
	}
}
