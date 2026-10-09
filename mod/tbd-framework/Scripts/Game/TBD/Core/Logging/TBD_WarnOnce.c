/**
 * @file TBD_WarnOnce.c
 * @brief Emits a WARNING log line at most once per channel and key.
 *
 * Role: de-duplicates recurring warnings so a menu or a per-tick path never spams the log.
 * Position: called by UI icon lookup, the kit preview dresser and the briefing service; writes
 * through `TBD_Log.Warn`.
 * State: the per-channel sets of keys already warned, for the life of the script VM.
 * Invariants: a key warns once per channel until that channel's set is cleared by its bound;
 * the line shape is exactly `[TBD][<channel>] <message>`.
 */

//! Once-per-key warning gate, partitioned by log channel.
class TBD_WarnOnce
{
	protected static ref map<string, ref set<string>> s_mWarnedByChannel; //!< channel -> keys already warned; null until first use

	//! Log `message` as a WARNING on `channel` the first time `key` is seen for that channel.
	//! @param channel the `TBD_Log` channel, which also scopes `key`
	//! @param key what makes two warnings the same one
	//! @param message the line text after the `[TBD][<channel>] ` prefix
	//! @param maxKeysPerChannel when at or above 0, the channel's set is cleared once it holds more
	//! than this many keys, so a key may warn again; -1 keeps every key forever
	//! @return true when the line was written, false when the key had already warned
	static bool Warn(string channel, string key, string message, int maxKeysPerChannel = -1)
	{
		if (!s_mWarnedByChannel)
			s_mWarnedByChannel = new map<string, ref set<string>>();

		set<string> warned = s_mWarnedByChannel.Get(channel);
		if (!warned)
		{
			warned = new set<string>();
			s_mWarnedByChannel.Set(channel, warned);
		}

		if (maxKeysPerChannel >= 0 && warned.Count() > maxKeysPerChannel)
			warned.Clear();

		if (warned.Contains(key))
			return false;

		warned.Insert(key);
		TBD_Log.Warn(channel, message);
		return true;
	}
}
