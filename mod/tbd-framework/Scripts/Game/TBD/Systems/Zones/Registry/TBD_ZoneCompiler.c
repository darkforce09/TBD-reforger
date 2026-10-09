/**
 * @file TBD_ZoneCompiler.c
 * @brief Prepares one mission zone: shape, bounds and play-area rules resolved once.
 *
 * Role: flattens a `zones[]` row into a `TBD_Zone`: circle or flat polygon, XZ bounds, and the
 * grace, warning cadence and penalty with every fallback logged.  Position: called by
 * `TBD_ZoneRegistry.Build`; binds `vehicleClasses` through `TBD_PlayAreaVehicleAxis`.
 * State: none.  Invariants: every zone leaves with resolved rules and never a sentinel; a zone with
 * no usable shape is INERT (shape `NONE`), never guessed; an unknown penalty falls back to `warn`,
 * never to `kill`.
 */

//! Zone preparation and play-area rule resolution.
//! @authority server
class TBD_ZoneCompiler
{
	static const string PENALTY_NONE = "none"; //!< `zoneRules.penalty`: log only
	static const string PENALTY_WARN = "warn"; //!< `zoneRules.penalty`: keep warning (the default)
	static const string PENALTY_KILL = "kill"; //!< `zoneRules.penalty`: end the character under one life

	static const float DEFAULT_GRACE_SECONDS = 30.0; //!< `graceSeconds` when absent or out of range
	static const float DEFAULT_WARN_EVERY_SECONDS = 5.0; //!< `warnEverySeconds` when absent or not positive
	static const float MAX_GRACE_SECONDS = 3600.0; //!< ceiling on `graceSeconds`, the schema maximum

	//! Log a `zone` line (id, type, shape, faction, bounds, vertices) and a `zoneRules` line for a
	//! usable enforced zone, so an operator can check the extents against the map before an event.
	//! @param zone the prepared zone; an unusable one logs nothing
	static void LogPrepared(notnull TBD_Zone zone)
	{
		if (!zone.IsUsable())
			return;

		int vertices = 0;
		if (zone.m_aFlat)
			vertices = zone.m_aFlat.Count() / 2;

		TBD_Log.Kv(TBD_ZoneRegistry.CH, "zone", string.Format("id=%1 type=%2 shape=%3 faction='%4' bounds=[%5,%6 %7,%8] vertices=%9",
			zone.m_sId,
			zone.m_sType,
			typename.EnumToString(TBD_EZoneShapeKind, zone.m_eShape),
			zone.m_sFaction,
			zone.m_fMinX, zone.m_fMinZ, zone.m_fMaxX, zone.m_fMaxZ,
			vertices));

		TBD_Log.Kv(TBD_ZoneRegistry.CH, "zoneRules", string.Format("id=%1 grace=%2s warnEvery=%3s penalty=%4",
			zone.m_sId,
			zone.m_fGraceSeconds,
			zone.m_fWarnEverySeconds,
			typename.EnumToString(TBD_EZonePenalty, zone.m_ePenalty)));
	}

	//! Flatten one `zones[]` row into its runtime form, reporting every defect by zone id for the
	//! types this module enforces.
	//! @param rawZone the wire row
	//! @param index its position in `zones[]`, used as the subject when `id` is absent
	//! @return the prepared zone; shape `NONE` when it has no usable shape
	static TBD_Zone Prepare(notnull TBD_MissionZoneStruct rawZone, int index)
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

		// Every zone gets resolved rules; only the enforced types report, since other zone types
		// carry rules for other subsystems.
		ResolveRules(zone, rawZone.rules, subject, EnforcesType(rawZone.type));

		if (!rawZone.shape)
		{
			// Other zone types are reported by TBD_MissionValidator.
			if (EnforcesType(rawZone.type))
				TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' (%2) has no shape -- inert, it will never contain anyone",
					subject, rawZone.type));
			return zone;
		}

		// JsonLoadContext allocates both nested members, so only their content tells them apart.
		bool hasPolygon = rawZone.shape.polygon && rawZone.shape.polygon.Count() > 0;
		bool hasCircle = rawZone.shape.circle && rawZone.shape.circle.r > 0;

		if (hasPolygon && hasCircle)
		{
			// The schema's oneOf forbids both; the polygon, the more specific shape, wins.
			TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' carries BOTH a circle and a polygon (the schema's shape is oneOf) -- using the polygon",
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

		// Neither member has content: a radius <= 0 or a shape lost in parsing; the radius is logged.
		if (EnforcesType(rawZone.type))
		{
			float radius = 0;
			if (rawZone.shape.circle)
				radius = rawZone.shape.circle.r;

			TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' (%2) has no usable shape -- no polygon vertices, and circle radius is %3 (schema requires > 0). Inert; it will never contain anyone.",
				subject, rawZone.type, radius));
		}

		return zone;
	}

	//! Flatten `[[x, z], ...]` into `[x, z, x, z, ...]` and compute the bounds. A pair that is not
	//! exactly two numbers is dropped (logged); fewer than 3 vertices leaves the zone INERT.
	//! @param zone the zone being prepared
	//! @param rings the authored vertex pairs
	//! @param subject the zone's id or index, for the log
	protected static void BuildPolygon(notnull TBD_Zone zone, notnull array<ref array<float>> rings, string subject)
	{
		array<float> flat = new array<float>();
		int malformed = 0;

		foreach (array<float> pair : rings)
		{
			if (!pair || pair.Count() != 2)
			{
				malformed++;
				continue;
			}

			flat.Insert(pair[0]);
			flat.Insert(pair[1]);
		}

		if (malformed > 0)
			TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' polygon: %2 vertex/vertices were not exactly [x, z] and were dropped",
				subject, malformed));

		int vertices = flat.Count() / 2;
		if (vertices < 3)
		{
			TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' polygon has %2 usable vertices (schema minimum 3) -- inert, it will never contain anyone",
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

	//! Resolve `graceSeconds`, `warnEverySeconds` and `penalty` into the zone's fields, starting from
	//! the defaults, and bind `vehicleClasses`. An out-of-range value or unknown penalty is logged
	//! and falls back (an unknown penalty to `warn`, never `kill`); `kill` itself is logged. When no
	//! rule is legible the zone is reported once: a typed reader cannot tell "authored nothing" from
	//! "authored only keys it does not declare", since the nested `rules` is always allocated.
	//! @param zone the zone being prepared
	//! @param rules the wire rules; may be null
	//! @param subject the zone's id or index, for the log
	//! @param report false for zone types this module does not enforce: defaults still apply, but
	//! nothing is logged
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
					TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' rules.graceSeconds=%2 is outside 0..%3 -- using the default %4 s",
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
					TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' rules.warnEverySeconds=%2 must be > 0 -- using the default %3 s",
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
				if (report)
					TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' rules.penalty=kill -- ONE LIFE: a player who stays in violation past %2 s is KILLED and can only return via '#tbd respawn'",
						subject, zone.m_fGraceSeconds));
			}
			else
			{
				zone.m_ePenalty = TBD_EZonePenalty.WARN;
				if (report)
					TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1' rules.penalty='%2' is not one of none|warn|kill -- using 'warn' (never guessing toward kill under one life)",
						subject, rules.penalty));
			}
		}

		TBD_PlayAreaVehicleAxis.Bind(zone.m_sId, rules);
		if (rules.vehicleClasses && rules.vehicleClasses.Count() > 0)
			legible++;

		if (legible == 0 && report)
		{
			TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("zone '%1': no rule this build understands (graceSeconds, warnEverySeconds, penalty) was readable -- running on defaults grace=%2s warnEvery=%3s penalty=warn. Either none was authored, or one was authored under a key this build does not declare and therefore cannot see; a typed JSON parser cannot tell those apart.",
				subject, DEFAULT_GRACE_SECONDS, DEFAULT_WARN_EVERY_SECONDS));
		}
	}

	//! Whether the play area enforces this zone type, which decides whether its defects are logged.
	//! @param type a `zones[].type`
	//! @return true for `boundary` and `base_protection`
	static bool EnforcesType(string type)
	{
		return type == TBD_ZoneRegistry.TYPE_BOUNDARY || type == TBD_ZoneRegistry.TYPE_BASE_PROTECTION;
	}
}
