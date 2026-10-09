/**
 * @file TBD_TelemetryQueueStorage.c
 * @brief The telemetry queue on disk: one sealed file per entry and two alternating state slots.
 *
 * Role: writes, deletes and reloads the entry files and the state slots under
 * `$profile:TBD/Telemetry/`.  Position: called by `TBD_TelemetryQueue`; seals every file through
 * `TBD_TelemetrySealedFile`; file IO follows `TBD_MissionArtifactCache`.
 * State: none; the files are the state.  Invariants: an entry file is `entry_<id>.entry` holding a
 * header line (kind, source match id, route, enqueue time) and the body; an entry file that fails
 * its seal or does not parse is deleted on reload and counted, so the caller can add it to the
 * drop total; a state save writes the slot the previous save did not, so one intact slot always
 * survives a torn save; entries reload in id order.
 */

//! The header line of an entry file. Field names are its JSON keys.
class TBD_TelemetryEntryHeader
{
	string kind; //!< JSON `kind`: the `TBD_ETelemetryEntryKind` name
	string source_match_id; //!< JSON `source_match_id`
	string route; //!< JSON `route`: the path the body is posted to
	int enqueued_at; //!< JSON `enqueued_at`: Unix epoch seconds
}

//! Entry files and state slots of the durable telemetry queue.
//! @authority server
class TBD_TelemetryQueueStorage
{
	static const string DIRECTORY = "$profile:TBD/Telemetry"; //!< queue directory
	protected static const string ENTRY_PREFIX = "entry_"; //!< entry file name before the id
	protected static const string ENTRY_EXTENSION = ".entry"; //!< entry file extension
	protected static const string STATE_SLOT_A = "$profile:TBD/Telemetry/state_a.state"; //!< state slot of even serials
	protected static const string STATE_SLOT_B = "$profile:TBD/Telemetry/state_b.state"; //!< state slot of odd serials

	//! Persist `entry`.
	//! @param failure why it is not on disk; empty on success
	//! @return true when the sealed entry file is written
	static bool WriteEntry(notnull TBD_TelemetryQueueEntry entry, out string failure)
	{
		FileIO.MakeDirectory(DIRECTORY);

		string header = string.Format("{\"kind\":\"%1\",\"source_match_id\":\"%2\",\"route\":\"%3\",\"enqueued_at\":%4}",
			typename.EnumToString(TBD_ETelemetryEntryKind, entry.m_eKind), TBD_BackendText.JsonEscape(entry.m_sSourceMatchId),
			TBD_BackendText.JsonEscape(entry.m_sRoute), entry.m_iEnqueuedAt);

		return TBD_TelemetrySealedFile.Write(EntryPath(entry.m_iId), header + "\n" + entry.m_sBody, failure);
	}

	//! Remove the file of entry `id`.
	//! @return true when no file of the entry remains
	static bool DeleteEntry(int id)
	{
		string path = EntryPath(id);
		if (!FileIO.FileExists(path))
			return true;

		return FileIO.DeleteFile(path);
	}

	//! Reload every entry file, oldest id first. A file that fails its seal or does not parse is
	//! deleted, logged and counted in `invalidCount`.
	//! @param entries receives the entries
	//! @param invalidCount set to the number of files deleted as invalid
	static void LoadEntries(notnull array<ref TBD_TelemetryQueueEntry> entries, out int invalidCount)
	{
		invalidCount = 0;
		FileIO.MakeDirectory(DIRECTORY);

		array<string> found = {};
		FileIO.FindFiles(found.Insert, DIRECTORY + "/", ENTRY_EXTENSION);

		array<int> ids = {};
		foreach (string name : found)
		{
			int id = EntryIdOf(name);
			if (id > 0 && ids.Find(id) < 0)
				ids.Insert(id);
		}

		ids.Sort();
		foreach (int entryId : ids)
		{
			string failure;
			TBD_TelemetryQueueEntry entry = ReadEntry(entryId, failure);
			if (entry)
			{
				entries.Insert(entry);
				continue;
			}

			invalidCount++;
			DeleteEntry(entryId);
			TBD_Log.Error(TBD_TelemetryQueue.CH_TELEMETRY, string.Format("entry %1 dropped on reload: %2", entryId, failure));
		}
	}

	//! Save `state` into the slot its serial selects, after advancing the serial.
	//! @param failure why it is not saved; empty on success
	//! @return true when the slot is sealed
	static bool SaveState(notnull TBD_TelemetryQueueState state, out string failure)
	{
		FileIO.MakeDirectory(DIRECTORY);
		state.serial++;
		string path = STATE_SLOT_A;
		if (state.serial % 2 == 1)
			path = STATE_SLOT_B;

		return TBD_TelemetrySealedFile.Write(path, state.ToJson(), failure);
	}

	//! The latest intact state: of the two slots, the valid one with the higher serial.
	//! @param slotsFound set to the number of slot files present, intact or not
	//! @return the state, or null when neither slot is intact
	static TBD_TelemetryQueueState LoadState(out int slotsFound)
	{
		slotsFound = 0;
		if (FileIO.FileExists(STATE_SLOT_A))
			slotsFound++;
		if (FileIO.FileExists(STATE_SLOT_B))
			slotsFound++;

		TBD_TelemetryQueueState first = ReadStateSlot(STATE_SLOT_A);
		TBD_TelemetryQueueState second = ReadStateSlot(STATE_SLOT_B);
		if (!first)
			return second;

		if (!second)
			return first;

		if (second.serial > first.serial)
			return second;

		return first;
	}

	//! The state in the slot at `path`.
	//! @return the state, or null when the slot is absent, torn or unparsable
	protected static TBD_TelemetryQueueState ReadStateSlot(string path)
	{
		if (!FileIO.FileExists(path))
			return null;

		string record;
		string failure;
		if (!TBD_TelemetrySealedFile.Read(path, record, failure))
		{
			TBD_Log.Warn(TBD_TelemetryQueue.CH_TELEMETRY, "state slot ignored: " + failure);
			return null;
		}

		return TBD_TelemetryQueueState.FromJson(record);
	}

	//! The entry in the file of `id`.
	//! @param failure why it cannot be used; empty on success
	//! @return the entry, or null
	protected static TBD_TelemetryQueueEntry ReadEntry(int id, out string failure)
	{
		string record;
		if (!TBD_TelemetrySealedFile.Read(EntryPath(id), record, failure))
			return null;

		int newline = record.IndexOf("\n");
		if (newline <= 0 || newline >= record.Length() - 1)
		{
			failure = "no header line and body";
			return null;
		}

		TBD_TelemetryEntryHeader header = new TBD_TelemetryEntryHeader();
		JsonLoadContext context = new JsonLoadContext();
		if (!context.LoadFromString(record.Substring(0, newline)) || !context.ReadValue("", header))
		{
			failure = "header does not parse";
			return null;
		}

		TBD_ETelemetryEntryKind kind;
		if (!KindOf(header.kind, kind) || header.source_match_id.IsEmpty() || header.route.IsEmpty())
		{
			failure = "header names no known kind, source match or route";
			return null;
		}

		TBD_TelemetryQueueEntry entry = new TBD_TelemetryQueueEntry();
		entry.m_iId = id;
		entry.m_eKind = kind;
		entry.m_sSourceMatchId = header.source_match_id;
		entry.m_sRoute = header.route;
		entry.m_iEnqueuedAt = header.enqueued_at;
		entry.m_sBody = record.Substring(newline + 1, record.Length() - newline - 1);
		return entry;
	}

	//! The kind named `name`.
	//! @param kind set to the kind on success
	//! @return false for a name that is no kind
	protected static bool KindOf(string name, out TBD_ETelemetryEntryKind kind)
	{
		kind = TBD_ETelemetryEntryKind.EVENT_BATCH;
		if (name == "REGISTRATION")
			kind = TBD_ETelemetryEntryKind.REGISTRATION;
		else if (name == "RESULTS")
			kind = TBD_ETelemetryEntryKind.RESULTS;
		else if (name != "EVENT_BATCH")
			return false;

		return true;
	}

	//! The file path of entry `id`.
	//! @return `$profile:TBD/Telemetry/entry_<id>.entry`
	static string EntryPath(int id)
	{
		return string.Format("%1/%2%3%4", DIRECTORY, ENTRY_PREFIX, id, ENTRY_EXTENSION);
	}

	//! The entry id in a found file name, whatever directory prefix the engine reports it with.
	//! @return the id, or 0 when the name is not an entry file
	protected static int EntryIdOf(string name)
	{
		int slash = name.LastIndexOf("/");
		string fileName = name;
		if (slash >= 0)
			fileName = name.Substring(slash + 1, name.Length() - slash - 1);

		if (!fileName.StartsWith(ENTRY_PREFIX) || !fileName.EndsWith(ENTRY_EXTENSION))
			return 0;

		int digits = fileName.Length() - ENTRY_PREFIX.Length() - ENTRY_EXTENSION.Length();
		if (digits <= 0)
			return 0;

		return fileName.Substring(ENTRY_PREFIX.Length(), digits).ToInt();
	}
}
