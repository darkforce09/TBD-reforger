/**
 * @file TBD_RadioPlan.c
 * @brief The mission's `radioPlan.nets[]`, content-validated once per loaded mission.
 *
 * Role: validates and caches the nets from `TBD_MissionLoader.GetMission().radioPlan` (the
 * loader's single parse; no second pass) and answers per-faction net lists.  Position: read by
 * `TBD_RadioService`, `TBD_RadioTuner`, `TBD_RadioClient` and `TBD_RadioComponent`.
 * State: the validated nets and the mission id they were built from, static, on the server.
 * Invariants: `JsonLoadContext` allocates nested refs even for absent keys, so every test is a
 * content test: the plan exists iff `nets.Count() > 0`, and a net is kept only with `freqMHz`
 * inside 30..512 (0, the all-zeros phantom, never is), a non-empty id and label, and a faction
 * that is empty (shared) or declared by the mission; each rejection warns once with its reason; at
 * most `MAX_NETS` nets are kept and truncation is logged; the plan is served only from a mission
 * that passed validation.
 */

//! One `radioPlan.nets[]` entry. Field names must equal the JSON keys -- `JsonLoadContext` maps
//! by name.
//! @contract mission.schema.json#/$defs/net
class TBD_MissionNetStruct
{
	string id; //!< JSON `id`; stable channel key; required
	string label; //!< JSON `label`; display name; required, at most 48 characters
	float freqMHz; //!< JSON `freqMHz`; megahertz, 30..512; 0 when absent, which is outside the band
	string faction; //!< JSON `faction`; empty serves every side, otherwise only that side
	string range; //!< JSON `range`; `long` prefers a backpack radio; `short` or empty a handheld
}

//! The `radioPlan` block itself. Declared here; owned as a field on `TBD_MissionDocumentStruct`.
//! @contract mission.schema.json#/$defs/radioPlan
class TBD_MissionRadioPlanStruct
{
	ref array<ref TBD_MissionNetStruct> nets; //!< JSON `nets`; allocated even when absent
}

//! Validated, cached radio plan of the loaded mission.
//! @authority server
class TBD_RadioPlan
{
	static const string CH_RADIO = "Radio"; //!< log channel of every radio line
	static const float FREQ_MHZ_MIN = 30; //!< lowest accepted `freqMHz`; the schema band, re-checked here
	static const float FREQ_MHZ_MAX = 512; //!< highest accepted `freqMHz`; the schema band, re-checked here
	static const int MAX_NETS = 32; //!< most nets kept per document; the schema's `nets.maxItems`
	static const int MAX_LABEL_CHARS = 48; //!< longest label kept; the schema's `net.label.maxLength`

	protected static ref array<ref TBD_MissionNetStruct> s_aNets; //!< validated nets in document order; empty once parsed
	protected static string s_sParsedMissionId; //!< mission id the nets were built from; a switch re-parses
	protected static bool s_bParsed; //!< true once parsed for `s_sParsedMissionId`, including a plan with no nets

	//! The nets a faction may use, in document order: shared nets (empty `faction`) and that
	//! faction's own. The answer is built, not filtered, so another side's nets never enter it.
	//! @param factionKey the player's faction
	//! @return the nets, never null
	static array<TBD_MissionNetStruct> GetNetsForFaction(string factionKey)
	{
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

	//! @return the validated nets across all sides; diagnostics only, never a player's answer
	static int GetTotalNetCount()
	{
		EnsureParsed();
		if (!s_aNets)
			return 0;

		return s_aNets.Count();
	}

	//! Convert a schema frequency to the engine's unit (`BaseTransceiver.SetFrequency` takes kHz),
	//! rounded so 42.5 MHz lands on 42500 kHz.
	//! @param freqMHz megahertz
	//! @return kilohertz
	static int FreqKHz(float freqMHz)
	{
		return TBD_Rounding.RoundToInt(freqMHz * 1000);
	}

	//! Format integer kilohertz as megahertz text, so no float-printing artefact reaches a player.
	//! @param freqKHz kilohertz
	//! @return for example `42.500 MHz` for 42500
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

	//! Drop the cache; statics outlive a world inside one process.
	static void Reset()
	{
		s_aNets = null;
		s_sParsedMissionId = string.Empty;
		s_bParsed = false;
	}

	//! Validate once per loaded mission id from the loader's parsed `radioPlan`. A mission that is
	//! not valid serves no plan, and drops a plan parsed earlier.
	protected static void EnsureParsed()
	{
		if (!TBD_MissionLoader.IsValid())
		{
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

		AcceptFromDoc(doc.radioPlan, doc.meta.id);
	}

	//! Content-validate the plan's nets into `s_aNets`, warning once per rejected net with its
	//! reason and logging the `plan` summary line.
	//! @param plan the loader's `radioPlan`; absent or empty is legal and logs `nets=0`
	//! @param missionId the mission id, for the log
	protected static void AcceptFromDoc(TBD_MissionRadioPlanStruct plan, string missionId)
	{
		// Presence is the nets count: the nested ref is allocated even when the key is absent.
		if (!plan || !plan.nets || plan.nets.IsEmpty())
		{
			TBD_Log.Kv(CH_RADIO, "plan",
				string.Format("mission=%1 nets=0 (mission authored no radioPlan -- legal)", missionId));
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
					"mission '%1' net rejected (%2) -- it will not be served to anyone.", missionId, fault));
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

	//! @return every faction key the mission declares, as map keys
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

	//! Why a net is unusable. `freqMHz` is checked first, since the band also rejects an allocated
	//! phantom net. A named faction must be declared by the mission: the schema only checks its
	//! pattern, and an undeclared one would be served to nobody.
	//! @param net the authored net
	//! @param knownFactions the mission's faction keys; empty skips the faction check
	//! @return empty when usable, otherwise the reason, phrased for the mission author
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
			return string.Format("id='%1' has an empty label -- a player would see a blank net", net.id);

		// Only a named faction is checked; a mission without factions is refused by the validator.
		if (!net.faction.IsEmpty() && !knownFactions.IsEmpty() && !knownFactions.Contains(net.faction))
		{
			return string.Format("id='%1' is scoped to faction '%2', which this mission does not declare -- it would be served to nobody",
				net.id, net.faction);
		}

		return string.Empty;
	}

	//! Cut the label to `MAX_LABEL_CHARS`; an over-long name never withholds the net.
	//! @return the label, truncated when longer than the cap
	protected static string CapLabel(string label)
	{
		if (label.Length() <= MAX_LABEL_CHARS)
			return label;

		return label.Substring(0, MAX_LABEL_CHARS);
	}
}
