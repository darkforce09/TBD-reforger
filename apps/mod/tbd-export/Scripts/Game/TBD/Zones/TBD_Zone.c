//! T-181.18 - a mission zone PREPARED for use at runtime, and the vocabulary that goes with it.
//!
//! The mission document's `TBD_MissionZoneStruct` is the wire shape: nullable, nested, and shaped
//! for JSON. This is what the enforcer actually reads - flattened, validated once at load, with
//! its rules resolved and its bounding box precomputed. Building it is a one-off; asking it a
//! question is hot-path (once per player per tick), so nothing here allocates or re-parses.

//------------------------------------------------------------------------------------------------
//! Which of the schema's two `oneOf` shapes a zone actually carries. `NONE` is not an error state
//! in itself - it is what a zone that authored neither (or authored a degenerate polygon) resolves
//! to, and it is why `TBD_Zone.Contains` can always answer without a null check at the call site.
enum TBD_EZoneShapeKind
{
	NONE,
	CIRCLE,
	POLYGON
}

//------------------------------------------------------------------------------------------------
//! What happens when a player stays in violation past the grace period.
//!
//! == THE ONE-LIFE DECISION, IN THE PLACE IT IS MADE ==========================================
//! TBD events are ONE LIFE: death is terminal (TBD_MOD_DESIGN.md S2). "Kill the player for leaving
//! the AO" therefore does not mean "teleport them back with a slap" - it means **permanent removal
//! from the event**, recoverable only by an admin `#tbd respawn`. That is a big enough consequence
//! that it must be an authored choice, never an inherited one.
//!
//! So the DEFAULT IS `WARN` and it is deliberate: a mission that says nothing about `penalty` gets
//! an AO that nags, logs, and never ends anybody's night. `KILL` exists, is fully implemented, and
//! is one JSON key away (`zones[].rules.penalty = "kill"`) for an operator who wants a hard AO.
//! Flip that default only with the operator's own words on the record.
//!
//! **DECLARATION ORDER IS LOAD-BEARING.** `TBD_ZoneRegistry.GoverningBoundary` picks the strictest
//! of several overlapping boundary zones by comparing these values with `>`, so the members must
//! stay in ascending order of severity. Reordering them silently inverts which zone's rules win.
enum TBD_EZonePenalty
{
	NONE,   //!< Track and log server-side; say nothing to the player. For instrumenting a zone.
	WARN,   //!< Tell the player, keep telling them, never act. THE DEFAULT.
	KILL    //!< Terminal under one life. Routed through the engine's own kill, never a second path.
}

//------------------------------------------------------------------------------------------------
//! One prepared zone.
class TBD_Zone
{
	//! How far outside a boundary a player may be and still count as inside, in metres.
	//!
	//! This is what turns the crossing-number test's genuinely-undefined "exactly on the edge" case
	//! into a defined one, and it is biased OUTWARD on purpose: the failure mode of being too
	//! generous is a player who gets away with standing 1 m outside the AO, and the failure mode of
	//! being too strict is a player killed for standing on the line. Under one life those are not
	//! comparable. Deliberately larger than any plausible float error at Everon's 12.8 km extent
	//! and far smaller than any meaningful tactical distance.
	static const float EDGE_MARGIN_M = 1.0;

	string m_sId;
	string m_sType;      //!< Raw schema enum value: boundary | base_protection | spawn | objective_*.
	string m_sLabel;     //!< May be empty - the schema does not require it.
	string m_sFaction;   //!< May be empty - the schema does not require it. Meaning is per zone type.

	TBD_EZoneShapeKind m_eShape;

	// Circle.
	float m_fCx;
	float m_fCz;
	float m_fR;

	// Polygon: flat x0,z0,x1,z1,... (see TBD_ZoneGeometry).
	ref array<float> m_aFlat;

	// Axis-aligned bounds, in world XZ. Precomputed so the common case (a player nowhere near this
	// zone) costs four float compares instead of a walk over every edge. Valid for both shapes.
	float m_fMinX;
	float m_fMinZ;
	float m_fMaxX;
	float m_fMaxZ;

	// Resolved rules - never sentinels, never null. See TBD_ZoneRegistry.ResolveRules.
	float m_fGraceSeconds;
	float m_fWarnEverySeconds;
	TBD_EZonePenalty m_ePenalty;

	//! True when this zone can answer a containment question at all. A zone that authored no shape,
	//! or a polygon with fewer than 3 usable vertices, is INERT: it never contains anything and the
	//! enforcer skips it entirely rather than treating "cannot tell" as "outside" and confining
	//! everyone. The registry reports it by id when it builds.
	bool IsUsable()
	{
		return m_eShape != TBD_EZoneShapeKind.NONE;
	}

	//------------------------------------------------------------------------------------------------
	//! Is this world XZ position inside the zone?
	//!
	//! Inclusive of the boundary within `EDGE_MARGIN_M` for BOTH shapes, so the two never disagree
	//! about what "on the line" means. An unusable zone answers `false` - see `IsUsable`; callers
	//! must not read that as "outside the play area", which is why the enforcer filters on
	//! `IsUsable()` before it ever asks.
	bool Contains(float px, float pz)
	{
		if (m_eShape == TBD_EZoneShapeKind.CIRCLE)
			return TBD_ZoneGeometry.IsPointInCircle(px, pz, m_fCx, m_fCz, m_fR, EDGE_MARGIN_M);

		if (m_eShape != TBD_EZoneShapeKind.POLYGON || !m_aFlat)
			return false;

		// Cheap reject first. The margin is added to the box so a point in the ambiguous edge band
		// is never thrown out before the edge-distance test below can accept it.
		if (px < m_fMinX - EDGE_MARGIN_M || px > m_fMaxX + EDGE_MARGIN_M)
			return false;
		if (pz < m_fMinZ - EDGE_MARGIN_M || pz > m_fMaxZ + EDGE_MARGIN_M)
			return false;

		if (TBD_ZoneGeometry.IsPointInPolygon(px, pz, m_aFlat))
			return true;

		// Outside by the crossing test, but within the margin of an edge: call it inside. This is
		// the branch that makes "standing exactly on the AO line" deterministic.
		return TBD_ZoneGeometry.DistanceToPolygonEdge(px, pz, m_aFlat) <= EDGE_MARGIN_M;
	}

	//------------------------------------------------------------------------------------------------
	//! What a human should be shown. The schema does not require `label`, so this falls back
	//! through id and then type rather than rendering an empty name in a warning a player has
	//! seconds to act on.
	string DisplayName()
	{
		if (!m_sLabel.IsEmpty())
			return m_sLabel;
		if (!m_sId.IsEmpty())
			return m_sId;
		return m_sType;
	}

	//------------------------------------------------------------------------------------------------
	//! Stable identifier for logs. Built in steps, not one long `+` chain: a 9-term concatenation
	//! is a measured `Formula too complex` in this compiler, whose SECOND diagnostic is a
	//! misleading `Incompatible parameter`.
	string LogKey()
	{
		string key = m_sType;
		key += ":";
		key += m_sId;
		return key;
	}
}
