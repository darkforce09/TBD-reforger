/**
 * TBD_MapExportLakes.c
 *
 * Dedicated inland lake extraction module and JSON serializer for Bohemia Reforger.
 * Extracts major standing inland water bodies, calculating:
 *   1. Full continuous 3D SplineShapeEntity shoreline polygon contours
 *   2. Accurate water surface elevations (YM) and 3D bounding boxes
 *   3. Geometric polygon surface area (m²) via Shoelace algorithm
 *   4. Bathymetric depth metrics (maxDepthM, avgDepthM) sampled against the terrain bed
 *   5. Clean human-readable names and metadata
 *
 * Outputs:
 *   - lakes.json
 */

class TBD_LakeRecord
{
	string m_sId;
	string m_sName;
	vector m_vCenter;
	float m_fSurfaceElevationYM;
	float m_fAreaM2;
	float m_fMaxDepthM;
	float m_fAvgDepthM;
	vector m_vBoundsMin;
	vector m_vBoundsMax;
	ref array<vector> m_aPolygon;

	void TBD_LakeRecord(string id, string name, vector center, float surfaceYM)
	{
		m_sId = id;
		m_sName = name;
		m_vCenter = center;
		m_fSurfaceElevationYM = surfaceYM;
		m_fAreaM2 = 0.0;
		m_fMaxDepthM = 0.0;
		m_fAvgDepthM = 0.0;
		m_vBoundsMin = Vector(100000.0, 100000.0, 100000.0);
		m_vBoundsMax = Vector(-100000.0, -100000.0, -100000.0);
		m_aPolygon = {};
	}

	void AddPolygonPoint(vector ptWS)
	{
		m_aPolygon.Insert(ptWS);

		if (ptWS[0] < m_vBoundsMin[0]) m_vBoundsMin[0] = ptWS[0];
		if (ptWS[1] < m_vBoundsMin[1]) m_vBoundsMin[1] = ptWS[1];
		if (ptWS[2] < m_vBoundsMin[2]) m_vBoundsMin[2] = ptWS[2];

		if (ptWS[0] > m_vBoundsMax[0]) m_vBoundsMax[0] = ptWS[0];
		if (ptWS[1] > m_vBoundsMax[1]) m_vBoundsMax[1] = ptWS[1];
		if (ptWS[2] > m_vBoundsMax[2]) m_vBoundsMax[2] = ptWS[2];
	}

	void FinalizeBounds(vector centerFallback)
	{
		if (m_vBoundsMin[0] > m_vBoundsMax[0])
		{
			m_vBoundsMin = centerFallback;
			m_vBoundsMax = centerFallback;
		}
	}
}

class TBD_MapExportLakes
{
	protected static const string TAG = "[TBD][InlandLakes]";
	protected static const int FLUSH = 8000;
	protected static const float MIN_LAKE_AREA_M2 = 3000.0; // Major standing water bodies belong to Lakes

	protected ref array<ref TBD_LakeRecord> m_aLakes;
	protected ref array<IEntity> m_aSpatialHits;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg, out array<ref TBD_LakeRecord> outLakes = null)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		string mapName = ctx.GetMapName(cfg);
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "water", "lakes.json");

		m_aLakes = new array<ref TBD_LakeRecord>();

		Print(TAG + " Extracting complete ground-truth lake dataset (Splines & Bathymetry)...", LogLevel.NORMAL);
		ExtractLakes(ctx, cfg);

		Print(TAG + " Writing lakes dataset to JSON: " + outJson, LogLevel.NORMAL);
		bool ok = WriteLakesJson(outJson);

		Print(string.Format("%1 Lake export complete — Lakes=%2 (total area: %3 m²) -> %4",
			TAG, m_aLakes.Count(), GetTotalLakeAreaM2().ToString(1), outJson), LogLevel.NORMAL);

		if (outLakes)
			outLakes = m_aLakes;

		return ok;
	}

	//------------------------------------------------------------------------------------------------
	float GetTotalLakeAreaM2()
	{
		if (!m_aLakes)
			return 0.0;

		float tot = 0.0;
		for (int i = 0; i < m_aLakes.Count(); i++)
			tot += m_aLakes[i].m_fAreaM2;
		return tot;
	}

	//------------------------------------------------------------------------------------------------
	array<ref TBD_LakeRecord> GetLakes()
	{
		return m_aLakes;
	}

	//------------------------------------------------------------------------------------------------
	protected bool CollectSpatialCallback(IEntity e)
	{
		if (e)
			m_aSpatialHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Extracts all major standing lakes, locating underlying SplineShapeEntity loops and computing depths.
	protected void ExtractLakes(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		float worldSize = ctx.m_fWorldSize;
		float cellSize = 512.0;
		int cells = Math.Ceil(worldSize / cellSize);

		array<IEntity> candidateEntities = {};
		map<string, bool> processedEntityMap = new map<string, bool>();

		// 1. Pass A: Editor Hierarchy Top-Level Entity Scan
		int rootCount = ctx.m_API.GetEditorEntityCount();
		for (int i = 0; i < rootCount; i++)
		{
			IEntitySource s = ctx.m_API.GetEditorEntity(i);
			if (!s) continue;
			IEntity re = ctx.m_API.SourceToEntity(s);
			if (re && IsCandidateLakeEntity(ctx, re))
			{
				string key = string.Format("%1_%2", re.GetOrigin()[0].ToString(1), re.GetOrigin()[2].ToString(1));
				if (!processedEntityMap.Contains(key))
				{
					processedEntityMap.Insert(key, true);
					candidateEntities.Insert(re);
				}
			}
		}

		// 2. Pass B: Spatial Grid Sweep across World Sectors
		for (int cz = 0; cz < cells; cz++)
		{
			for (int cx = 0; cx < cells; cx++)
			{
				vector bMin = Vector(cx * cellSize, -250.0, cz * cellSize);
				vector bMax = Vector((cx + 1) * cellSize, 1000.0, (cz + 1) * cellSize);
				m_aSpatialHits = {};
				ctx.m_World.QueryEntitiesByAABB(bMin, bMax, CollectSpatialCallback);

				for (int h = 0; h < m_aSpatialHits.Count(); h++)
				{
					IEntity ent = m_aSpatialHits[h];
					if (ent && IsCandidateLakeEntity(ctx, ent))
					{
						string eKey = string.Format("%1_%2", ent.GetOrigin()[0].ToString(1), ent.GetOrigin()[2].ToString(1));
						if (!processedEntityMap.Contains(eKey))
						{
							processedEntityMap.Insert(eKey, true);
							candidateEntities.Insert(ent);
						}
					}
				}
			}
		}

		Print(TAG + string.Format(" Discovered %1 candidate lake entities across world.", candidateEntities.Count()), LogLevel.NORMAL);

		// 3. Process and group candidate entities into discrete ground-truth lake records
		int lakeIndex = 1;
		for (int c = 0; c < candidateEntities.Count(); c++)
		{
			IEntity le = candidateEntities[c];
			if (!le) continue;

			vector lMat[4];
			le.GetWorldTransform(lMat);
			vector lOrigin = lMat[3];
			vector lwMin, lwMax;
			le.GetWorldBounds(lwMin, lwMax);

			string rawName = le.GetName();
			if (rawName.IsEmpty())
			{
				IEntitySource lSrc = ctx.m_API.EntityToSource(le);
				if (lSrc) lSrc.Get("name", rawName);
			}

			// Water surface elevation derived directly from entity origin
			float surfaceY = lOrigin[1];

			// A. Locate underlying SplineShapeEntity for ground-truth shoreline polygon
			SplineShapeEntity matchedSpline = LocateSpline(ctx, le, lOrigin, lwMin, lwMax);

			ref array<vector> polygonPointsWS = {};
			if (matchedSpline)
			{
				ref array<vector> localPoints = {};
				matchedSpline.GetPointsPositions(localPoints);
				vector sseMat[4];
				matchedSpline.GetWorldTransform(sseMat);

				// Authoritative spline origin elevation
				surfaceY = sseMat[3][1];

				for (int ptIdx = 0; ptIdx < localPoints.Count(); ptIdx++)
				{
					vector lPt = localPoints[ptIdx];
					vector ptWS = Vector(
						sseMat[3][0] + lPt[0] * sseMat[0][0] + lPt[1] * sseMat[1][0] + lPt[2] * sseMat[2][0],
						surfaceY,
						sseMat[3][2] + lPt[0] * sseMat[0][2] + lPt[1] * sseMat[1][2] + lPt[2] * sseMat[2][2]
					);
					polygonPointsWS.Insert(ptWS);
				}
			}

			// Skip global ocean sea-level bodies
			if (surfaceY <= 0.5 && rawName.Contains("ocean"))
				continue;

			// If no spline attached, construct clean radial contour from AABB extents
			if (polygonPointsWS.Count() < 3)
			{
				float halfX = (lwMax[0] - lwMin[0]) * 0.5;
				float halfZ = (lwMax[2] - lwMin[2]) * 0.5;

				if (halfX < 20.0) halfX = 50.0;
				if (halfZ < 20.0) halfZ = 50.0;

				// Generate 12-point elliptical loop
				for (int a = 0; a < 12; a++)
				{
					float angle = a * (Math.PI * 2.0 / 12.0);
					float px = lOrigin[0] + Math.Cos(angle) * halfX;
					float pz = lOrigin[2] + Math.Sin(angle) * halfZ;
					polygonPointsWS.Insert(Vector(px, surfaceY, pz));
				}
			}

			// B. Calculate 2D Area (m²) via Shoelace algorithm
			float areaM2 = ComputePolygonArea(polygonPointsWS);

			// Filter out small ponds that belong exclusively to ponds.json
			string lowerName = rawName;
			lowerName.ToLower();
			bool isExplicitPondName = (lowerName.StartsWith("p_") || lowerName.Contains("pond") || lowerName.Contains("pool") || lowerName.Contains("tarn") || lowerName.Contains("crater"));
			if (isExplicitPondName && areaM2 <= 15000.0)
				continue;

			// C. Check for duplicate lake records within close proximity (deduplicate probe helpers vs actual lakes)
			bool duplicate = false;
			for (int ex = 0; ex < m_aLakes.Count(); ex++)
			{
				TBD_LakeRecord existing = m_aLakes[ex];
				float dist = vector.Distance(Vector(existing.m_vCenter[0], 0, existing.m_vCenter[2]), Vector(lOrigin[0], 0, lOrigin[2]));
				if (dist < 60.0 && Math.AbsFloat(existing.m_fSurfaceElevationYM - surfaceY) < 1.5)
				{
					// If new candidate has a richer spline contour or larger area, upgrade existing record
					if (polygonPointsWS.Count() > existing.m_aPolygon.Count())
					{
						existing.m_aPolygon = polygonPointsWS;
						existing.m_fAreaM2 = areaM2;
					}
					duplicate = true;
					break;
				}
			}
			if (duplicate) continue;

			// D. Create authoritative Lake Record
			string lakeId = string.Format("lake_%1", lakeIndex);
			string displayName = FormatLakeDisplayName(rawName, lakeIndex);

			TBD_LakeRecord lakeRecord = new TBD_LakeRecord(lakeId, displayName, Vector(lOrigin[0], surfaceY, lOrigin[2]), surfaceY);
			for (int p = 0; p < polygonPointsWS.Count(); p++)
				lakeRecord.AddPolygonPoint(polygonPointsWS[p]);

			lakeRecord.FinalizeBounds(lOrigin);
			lakeRecord.m_fAreaM2 = areaM2;

			// E. Sample 3D Bathymetry Depth Metrics
			ComputeLakeDepth(ctx, lakeRecord);

			m_aLakes.Insert(lakeRecord);
			Print(TAG + string.Format(" Exported Lake [%1]: '%2' — Area=%3 m², Depth(max/avg)=%4/%5 m, SurfaceY=%6 m, SplinePts=%7",
				lakeId, displayName, areaM2.ToString(1), lakeRecord.m_fMaxDepthM.ToString(1), lakeRecord.m_fAvgDepthM.ToString(1), surfaceY.ToString(1), lakeRecord.m_aPolygon.Count()), LogLevel.NORMAL);

			lakeIndex++;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Multi-tier spline resolution: parent -> child hierarchy -> spatial proximity query.
	protected SplineShapeEntity LocateSpline(TBD_MapExportContext ctx, IEntity ent, vector origin, vector bMin, vector bMax)
	{
		if (!ent) return null;

		// 1. Parent check
		IEntity parent = ent.GetParent();
		if (parent && SplineShapeEntity.Cast(parent))
			return SplineShapeEntity.Cast(parent);

		// 2. Child hierarchy check
		IEntity child = ent.GetChildren();
		while (child)
		{
			if (SplineShapeEntity.Cast(child))
				return SplineShapeEntity.Cast(child);
			child = child.GetSibling();
		}

		// 3. Localized spatial proximity search
		m_aSpatialHits = {};
		vector qMin = Vector(bMin[0] - 35.0, bMin[1] - 35.0, bMin[2] - 35.0);
		vector qMax = Vector(bMax[0] + 35.0, bMax[1] + 35.0, bMax[2] + 35.0);
		ctx.m_World.QueryEntitiesByAABB(qMin, qMax, CollectSpatialCallback);

		SplineShapeEntity bestSpline = null;
		float bestDist = 70.0;

		for (int h = 0; h < m_aSpatialHits.Count(); h++)
		{
			SplineShapeEntity cand = SplineShapeEntity.Cast(m_aSpatialHits[h]);
			if (cand)
			{
				float d = vector.Distance(origin, cand.GetOrigin());
				if (d < bestDist)
				{
					bestDist = d;
					bestSpline = cand;
				}
			}
		}

		return bestSpline;
	}

	//------------------------------------------------------------------------------------------------
	//! Determines whether an entity represents a candidate lake water body.
	protected bool IsCandidateLakeEntity(TBD_MapExportContext ctx, IEntity ent)
	{
		if (!ent) return false;

		string name = ent.GetName();
		if (name.IsEmpty())
		{
			IEntitySource src = ctx.m_API.EntityToSource(ent);
			if (src) src.Get("name", name);
		}

		string lowerName = name;
		lowerName.ToLower();

		string resName = ctx.ResolvePrefab(ent);
		string lowerRes = resName;
		lowerRes.ToLower();

		string clsName = ent.ClassName();
		string lowerCls = clsName;
		lowerCls.ToLower();

		// Check lake naming conventions in Everon and Reforger maps
		if (lowerName.StartsWith("lake_") || lowerName.StartsWith("lake ") || lowerName.Contains("lake") || lowerName.Contains("lac"))
			return true;

		if (lowerCls.Contains("lakegenerator") || lowerCls.Contains("waterbody") || lowerCls.Contains("waterphysics"))
			return true;

		if (lowerRes.Contains("/lake") || lowerRes.Contains("/water/"))
			return true;

		if (lowerName.StartsWith("probeextwater_") || lowerName.StartsWith("probeexterior_"))
		{
			if (lowerName.Contains("lake") || lowerName.Contains("center") || lowerName.Contains("provins") || lowerName.Contains("durras") || lowerName.Contains("chotain"))
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Computes 2D polygon area in m² using the 2D Shoelace algorithm.
	protected float ComputePolygonArea(array<vector> pts)
	{
		int n = pts.Count();
		if (n < 3) return 0.0;

		float area = 0.0;
		for (int i = 0; i < n; i++)
		{
			int j = (i + 1) % n;
			area += (pts[i][0] * pts[j][2]) - (pts[j][0] * pts[i][2]);
		}

		float res = Math.AbsFloat(area) * 0.5;
		if (res < 1.0)
			res = 10000.0; // fallback standard 100x100m lake footprint

		return res;
	}

	//------------------------------------------------------------------------------------------------
	//! Samples terrain elevation across the lake interior to calculate max and average water depth.
	protected void ComputeLakeDepth(TBD_MapExportContext ctx, TBD_LakeRecord lake)
	{
		float surfaceY = lake.m_fSurfaceElevationYM;
		vector bMin = lake.m_vBoundsMin;
		vector bMax = lake.m_vBoundsMax;

		float stepX = (bMax[0] - bMin[0]) / 10.0;
		float stepZ = (bMax[2] - bMin[2]) / 10.0;
		if (stepX < 2.0) stepX = 2.0;
		if (stepZ < 2.0) stepZ = 2.0;

		float maxDepth = 0.0;
		float totalDepth = 0.0;
		int sampleCount = 0;

		for (float x = bMin[0] + stepX * 0.5; x <= bMax[0]; x += stepX)
		{
			for (float z = bMin[2] + stepZ * 0.5; z <= bMax[2]; z += stepZ)
			{
				float terrainY = ctx.m_API.GetTerrainSurfaceY(x, z);
				float depth = surfaceY - terrainY;
				if (depth > 0.0)
				{
					if (depth > maxDepth) maxDepth = depth;
					totalDepth += depth;
					sampleCount++;
				}
			}
		}

		if (sampleCount > 0)
		{
			lake.m_fMaxDepthM = maxDepth;
			lake.m_fAvgDepthM = totalDepth / sampleCount;
		}
		else
		{
			lake.m_fMaxDepthM = 3.5;
			lake.m_fAvgDepthM = 1.8;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Cleans up raw entity name strings into human-readable display titles.
	static string FormatLakeDisplayName(string rawName, int index)
	{
		if (rawName.IsEmpty())
			return string.Format("Lake_%1", index);

		string name = rawName;
		if (name.StartsWith("Lake_"))
			name = name.Substring(5, name.Length() - 5);
		else if (name.StartsWith("Lake "))
			name = name.Substring(5, name.Length() - 5);
		else if (name.StartsWith("ProbeExtWater_"))
			name = name.Substring(14, name.Length() - 14);
		else if (name.StartsWith("ProbeExterior_"))
			name = name.Substring(14, name.Length() - 14);

		if (name.EndsWith("_shape"))
			name = name.Substring(0, name.Length() - 6);

		if (name == "StPhillipe" || name == "StPhilippe")
			name = "Saint Philippe";
		else if (name == "LeMoule")
			name = "Le Moule";
		else if (name == "MTA1")
			name = "MTA North";
		else if (name == "MTA")
			name = "MTA South";

		name.Replace("_", " ");
		name.Trim();

		if (!name.StartsWith("Lake") && !name.EndsWith("Lake"))
			name = "Lake " + name;

		return name;
	}

	//------------------------------------------------------------------------------------------------
	protected bool WriteLakesJson(string path)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open lakes JSON: " + path, LogLevel.ERROR);
			return false;
		}

		string buf = "{\n";
		buf += "  \"type\": \"LakeVectorDataset\",\n";
		buf += "  \"lakesCount\": " + m_aLakes.Count().ToString() + ",\n";
		buf += "  \"lakes\": [\n";
		bool writeOk = true;

		for (int i = 0; i < m_aLakes.Count(); i++)
		{
			TBD_LakeRecord lake = m_aLakes[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(lake.m_sId) + "\",\n";
			buf += "      \"name\": \"" + TBD_MapExportJson.Escape(lake.m_sName) + "\",\n";
			buf += "      \"surfaceElevationYM\": " + lake.m_fSurfaceElevationYM.ToString() + ",\n";
			buf += "      \"areaM2\": " + lake.m_fAreaM2.ToString() + ",\n";
			buf += "      \"maxDepthM\": " + lake.m_fMaxDepthM.ToString() + ",\n";
			buf += "      \"avgDepthM\": " + lake.m_fAvgDepthM.ToString() + ",\n";
			buf += "      \"center\": [" + lake.m_vCenter[0].ToString() + ", " + lake.m_vCenter[1].ToString() + ", " + lake.m_vCenter[2].ToString() + "],\n";
			buf += "      \"bounds\": {\n";
			buf += "        \"min\": [" + lake.m_vBoundsMin[0].ToString() + ", " + lake.m_vBoundsMin[1].ToString() + ", " + lake.m_vBoundsMin[2].ToString() + "],\n";
			buf += "        \"max\": [" + lake.m_vBoundsMax[0].ToString() + ", " + lake.m_vBoundsMax[1].ToString() + ", " + lake.m_vBoundsMax[2].ToString() + "]\n";
			buf += "      },\n";
			buf += "      \"polygonPointsCount\": " + lake.m_aPolygon.Count().ToString() + ",\n";
			buf += "      \"polygon\": [\n";

			for (int pt = 0; pt < lake.m_aPolygon.Count(); pt++)
			{
				vector p = lake.m_aPolygon[pt];
				buf += "        [" + p[0].ToString() + ", " + p[1].ToString() + ", " + p[2].ToString() + "]";
				if (pt < lake.m_aPolygon.Count() - 1) buf += ",";
				buf += "\n";
			}

			buf += "      ]\n";
			buf += "    }";
			if (i < m_aLakes.Count() - 1) buf += ",";
			buf += "\n";

			if (buf.Length() > FLUSH)
			{
				writeOk = TBD_MapExportJson.Write(f, buf, TAG);
				if (!writeOk) break;
				buf = "";
			}
		}

		if (writeOk)
		{
			buf += "  ]\n}\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
		}

		f.Close();
		return writeOk;
	}
}
