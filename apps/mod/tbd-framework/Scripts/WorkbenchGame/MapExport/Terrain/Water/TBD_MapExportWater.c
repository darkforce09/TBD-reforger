/**
 * TBD_MapExportWater.c
 *
 * Ground-truth water surface, 3D bathymetry (depth), and water body extractor.
 * Evaluates full-map global water (ocean, lakes, ponds, and rivers) via native
 * multi-altitude engine water probing.
 *
 * Outputs:
 *   - TBD_WaterExport_mask.txt    (ASCII matrix: 0=Land, 1=Ocean, 2=Pond/Lake, 3=River)
 *   - TBD_WaterExport_depth.txt   (ASCII matrix of water depth in decimeters, 0.1 m resolution)
 *   - TBD_WaterExport_vectors.json (Catalog of discrete water bodies with exact bounds & centers)
 *   - TBD_WaterExport_meta.json   (Classification statistics, bounds, and discrete water bodies catalog)
 */

class TBD_WaterBodyRecord
{
	string m_sId;
	string m_sType;
	vector m_vCenter;
	float m_fSurfaceY;
	vector m_vMin;
	vector m_vMax;
	int m_iPixelCount;
	float m_fMaxDepth;

	void TBD_WaterBodyRecord(string id, string typeName, vector center, float surfaceY)
	{
		m_sId = id;
		m_sType = typeName;
		m_vCenter = center;
		m_fSurfaceY = surfaceY;
		m_vMin = center;
		m_vMax = center;
		m_iPixelCount = 0;
		m_fMaxDepth = 0.0;
	}

	void Update(vector worldPos, float depthM)
	{
		m_iPixelCount++;
		if (depthM > m_fMaxDepth)
			m_fMaxDepth = depthM;

		if (worldPos[0] < m_vMin[0]) m_vMin[0] = worldPos[0];
		if (worldPos[1] < m_vMin[1]) m_vMin[1] = worldPos[1];
		if (worldPos[2] < m_vMin[2]) m_vMin[2] = worldPos[2];

		if (worldPos[0] > m_vMax[0]) m_vMax[0] = worldPos[0];
		if (worldPos[1] > m_vMax[1]) m_vMax[1] = worldPos[1];
		if (worldPos[2] > m_vMax[2]) m_vMax[2] = worldPos[2];
	}
}

class TBD_MapExportWater
{
	protected static const string TAG = "[TBD][Water]";
	protected static const int FLUSH = 8000;

	//------------------------------------------------------------------------------------------------
	//! Primary export execution method.
	static bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		TBD_MapExportWater exporter = new TBD_MapExportWater();
		return exporter.Execute(ctx, cfg);
	}

	//------------------------------------------------------------------------------------------------
	bool Execute(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		float worldSize = ctx.m_fWorldSize;

		// Planar resolution in meters per pixel
		float mpp = cfg.m_fWaterMetersPerPixel;
		if (mpp <= 0.01)
			mpp = cfg.m_fDemMetersPerPixel;
		if (mpp <= 0.01)
			mpp = 1.0;

		int w = Math.Round(worldSize / mpp);
		int h = w;
		if (w <= 0 || h <= 0)
		{
			w = 6400;
			h = 6400;
		}

		string mapName = ctx.GetMapName(cfg);
		string outMask = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "water", "bathymetry_mask.txt");
		string outDepth = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "water", "bathymetry_depth.txt");
		string outVectors = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "water", "lakes.json");
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "water", "water_meta.json");

		Print(string.Format("%1 Rasterizing water grid %2x%3 (res %4 m/px, world %5 m)...",
			TAG, w, h, mpp, worldSize));

		FileHandle fMask = FileIO.OpenFile(outMask, FileMode.WRITE);
		if (!fMask)
		{
			Print(TAG + " Could not open " + outMask + " for write", LogLevel.ERROR);
			return false;
		}

		FileHandle fDepth = FileIO.OpenFile(outDepth, FileMode.WRITE);
		if (!fDepth)
		{
			fMask.Close();
			Print(TAG + " Could not open " + outDepth + " for write", LogLevel.ERROR);
			return false;
		}

		map<string, ref TBD_WaterBodyRecord> bodiesMap = new map<string, ref TBD_WaterBodyRecord>();
		array<ref TBD_WaterBodyRecord> bodiesList = {};

		int oceanPx = 0;
		int pondPx = 0;
		int riverPx = 0;
		int landPx = 0;

		float minWaterY = 100000.0;
		float maxWaterY = -100000.0;

		string bufMask = "";
		string bufDepth = "";
		float sx = worldSize / (w - 1);
		float sz = worldSize / (h - 1);
		bool writeOk = true;

		vector outWaterPos;
		EWaterSurfaceType outType;
		vector transformWS[4];
		vector obbExtents;

		for (int py = 0; py < h; py++)
		{
			float wz = py * sz;
			for (int px = 0; px < w; px++)
			{
				float wx = px * sx;
				float terrainY = ctx.m_API.GetTerrainSurfaceY(wx, wz);

				// Step A: Multi-altitude probe of native engine water surface
				bool isWater = ChimeraWorldUtils.TryGetWaterSurface(ctx.m_World, Vector(wx, terrainY - 0.05, wz), outWaterPos, outType, transformWS, obbExtents);
				if (!isWater && terrainY > 0.0)
				{
					isWater = ChimeraWorldUtils.TryGetWaterSurface(ctx.m_World, Vector(wx, terrainY + 0.35, wz), outWaterPos, outType, transformWS, obbExtents);
					if (!isWater)
						isWater = ChimeraWorldUtils.TryGetWaterSurface(ctx.m_World, Vector(wx, terrainY + 0.9, wz), outWaterPos, outType, transformWS, obbExtents);
					if (!isWater)
						isWater = ChimeraWorldUtils.TryGetWaterSurface(ctx.m_World, Vector(wx, terrainY + 1.6, wz), outWaterPos, outType, transformWS, obbExtents);
				}

				int typeCode = 0; // 0 = Land
				float waterSurfaceY = 0.0;

				if (isWater)
				{
					waterSurfaceY = outWaterPos[1];
					if (outType == EWaterSurfaceType.WST_OCEAN)
					{
						typeCode = 1; // 1 = Ocean
					}
					else if (outType == EWaterSurfaceType.WST_POND)
					{
						typeCode = 2; // 2 = Pond/Lake
					}
					else if (outType == EWaterSurfaceType.WST_RIVER)
					{
						typeCode = 3; // 3 = River
					}
					else
					{
						if (terrainY > 0.0 && waterSurfaceY > 1.0)
							typeCode = 2;
						else
							typeCode = 1;
					}
				}
				else if (terrainY <= 0.0)
				{
					// Sub-sea level ocean fallback
					typeCode = 1;
					waterSurfaceY = 0.0;
				}

				// Step B: Calculate accurate 3D bathymetry depth (Y_water_surface - Y_terrain)
				int depthDm = 0;
				if (typeCode != 0)
				{
					if (waterSurfaceY < minWaterY) minWaterY = waterSurfaceY;
					if (waterSurfaceY > maxWaterY) maxWaterY = waterSurfaceY;

					float depthM = waterSurfaceY - terrainY;
					if (depthM < 0.0)
						depthM = 0.0;

					if (typeCode == 3 && depthM < 0.2)
						depthM = 0.5; // Ensure carved riverbed minimum depth

					int calcDm = Math.Round(depthM * 10.0);
					if (calcDm > 65535) calcDm = 65535;
					depthDm = calcDm;

					if (typeCode == 1) oceanPx++;
					else if (typeCode == 2) pondPx++;
					else if (typeCode == 3) riverPx++;

					// Catalog discrete inland water bodies
					if (typeCode == 2 || typeCode == 3)
					{
						string prefix = "pond_";
						string typeStr = "pond";
						if (typeCode == 3)
						{
							prefix = "river_";
							typeStr = "river";
						}

						int qx = Math.Round(wx / 64.0) * 64;
						int qz = Math.Round(wz / 64.0) * 64;
						string bodyKey = string.Format("%1_%2_%3", qx, qz, typeCode.ToString());

						ref TBD_WaterBodyRecord rec;
						if (!bodiesMap.Find(bodyKey, rec))
						{
							string id = prefix + (bodiesList.Count() + 1).ToString();
							rec = new TBD_WaterBodyRecord(id, typeStr, Vector(wx, waterSurfaceY, wz), waterSurfaceY);
							bodiesMap.Set(bodyKey, rec);
							bodiesList.Insert(rec);
						}
						rec.Update(Vector(wx, terrainY, wz), depthM);
					}
				}
				else
				{
					landPx++;
				}

				bufMask += typeCode.ToString();
				bufDepth += depthDm.ToString();

				if (px < w - 1)
				{
					bufMask += " ";
					bufDepth += " ";
				}

				if (bufMask.Length() > FLUSH)
				{
					writeOk = TBD_MapExportJson.Write(fMask, bufMask, TAG);
					if (writeOk)
						writeOk = TBD_MapExportJson.Write(fDepth, bufDepth, TAG);
					if (!writeOk) break;
					bufMask = "";
					bufDepth = "";
				}
			}
			if (!writeOk)
				break;

			bufMask += "\n";
			bufDepth += "\n";
			if (bufMask.Length() > FLUSH)
			{
				writeOk = TBD_MapExportJson.Write(fMask, bufMask, TAG);
				if (writeOk)
					writeOk = TBD_MapExportJson.Write(fDepth, bufDepth, TAG);
				if (!writeOk) break;
				bufMask = "";
				bufDepth = "";
			}

			if (py % 1024 == 0)
				Print(string.Format("%1 Water row %2 / %3", TAG, py, h));
		}

		if (writeOk && bufMask.Length() > 0)
			writeOk = TBD_MapExportJson.Write(fMask, bufMask, TAG);
		if (writeOk && bufDepth.Length() > 0)
			writeOk = TBD_MapExportJson.Write(fDepth, bufDepth, TAG);

		fMask.Close();
		fDepth.Close();

		if (!writeOk)
		{
			FileIO.DeleteFile(outMask);
			FileIO.DeleteFile(outDepth);
			Print(TAG + " ABORTED: Water raster write failed.", LogLevel.ERROR);
			return false;
		}

		if (minWaterY > maxWaterY)
		{
			minWaterY = 0.0;
			maxWaterY = 0.0;
		}

		// Export vectors catalog
		ExportVectorsJson(outVectors, bodiesList);

		// Step 3: Write metadata JSON
		WriteMetaJson(outMeta, w, h, mpp, worldSize, landPx, oceanPx, pondPx, riverPx, minWaterY, maxWaterY, bodiesList);

		Print(string.Format("%1 DONE — Ocean=%2 px, Pond=%3 px, River=%4 px, Bodies=%5 -> %6",
			TAG, oceanPx, pondPx, riverPx, bodiesList.Count(), outMask));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected bool ExportVectorsJson(string outVectorsPath, array<ref TBD_WaterBodyRecord> bodiesList)
	{
		FileHandle f = FileIO.OpenFile(outVectorsPath, FileMode.WRITE);
		if (!f)
			return false;

		string buf = "{\n";
		buf += "  \"waterBodies\": [\n";
		bool writeOk = true;

		for (int i = 0; i < bodiesList.Count(); i++)
		{
			TBD_WaterBodyRecord rec = bodiesList[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(rec.m_sId) + "\",\n";
			buf += "      \"type\": \"" + TBD_MapExportJson.Escape(rec.m_sType) + "\",\n";
			buf += "      \"surfaceElevationYM\": " + rec.m_fSurfaceY.ToString() + ",\n";
			buf += "      \"center\": [" + rec.m_vCenter[0].ToString() + ", " + rec.m_vCenter[2].ToString() + "],\n";
			buf += "      \"pixelCount\": " + rec.m_iPixelCount.ToString() + ",\n";
			buf += "      \"maxDepthM\": " + rec.m_fMaxDepth.ToString() + ",\n";
			buf += "      \"bbox\": [" + rec.m_vMin[0].ToString() + ", " + rec.m_vMin[1].ToString() + ", " + rec.m_vMin[2].ToString() + ", "
				+ rec.m_vMax[0].ToString() + ", " + rec.m_vMax[1].ToString() + ", " + rec.m_vMax[2].ToString() + "]\n";
			buf += "    }";
			if (i < bodiesList.Count() - 1)
				buf += ",";
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

	//------------------------------------------------------------------------------------------------
	//! Writes metadata JSON with comprehensive statistics and catalog.
	protected void WriteMetaJson(string outMetaPath, int w, int h, float mpp, float worldSize, int landPx, int oceanPx, int pondPx, int riverPx, float minWaterY, float maxWaterY, array<ref TBD_WaterBodyRecord> bodiesList)
	{
		FileHandle mh = FileIO.OpenFile(outMetaPath, FileMode.WRITE);
		if (!mh)
			return;

		float pixelAreaM2 = mpp * mpp;
		string mj = "{\n";
		mj += "  \"method\": \"mod-water-hydrology-resample\",\n";
		mj += "  \"widthPx\": " + w.ToString() + ",\n";
		mj += "  \"heightPx\": " + h.ToString() + ",\n";
		mj += "  \"planarResolutionM\": " + mpp.ToString() + ",\n";
		mj += "  \"depthUnit\": \"decimeters\",\n";
		mj += "  \"depthScaleToMeters\": 0.1,\n";
		mj += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		mj += "  \"landPixelCount\": " + landPx.ToString() + ",\n";
		mj += "  \"oceanPixelCount\": " + oceanPx.ToString() + ",\n";
		mj += "  \"inlandPondPixelCount\": " + pondPx.ToString() + ",\n";
		mj += "  \"riverPixelCount\": " + riverPx.ToString() + ",\n";
		mj += "  \"minWaterSurfaceYM\": " + minWaterY.ToString() + ",\n";
		mj += "  \"maxWaterSurfaceYM\": " + maxWaterY.ToString() + ",\n";
		mj += "  \"maskFile\": \"bathymetry_mask.txt\",\n";
		mj += "  \"depthFile\": \"bathymetry_depth.txt\",\n";
		mj += "  \"vectorsFile\": \"lakes.json\",\n";
		mj += "  \"inlandWaterBodiesCount\": " + bodiesList.Count().ToString() + ",\n";
		mj += "  \"inlandWaterBodies\": [\n";

		for (int i = 0; i < bodiesList.Count(); i++)
		{
			TBD_WaterBodyRecord rec = bodiesList[i];
			float totalAreaM2 = rec.m_iPixelCount * pixelAreaM2;
			mj += "    {\n";
			mj += "      \"id\": \"" + TBD_MapExportJson.Escape(rec.m_sId) + "\",\n";
			mj += "      \"type\": \"" + TBD_MapExportJson.Escape(rec.m_sType) + "\",\n";
			mj += "      \"surfaceYM\": " + rec.m_fSurfaceY.ToString() + ",\n";
			mj += "      \"center\": [" + rec.m_vCenter[0].ToString() + ", " + rec.m_vCenter[2].ToString() + "],\n";
			mj += "      \"areaM2\": " + totalAreaM2.ToString() + ",\n";
			mj += "      \"maxDepthM\": " + rec.m_fMaxDepth.ToString() + ",\n";
			mj += "      \"bbox\": [" + rec.m_vMin[0].ToString() + ", " + rec.m_vMin[1].ToString() + ", " + rec.m_vMin[2].ToString() + ", "
				+ rec.m_vMax[0].ToString() + ", " + rec.m_vMax[1].ToString() + ", " + rec.m_vMax[2].ToString() + "]\n";
			mj += "    }";
			if (i < bodiesList.Count() - 1)
				mj += ",";
			mj += "\n";
		}

		mj += "  ]\n";
		mj += "}\n";
		TBD_MapExportJson.Write(mh, mj, TAG);
		mh.Close();
	}
}
