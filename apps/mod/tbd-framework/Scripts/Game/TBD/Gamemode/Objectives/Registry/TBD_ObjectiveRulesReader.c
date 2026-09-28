/**
 * @file TBD_ObjectiveRulesReader.c
 * @brief Reads the objective keys of `zones[].rules` in a second typed pass over the mission JSON.
 *
 * Role: binds the objective half of the closed `zoneRules` vocabulary (capture, hold, destroy,
 * points and announcement keys) that the loader's `TBD_MissionZoneRulesStruct` does not declare,
 * keeping that vocabulary beside the code that interprets it.  Position: called by
 * `TBD_ObjectiveRegistry.Build`; reads the held mission JSON through `TBD_MissionJsonPass`;
 * consumed by `TBD_ObjectiveRuleResolver`.
 * State: the parsed `zones[]` of the current world, cleared by `TBD_ObjectiveRegistry.Clear`
 * because statics outlive a world.  Invariants: a client holds no mission text and reads nothing;
 * numeric and string keys carry an ABSENT sentinel because nested refs are always allocated;
 * every bool initialiser equals the value an author gets by writing nothing, so absent and
 * authored-false agree.
 */

//! The objective keys of `#/$defs/zoneRules`. Adding a rule is a member here plus the matching
//! schema property; an undeclared key fails schema validation.
//! @contract mission.schema.json#/$defs/zoneRules
class TBD_ObjectiveRulesStruct
{
	static const float ABSENT = -1000000; //!< "key absent" sentinel for the float keys; no author types it and JSON has no NaN
	static const int ABSENT_INT = -1; //!< "key absent" sentinel for `targetCount`, whose 0 means "all of them"

	float captureSeconds = ABSENT; //!< `captureSeconds` > 0: uninterrupted presence to take a neutral objective
	float neutralizeSeconds = ABSENT; //!< `neutralizeSeconds` >= 0: presence to tear a held objective to neutral; 0 = instant
	float holdSeconds = ABSENT; //!< `holdSeconds` > 0: how long the zone's faction must hold; required on hold zones
	float decayRate = ABSENT; //!< `decayRate` > 0: progress-seconds lost per second while decaying
	float announceEverySeconds = ABSENT; //!< `announceEverySeconds` > 0: progress log cadence
	float points = ABSENT; //!< `points` >= 0: carried and reported only

	int targetCount = ABSENT_INT; //!< `targetCount` >= 0: targets to destroy; 0 or absent = all

	string targetAlias; //!< `targetAlias`: registry alias of the thing to destroy; empty = absent
	string onEmpty; //!< `onEmpty`: "hold" or "decay"; empty = absent

	bool contestable = true; //!< `contestable`: an enemy inside stops a capture; default true
	bool pauseOnEnemy = true; //!< `pauseOnEnemy`: an enemy inside pauses the hold clock; default true
	bool resetOnEnemy = false; //!< `resetOnEnemy`: an enemy inside resets the hold clock; default false
	bool requireHolderPresent = false; //!< `requireHolderPresent`: the hold clock runs only with a holder inside; default false
}

//! Just enough of a zone to join this pass onto the loader's; `shape` is not declared, because
//! geometry belongs to `TBD_Zone`.
//! @contract mission.schema.json#/$defs/zone partial
class TBD_ObjectiveZoneStruct
{
	string id; //!< `id`
	string type; //!< `type`
	ref TBD_ObjectiveRulesStruct rules; //!< `rules`, always allocated
}

//! The document root of the rules pass; declares `zones` only. The primary loader's
//! `TBD_MissionDocumentStruct` models `entities[]` and the other keys; this root re-reads zone
//! `rules.*` alone.
//! @contract mission.schema.json#/ partial
class TBD_ObjectiveDocStruct
{
	ref array<ref TBD_ObjectiveZoneStruct> zones; //!< `zones[]`
}

//! Reads the objective rules once per world and hands them out by zone.
class TBD_ObjectiveRulesReader
{
	protected static ref array<ref TBD_ObjectiveZoneStruct> s_aZones; //!< the parsed `zones[]`; null until a successful `Read`
	protected static bool s_bRead; //!< `Read` ran since the last `Clear`
	protected static bool s_bOk; //!< the pass produced a `zones[]` array

	//! Whether `Read` has run since the last `Clear`.
	static bool IsRead()
	{
		return s_bRead;
	}

	//! Whether the pass produced a `zones[]` array. False means every objective runs on
	//! defaults, which the registry reports once.
	static bool IsOk()
	{
		return s_bOk;
	}

	//! Drop the parsed zones; the next `Read` parses again.
	static void Clear()
	{
		s_aZones = null;
		s_bRead = false;
		s_bOk = false;
	}

	//! How many zones this pass saw, compared by the registry with the loader's count.
	static int Count()
	{
		if (!s_aZones)
			return 0;

		return s_aZones.Count();
	}

	//! Parse `zones[]`. Idempotent: only the first call after a `Clear` does work.
	//! @return true when the document carries a `zones[]` array
	//! @authority server
	static bool Read()
	{
		if (s_bRead)
			return s_bOk;

		s_bRead = true;
		s_bOk = false;

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
			return false;

		TBD_ObjectiveDocStruct doc = new TBD_ObjectiveDocStruct();
		if (!ctx.ReadValue("", doc))
			return false;

		if (!doc.zones)
			return false;

		s_aZones = doc.zones;
		s_bOk = true;
		return true;
	}

	//! The rules for one zone. Both passes read the same array in the same order, so the index
	//! joins first (the only join for a blank id); a mismatched id, which a `null` entry in
	//! `zones[]` causes, falls back to the first zone with that id.
	//! @param index the zone's index in the loader's pass
	//! @param zoneId the zone's id
	//! @return the zone's rules, or null when this pass has none for it
	static TBD_ObjectiveRulesStruct ForZone(int index, string zoneId)
	{
		if (!s_aZones)
			return null;

		if (index >= 0 && index < s_aZones.Count())
		{
			TBD_ObjectiveZoneStruct atIndex = s_aZones[index];
			if (atIndex && atIndex.id == zoneId)
				return atIndex.rules;
		}

		if (zoneId.IsEmpty())
			return null;

		foreach (TBD_ObjectiveZoneStruct zone : s_aZones)
		{
			if (zone && zone.id == zoneId)
				return zone.rules;
		}

		return null;
	}
}
