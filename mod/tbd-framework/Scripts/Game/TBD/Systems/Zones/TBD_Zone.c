/**
 * @file TBD_Zone.c
 * @brief One mission zone prepared for runtime tests: shape, bounds and play-area rules.
 *
 * Role: the flattened, validated form of a `zones[]` row and its containment test.
 * Position: built by `TBD_ZoneCompiler`; held by `TBD_ZoneRegistry`; tested by the play area, triggers,
 * objectives and `TBD_EntityQuery`.
 * State: the fields below, set once at build.  Invariants: `Contains` allocates nothing and counts
 * a point within `EDGE_MARGIN_M` of the edge as inside for both shapes; an unusable zone contains
 * nothing and callers filter on `IsUsable` before asking.
 */

//! Which of the schema's two `oneOf` shapes a zone carries.
enum TBD_EZoneShapeKind
{
	NONE,    //!< no usable shape: neither authored, or a degenerate polygon
	CIRCLE,  //!< `shape.circle`
	POLYGON  //!< `shape.polygon` with at least 3 vertices
}

//! What happens when a player stays in violation past the grace period. Events are one life, so
//! `KILL` removes a player from the event and must be authored; the default is `WARN`. The
//! members stay in ascending order of severity: `TBD_ZoneRegistry.GoverningBoundary` compares them
//! with `>`.
enum TBD_EZonePenalty
{
	NONE,   //!< Track and log server-side; say nothing to the player. For instrumenting a zone.
	WARN,   //!< Tell the player, keep telling them, never act. THE DEFAULT.
	KILL    //!< Terminal under one life. Routed through the engine's own kill, never a second path.
}

//! One prepared zone.
class TBD_Zone
{
	static const float EDGE_MARGIN_M = 1.0; //!< metres outside an edge that still count as inside, biased generous under one life

	string m_sId; //!< `zones[].id`
	string m_sType; //!< `zones[].type`: boundary, base_protection, spawn or objective_*
	string m_sLabel; //!< `zones[].label`; may be empty
	string m_sFaction; //!< `zones[].faction`; may be empty, meaning per zone type

	TBD_EZoneShapeKind m_eShape; //!< which shape the zone carries

	float m_fCx; //!< circle centre X in metres
	float m_fCz; //!< circle centre Z in metres
	float m_fR; //!< circle radius in metres

	ref array<float> m_aFlat; //!< polygon vertices flattened as x0, z0, x1, z1, ...; null for a circle

	float m_fMinX; //!< XZ bounding box minimum X in metres, for a cheap reject
	float m_fMinZ; //!< XZ bounding box minimum Z in metres
	float m_fMaxX; //!< XZ bounding box maximum X in metres
	float m_fMaxZ; //!< XZ bounding box maximum Z in metres

	float m_fGraceSeconds; //!< resolved `graceSeconds`; default 30
	float m_fWarnEverySeconds; //!< resolved `warnEverySeconds`; default 5
	TBD_EZonePenalty m_ePenalty; //!< resolved `penalty`; default WARN

	//! Whether this zone can answer a containment question; an INERT zone is skipped, never read
	//! as "outside".
	//! @return false for a zone with no usable shape
	bool IsUsable()
	{
		return m_eShape != TBD_EZoneShapeKind.NONE;
	}

	//! Whether this world XZ position is inside the zone, the edge band of `EDGE_MARGIN_M`
	//! included for both shapes. Height is ignored.
	//! @param px world X in metres
	//! @param pz world Z in metres
	//! @return true when inside; false for an unusable zone
	bool Contains(float px, float pz)
	{
		if (m_eShape == TBD_EZoneShapeKind.CIRCLE)
			return TBD_ZoneGeometry.IsPointInCircle(px, pz, m_fCx, m_fCz, m_fR, EDGE_MARGIN_M);

		if (m_eShape != TBD_EZoneShapeKind.POLYGON || !m_aFlat)
			return false;

		// Cheap reject on the margin-grown box, so the edge band is never rejected early.
		if (px < m_fMinX - EDGE_MARGIN_M || px > m_fMaxX + EDGE_MARGIN_M)
			return false;
		if (pz < m_fMinZ - EDGE_MARGIN_M || pz > m_fMaxZ + EDGE_MARGIN_M)
			return false;

		if (TBD_ZoneGeometry.IsPointInPolygon(px, pz, m_aFlat))
			return true;

		// Outside by the crossing test but within the margin of an edge: inside.
		return TBD_ZoneGeometry.DistanceToPolygonEdge(px, pz, m_aFlat) <= EDGE_MARGIN_M;
	}

	//! The name a player is shown.
	//! @return the label, else the id, else the type
	string DisplayName()
	{
		if (!m_sLabel.IsEmpty())
			return m_sLabel;
		if (!m_sId.IsEmpty())
			return m_sId;
		return m_sType;
	}

	//! Stable identifier for logs, built in steps (a long `+` chain is `Formula too complex`).
	//! @return `<type>:<id>`
	string LogKey()
	{
		string key = m_sType;
		key += ":";
		key += m_sId;
		return key;
	}
}
