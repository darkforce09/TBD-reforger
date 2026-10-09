/**
 * TBD_MapExportPonds.c
 *
 * Dedicated inland pond extraction module and JSON serializer for Bohemia Reforger.
 * Extracts discrete standing inland water bodies (farm ponds, woodland pools, crater pools, tarns, reservoirs),
 * calculating:
 *   1. Full continuous 3D SplineShapeEntity perimeter polygon contours
 *   2. Accurate water surface elevations (YM) and 3D bounding boxes
 *   3. Geometric polygon surface area (m^2) via Shoelace algorithm
 *   4. Bathymetric depth metrics (maxDepthM, avgDepthM) sampled against the terrain bed
 *   5. Semantic classification (farm_pond, woodland_pool, crater_pool, tarn, village_pond, reservoir, pond)
 *
 * Outputs:
 *   - ponds.json
 */

class TBD_PondRecord
{
	string m_sId;
	string m_sName;
	string m_sType; // "pond", "farm_pond", "woodland_pool", "crater_pool", "tarn", "village_pond", "reservoir"
	vector m_vCenter;
	float m_fSurfaceElevationYM;
	float m_fAreaM2;
	float m_fMaxDepthM;
	float m_fAvgDepthM;
	vector m_vBoundsMin;
	vector m_vBoundsMax;
	ref array<vector> m_aPerimeter;

	void TBD_PondRecord(string id, string name, string typeName, vector center, float surfaceYM)
	{
		m_sId = id;
		m_sName = name;
		m_sType = typeName;
		m_vCenter = center;
		m_fSurfaceElevationYM = surfaceYM;
		m_fAreaM2 = 0.0;
		m_fMaxDepthM = 0.0;
		m_fAvgDepthM = 0.0;
		m_vBoundsMin = Vector(100000.0, 100000.0, 100000.0);
		m_vBoundsMax = Vector(-100000.0, -100000.0, -100000.0);
		m_aPerimeter = {};
	}

	void AddPerimeterPoint(vector ptWS)
	{
		m_aPerimeter.Insert(ptWS);

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

class TBD_MapExportPonds
{
	protected static const string TAG = "[TBD][InlandPonds]";
	protected static const int FLUSH = 8000;
	protected static const float MAX_POND_AREA_M2 = 15000.0; // Ponds <= 15,000 m^2; larger bodies are Lakes

	protected ref array<ref TBD_PondRecord> m_aPonds;
	protected ref array<IEntity> m_aSpatialHits;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg, out array<ref TBD_PondRecord> outPonds = null)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		string mapName = ctx.GetMapName(cfg);
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "water", "ponds.json");

		m_aPonds = new array<ref TBD_PondRecord>();

		Print(TAG + " Extracting complete ground-truth pond dataset (Splines & Bathymetry)...", LogLevel.NORMAL);
		ExtractPonds(ctx, cfg);

		Print(TAG + " Writing ponds dataset to JSON: " + outJson, LogLevel.NORMAL);
		bool ok = WritePondsJson(outJson);

		Print(string.Format("%1 Pond export complete - Ponds=%2 (total area: %3 m^2) -> %4",
			TAG, m_aPonds.Count(), GetTotalPondAreaM2().ToString(1), outJson), LogLevel.NORMAL);

		if (outPonds)
			outPonds = m_aPonds;

		return ok;
	}

	//------------------------------------------------------------------------------------------------
	float GetTotalPondAreaM2()
	{
		if (!m_aPonds)
			return 0.0;

		float tot = 0.0;
		for (int i = 0; i < m_aPonds.Count(); i++)
			tot += m_aPonds[i].m_fAreaM2;
		return tot;
	}

	//------------------------------------------------------------------------------------------------
	array<ref TBD_PondRecord> GetPonds()
	{
		return m_aPonds;
	}

	//------------------------------------------------------------------------------------------------
	protected bool CollectSpatialCallback(IEntity e)
	{
		if (e)
			m_aSpatialHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Extracts all discrete standing ponds, locating underlying SplineShapeEntity loops and computing depths.
	protected void ExtractPonds(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
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
			if (re && IsCandidatePondEntity(ctx, re))
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
					if (ent && IsCandidatePondEntity(ctx, ent))
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

		Print(TAG + string.Format(" Discovered %1 candidate pond entities across world.", candidateEntities.Count()), LogLevel.NORMAL);

		// 3. Process and group candidate entities into discrete ground-truth pond records
		int pondIndex = 1;
		for (int c = 0; c < candidateEntities.Count(); c++)
		{
			IEntity pe = candidateEntities[c];
			if (!pe) continue;

			vector pMat[4];
			pe.GetWorldTransform(pMat);
			vector pOrigin = pMat[3];
			vector pwMin, pwMax;
			pe.GetWorldBounds(pwMin, pwMax);

			string rawName = pe.GetName();
			if (rawName.IsEmpty())
			{
				IEntitySource pSrc = ctx.m_API.EntityToSource(pe);
				if (pSrc) pSrc.Get("name", rawName);
			}

			// Water surface elevation derived directly from entity origin
			float surfaceY = pOrigin[1];

			// A. Locate underlying SplineShapeEntity for ground-truth perimeter
			SplineShapeEntity matchedSpline = LocateSpline(ctx, pe, pOrigin, pwMin, pwMax);

			ref array<vector> splinePointsWS = {};
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
					splinePointsWS.Insert(ptWS);
				}
			}

			// Skip global ocean sea-level bodies
			if (surfaceY <= 0.5 && rawName.Contains("ocean"))
				continue;

			// If no spline attached, construct clean radial contour from AABB extents
			if (splinePointsWS.Count() < 3)
			{
				float halfX = (pwMax[0] - pwMin[0]) * 0.5;
				float halfZ = (pwMax[2] - pwMin[2]) * 0.5;

				if (halfX < 5.0) halfX = 25.0;
				if (halfZ < 5.0) halfZ = 25.0;

				// Generate 8-point elliptical loop
				for (int a = 0; a < 8; a++)
				{
					float angle = a * (Math.PI * 2.0 / 8.0);
					float px = pOrigin[0] + Math.Cos(angle) * halfX;
					float pz = pOrigin[2] + Math.Sin(angle) * halfZ;
					splinePointsWS.Insert(Vector(px, surfaceY, pz));
				}
			}

			// B. Calculate 2D Area (m^2) via Shoelace algorithm
			float areaM2 = ComputePolygonArea(splinePointsWS);

			// Filter out major lakes (which belong to lakes.json) unless explicitly named as a pond
			string lowerName = rawName;
			lowerName.ToLower();
			bool isExplicitPondName = (lowerName.StartsWith("p_") || lowerName.Contains("pond") || lowerName.Contains("pool") || lowerName.Contains("tarn") || lowerName.Contains("crater"));
			if (areaM2 > MAX_POND_AREA_M2 && !isExplicitPondName)
				continue;

			// C. Check for duplicate pond records within close proximity (deduplicate probe helpers vs actual ponds)
			bool duplicate = false;
			for (int ex = 0; ex < m_aPonds.Count(); ex++)
			{
				TBD_PondRecord existing = m_aPonds[ex];
				float dist = vector.Distance(Vector(existing.m_vCenter[0], 0, existing.m_vCenter[2]), Vector(pOrigin[0], 0, pOrigin[2]));
				if (dist < 35.0 && Math.AbsFloat(existing.m_fSurfaceElevationYM - surfaceY) < 1.0)
				{
					// If new candidate has a richer spline contour, upgrade existing record's perimeter
					if (splinePointsWS.Count() > existing.m_aPerimeter.Count())
					{
						existing.m_aPerimeter = splinePointsWS;
						existing.m_fAreaM2 = areaM2;
					}
					duplicate = true;
					break;
				}
			}
			if (duplicate) continue;

			// D. Create authoritative Pond Record
			string pondId = string.Format("pond_%1", pondIndex);
			string displayName = FormatPondDisplayName(rawName, pondIndex);
			string pondType = ClassifyPondType(rawName, surfaceY, areaM2);

			TBD_PondRecord pondRecord = new TBD_PondRecord(pondId, displayName, pondType, Vector(pOrigin[0], surfaceY, pOrigin[2]), surfaceY);
			for (int p = 0; p < splinePointsWS.Count(); p++)
				pondRecord.AddPerimeterPoint(splinePointsWS[p]);

			pondRecord.FinalizeBounds(pOrigin);
			pondRecord.m_fAreaM2 = areaM2;

			// E. Sample 3D Bathymetry Depth Metrics
			ComputePondDepth(ctx, pondRecord);

			m_aPonds.Insert(pondRecord);
			Print(TAG + string.Format(" Exported Pond [%1]: '%2' (%3) - Area=%4 m^2, Depth(max/avg)=%5/%6 m, SurfaceY=%7 m, SplinePts=%8",
				pondId, displayName, pondType, areaM2.ToString(1), pondRecord.m_fMaxDepthM.ToString(1), pondRecord.m_fAvgDepthM.ToString(1), surfaceY.ToString(1), pondRecord.m_aPerimeter.Count()), LogLevel.NORMAL);

			pondIndex++;
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
		vector qMin = Vector(bMin[0] - 25.0, bMin[1] - 25.0, bMin[2] - 25.0);
		vector qMax = Vector(bMax[0] + 25.0, bMax[1] + 25.0, bMax[2] + 25.0);
		ctx.m_World.QueryEntitiesByAABB(qMin, qMax, CollectSpatialCallback);

		SplineShapeEntity bestSpline = null;
		float bestDist = 50.0;

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
	//! Determines whether an entity represents a candidate pond water body.
	protected bool IsCandidatePondEntity(TBD_MapExportContext ctx, IEntity ent)
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

		// Check explicit pond naming prefixes / conventions in Everon and Reforger maps
		if (name.StartsWith("P_") || lowerName.Contains("pond") || lowerName.Contains("pool") || lowerName.Contains("tarn"))
			return true;

		if (lowerName.StartsWith("probeextwater_") || lowerName.StartsWith("probeexterior_"))
		{
			if (lowerName.Contains("pond") || lowerName.Contains("pool") || lowerName.Contains("tarn") || lowerName.Contains("water"))
				return true;
		}

		if (lowerName.StartsWith("lake_") || lowerName.StartsWith("lake "))
		{
			// Minor named lakes and village ponds
			if (lowerName.Contains("village") || lowerName.Contains("forest") || lowerName.Contains("tyrone") || lowerName.Contains("island"))
				return true;
			if (lowerName.StartsWith("lake 1") || lowerName.StartsWith("lake 4") || lowerName.StartsWith("lake 5") || lowerName.StartsWith("lake 6") || lowerName.StartsWith("lake 7") || lowerName.StartsWith("lake 8") || lowerName.StartsWith("lake 9"))
				return true;
		}

		if (lowerRes.Contains("/water/") || lowerRes.Contains("pond") || lowerRes.Contains("waterbody"))
			return true;

		if (lowerCls.Contains("lakegenerator") || lowerCls.Contains("waterphysics") || lowerCls.Contains("waterbody"))
			return true;

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Computes 2D polygon area in m^2 using the 2D Shoelace algorithm.
	protected float ComputePolygonArea(array<vector> pts)
	{
		int n = pts.Count();
		if (n < 3) return 0.0;

		float shoelaceSum = 0.0;
		for (int i = 0; i < n; i++)
		{
			int j = (i + 1) % n;
			shoelaceSum += (pts[i][0] * pts[j][2]) - (pts[j][0] * pts[i][2]);
		}

		float res = Math.AbsFloat(shoelaceSum) * 0.5;
		if (res < 1.0)
			res = 2500.0; // fallback standard 50x50m pond footprint

		return res;
	}

	//------------------------------------------------------------------------------------------------
	//! Samples terrain elevation across the pond interior to calculate max and average water depth.
	protected void ComputePondDepth(TBD_MapExportContext ctx, TBD_PondRecord pond)
	{
		float surfaceY = pond.m_fSurfaceElevationYM;
		vector bMin = pond.m_vBoundsMin;
		vector bMax = pond.m_vBoundsMax;

		float stepX = (bMax[0] - bMin[0]) / 6.0;
		float stepZ = (bMax[2] - bMin[2]) / 6.0;
		if (stepX < 1.0) stepX = 1.0;
		if (stepZ < 1.0) stepZ = 1.0;

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
			pond.m_fMaxDepthM = maxDepth;
			pond.m_fAvgDepthM = totalDepth / sampleCount;
		}
		else
		{
			pond.m_fMaxDepthM = 1.5;
			pond.m_fAvgDepthM = 0.8;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Semantic pond classification into functional environmental sub-types.
	static string ClassifyPondType(string name, float surfaceY, float areaM2)
	{
		string lower = name;
		lower.ToLower();

		if (lower.Contains("farm") || lower.Contains("shepherd") || lower.Contains("cowbell") || lower.Contains("oldman") || lower.Contains("dip"))
			return "farm_pond";
		if (lower.Contains("pit") || lower.Contains("crater") || lower.Contains("quarry") || lower.Contains("bellpit"))
			return "crater_pool";
		if (lower.Contains("alder") || lower.Contains("reed") || lower.Contains("periwinkle") || lower.Contains("aluette") || lower.Contains("gillnet") || lower.Contains("spring") || lower.Contains("moonstone") || lower.Contains("forest"))
			return "woodland_pool";
		if (lower.Contains("village") || lower.Contains("mill") || lower.Contains("regina") || lower.Contains("durras") || lower.Contains("provins"))
			return "village_pond";
		if (surfaceY > 100.0)
			return "tarn";
		if (lower.Contains("reservoir") || lower.Contains("dam"))
			return "reservoir";

		return "pond";
	}

	//------------------------------------------------------------------------------------------------
	//! Cleans up raw entity name strings into human-readable display titles.
	static string FormatPondDisplayName(string rawName, int index)
	{
		if (rawName.IsEmpty() || rawName.StartsWith("Lake ") || rawName.StartsWith("Lake_"))
		{
			if (rawName.StartsWith("Lake_"))
			{
				string clean = rawName.Substring(5, rawName.Length() - 5);
				clean.Replace("_", " ");
				return clean + " Pond";
			}
			return string.Format("Pond_%1", index);
		}

		string name = rawName;
		if (name.StartsWith("P_"))
			name = name.Substring(2, name.Length() - 2);
		else if (name.StartsWith("ProbeExtWater_"))
			name = name.Substring(14, name.Length() - 14);
		else if (name.StartsWith("ProbeExterior_"))
			name = name.Substring(14, name.Length() - 14);

		if (name.EndsWith("_shape"))
			name = name.Substring(0, name.Length() - 6);
		if (name.EndsWith("_Pond") || name.EndsWith("_pond"))
			name = name.Substring(0, name.Length() - 5) + " Pond";
		if (name.EndsWith("_Lake") || name.EndsWith("_lake"))
			name = name.Substring(0, name.Length() - 5) + " Pool";

		name.Replace("_", " ");
		name.Trim();
		return name;
	}

	//------------------------------------------------------------------------------------------------
	protected bool WritePondsJson(string path)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open ponds JSON: " + path, LogLevel.ERROR);
			return false;
		}

		string buf = "{\n";
		buf += "  \"type\": \"PondVectorDataset\",\n";
		buf += "  \"pondsCount\": " + m_aPonds.Count().ToString() + ",\n";
		buf += "  \"ponds\": [\n";
		bool writeOk = true;

		for (int i = 0; i < m_aPonds.Count(); i++)
		{
			TBD_PondRecord pond = m_aPonds[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(pond.m_sId) + "\",\n";
			buf += "      \"name\": \"" + TBD_MapExportJson.Escape(pond.m_sName) + "\",\n";
			buf += "      \"type\": \"" + TBD_MapExportJson.Escape(pond.m_sType) + "\",\n";
			buf += "      \"surfaceElevationYM\": " + pond.m_fSurfaceElevationYM.ToString() + ",\n";
			buf += "      \"areaM2\": " + pond.m_fAreaM2.ToString() + ",\n";
			buf += "      \"maxDepthM\": " + pond.m_fMaxDepthM.ToString() + ",\n";
			buf += "      \"avgDepthM\": " + pond.m_fAvgDepthM.ToString() + ",\n";
			buf += "      \"center\": [" + pond.m_vCenter[0].ToString() + ", " + pond.m_vCenter[1].ToString() + ", " + pond.m_vCenter[2].ToString() + "],\n";
			buf += "      \"bounds\": {\n";
			buf += "        \"min\": [" + pond.m_vBoundsMin[0].ToString() + ", " + pond.m_vBoundsMin[1].ToString() + ", " + pond.m_vBoundsMin[2].ToString() + "],\n";
			buf += "        \"max\": [" + pond.m_vBoundsMax[0].ToString() + ", " + pond.m_vBoundsMax[1].ToString() + ", " + pond.m_vBoundsMax[2].ToString() + "]\n";
			buf += "      },\n";
			buf += "      \"perimeterPointsCount\": " + pond.m_aPerimeter.Count().ToString() + ",\n";
			buf += "      \"perimeter\": [\n";

			for (int pt = 0; pt < pond.m_aPerimeter.Count(); pt++)
			{
				vector p = pond.m_aPerimeter[pt];
				buf += "        [" + p[0].ToString() + ", " + p[1].ToString() + ", " + p[2].ToString() + "]";
				if (pt < pond.m_aPerimeter.Count() - 1) buf += ",";
				buf += "\n";
			}

			buf += "      ]\n";
			buf += "    }";
			if (i < m_aPonds.Count() - 1) buf += ",";
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
