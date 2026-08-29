//! T-181.18 - turns the mission document's `zones[]` into prepared `TBD_Zone`s, once, and answers
//! "is this player where they are allowed to be".
//!
//! -- Server-side only ------------------------------------------------------------------------
//! Clients hold NO mission document (`TBD_FrameworkManager.OnPostInit` returns early for
//! `RplMode.Client` before `BeginLoad()`), so a client calling `Build()` would build an empty
//! registry and conclude the whole world is out of bounds. Nothing here is client-safe and nothing
//! here is called from a client path; `TBD_PlayAreaComponent` is the only caller and it is
//! authority-gated.
//!
//! -- Static, and therefore explicitly cleared ------------------------------------------------
//! A recorded landmine in this program is that statics OUTLIVE A WORLD inside one process
//! (`SelectMissionByNumber` restarts the scenario in-process). A registry built for mission A and
//! left standing would confine mission B's players to mission A's AO. `Clear()` exists for that
//! and `TBD_PlayAreaComponent.OnDelete` is required to call it.
//!
//! -- What "in bounds" means (the semantics, stated once) -------------------------------------
//! * `boundary` with no `faction`  - the AO. Applies to everyone.
//! * `boundary` with `faction: x`  - applies to side x only; other sides are unconstrained by it.
//! * A player is IN BOUNDS when inside **at least one** boundary zone that applies to them
//!   (UNION, not intersection). An AO drawn as three overlapping polygons is one AO, which is how
//!   an author would expect it to read.
//! * A player to whom **no** boundary zone applies has **no play-area restriction**. A mission
//!   with no boundary zone at all restricts nobody - see `TBD_PlayAreaComponent`.
//! * `base_protection` with `faction: x` - x's protected ground. A player who is NOT of x and IS
//!   inside it is in violation. (This is the inverse containment test, same warn/grace/penalty
//!   machinery.) With no `faction` it protects nobody and is reported and skipped.
class TBD_ZoneRegistry
{
	//! Log channel. A literal rather than a `TBD_Log.CH_*` constant because `TBD_Log.c` belongs to
	//! another slice's lane; keeping the string in one place here preserves the greppable-tag
	//! property that the constants exist for. Fold it into `TBD_Log` when that file is next open.
	static const string CH = "Zones";

	//! Schema enum values (`mission.schema.json#/$defs/zone/type`) this module acts on.
	static const string TYPE_BOUNDARY         = "boundary";
	static const string TYPE_BASE_PROTECTION  = "base_protection";

	//! `rules.penalty` CLOSED vocabulary (`none`|`warn`|`kill`) under `#/$defs/zoneRules`
	//! (`additionalProperties: false`, T-241). Undeclared keys fail schema validation - not an
	//! open-object free-for-all. Adding a penalty value is an enum entry there PLUS a constant here.
	static const string PENALTY_NONE = "none";
	static const string PENALTY_WARN = "warn";
	static const string PENALTY_KILL = "kill";

	//! Defaults applied when a rule is absent, out of range, or unreadable. Every one of these is
	//! also named in the diagnostic that reports the fallback, so an operator reading the log
	//! never has to come here to find out what they got.
	static const float DEFAULT_GRACE_SECONDS = 30.0;
	static const float DEFAULT_WARN_EVERY_SECONDS = 5.0;

	//! Sanity ceiling on an authored grace. Pinned in schema as `zoneRules.graceSeconds.maximum`
	//! = 3600 (T-275 / mission.schema.json). Guard against a typo (`graceSeconds: 30000`)
	//! silently disabling enforcement for the whole round - schema rejects it upstream; this
	//! remains the runtime fallback if a document somehow reaches us out of band.
	static const float MAX_GRACE_SECONDS = 3600.0;

	protected static ref array<ref TBD_Zone> s_aZones;
	protected static bool s_bBuilt;
	protected static int s_iBoundaryCount;
	protected static int s_iBaseProtectionCount;

	//------------------------------------------------------------------------------------------------
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//------------------------------------------------------------------------------------------------
	static int GetBoundaryCount()
	{
		return s_iBoundaryCount;
	}

	//------------------------------------------------------------------------------------------------
	static int GetBaseProtectionCount()
	{
		return s_iBaseProtectionCount;
	}

	//------------------------------------------------------------------------------------------------
	//! Every prepared zone, including unusable ones (they are kept so the count in the summary
	//! matches the document). Null until `Build()` has run.
	static array<ref TBD_Zone> GetAll()
	{
		return s_aZones;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop everything. MUST be called on world teardown - see the class header.
	static void Clear()
	{
		s_aZones = null;
		s_bBuilt = false;
		s_iBoundaryCount = 0;
		s_iBaseProtectionCount = 0;
	}

	//------------------------------------------------------------------------------------------------
	//! Prepare every zone in the loaded mission. Safe to call repeatedly; only the first call after
	//! a `Clear()` does work.
	//!
	//! Returns false when there is no valid mission to build from - the caller keeps waiting rather
	//! than caching an empty registry as if it were the answer.
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		array<ref TBD_MissionZoneStruct> raw = TBD_MissionLoader.GetZones();
		if (!raw)
			return false;

		s_aZones = new array<ref TBD_Zone>();
		s_iBoundaryCount = 0;
		s_iBaseProtectionCount = 0;

		int usable = 0;
		int circles = 0;
		int polygons = 0;

		foreach (int index, TBD_MissionZoneStruct rawZone : raw)
		{
			if (!rawZone)
			{
				TBD_Log.Warn(CH, string.Format("zones[%1] is null - skipped", index));
				continue;
			}

			TBD_Zone zone = Prepare(rawZone, index);
			s_aZones.Insert(zone);

			if (zone.IsUsable())
			{
				usable++;
				if (zone.m_eShape == TBD_EZoneShapeKind.CIRCLE)
					circles++;
				else
					polygons++;
			}

			if (zone.m_sType == TYPE_BOUNDARY && zone.IsUsable())
			{
				s_iBoundaryCount++;
			}
			else if (zone.m_sType == TYPE_BASE_PROTECTION && zone.IsUsable())
			{
				if (zone.m_sFaction.IsEmpty())
				{
					// Said out loud rather than left to be inferred from a count of 0. A protection
					// zone works by asking "is this player NOT of the owning faction", so one that
					// names no faction has no question to ask and can never be violated by anyone.
					TBD_Log.Warn(CH, string.Format("zone '%1' is base_protection but names no faction - it protects nobody and is not enforced. Set `faction` to the side whose ground this is.",
						zone.m_sId));
				}
				else
				{
					s_iBaseProtectionCount++;
				}
			}

			if (EnforcesType(zone.m_sType))
				LogPrepared(zone);
		}

		s_bBuilt = true;

		// One greppable summary line. `polygons=` here is also the standing proof that the typed
		// JSON reader really does populate `ref array<ref array<float>>` at runtime: a build that
		// silently lost nested arrays would report every polygon zone as unusable and this number
		// would be 0 on a mission that authored them.
		TBD_Log.Kv(CH, "built", string.Format("zones=%1 usable=%2 circle=%3 polygon=%4 boundary=%5 baseProtection=%6",
			raw.Count(), usable, circles, polygons, s_iBoundaryCount, s_iBaseProtectionCount));

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! One line per enforced zone, at load. Two jobs:
	//!   * OPERATIONAL - an operator can read the AO's extents straight out of the log and check
	//!     them against the map before an event, instead of discovering at H-hour that the AO is
	//!     1 km east of where it was drawn.
	//!   * PROOF - the bounds are computed from the parsed floats, so a run that prints plausible
	//!     coordinates has demonstrated that `ref array<ref array<float>>` really did populate from
	//!     JSON, not merely that a non-null array arrived. Compile probes cannot show that; this
	//!     shows it in every single run.
	protected static void LogPrepared(notnull TBD_Zone zone)
	{
		if (!zone.IsUsable())
			return;

		int vertices = 0;
		if (zone.m_aFlat)
			vertices = zone.m_aFlat.Count() / 2;

		TBD_Log.Kv(CH, "zone", string.Format("id=%1 type=%2 shape=%3 faction='%4' bounds=[%5,%6 %7,%8] vertices=%9",
			zone.m_sId,
			zone.m_sType,
			typename.EnumToString(TBD_EZoneShapeKind, zone.m_eShape),
			zone.m_sFaction,
			zone.m_fMinX, zone.m_fMinZ, zone.m_fMaxX, zone.m_fMaxZ,
			vertices));

		TBD_Log.Kv(CH, "zoneRules", string.Format("id=%1 grace=%2s warnEvery=%3s penalty=%4",
			zone.m_sId,
			zone.m_fGraceSeconds,
			zone.m_fWarnEverySeconds,
			typename.EnumToString(TBD_EZonePenalty, zone.m_ePenalty)));
	}

	//------------------------------------------------------------------------------------------------
	//! Flatten one document zone into its runtime form, reporting every defect by zone id.
	protected static TBD_Zone Prepare(notnull TBD_MissionZoneStruct rawZone, int index)
	{
		TBD_Zone zone = new TBD_Zone();
		zone.m_sId = rawZone.id;
		zone.m_sType = rawZone.type;
		zone.m_sLabel = rawZone.label;
		zone.m_sFaction = rawZone.faction;
		zone.m_eShape = TBD_EZoneShapeKind.NONE;

		string subject = rawZone.id;
		if (subject.IsEmpty())
			subject = string.Format("zones[%1]", index);

		// Diagnostics are reported only for the zone types THIS module enforces. Every zone gets
		// its rules resolved (so no field is ever left at a sentinel), but the golden mission's
		// `objective_capture` zone legitimately carries `captureSeconds`/`contestable`/`points` -
		// rules for a subsystem that is not this one. Warning that "this build could not read a
		// single key it understands" about those would be false and would train an operator to
		// ignore the message that matters.
		ResolveRules(zone, rawZone.rules, subject, EnforcesType(rawZone.type));

		if (!rawZone.shape)
		{
			// Only worth shouting about for the types this module enforces; a spawn or objective
			// zone with no shape is somebody else's problem and TBD_MissionValidator already
			// reports it.
			if (EnforcesType(rawZone.type))
				TBD_Log.Warn(CH, string.Format("zone '%1' (%2) has no shape - inert, it will never contain anyone",
					subject, rawZone.type));
			return zone;
		}

		// -- Which shape did the author actually draw? ----------------------------------------
		// NOT `if (shape.circle)`. `JsonLoadContext` allocates a nested `ref` field whether or not
		// the JSON key was there (measured - see the landmine on TBD_MissionShapeStruct), so both
		// members are always non-null and only their CONTENT distinguishes them. Getting this
		// wrong is not theoretical: the first cut of this function took the circle branch for the
		// golden mission's polygon-only boundary zone and declared it inert with "radius 0".
		bool hasPolygon = rawZone.shape.polygon && rawZone.shape.polygon.Count() > 0;
		bool hasCircle = rawZone.shape.circle && rawZone.shape.circle.r > 0;

		if (hasPolygon && hasCircle)
		{
			// The schema's `oneOf` forbids this, so the document is not schema-valid. Say so and
			// take the polygon: it is the more specific of the two and the one an author who drew
			// both almost certainly meant.
			TBD_Log.Warn(CH, string.Format("zone '%1' carries BOTH a circle and a polygon (the schema's shape is oneOf) - using the polygon",
				subject));
			hasCircle = false;
		}

		if (hasPolygon)
		{
			BuildPolygon(zone, rawZone.shape.polygon, subject);
			return zone;
		}

		if (hasCircle)
		{
			TBD_MissionCircleStruct c = rawZone.shape.circle;
			zone.m_eShape = TBD_EZoneShapeKind.CIRCLE;
			zone.m_fCx = c.x;
			zone.m_fCz = c.z;
			zone.m_fR = c.r;
			zone.m_fMinX = c.x - c.r;
			zone.m_fMaxX = c.x + c.r;
			zone.m_fMinZ = c.z - c.r;
			zone.m_fMaxZ = c.z + c.r;
			return zone;
		}

		// No usable content in either member. On a schema-valid document this cannot happen, so it
		// means the document is not schema-valid (a circle with radius <= 0 violates
		// `exclusiveMinimum: 0`) or the shape did not survive parsing. Report the radius we did see
		// so an operator can tell the two apart.
		if (EnforcesType(rawZone.type))
		{
			float radius = 0;
			if (rawZone.shape.circle)
				radius = rawZone.shape.circle.r;

			TBD_Log.Warn(CH, string.Format("zone '%1' (%2) has no usable shape - no polygon vertices, and circle radius is %3 (schema requires > 0). Inert; it will never contain anyone.",
				subject, rawZone.type, radius));
		}

		return zone;
	}

	//------------------------------------------------------------------------------------------------
	//! `[[x,z],[x,z],...]` -> flat `[x,z,x,z,...]` plus bounds. Rejects rather than guesses.
	protected static void BuildPolygon(notnull TBD_Zone zone, notnull array<ref array<float>> rings, string subject)
	{
		array<float> flat = new array<float>();
		int malformed = 0;

		foreach (array<float> pair : rings)
		{
			// Schema: inner arrays are exactly 2 numbers. A pair that is not is dropped rather than
			// padded - inventing a coordinate would move the AO's outline somewhere nobody drew.
			if (!pair || pair.Count() != 2)
			{
				malformed++;
				continue;
			}

			flat.Insert(pair[0]);
			flat.Insert(pair[1]);
		}

		if (malformed > 0)
			TBD_Log.Warn(CH, string.Format("zone '%1' polygon: %2 vertex/vertices were not exactly [x, z] and were dropped",
				subject, malformed));

		int vertices = flat.Count() / 2;
		if (vertices < 3)
		{
			TBD_Log.Warn(CH, string.Format("zone '%1' polygon has %2 usable vertices (schema minimum 3) - inert, it will never contain anyone",
				subject, vertices));
			return;
		}

		zone.m_eShape = TBD_EZoneShapeKind.POLYGON;
		zone.m_aFlat = flat;

		zone.m_fMinX = flat[0];
		zone.m_fMaxX = flat[0];
		zone.m_fMinZ = flat[1];
		zone.m_fMaxZ = flat[1];
		for (int i = 1; i < vertices; i++)
		{
			float x = flat[i * 2];
			float z = flat[(i * 2) + 1];
			if (x < zone.m_fMinX)
				zone.m_fMinX = x;
			if (x > zone.m_fMaxX)
				zone.m_fMaxX = x;
			if (z < zone.m_fMinZ)
				zone.m_fMinZ = z;
			if (z > zone.m_fMaxZ)
				zone.m_fMaxZ = z;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve `rules` into plain fields, and make every fallback LOUD.
	//!
	//! See `TBD_MissionZoneRulesStruct` for why a typed parser cannot see an unknown key. What this
	//! CAN do, and does:
	//!   * no rule legible                    -> WARNING naming the zone and the defaults it got
	//!   * key present but out of range       -> WARNING naming the zone, the value and the default
	//!   * `penalty` string unrecognised      -> WARNING naming the value; falls back to WARN, never
	//!                                           to KILL, because guessing wrong toward KILL ends a
	//!                                           player's one life on a typo
	//!
	//! Note the deliberate absence of a "`rules` was absent" case. `JsonLoadContext` allocates the
	//! nested `rules` object whether or not the key was authored (measured - see the landmine on
	//! `TBD_MissionShapeStruct`), so `!rules` never fires on a parsed document and the two cases
	//! "authored nothing" and "authored only keys this build cannot see" are INDISTINGUISHABLE from
	//! here. The diagnostic below is worded to be true of both rather than asserting the one it
	//! cannot know. An earlier draft claimed "you authored a rules{} object" and the runtime probe
	//! caught it saying that about a zone with no `rules` key at all.
	//!
	//! `report` is false for zone types this module does not enforce: their rules belong to another
	//! subsystem (the golden mission's `objective_capture` zone carries `captureSeconds`,
	//! `contestable`, `points`) and complaining that this build cannot read them would be false.
	//! Defaults are still resolved for every zone so no field is ever left holding a sentinel.
	protected static void ResolveRules(notnull TBD_Zone zone, TBD_MissionZoneRulesStruct rules, string subject, bool report)
	{
		zone.m_fGraceSeconds = DEFAULT_GRACE_SECONDS;
		zone.m_fWarnEverySeconds = DEFAULT_WARN_EVERY_SECONDS;
		zone.m_ePenalty = TBD_EZonePenalty.WARN;

		if (!rules)
			return;

		int legible = 0;

		if (rules.graceSeconds != TBD_MissionZoneRulesStruct.ABSENT)
		{
			legible++;
			if (rules.graceSeconds < 0 || rules.graceSeconds > MAX_GRACE_SECONDS)
			{
				if (report)
					TBD_Log.Warn(CH, string.Format("zone '%1' rules.graceSeconds=%2 is outside 0..%3 - using the default %4 s",
						subject, rules.graceSeconds, MAX_GRACE_SECONDS, DEFAULT_GRACE_SECONDS));
			}
			else
			{
				zone.m_fGraceSeconds = rules.graceSeconds;
			}
		}

		if (rules.warnEverySeconds != TBD_MissionZoneRulesStruct.ABSENT)
		{
			legible++;
			if (rules.warnEverySeconds <= 0)
			{
				if (report)
					TBD_Log.Warn(CH, string.Format("zone '%1' rules.warnEverySeconds=%2 must be > 0 - using the default %3 s",
						subject, rules.warnEverySeconds, DEFAULT_WARN_EVERY_SECONDS));
			}
			else
			{
				zone.m_fWarnEverySeconds = rules.warnEverySeconds;
			}
		}

		if (!rules.penalty.IsEmpty())
		{
			legible++;
			if (rules.penalty == PENALTY_NONE)
			{
				zone.m_ePenalty = TBD_EZonePenalty.NONE;
			}
			else if (rules.penalty == PENALTY_WARN)
			{
				zone.m_ePenalty = TBD_EZonePenalty.WARN;
			}
			else if (rules.penalty == PENALTY_KILL)
			{
				zone.m_ePenalty = TBD_EZonePenalty.KILL;
				// Loud on purpose. This is the line an operator should find in the log when they
				// ask why somebody's one life ended at the edge of the map.
				if (report)
					TBD_Log.Warn(CH, string.Format("zone '%1' rules.penalty=kill - ONE LIFE: a player who stays in violation past %2 s is KILLED and can only return via '#tbd respawn'",
						subject, zone.m_fGraceSeconds));
			}
			else
			{
				zone.m_ePenalty = TBD_EZonePenalty.WARN;
				if (report)
					TBD_Log.Warn(CH, string.Format("zone '%1' rules.penalty='%2' is not one of none|warn|kill - using 'warn' (never guessing toward kill under one life)",
						subject, rules.penalty));
			}
		}

		if (legible == 0 && report)
		{
			TBD_Log.Warn(CH, string.Format("zone '%1': no rule this build understands (graceSeconds, warnEverySeconds, penalty) was readable - running on defaults grace=%2s warnEvery=%3s penalty=warn. Either none was authored, or one was authored under a key this build does not declare and therefore cannot see; a typed JSON parser cannot tell those apart.",
				subject, DEFAULT_GRACE_SECONDS, DEFAULT_WARN_EVERY_SECONDS));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Zone types this module enforces. Used only to decide whether a defect is worth a log line
	//! here - a malformed objective zone is not this slice's business.
	protected static bool EnforcesType(string type)
	{
		return type == TYPE_BOUNDARY || type == TYPE_BASE_PROTECTION;
	}

	//------------------------------------------------------------------------------------------------
	//! Does any usable `boundary` zone apply to this faction? Empty `factionKey` (a player with no
	//! resolved slot) matches only the unfactioned, everyone-applies zones.
	//!
	//! Callers need this SEPARATELY from the containment test: "no boundary applies to me" and
	//! "a boundary applies and I am outside it" are opposite verdicts, and collapsing them would
	//! confine every player on a mission that has no AO.
	static bool HasBoundaryFor(string factionKey)
	{
		if (!s_aZones)
			return false;

		foreach (TBD_Zone zone : s_aZones)
		{
			if (zone && zone.m_sType == TYPE_BOUNDARY && zone.IsUsable() && AppliesToFaction(zone, factionKey))
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Is this position inside at least one boundary zone that applies to `factionKey`?
	//! Undefined (and never asked) when `HasBoundaryFor` is false.
	static bool IsInsideBoundary(string factionKey, float px, float pz)
	{
		if (!s_aZones)
			return true;

		foreach (TBD_Zone zone : s_aZones)
		{
			if (!zone || zone.m_sType != TYPE_BOUNDARY || !zone.IsUsable())
				continue;
			if (!AppliesToFaction(zone, factionKey))
				continue;
			if (zone.Contains(px, pz))
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! The first `base_protection` zone this player is illegally standing in, or null.
	//!
	//! "Illegally" = the zone names a faction, the player is not of it, and the player is inside.
	//! A zone with no faction protects nobody; `Build()` already declined to count it and
	//! `HasProtectionAgainst` below reports it once at load rather than every tick.
	static TBD_Zone FindViolatedProtection(string factionKey, float px, float pz)
	{
		if (!s_aZones)
			return null;

		foreach (TBD_Zone zone : s_aZones)
		{
			if (!zone || zone.m_sType != TYPE_BASE_PROTECTION || !zone.IsUsable())
				continue;
			if (zone.m_sFaction.IsEmpty())
				continue;
			// Own side is welcome. A player with no resolved faction is treated as an outsider -
			// the conservative reading, and it only ever produces a warning by default.
			if (zone.m_sFaction == factionKey)
				continue;
			if (zone.Contains(px, pz))
				return zone;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! The boundary zone whose rules govern this faction's violation. With several applicable
	//! boundary zones the STRICTEST wins - shortest grace, and KILL beats WARN beats NONE - so an
	//! author who overlaps a hard inner AO with a soft outer one gets the hard answer rather than
	//! whichever happened to be listed first.
	static TBD_Zone GoverningBoundary(string factionKey)
	{
		if (!s_aZones)
			return null;

		TBD_Zone strictest;
		foreach (TBD_Zone zone : s_aZones)
		{
			if (!zone || zone.m_sType != TYPE_BOUNDARY || !zone.IsUsable())
				continue;
			if (!AppliesToFaction(zone, factionKey))
				continue;

			if (!strictest)
			{
				strictest = zone;
				continue;
			}

			if (zone.m_ePenalty > strictest.m_ePenalty)
			{
				strictest = zone;
				continue;
			}

			if (zone.m_ePenalty == strictest.m_ePenalty && zone.m_fGraceSeconds < strictest.m_fGraceSeconds)
				strictest = zone;
		}

		return strictest;
	}

	//------------------------------------------------------------------------------------------------
	//! A boundary zone applies to a player when it names no faction (everyone) or names theirs.
	protected static bool AppliesToFaction(notnull TBD_Zone zone, string factionKey)
	{
		if (zone.m_sFaction.IsEmpty())
			return true;

		return zone.m_sFaction == factionKey;
	}
}
