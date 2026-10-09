/**
 * @file TBD_TelemetryQueue.c
 * @brief The durable, bounded queue of match registrations, results revisions and event batches.
 *
 * Role: keeps every telemetry report until the API acknowledges it, enforces the capacity and its
 * reserve, replaces an unsent results revision by a newer one, and issues the per-match results
 * revisions and event sequences.  Position: fed by `TBD_MatchRegistration`,
 * `TBD_MatchResultsRevision` and the detailed-event capture through `EnqueueEventBatch`; drained by
 * `TBD_TelemetryDelivery`; read by `TBD_RuntimeStatusReadings` through `Stats`; persisted by
 * `TBD_TelemetryQueueStorage` under `$profile:TBD/Telemetry/`.
 * State: the entries in send order, the queue state and the id of the entry in flight; server
 * statics loaded from disk once per process, at the first game start, and written through on
 * every change.  Invariants: at most `CAPACITY` entries, of which event batches hold at most
 * `CAPACITY - RESERVED_FOR_REPORTS`; a full queue drops its oldest event batch, or its oldest entry
 * when only registrations and results remain, never the entry in flight; every drop is counted in
 * `dropped_total` and logged at ERROR; the state is saved before an entry file is written, so an
 * entry id is never issued twice; results revisions and event sequences only grow.
 */

//! Durable telemetry queue: capacity, reserve, drops, per-match counters, reload.
//! @authority server
class TBD_TelemetryQueue
{
	static const string CH_TELEMETRY = "Telemetry"; //!< log channel: `grep '\[TBD\]\[Telemetry\]' console.log`
	static const int CAPACITY = 512; //!< entries the queue holds at most
	static const int RESERVED_FOR_REPORTS = 32; //!< capacity event batches may never take: kept for registrations and results

	protected static bool s_bLoaded; //!< the queue has been read from disk in this process
	protected static ref array<ref TBD_TelemetryQueueEntry> s_aEntries; //!< waiting entries in send order
	protected static ref TBD_TelemetryQueueState s_State; //!< next entry id, drop total and per-match counters
	protected static int s_iInFlightId; //!< id of the entry being sent; 0 when none

	//! Read the queue from disk once per process: the latest intact state slot and every intact
	//! entry file. Files that fail their seal are deleted and counted as dropped. Called at game
	//! start and by every entry point, so the queue is loaded before any use.
	static void EnsureLoaded()
	{
		if (s_bLoaded)
			return;

		s_bLoaded = true;
		s_aEntries = new array<ref TBD_TelemetryQueueEntry>();

		int slotsFound;
		s_State = TBD_TelemetryQueueStorage.LoadState(slotsFound);
		if (!s_State)
		{
			s_State = new TBD_TelemetryQueueState();
			if (slotsFound > 0)
				TBD_Log.Error(CH_TELEMETRY, "no intact state slot: entry ids continue after the entries on disk, per-match counters restart");
		}

		int invalid;
		TBD_TelemetryQueueStorage.LoadEntries(s_aEntries, invalid);
		foreach (TBD_TelemetryQueueEntry entry : s_aEntries)
		{
			if (entry.m_iId >= s_State.next_entry_id)
				s_State.next_entry_id = entry.m_iId + 1;
		}

		s_State.dropped_total += invalid;
		SaveState();

		TBD_Log.Kv(CH_TELEMETRY, "loaded", string.Format("directory=%1 backlog=%2 capacity=%3 dropped_total=%4 torn=%5",
			TBD_TelemetryQueueStorage.DIRECTORY, s_aEntries.Count(), CAPACITY, s_State.dropped_total, invalid));
	}

	// ENQUEUE

	//! Queue the registration of `sourceMatchId` and remember its body, which is sent again ahead of
	//! any report the API answers `MATCH_NOT_REGISTERED`.
	//! @param body a `MatchRegistration` body
	//! @return true when the entry is on disk
	static bool EnqueueRegistration(string sourceMatchId, string body)
	{
		EnsureLoaded();
		TBD_TelemetryMatchCounters counters = s_State.EnsureMatch(sourceMatchId, QueuedMatchIds());
		counters.registration_body = body;
		SaveState();
		return Add(TBD_ETelemetryEntryKind.REGISTRATION, sourceMatchId, body, false);
	}

	//! Queue a results revision of `sourceMatchId`. An unsent results entry of the same match, other
	//! than the one in flight, is replaced in place: it keeps its position and takes the new body.
	//! @param body a `MatchResultsRevision` body carrying a revision from `NextResultsRevision`
	//! @return true when the entry is on disk
	static bool EnqueueResults(string sourceMatchId, string body)
	{
		EnsureLoaded();
		int index = FindIndex(TBD_ETelemetryEntryKind.RESULTS, sourceMatchId, true);
		if (index < 0)
			return Add(TBD_ETelemetryEntryKind.RESULTS, sourceMatchId, body, false);

		TBD_TelemetryQueueEntry replaced = s_aEntries[index];
		TBD_TelemetryQueueEntry entry = Persist(TBD_ETelemetryEntryKind.RESULTS, sourceMatchId, body);
		if (!entry)
			return false;

		TBD_TelemetryQueueStorage.DeleteEntry(replaced.m_iId);
		s_aEntries[index] = entry;
		TBD_Log.Kv(CH_TELEMETRY, "results-replaced", string.Format("old=%1 new=%2 source='%3'", replaced.m_iId, entry.m_iId, sourceMatchId));
		return true;
	}

	//! Queue one detailed event batch of `sourceMatchId`, built by `TBD_MatchEventBatch.Enqueue`. The
	//! events carry sequences from `ReserveEventSequences`; batches are sent in the order they are
	//! queued. When the event batches already fill their share of the capacity, the oldest one is
	//! dropped first.
	//! @param body a `MatchEventBatch` body of 1 to 500 events
	//! @return true when the entry is on disk
	static bool EnqueueEventBatch(string sourceMatchId, string body)
	{
		EnsureLoaded();
		return Add(TBD_ETelemetryEntryKind.EVENT_BATCH, sourceMatchId, body, false);
	}

	// PER-MATCH COUNTERS

	//! Issue the next results revision of `sourceMatchId` (1 for the first) and persist it.
	//! @return the revision
	static int NextResultsRevision(string sourceMatchId)
	{
		EnsureLoaded();
		TBD_TelemetryMatchCounters counters = s_State.EnsureMatch(sourceMatchId, QueuedMatchIds());
		counters.results_revision++;
		SaveState();
		return counters.results_revision;
	}

	//! Reserve `count` consecutive event sequences of `sourceMatchId` and persist the reservation.
	//! @param count sequences wanted, at least 1
	//! @return the first reserved sequence (1 for the first event of a match), or 0 when `count` < 1
	static int ReserveEventSequences(string sourceMatchId, int count)
	{
		if (count < 1)
			return 0;

		EnsureLoaded();
		TBD_TelemetryMatchCounters counters = s_State.EnsureMatch(sourceMatchId, QueuedMatchIds());
		int first = counters.event_sequence + 1;
		counters.event_sequence += count;
		SaveState();
		return first;
	}

	//! The last event sequence issued for `sourceMatchId`.
	//! @return the sequence, or 0 when none was issued or the match is not kept
	static int LastEventSequence(string sourceMatchId)
	{
		EnsureLoaded();
		TBD_TelemetryMatchCounters counters = s_State.FindMatch(sourceMatchId);
		if (!counters)
			return 0;

		return counters.event_sequence;
	}

	// DELIVERY

	//! The entry to send next.
	//! @return the first entry in send order, or null when the queue is empty
	static TBD_TelemetryQueueEntry Head()
	{
		EnsureLoaded();
		if (s_aEntries.IsEmpty())
			return null;

		return s_aEntries[0];
	}

	//! The queued entry `id`.
	//! @return the entry, or null when it is not queued
	static TBD_TelemetryQueueEntry Find(int id)
	{
		int index = IndexOfId(id);
		if (index < 0)
			return null;

		return s_aEntries[index];
	}

	//! Mark entry `id` as the one in flight (0 for none): neither replaced nor dropped for room.
	static void SetInFlight(int id)
	{
		s_iInFlightId = id;
	}

	//! Remove entry `id`, acknowledged by the API.
	static void Acknowledge(int id)
	{
		int index = IndexOfId(id);
		if (index < 0)
			return;

		s_aEntries.RemoveOrdered(index);
		TBD_TelemetryQueueStorage.DeleteEntry(id);
	}

	//! Remove entry `id` without delivering it: counted in `dropped_total` and logged at ERROR.
	//! @param reason why, for the log line
	static void Drop(int id, string reason)
	{
		int index = IndexOfId(id);
		if (index < 0)
			return;

		TBD_TelemetryQueueEntry entry = s_aEntries[index];
		s_aEntries.RemoveOrdered(index);
		TBD_TelemetryQueueStorage.DeleteEntry(id);
		CountDrop(string.Format("dropped %1 route=%2 -- %3", entry.Describe(), entry.m_sRoute, reason));
	}

	//! Put the registration of `sourceMatchId` at the front: a queued one moves there, else the
	//! remembered registration body is queued again at the front.
	//! @return false when no registration of the match is queued or remembered
	static bool PutRegistrationFirst(string sourceMatchId)
	{
		EnsureLoaded();
		int index = FindIndex(TBD_ETelemetryEntryKind.REGISTRATION, sourceMatchId, false);
		if (index >= 0)
		{
			TBD_TelemetryQueueEntry queued = s_aEntries[index];
			s_aEntries.RemoveOrdered(index);
			s_aEntries.InsertAt(queued, 0);
			return true;
		}

		TBD_TelemetryMatchCounters counters = s_State.FindMatch(sourceMatchId);
		if (!counters || counters.registration_body.IsEmpty())
			return false;

		return Add(TBD_ETelemetryEntryKind.REGISTRATION, sourceMatchId, counters.registration_body, true);
	}

	//! The queue reading a heartbeat carries.
	//! @return backlog, capacity, drop total and the age of the oldest entry
	static TBD_TelemetryQueueStatsStruct Stats()
	{
		EnsureLoaded();
		TBD_TelemetryQueueStatsStruct stats = new TBD_TelemetryQueueStatsStruct();
		stats.backlog = s_aEntries.Count();
		stats.capacity = CAPACITY;
		stats.dropped_total = s_State.dropped_total;

		if (!s_aEntries.IsEmpty())
		{
			int oldest = s_aEntries[0].m_iEnqueuedAt;
			foreach (TBD_TelemetryQueueEntry entry : s_aEntries)
			{
				if (entry.m_iEnqueuedAt < oldest)
					oldest = entry.m_iEnqueuedAt;
			}

			stats.oldest_age_seconds = Math.Max(0, System.GetUnixTime() - oldest);
		}

		return stats;
	}

	// INTERNALS

	//! Make room for an entry of `kind`, persist it and insert it at the back, or at the front.
	//! @return true when the entry is queued
	protected static bool Add(TBD_ETelemetryEntryKind kind, string sourceMatchId, string body, bool atFront)
	{
		MakeRoom(kind);
		TBD_TelemetryQueueEntry entry = Persist(kind, sourceMatchId, body);
		if (!entry)
			return false;

		if (atFront)
			s_aEntries.InsertAt(entry, 0);
		else
			s_aEntries.Insert(entry);

		return true;
	}

	//! A new entry with a fresh id, written to disk. The state is saved first, so the id is never
	//! issued again; a write that fails is counted as a drop.
	//! @return the entry, or null when its file could not be written
	protected static TBD_TelemetryQueueEntry Persist(TBD_ETelemetryEntryKind kind, string sourceMatchId, string body)
	{
		TBD_TelemetryQueueEntry entry = new TBD_TelemetryQueueEntry();
		entry.m_iId = s_State.next_entry_id;
		entry.m_eKind = kind;
		entry.m_sSourceMatchId = sourceMatchId;
		entry.m_sRoute = TBD_TelemetryQueueEntry.RouteOf(kind);
		entry.m_sBody = body;
		entry.m_iEnqueuedAt = System.GetUnixTime();

		s_State.next_entry_id++;
		SaveState();

		string failure;
		if (TBD_TelemetryQueueStorage.WriteEntry(entry, failure))
			return entry;

		TBD_TelemetryQueueStorage.DeleteEntry(entry.m_iId);
		CountDrop(string.Format("not queued %1 -- %2", entry.Describe(), failure));
		return null;
	}

	//! Drop entries until one of `kind` fits: event batches may fill only the unreserved capacity,
	//! and the whole queue holds `CAPACITY`. The victim is the oldest event batch, or the oldest
	//! entry when only registrations and results remain; the entry in flight is never the victim.
	protected static void MakeRoom(TBD_ETelemetryEntryKind kind)
	{
		while (NeedsRoom(kind))
		{
			TBD_TelemetryQueueEntry victim = OldestEvictable();
			if (!victim)
				return;

			Drop(victim.m_iId, string.Format("queue full (capacity %1, %2 reserved for registrations and results)",
				CAPACITY, RESERVED_FOR_REPORTS));
		}
	}

	//! Whether an entry of `kind` needs a drop before it fits.
	//! @return true when the queue or the event batches' share is full
	protected static bool NeedsRoom(TBD_ETelemetryEntryKind kind)
	{
		if (s_aEntries.Count() >= CAPACITY)
			return true;

		if (kind != TBD_ETelemetryEntryKind.EVENT_BATCH)
			return false;

		int batches = 0;
		foreach (TBD_TelemetryQueueEntry entry : s_aEntries)
		{
			if (!entry.IsReserved())
				batches++;
		}

		return batches >= CAPACITY - RESERVED_FOR_REPORTS;
	}

	//! The oldest event batch not in flight, else the oldest entry not in flight.
	//! @return the entry, or null when only the entry in flight remains
	protected static TBD_TelemetryQueueEntry OldestEvictable()
	{
		TBD_TelemetryQueueEntry oldestAny;
		foreach (TBD_TelemetryQueueEntry entry : s_aEntries)
		{
			if (entry.m_iId == s_iInFlightId)
				continue;

			if (!entry.IsReserved())
				return entry;

			if (!oldestAny)
				oldestAny = entry;
		}

		return oldestAny;
	}

	//! The index of the first entry of `kind` for `sourceMatchId`.
	//! @param skipInFlight pass over the entry in flight
	//! @return the index, or -1
	protected static int FindIndex(TBD_ETelemetryEntryKind kind, string sourceMatchId, bool skipInFlight)
	{
		for (int i = 0; i < s_aEntries.Count(); i++)
		{
			TBD_TelemetryQueueEntry entry = s_aEntries[i];
			if (skipInFlight && entry.m_iId == s_iInFlightId)
				continue;

			if (entry.m_eKind == kind && entry.m_sSourceMatchId == sourceMatchId)
				return i;
		}

		return -1;
	}

	//! The index of entry `id`.
	//! @return the index, or -1
	protected static int IndexOfId(int id)
	{
		if (!s_aEntries)
			return -1;

		for (int i = 0; i < s_aEntries.Count(); i++)
		{
			if (s_aEntries[i].m_iId == id)
				return i;
		}

		return -1;
	}

	//! The source match ids the queued entries name.
	//! @return the ids, without duplicates
	protected static array<string> QueuedMatchIds()
	{
		array<string> ids = {};
		foreach (TBD_TelemetryQueueEntry entry : s_aEntries)
		{
			if (ids.Find(entry.m_sSourceMatchId) < 0)
				ids.Insert(entry.m_sSourceMatchId);
		}

		return ids;
	}

	//! Count one drop, persist the total and log `line` at ERROR.
	protected static void CountDrop(string line)
	{
		s_State.dropped_total++;
		SaveState();
		TBD_Log.Error(CH_TELEMETRY, string.Format("%1 (dropped_total=%2)", line, s_State.dropped_total));
	}

	//! Save the state into its next slot; a failure is logged at ERROR and the queue carries on in
	//! memory.
	protected static void SaveState()
	{
		string failure;
		if (!TBD_TelemetryQueueStorage.SaveState(s_State, failure))
			TBD_Log.Error(CH_TELEMETRY, "state not saved: " + failure);
	}
}
