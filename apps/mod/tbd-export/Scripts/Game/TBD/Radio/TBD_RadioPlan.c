//! T-181.40 / T-293 - the mission document's `radioPlan.nets[]`, validated from the loader parse.
//!
//! -- Canonical field path (T-293) ------------------------------------------------------------
//! `TBD_MissionDocumentStruct.radioPlan` (Backend/TBD_MissionLoader.c) is filled by the ONE
//! `JsonLoadContext` pass in `TBD_MissionLoader`. `EnsureParsed` reads that field - it does NOT
//! re-bind the raw document text through a projection. A second pass would only re-bind the same
//! bytes the loader already consumed, with two ways to drift and a lying comment claiming the
//! field was missing.
//!
//! -- PRESENCE IS NOT CONTENT (the landmine this file is built around) ------------------------
//! `JsonLoadContext` ALLOCATES a nested `ref <class>` field even when the JSON key is ABSENT.
//! Measured on a live boot against `golden-missions/bridgehead-at-levie.json`: an unauthored
//! `shape.circle` came back non-null with `x=0 z=0 r=0`. So `if (doc.radioPlan)` is ALWAYS
//! true after a successful load and proves nothing, and a mission with no radio plan would
//! otherwise arrive as a plan containing garbage.
//!
//! Every test in this file is therefore a CONTENT test:
//!   * the plan exists iff `nets.Count() > 0` (a container count, not a null check);
//!   * a net exists iff its `freqMHz` is inside the schema band 30..512 AND its `id` and `label`
//!     are non-empty.
//! `freqMHz` is the sentinel that does the real work: an all-zeros `TBD_MissionNetStruct` scores
//! `freqMHz == 0`, which is outside the band and can never be authored, so a phantom net is
//! rejected by the same rule that rejects a malformed one.
//!
//! -- `required` in the schema does NOT mean non-empty ----------------------------------------
//! `$defs/net` requires `id`, `label` and `freqMHz`, and `label` DOES carry `minLength: 1` - but
//! `golden-missions/empty-warning-fields.json` already ships a committed, schema-valid marker
//! whose required strings are both `""`, so "required, therefore complete" is a claim about
//! presence and not about content. Nothing downstream of this file may assume a net's strings are
//! non-empty on the strength of the schema alone; they are non-empty because this file checked.
//! @contract mission.schema.json#/$defs/radioPlan
//! @contract mission.schema.json#/$defs/net

//! One `radioPlan.nets[]` entry. Field names must equal the JSON keys - `JsonLoadContext` maps
//! by name.
//! @contract mission.schema.json#/$defs/net
class TBD_MissionNetStruct
{
	string id;        //!< `net:<id>` - stable channel key. Schema-required.
	string label;     //!< Display name ("Alpha Squad"). Schema-required, `minLength: 1`.
	//! Megahertz, 30..512 per the schema. `0` is the ABSENT sentinel: it is outside the band, so
	//! it cannot be confused with authored data. This is the presence test for the whole struct.
	float freqMHz;
	//! Optional faction scope. EMPTY = the net belongs to no one side and is served to everybody.
	//! Non-empty = side-scoped intelligence, and `TBD_RadioService` will not serve it to anyone
	//! else. `JsonLoadContext` leaves a missing string at its initializer, so empty means absent.
	string faction;
	//! `short` | `long` (T-292; schema default `short`). Empty = absent -> same handheld path as
	//! `short` (`LongRangeFlag` -> 0). Decides WHICH radio a net prefers - see `TBD_RadioTuner`.
	//! Retired value `any` is schema-rejected; still treated as handheld if it somehow arrives.
	string range;
}

//! The `radioPlan` block itself. Declared here; owned as a field on `TBD_MissionDocumentStruct`.
//! @contract mission.schema.json#/$defs/radioPlan
class TBD_MissionRadioPlanStruct
{
	ref array<ref TBD_MissionNetStruct> nets;
}

//! Validated, cached radio plan for the currently loaded mission.
//! Source of nets: `TBD_MissionLoader.GetMission().radioPlan` (already parsed) - T-293.
class TBD_RadioPlan
{
	//! Greppable channel for everything this slice logs: `grep '\[TBD\]\[Radio\]' console.log`.
	static const string CH_RADIO = "Radio";

	//! Schema band for `freqMHz`. Duplicated here deliberately: the mod cannot run the JSON schema,
	//! so the band is re-asserted at the point of use rather than assumed to have been enforced
	//! upstream. A document that reached us out of band is a document we do not trust.
	static const float FREQ_MHZ_MIN = 30;
	static const float FREQ_MHZ_MAX = 512;

	//! Hard cap on nets accepted from one document. Pinned in schema as
	//! `radioPlan.nets.maxItems` = 32 (T-275 / mission.schema.json). The mod cannot run the JSON
	//! schema, so the ceiling is re-asserted here; truncation is LOGGED, never silent.
	static const int MAX_NETS = 32;

	//! Longest label carried anywhere. Pinned in schema as `net.label.maxLength` = 48 (T-275).
	static const int MAX_LABEL_CHARS = 48;

	//! Validated nets, in document order. Empty (never null) once parsed.
	protected static ref array<ref TBD_MissionNetStruct> s_aNets;

	//! Which mission `s_aNets` was built from, so an admin mission switch re-parses instead of
	//! serving the previous round's frequencies.
	protected static string s_sParsedMissionId;

	//! True once a parse attempt has completed for `s_sParsedMissionId` - including the perfectly
	//! legal outcome "this mission authored no radio plan", which must not re-parse every request.
	protected static bool s_bParsed;

	//------------------------------------------------------------------------------------------------
	//! @authority server - nets this faction may use, in document order.
	//!
	//! A net with an EMPTY `faction` is shared and is returned to every side; a net with a faction
	//! is returned only to that side. Never returns null.
	//!
	//! This is the only place the faction scope is applied, and it is applied by BUILDING the
	//! answer rather than by filtering a full list later - the other side's nets never enter the
	//! array, so there is nothing downstream to leak.
	static array<TBD_MissionNetStruct> GetNetsForFaction(string factionKey)
	{
		// NOT named `out` - that is a reserved keyword in Enfusion and fails with
		// `Expected name, not a keyword 'out'`.
		array<TBD_MissionNetStruct> scoped = {};

		EnsureParsed();
		if (!s_aNets)
			return scoped;

		foreach (TBD_MissionNetStruct net : s_aNets)
		{
			if (!net)
				continue;

			if (!net.faction.IsEmpty() && net.faction != factionKey)
				continue;

			scoped.Insert(net);
		}

		return scoped;
	}

	//------------------------------------------------------------------------------------------------
	//! Total validated nets in the loaded mission, across all sides. Diagnostics only - never used
	//! to answer a player.
	static int GetTotalNetCount()
	{
		EnsureParsed();
		if (!s_aNets)
			return 0;

		return s_aNets.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! Kilohertz for a schema `freqMHz`. The engine's radio API is kHz throughout
	//! (`BaseTransceiver.SetFrequency` - "Frequency in KHz"), the schema is MHz, and this is the
	//! ONE place the two meet.
	//!
	//! Rounded, not truncated: `42.5` MHz must land on `42500` kHz and float multiplication does
	//! not promise to. Enfusion has no ternary operator, so the sign branch is written out.
	static int FreqKHz(float freqMHz)
	{
		float khz = freqMHz * 1000;
		if (khz >= 0)
			return khz + 0.5;

		return khz - 0.5;
	}

	//------------------------------------------------------------------------------------------------
	//! `42500` -> `"42.500 MHz"`. Formatted from the INTEGER kHz, never from the float, so the
	//! text a player reads cannot pick up a float-printing artefact.
	static string FormatMHz(int freqKHz)
	{
		int whole = freqKHz / 1000;
		int frac = freqKHz % 1000;
		if (frac < 0)
			frac = -frac;

		string pad = string.Empty;
		if (frac < 100)
			pad = "0";
		if (frac < 10)
			pad = "00";

		// Built in steps: a long `+` chain trips `Formula too complex` (measured at 9 fields), and
		// its second diagnostic is a misleading `Incompatible parameter`.
		string text = whole.ToString();
		text = text + ".";
		text = text + pad;
		text = text + frac.ToString();
		text = text + " MHz";
		return text;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop the cache. Called when the world goes away - statics outlive a world inside one
	//! process, which is a recorded landmine in this program.
	static void Reset()
	{
		s_aNets = null;
		s_sParsedMissionId = string.Empty;
		s_bParsed = false;
	}

	//------------------------------------------------------------------------------------------------
	//! Validate once per loaded mission, from the already-parsed document field.
	//!
	//! Gated on `TBD_MissionLoader.IsValid()`: serving frequencies out of a mission the validator
	//! rejected would be exactly the kind of quiet half-load this program keeps getting bitten by.
	protected static void EnsureParsed()
	{
		if (!TBD_MissionLoader.IsValid())
		{
			// A mission was unloaded or replaced by a bad one - do not keep serving the old plan.
			if (s_bParsed)
				Reset();

			return;
		}

		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return;

		if (s_bParsed && s_sParsedMissionId == doc.meta.id)
			return;

		s_aNets = {};
		s_sParsedMissionId = doc.meta.id;
		s_bParsed = true;

		// T-293 Class-R pin: canonical path is doc.radioPlan - never a raw-JSON second pass.
		AcceptFromDoc(doc.radioPlan, doc.meta.id);
	}

	//------------------------------------------------------------------------------------------------
	//! Content-validate nets from the loader's `radioPlan` field. Fills `s_aNets`; every rejection
	//! is reported once, at load, with the reason - a mission whose radio plan is silently
	//! half-ignored is worse than one that says so.
	protected static void AcceptFromDoc(TBD_MissionRadioPlanStruct plan, string missionId)
	{
		// NOT `if (plan)`. That is ALWAYS true after a successful loader parse - JsonLoadContext
		// allocates a nested `ref` field whether or not the JSON key was present. The container
		// COUNT is the only reliable presence test, and an absent `radioPlan` is LEGAL: `radioPlan`
		// is not in the schema's top-level `required` list, and `golden-missions/empty-warning-fields.json`
		// authors none. Behave as before: no nets, no complaint.
		if (!plan || !plan.nets || plan.nets.IsEmpty())
		{
			TBD_Log.Kv(CH_RADIO, "plan",
				string.Format("mission=%1 nets=0 (mission authored no radioPlan - legal)", missionId));
			return;
		}

		array<ref TBD_MissionNetStruct> authored = plan.nets;
		int total = authored.Count();
		int rejected = 0;

		map<string, bool> knownFactions = CollectFactionKeys();

		foreach (TBD_MissionNetStruct net : authored)
		{
			if (s_aNets.Count() >= MAX_NETS)
				break;

			string fault = Fault(net, knownFactions);
			if (!fault.IsEmpty())
			{
				rejected++;
				TBD_Log.Warn(CH_RADIO, string.Format(
					"mission '%1' net rejected (%2) - it will not be served to anyone.", missionId, fault));
				continue;
			}

			net.label = CapLabel(net.label);
			s_aNets.Insert(net);
		}

		if (total > s_aNets.Count() + rejected)
		{
			TBD_Log.Warn(CH_RADIO, string.Format(
				"mission '%1' authored %2 nets; accepted the first %3 (cap %4).",
				missionId, total, s_aNets.Count(), MAX_NETS));
		}

		TBD_Log.Kv(CH_RADIO, "plan", string.Format(
			"mission=%1 authored=%2 accepted=%3 rejected=%4",
			missionId, total, s_aNets.Count(), rejected));
	}

	//------------------------------------------------------------------------------------------------
	//! Every faction key the mission actually declares. `map<string, bool>` and not `set<string>`
	//! because Enforce's `set` removal is by INDEX, which is a recorded landmine in this program.
	protected static map<string, bool> CollectFactionKeys()
	{
		map<string, bool> keys = new map<string, bool>();

		array<ref TBD_MissionFactionStruct> factions = TBD_MissionLoader.GetFactions();
		if (!factions)
			return keys;

		foreach (TBD_MissionFactionStruct faction : factions)
		{
			if (faction && !faction.key.IsEmpty())
				keys.Set(faction.key, true);
		}

		return keys;
	}

	//! Empty string = this net is usable. Anything else is the reason it is not, phrased for the
	//! mission author who has to fix it.
	//!
	//! `freqMHz` is checked FIRST and against the band, because that is the check that also
	//! distinguishes an authored net from a struct `JsonLoadContext` allocated out of nothing.
	//!
	//! -- The cross-reference the JSON schema structurally cannot make ------------------------
	//! `net.faction` is a `factionKey` by PATTERN (`^[a-z][a-z0-9_]*$`) and nothing more: no schema
	//! keyword can require it to name a faction this document actually declares. A typo therefore
	//! validates perfectly and then matches no player on either side, so the net is served to
	//! NOBODY and the mission author is told nothing. That is precisely the silent half-load this
	//! program keeps getting bitten by, so it is an explicit rejection with a named reason here.
	//! Same shape as T-181.34's kit-alias decision: the vocabulary is not a closed enum, so
	//! existence is checked against the document the game server is actually reading.
	protected static string Fault(TBD_MissionNetStruct net, map<string, bool> knownFactions)
	{
		if (!net)
			return "null entry";

		if (net.freqMHz < FREQ_MHZ_MIN || net.freqMHz > FREQ_MHZ_MAX)
		{
			return string.Format("id='%1' freqMHz=%2 is outside the schema band %3..%4",
				net.id, net.freqMHz, FREQ_MHZ_MIN, FREQ_MHZ_MAX);
		}

		if (net.id.IsEmpty())
			return "a net has an empty id";

		if (net.label.IsEmpty())
			return string.Format("id='%1' has an empty label - a player would see a blank net", net.id);

		// An empty faction is LEGAL and means "shared net", so only a NAMED faction is checked.
		// `knownFactions` empty means the document declared no factions at all, which the mission
		// validator already refuses on its own account - do not pile a second complaint on top.
		if (!net.faction.IsEmpty() && !knownFactions.IsEmpty() && !knownFactions.Contains(net.faction))
		{
			return string.Format("id='%1' is scoped to faction '%2', which this mission does not declare - it would be served to nobody",
				net.id, net.faction);
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Bound the label without ever dropping the net. An over-long name is truncated; it is never
	//! a reason to withhold a channel a player needs.
	protected static string CapLabel(string label)
	{
		if (label.Length() <= MAX_LABEL_CHARS)
			return label;

		return label.Substring(0, MAX_LABEL_CHARS);
	}
}
