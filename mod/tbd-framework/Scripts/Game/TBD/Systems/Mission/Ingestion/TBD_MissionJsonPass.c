/**
 * @file TBD_MissionJsonPass.c
 * @brief Opens the loaded mission JSON for a second typed read by a runtime system.
 *
 * Role: the shared first half of every second-pass read: fetch the held mission text, optionally
 * rename one JSON key, and parse it into a `JsonLoadContext` whose root the caller reads into its
 * own struct.  Position: called by the trigger, task, weather, audio, spawner, waypoint, group,
 * scatter, objective and state readers; reads `TBD_MissionLoader.GetRawJson`.
 * State: none.  Invariants: never mutates the loader's cached text; a null context always comes
 * with an outcome other than `LOADED`, and a non-null one always with `LOADED`.
 */

//! Why `TBD_MissionJsonPass.LoadRoot` did or did not return a context.
enum TBD_EMissionJsonPassOutcome
{
	LOADED, //!< the context holds the parsed document; read its root with `ReadValue("", doc)`
	NO_DOCUMENT, //!< no mission text is held: a client, or no mission loaded
	NOT_JSON //!< the held text did not parse as JSON
}

//! Second-pass reader entry point.
class TBD_MissionJsonPass
{
	//! Parse the held mission JSON for a typed second read.
	//! @param outcome set to why the call did or did not return a context
	//! @param renameKeyFrom a JSON key to rename before parsing, for a key that is an Enforce
	//! keyword (for example `event`); empty renames nothing
	//! @param renameKeyTo the key name `renameKeyFrom` becomes (for example `cueEvent`)
	//! @return the loaded context, or null with `outcome` set to `NO_DOCUMENT` or `NOT_JSON`
	static JsonLoadContext LoadRoot(out TBD_EMissionJsonPassOutcome outcome, string renameKeyFrom = "", string renameKeyTo = "")
	{
		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
		{
			outcome = TBD_EMissionJsonPassOutcome.NO_DOCUMENT;
			return null;
		}

		if (!renameKeyFrom.IsEmpty())
		{
			string rewritten = string.Format("%1", raw);
			rewritten.Replace("\"" + renameKeyFrom + "\":", "\"" + renameKeyTo + "\":");
			raw = rewritten;
		}

		JsonLoadContext context = new JsonLoadContext();
		if (!context.LoadFromString(raw))
		{
			outcome = TBD_EMissionJsonPassOutcome.NOT_JSON;
			return null;
		}

		outcome = TBD_EMissionJsonPassOutcome.LOADED;
		return context;
	}
}
