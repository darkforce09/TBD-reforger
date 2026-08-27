/**
 * TBD_MapExportRivers.c
 *
 * Dedicated ground-truth river export engine for Bohemia Reforger (Everon).
 * Extracts all native RiverEntity instances with:
 *   1. Full continuous 3D SplineShapeEntity control points
 *   2. Complete RiverPartEntity subparts with 3D world bounds, local half-extents, lengths, and downhill flow sorting
 *   3. Native RiverEntity properties (Width, SplineOffsetUp, ReverseFlow, Material, Surface)
 *
 * Outputs:
 *   - rivers.json
 */

class TBD_RiverPartExport
{
	int m_iFlowIndex;
	vector m_vCenterWS;
	float m_fLengthM;
	float m_fWidthM;
	float m_fHeightM;
	float m_fMinYM;
	float m_fMaxYM;
	vector m_vHalfExtentsLocal;
	vector m_vBoundsWorldMin;
	vector m_vBoundsWorldMax;

	void TBD_RiverPartExport(int flowIdx, vector center, float lenM, float widM, float hgtM, float minY, float maxY, vector halfExt, vector bMin, vector bMax)
	{
		m_iFlowIndex = flowIdx;
		m_vCenterWS = center;
		m_fLengthM = lenM;
		m_fWidthM = widM;
		m_fHeightM = hgtM;
		m_fMinYM = minY;
		m_fMaxYM = maxY;
		m_vHalfExtentsLocal = halfExt;
		m_vBoundsWorldMin = bMin;
		m_vBoundsWorldMax = bMax;
	}
}

class TBD_RiverExport
{
	string m_sId;
	string m_sName;
	float m_fWidthM;
	float m_fSplineOffsetUpM;
	int m_iReverseFlow;
	string m_sMaterial;
	string m_sSurface;
	float m_fTotalLengthM;
	vector m_vBoundsMin;
	vector m_vBoundsMax;
	ref array<vector> m_aSplinePointsWS;
	ref array<ref TBD_RiverPartExport> m_aParts;

	void TBD_RiverExport(string id, string name, float widthM, float offsetUpM, int revFlow, string mat, string surf)
	{
		m_sId = id;
		m_sName = name;
		m_fWidthM = widthM;
		m_fSplineOffsetUpM = offsetUpM;
		m_iReverseFlow = revFlow;
		m_sMaterial = mat;
		m_sSurface = surf;
		m_fTotalLengthM = 0.0;
		m_vBoundsMin = Vector(100000, 100000, 100000);
		m_vBoundsMax = Vector(-100000, -100000, -100000);
		m_aSplinePointsWS = {};
		m_aParts = {};
	}

	void AddSplinePoint(vector ptWS)
	{
		if (m_aSplinePointsWS.Count() > 0)
		{
			vector prev = m_aSplinePointsWS[m_aSplinePointsWS.Count() - 1];
			m_fTotalLengthM += vector.Distance(prev, ptWS);
		}

		m_aSplinePointsWS.Insert(ptWS);

		if (ptWS[0] < m_vBoundsMin[0]) m_vBoundsMin[0] = ptWS[0];
		if (ptWS[1] < m_vBoundsMin[1]) m_vBoundsMin[1] = ptWS[1];
		if (ptWS[2] < m_vBoundsMin[2]) m_vBoundsMin[2] = ptWS[2];

		if (ptWS[0] > m_vBoundsMax[0]) m_vBoundsMax[0] = ptWS[0];
		if (ptWS[1] > m_vBoundsMax[1]) m_vBoundsMax[1] = ptWS[1];
		if (ptWS[2] > m_vBoundsMax[2]) m_vBoundsMax[2] = ptWS[2];
	}

	void AddPart(TBD_RiverPartExport part)
	{
		m_aParts.Insert(part);

		if (part.m_vBoundsWorldMin[0] < m_vBoundsMin[0]) m_vBoundsMin[0] = part.m_vBoundsWorldMin[0];
		if (part.m_vBoundsWorldMin[1] < m_vBoundsMin[1]) m_vBoundsMin[1] = part.m_vBoundsWorldMin[1];
		if (part.m_vBoundsWorldMin[2] < m_vBoundsMin[2]) m_vBoundsMin[2] = part.m_vBoundsWorldMin[2];

		if (part.m_vBoundsWorldMax[0] > m_vBoundsMax[0]) m_vBoundsMax[0] = part.m_vBoundsWorldMax[0];
		if (part.m_vBoundsWorldMax[1] > m_vBoundsMax[1]) m_vBoundsMax[1] = part.m_vBoundsWorldMax[1];
		if (part.m_vBoundsWorldMax[2] > m_vBoundsMax[2]) m_vBoundsMax[2] = part.m_vBoundsWorldMax[2];
	}
}

class TBD_MapExportRivers
{
	protected static const string TAG = "[TBD][InlandRivers]";
	protected static const int FLUSH = 8000;

	protected ref array<ref TBD_RiverExport> m_aRivers;
	protected ref array<IEntity> m_aSpatialHits;
	protected ref array<RiverPartEntity> m_aRiverPartsHit;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg, out array<ref TBD_RiverExport> outRivers = null)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		string mapName = ctx.GetMapName(cfg);
		float worldSize = ctx.m_fWorldSize;
		string outVectors = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "water", "rivers.json");

		m_aRivers = new array<ref TBD_RiverExport>();

		Print(TAG + " Extracting complete ground-truth river dataset (Splines & Subparts)...", LogLevel.NORMAL);
		ExportAllRivers(ctx, worldSize);

		Print(TAG + " Writing river vectors dataset to JSON: " + outVectors, LogLevel.NORMAL);
		bool ok = WriteVectorsJson(outVectors);

		int totalParts = 0;
		for (int i = 0; i < m_aRivers.Count(); i++)
			totalParts += m_aRivers[i].m_aParts.Count();

		Print(string.Format("%1 River export complete — Rivers=%2 (total length: %3m, total subparts: %4) -> %5",
			TAG, m_aRivers.Count(), GetTotalRiversLength().ToString(1), totalParts, outVectors), LogLevel.NORMAL);

		if (outRivers)
			outRivers = m_aRivers;

		return ok;
	}

	//------------------------------------------------------------------------------------------------
	float GetTotalRiversLength()
	{
		if (!m_aRivers)
			return 0.0;

		float tot = 0.0;
		for (int i = 0; i < m_aRivers.Count(); i++)
			tot += m_aRivers[i].m_fTotalLengthM;
		return tot;
	}

	//------------------------------------------------------------------------------------------------
	int GetTotalPartsCount()
	{
		if (!m_aRivers)
			return 0;

		int totalParts = 0;
		for (int i = 0; i < m_aRivers.Count(); i++)
			totalParts += m_aRivers[i].m_aParts.Count();
		return totalParts;
	}

	//------------------------------------------------------------------------------------------------
	array<ref TBD_RiverExport> GetRivers()
	{
		return m_aRivers;
	}

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntityCallback(IEntity e)
	{
		if (e)
			m_aSpatialHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected bool CollectRiverPartCallback(IEntity e)
	{
		RiverPartEntity part = RiverPartEntity.Cast(e);
		if (part)
			m_aRiverPartsHit.Insert(part);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void ExportAllRivers(TBD_MapExportContext ctx, float worldSize)
	{
		array<RiverEntity> allRiverEntities = {};
		RiverEntity.GetExistingInstances(allRiverEntities);

		Print(TAG + string.Format(" Found %1 RiverEntity instances loaded in world.", allRiverEntities.Count()), LogLevel.NORMAL);

		int riverIndex = 1;
		for (int r = 0; r < allRiverEntities.Count(); r++)
		{
			RiverEntity re = allRiverEntities[r];
			if (!re) continue;

			vector rMat[4];
			re.GetWorldTransform(rMat);
			vector rOrigin = rMat[3];
			vector rwMin, rwMax;
			re.GetWorldBounds(rwMin, rwMax);

			string rName = re.GetName();
			if (rName.IsEmpty())
				rName = string.Format("RiverEntity_%1", riverIndex);

			// A. Read River parameters from EntitySource container
			IEntitySource rSrc = ctx.m_API.EntityToSource(re);
			float widthM = 5.0;
			float offsetUpM = 0.2;
			int revFlow = 0;
			string mat = "";
			string surf = "";

			if (rSrc)
			{
				rSrc.Get("Width", widthM);
				rSrc.Get("SplineOffsetUp", offsetUpM);
				rSrc.Get("ReverseFlow", revFlow);
				rSrc.Get("Material", mat);
				rSrc.Get("Surface", surf);
			}

			if (widthM <= 0.0) widthM = 5.0;

			string riverId = string.Format("river_%1", riverIndex);
			TBD_RiverExport riverExport = new TBD_RiverExport(riverId, rName, widthM, offsetUpM, revFlow, mat, surf);

			// B. Locate and extract underlying SplineShapeEntity control points
			SplineShapeEntity matchedSpline = null;
			IEntity rParent = re.GetParent();
			if (rParent && SplineShapeEntity.Cast(rParent))
				matchedSpline = SplineShapeEntity.Cast(rParent);

			if (!matchedSpline)
			{
				IEntity rChild = re.GetChildren();
				while (rChild)
				{
					if (SplineShapeEntity.Cast(rChild))
					{
						matchedSpline = SplineShapeEntity.Cast(rChild);
						break;
					}
					rChild = rChild.GetSibling();
				}
			}

			if (!matchedSpline)
			{
				m_aSpatialHits = {};
				vector qMin = Vector(rwMin[0] - 15.0, rwMin[1] - 15.0, rwMin[2] - 15.0);
				vector qMax = Vector(rwMax[0] + 15.0, rwMax[1] + 15.0, rwMax[2] + 15.0);
				ctx.m_World.QueryEntitiesByAABB(qMin, qMax, CollectEntityCallback);

				float bestDist = 100.0;
				for (int h = 0; h < m_aSpatialHits.Count(); h++)
				{
					SplineShapeEntity candSpline = SplineShapeEntity.Cast(m_aSpatialHits[h]);
					if (candSpline)
					{
						float d = vector.Distance(rOrigin, candSpline.GetOrigin());
						if (d < bestDist)
						{
							bestDist = d;
							matchedSpline = candSpline;
						}
					}
				}
			}

			if (matchedSpline)
			{
				ref array<vector> localPoints = {};
				matchedSpline.GetPointsPositions(localPoints);
				vector sseMat[4];
				matchedSpline.GetWorldTransform(sseMat);
				vector sseOrigin = sseMat[3];

				for (int ptIdx = 0; ptIdx < localPoints.Count(); ptIdx++)
				{
					vector lPt = localPoints[ptIdx];
					vector ptWS = Vector(sseOrigin[0] + lPt[0], sseOrigin[1] + lPt[1], sseOrigin[2] + lPt[2]);
					riverExport.AddSplinePoint(ptWS);
				}
			}

			// C. Query and extract all RiverPartEntity subparts for this river
			m_aRiverPartsHit = {};
			vector partQMin = Vector(rwMin[0] - 15.0, rwMin[1] - 15.0, rwMin[2] - 15.0);
			vector partQMax = Vector(rwMax[0] + 15.0, rwMax[1] + 15.0, rwMax[2] + 15.0);
			ctx.m_World.QueryEntitiesByAABB(partQMin, partQMax, CollectRiverPartCallback);

			array<RiverPartEntity> uniqueParts = {};
			for (int pi = 0; pi < m_aRiverPartsHit.Count(); pi++)
			{
				RiverPartEntity candidatePart = m_aRiverPartsHit[pi];
				if (!candidatePart) continue;
				bool alreadyHave = false;
				for (int up = 0; up < uniqueParts.Count(); up++)
				{
					if (uniqueParts[up] == candidatePart)
					{
						alreadyHave = true;
						break;
					}
				}
				if (!alreadyHave)
					uniqueParts.Insert(candidatePart);
			}

			// Downhill Sort (descending center Y) for clean flow topology
			int nParts = uniqueParts.Count();
			for (int i = 0; i < nParts - 1; i++)
			{
				for (int j = 0; j < nParts - i - 1; j++)
				{
					vector pwbMinA, pwbMaxA, pwbMinB, pwbMaxB;
					uniqueParts[j].GetWorldBounds(pwbMinA, pwbMaxA);
					uniqueParts[j + 1].GetWorldBounds(pwbMinB, pwbMaxB);
					float yA = (pwbMinA[1] + pwbMaxA[1]) * 0.5;
					float yB = (pwbMinB[1] + pwbMaxB[1]) * 0.5;

					if (yA < yB)
					{
						RiverPartEntity tempPart = uniqueParts[j];
						uniqueParts[j] = uniqueParts[j + 1];
						uniqueParts[j + 1] = tempPart;
					}
				}
			}

			for (int p = 0; p < uniqueParts.Count(); p++)
			{
				RiverPartEntity rPart = uniqueParts[p];
				vector pbMin, pbMax;
				rPart.GetBounds(pbMin, pbMax);
				vector pwbMin, pwbMax;
				rPart.GetWorldBounds(pwbMin, pwbMax);

				float pHalfX = (pbMax[0] - pbMin[0]) * 0.5;
				float pHalfY = (pbMax[1] - pbMin[1]) * 0.5;
				float pHalfZ = (pbMax[2] - pbMin[2]) * 0.5;

				float partLen = pHalfZ * 2.0;
				float partWid = pHalfX * 2.0;
				float partHgt = pHalfY * 2.0;

				vector trueCenter = Vector((pwbMin[0] + pwbMax[0]) * 0.5, (pwbMin[1] + pwbMax[1]) * 0.5, (pwbMin[2] + pwbMax[2]) * 0.5);
				vector halfExt = Vector(pHalfX, pHalfY, pHalfZ);

				TBD_RiverPartExport partExport = new TBD_RiverPartExport(p, trueCenter, partLen, partWid, partHgt, pwbMin[1], pwbMax[1], halfExt, pwbMin, pwbMax);
				riverExport.AddPart(partExport);
			}

			m_aRivers.Insert(riverExport);
			Print(TAG + string.Format(" Exported River: '%1' (%2 spline pts, %3 subparts, total len: %4m, width: %5m)",
				rName, riverExport.m_aSplinePointsWS.Count(), riverExport.m_aParts.Count(), riverExport.m_fTotalLengthM.ToString(1), riverExport.m_fWidthM.ToString(1)), LogLevel.NORMAL);

			riverIndex++;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected bool WriteVectorsJson(string path)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open vectors JSON: " + path, LogLevel.ERROR);
			return false;
		}

		string buf = "{\n";
		buf += "  \"type\": \"RiverVectorDataset\",\n";
		buf += "  \"riversCount\": " + m_aRivers.Count().ToString() + ",\n";
		buf += "  \"rivers\": [\n";
		bool writeOk = true;

		for (int r = 0; r < m_aRivers.Count(); r++)
		{
			TBD_RiverExport riv = m_aRivers[r];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(riv.m_sId) + "\",\n";
			buf += "      \"name\": \"" + TBD_MapExportJson.Escape(riv.m_sName) + "\",\n";
			buf += "      \"widthM\": " + riv.m_fWidthM.ToString() + ",\n";
			buf += "      \"offsetUpM\": " + riv.m_fSplineOffsetUpM.ToString() + ",\n";
			buf += "      \"reverseFlow\": " + riv.m_iReverseFlow.ToString() + ",\n";
			buf += "      \"material\": \"" + TBD_MapExportJson.Escape(riv.m_sMaterial) + "\",\n";
			buf += "      \"surface\": \"" + TBD_MapExportJson.Escape(riv.m_sSurface) + "\",\n";
			buf += "      \"totalLengthM\": " + riv.m_fTotalLengthM.ToString() + ",\n";
			buf += "      \"bounds\": {\n";
			buf += "        \"min\": [" + riv.m_vBoundsMin[0].ToString() + ", " + riv.m_vBoundsMin[1].ToString() + ", " + riv.m_vBoundsMin[2].ToString() + "],\n";
			buf += "        \"max\": [" + riv.m_vBoundsMax[0].ToString() + ", " + riv.m_vBoundsMax[1].ToString() + ", " + riv.m_vBoundsMax[2].ToString() + "]\n";
			buf += "      },\n";

			// 1. Spline Control Points
			buf += "      \"splinePointsCount\": " + riv.m_aSplinePointsWS.Count().ToString() + ",\n";
			buf += "      \"splinePoints\": [\n";
			for (int pt = 0; pt < riv.m_aSplinePointsWS.Count(); pt++)
			{
				vector p = riv.m_aSplinePointsWS[pt];
				buf += "        [" + p[0].ToString() + ", " + p[1].ToString() + ", " + p[2].ToString() + "]";
				if (pt < riv.m_aSplinePointsWS.Count() - 1) buf += ",";
				buf += "\n";
			}
			buf += "      ],\n";

			// 2. RiverPartEntity Subparts
			buf += "      \"partsCount\": " + riv.m_aParts.Count().ToString() + ",\n";
			buf += "      \"parts\": [\n";
			for (int pi = 0; pi < riv.m_aParts.Count(); pi++)
			{
				TBD_RiverPartExport part = riv.m_aParts[pi];
				buf += "        {\n";
				buf += "          \"flowIndex\": " + part.m_iFlowIndex.ToString() + ",\n";
				buf += "          \"centerWS\": [" + part.m_vCenterWS[0].ToString() + ", " + part.m_vCenterWS[1].ToString() + ", " + part.m_vCenterWS[2].ToString() + "],\n";
				buf += "          \"lengthM\": " + part.m_fLengthM.ToString() + ",\n";
				buf += "          \"widthM\": " + part.m_fWidthM.ToString() + ",\n";
				buf += "          \"heightM\": " + part.m_fHeightM.ToString() + ",\n";
				buf += "          \"elevationRangeM\": [" + part.m_fMinYM.ToString() + ", " + part.m_fMaxYM.ToString() + "],\n";
				buf += "          \"halfExtentsLocal\": [" + part.m_vHalfExtentsLocal[0].ToString() + ", " + part.m_vHalfExtentsLocal[1].ToString() + ", " + part.m_vHalfExtentsLocal[2].ToString() + "],\n";
				buf += "          \"boundsWorld\": {\n";
				buf += "            \"min\": [" + part.m_vBoundsWorldMin[0].ToString() + ", " + part.m_vBoundsWorldMin[1].ToString() + ", " + part.m_vBoundsWorldMin[2].ToString() + "],\n";
				buf += "            \"max\": [" + part.m_vBoundsWorldMax[0].ToString() + ", " + part.m_vBoundsWorldMax[1].ToString() + ", " + part.m_vBoundsWorldMax[2].ToString() + "]\n";
				buf += "          }\n";
				buf += "        }";
				if (pi < riv.m_aParts.Count() - 1) buf += ",";
				buf += "\n";
			}
			buf += "      ]\n";

			buf += "    }";
			if (r < m_aRivers.Count() - 1) buf += ",";
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
