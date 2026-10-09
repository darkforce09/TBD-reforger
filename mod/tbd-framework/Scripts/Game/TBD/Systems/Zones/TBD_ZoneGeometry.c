/**
 * @file TBD_ZoneGeometry.c
 * @brief Pure 2D (world XZ) containment maths for circles and flat polygons.
 *
 * Role: point-in-circle, point-in-polygon, and point-to-edge distance over a flat, implicitly
 * closed `[x0, z0, x1, z1, ...]` ring.  Position: called by `TBD_Zone.Contains`, the trigger
 * snapshot and the objective code.
 * State: none; pure functions.  Invariants: Y is ignored; the crossing test is hand-rolled
 * rather than `Math2D.IsPointInPolygon`, whose edge and vertex behaviour is undocumented, and its
 * undefined on-edge case is resolved by `TBD_Zone`'s edge margin.
 */

//! XZ containment maths.
class TBD_ZoneGeometry
{
	//! Squared XZ distance, for callers that only compare.
	//! @return (ax - bx)^2 + (az - bz)^2 in square metres
	static float DistanceSqXZ(float ax, float az, float bx, float bz)
	{
		float dx = ax - bx;
		float dz = az - bz;
		return (dx * dx) + (dz * dz);
	}

	//! Point in circle, rim included, with `marginM` added to the radius.
	//! @return true when inside; false when the effective radius is not positive
	static bool IsPointInCircle(float px, float pz, float cx, float cz, float r, float marginM)
	{
		float effective = r + marginM;
		if (effective <= 0)
			return false;

		return DistanceSqXZ(px, pz, cx, cz) <= (effective * effective);
	}

	//! Crossing-number point-in-polygon: a ray in +X crosses an odd number of edges from inside.
	//! Needs no orientation or convexity assumption.
	//!
	//! * **Vertices.** The z-straddle test is `(zi > pz) != (zj > pz)` -- strictly greater on both
	//!   sides. That makes every edge HALF-OPEN in z: it owns its lower endpoint and not its upper
	//!   one. A ray passing exactly through a vertex therefore crosses exactly one of the two edges
	//!   meeting there, never zero and never two, so the classic "ray through a vertex counts
	//!   twice" bug cannot occur. This is the standard PNPOLY guarantee and it is the reason the
	//!   comparison is written this way rather than with `>=`.
	//! * **Horizontal edges.** An edge with `zi == zj` makes both sides of the straddle test equal,
	//!   so it is skipped. That is also what makes the division below safe: the divisor `zj - zi`
	//!   is provably non-zero on every line that reaches it.
	//! * **A point exactly ON an edge is UNDEFINED here** and may report either way -- that is
	//!   inherent to a crossing-number test, not an oversight. It is resolved one level up:
	//!   `TBD_Zone.Contains` also accepts anything within `marginM` of an edge, which turns the
	//!   ambiguous band into a deterministically-inside band. Callers that want the raw predicate
	//!   can still have it.
	//! * **Self-intersecting rings** follow the even-odd rule (a doubly-enclosed lobe reads as
	//!   outside). Nothing rejects such a ring; the mission author owns that.
	//!
	//! @param flat closed ring as x0,z0,x1,z1,... -- must hold at least 3 vertices (6 floats).
	static bool IsPointInPolygon(float px, float pz, notnull array<float> flat)
	{
		int count = flat.Count();
		int vertices = count / 2;
		// Fewer than 3 vertices is not a polygon. Refuse rather than guess -- a degenerate ring
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

	//! Shortest XZ distance from a point to the segment ab, for the inclusive edge band.
	//! @return the distance in metres
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

	//! Shortest XZ distance from a point to the polygon's outline, not its interior; combine with
	//! `IsPointInPolygon`, never decide containment with it alone.
	//! @return the distance in metres; `float.MAX` for a ring with fewer than 2 vertices
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
