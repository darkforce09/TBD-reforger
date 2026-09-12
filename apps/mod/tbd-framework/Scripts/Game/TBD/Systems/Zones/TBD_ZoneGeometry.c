//! T-181.18 — 2D (world XZ) containment maths for mission zones. Pure functions, no state, no
//! engine calls: everything here is decidable by reading it, which is the point. Y is ignored
//! throughout — a mission zone is a footprint on the map, and a player in a helicopter over the AO
//! is inside it.
//!
//! ── Why this is hand-rolled when the engine ships one ───────────────────────────────────────
//! `Math2D.IsPointInPolygon(array<float> poly, float x, float y)` EXISTS — proved by compile probe
//! against this exact engine build, with a failing negative control
//! (`Math2D.ZZ_IsPointInPolygonDoesNotExist` -> `Undefined function`). It is used by other
//! frameworks. It is deliberately not used here, for one reason: its behaviour for a point exactly
//! ON an edge or ON a vertex is undocumented and unprovable from this lane, and under ONE LIFE the
//! on-the-line case is the case that matters — it is the difference between "warned" and "removed
//! from the event". A hand-rolled test whose edge behaviour is written down and biased outward by
//! an explicit margin is worth more than a shorter call whose behaviour we would be guessing at.
//! If a later slice ever wants to swap it in, the seam is `TBD_Zone.Contains` and nothing else.
//!
//! ── Polygon representation ──────────────────────────────────────────────────────────────────
//! A FLAT `array<float>` of `[x0, z0, x1, z1, …]`, implicitly closed (the last vertex joins the
//! first; no duplicated closing vertex). Flat rather than nested because every routine below walks
//! it index-wise and a flat array halves the allocations; `TBD_ZoneRegistry` does the one-time
//! conversion out of the mission document's `[[x,z],…]`.
class TBD_ZoneGeometry
{
	//------------------------------------------------------------------------------------------------
	//! Squared XZ distance. Squared so the callers that only compare never pay for a Sqrt.
	static float DistanceSqXZ(float ax, float az, float bx, float bz)
	{
		float dx = ax - bx;
		float dz = az - bz;
		return (dx * dx) + (dz * dz);
	}

	//------------------------------------------------------------------------------------------------
	//! Point in circle, inclusive of the rim, with `marginM` added to the radius.
	//!
	//! A positive margin makes the boundary generous: a player standing exactly on the rim is
	//! INSIDE. That direction is chosen deliberately and everywhere in this file — see
	//! `TBD_Zone.EDGE_MARGIN_M`.
	static bool IsPointInCircle(float px, float pz, float cx, float cz, float r, float marginM)
	{
		float effective = r + marginM;
		if (effective <= 0)
			return false;

		return DistanceSqXZ(px, pz, cx, cz) <= (effective * effective);
	}

	//------------------------------------------------------------------------------------------------
	//! Crossing-number (ray casting) point-in-polygon over a flat `[x,z,…]` ring.
	//!
	//! ── The algorithm and why it is this one ────────────────────────────────────────────────
	//! Cast a ray from the point in +X and count how many edges it crosses; odd = inside. Chosen
	//! over the winding-number test because it needs no trigonometry, no orientation assumption
	//! (a clockwise and a counter-clockwise ring give the same answer, and the website makes no
	//! promise about winding order) and no branch on convexity.
	//!
	//! ── Vertex and edge cases, spelled out ──────────────────────────────────────────────────
	//! * **Vertices.** The z-straddle test is `(zi > pz) != (zj > pz)` — strictly greater on both
	//!   sides. That makes every edge HALF-OPEN in z: it owns its lower endpoint and not its upper
	//!   one. A ray passing exactly through a vertex therefore crosses exactly one of the two edges
	//!   meeting there, never zero and never two, so the classic "ray through a vertex counts
	//!   twice" bug cannot occur. This is the standard PNPOLY guarantee and it is the reason the
	//!   comparison is written this way rather than with `>=`.
	//! * **Horizontal edges.** An edge with `zi == zj` makes both sides of the straddle test equal,
	//!   so it is skipped. That is also what makes the division below safe: the divisor `zj - zi`
	//!   is provably non-zero on every line that reaches it.
	//! * **A point exactly ON an edge is UNDEFINED here** and may report either way — that is
	//!   inherent to a crossing-number test, not an oversight. It is resolved one level up:
	//!   `TBD_Zone.Contains` also accepts anything within `marginM` of an edge, which turns the
	//!   ambiguous band into a deterministically-inside band. Callers that want the raw predicate
	//!   can still have it; callers deciding whether to end somebody's event must not.
	//! * **Self-intersecting rings** follow the even-odd rule (a doubly-enclosed lobe reads as
	//!   outside). Nothing rejects such a ring; the mission author owns that.
	//!
	//! @param flat closed ring as x0,z0,x1,z1,… — must hold at least 3 vertices (6 floats).
	static bool IsPointInPolygon(float px, float pz, notnull array<float> flat)
	{
		int count = flat.Count();
		int vertices = count / 2;
		// Fewer than 3 vertices is not a polygon. Refuse rather than guess — a degenerate ring
		// that quietly answered "inside" would switch a play area off without saying so.
		if (vertices < 3)
			return false;

		bool inside = false;
		int j = vertices - 1;
		for (int i = 0; i < vertices; i++)
		{
			float xi = flat[i * 2];
			float zi = flat[(i * 2) + 1];
			float xj = flat[j * 2];
			float zj = flat[(j * 2) + 1];

			if ((zi > pz) != (zj > pz))
			{
				// Safe: the straddle test above is false whenever zi == zj.
				float crossX = (((xj - xi) * (pz - zi)) / (zj - zi)) + xi;
				if (px < crossX)
					inside = !inside;
			}

			j = i;
		}

		return inside;
	}

	//------------------------------------------------------------------------------------------------
	//! Shortest XZ distance from a point to the segment ab. Used to build the inclusive edge band.
	static float DistanceToSegmentXZ(float px, float pz, float ax, float az, float bx, float bz)
	{
		float abx = bx - ax;
		float abz = bz - az;
		float lengthSq = (abx * abx) + (abz * abz);

		// Degenerate segment (a duplicated vertex) collapses to a point.
		if (lengthSq <= 0)
			return Math.Sqrt(DistanceSqXZ(px, pz, ax, az));

		// Projection parameter, clamped to the segment so the nearest point is never off the end.
		float t = (((px - ax) * abx) + ((pz - az) * abz)) / lengthSq;
		t = Math.Clamp(t, 0, 1);

		float qx = ax + (t * abx);
		float qz = az + (t * abz);
		return Math.Sqrt(DistanceSqXZ(px, pz, qx, qz));
	}

	//------------------------------------------------------------------------------------------------
	//! Shortest XZ distance from a point to the polygon's OUTLINE (not its interior): a point deep
	//! inside a large ring is far from the outline, exactly like a point far outside it. Callers
	//! combine this with `IsPointInPolygon` — never use it alone to decide containment.
	//!
	//! Returns a large positive number for a degenerate ring, so a caller comparing against a small
	//! margin treats it as "not near an edge" rather than accidentally as "on the edge".
	static float DistanceToPolygonEdge(float px, float pz, notnull array<float> flat)
	{
		int vertices = flat.Count() / 2;
		if (vertices < 2)
			return float.MAX;

		float best = float.MAX;
		int j = vertices - 1;
		for (int i = 0; i < vertices; i++)
		{
			float d = DistanceToSegmentXZ(px, pz,
				flat[j * 2], flat[(j * 2) + 1],
				flat[i * 2], flat[(i * 2) + 1]);

			if (d < best)
				best = d;

			j = i;
		}

		return best;
	}
}
