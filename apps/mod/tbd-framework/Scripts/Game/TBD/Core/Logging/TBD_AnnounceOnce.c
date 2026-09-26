/**
 * @file TBD_AnnounceOnce.c
 * @brief Emits an informational log line at most once per key until the key is rearmed.
 *
 * Role: the "say it once per world" gate behind the armed and idle announcements of the mission
 * runtimes.  Position: called by the trigger, task, weather, audio, spawner, waypoint and
 * objective runtimes; writes through `TBD_Log.Event` and `TBD_Log.Kv`.
 * State: the set of keys already announced, for the life of the script VM; cleared per key by
 * `Rearm`.  Invariants: a claimed key stays silent until `Rearm(key)`; the gate never logs by
 * itself when only `Claim` is used.
 */

//! Once-per-key announcement gate.
class TBD_AnnounceOnce
{
	protected static ref set<string> s_Announced; //!< keys already announced; null until first use

	//! Claim `key` for one announcement.
	//! @param key the announcement identity, scoped by the caller (for example `Triggers.armed`)
	//! @return true the first time since the last `Rearm(key)`, false afterwards
	static bool Claim(string key)
	{
		if (!s_Announced)
			s_Announced = new set<string>();

		if (s_Announced.Contains(key))
			return false;

		s_Announced.Insert(key);
		return true;
	}

	//! Let `key` announce again, for example when a new world or mission starts. A key that was
	//! never claimed is ignored.
	static void Rearm(string key)
	{
		if (!s_Announced)
			return;

		s_Announced.RemoveItem(key);
	}

	//! Write `[TBD][<channel>] <message>` at NORMAL level once per `key`.
	//! @return true when the line was written
	static bool Event(string channel, string key, string message)
	{
		if (!Claim(key))
			return false;

		TBD_Log.Event(channel, message);
		return true;
	}

	//! Write the structured `[TBD][<channel>] <eventName> <keyValues>` line once per `key`.
	//! @return true when the line was written
	static bool Kv(string channel, string key, string eventName, string keyValues)
	{
		if (!Claim(key))
			return false;

		TBD_Log.Kv(channel, eventName, keyValues);
		return true;
	}
}
