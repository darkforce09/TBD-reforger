/**
 * TBD_MapExportDEM.c
 *
 * Resamples terrain elevation (WorldEditorAPI.GetTerrainSurfaceY) across the world grid
 * and writes a uint16 ASCII elevation matrix + metadata JSON to the configured output path.
 */

class TBD_MapExportAnchor
{
	string id;
	float x;
	float z;
	float surfaceY;
}

class TBD_MapExportDEM
{
	protected static const string TAG = "[TBD][DEM]";
	protected static const int FLUSH = 8000;
	protected static const float DEFAULT_HMIN = -204.78;
	protected static const float DEFAULT_HMAX = 375.53;

	//------------------------------------------------------------------------------------------------
	protected static int EncodeU16(float y, float hMin, float hMax)
	{
		if (hMax <= hMin)
			return 0;
		float t = (y - hMin) / (hMax - hMin);
		float r = Math.Round(t * 65535.0);
		int u = r;
		if (u < 0) u = 0;
		if (u > 65535) u = 65535;
		return u;
	}

	//------------------------------------------------------------------------------------------------
	protected static void AddAnchor(array<ref TBD_MapExportAnchor> a, string id, float x, float z)
	{
		TBD_MapExportAnchor t = new TBD_MapExportAnchor();
		t.id = id;
		t.x = x;
		t.z = z;
		t.surfaceY = 0;
		a.Insert(t);
	}

	//------------------------------------------------------------------------------------------------
	protected static ref array<ref TBD_MapExportAnchor> BuildAnchors(float worldSize)
	{
		array<ref TBD_MapExportAnchor> a = {};
		AddAnchor(a, "center", worldSize * 0.5, worldSize * 0.5);
		AddAnchor(a, "sw", worldSize * 0.15, worldSize * 0.15);
		AddAnchor(a, "ne", worldSize * 0.85, worldSize * 0.85);
		AddAnchor(a, "nw", worldSize * 0.15, worldSize * 0.85);
		AddAnchor(a, "se", worldSize * 0.85, worldSize * 0.15);
		// Everon bridgehead anchors
		if (worldSize >= 12800.0)
		{
			AddAnchor(a, "bridgehead-sl", 4839.2, 6620.8);
			AddAnchor(a, "bridgehead-tl0", 4836.9, 6626.5);
			AddAnchor(a, "bridgehead-tl1", 4831.2, 6628.8);
		}
		return a;
	}

	//------------------------------------------------------------------------------------------------
	static bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		float worldSize = ctx.m_fWorldSize;
		float mpp = cfg.m_fDemMetersPerPixel;
		if (mpp <= 0.1)
			mpp = 2.0;

		int w = Math.Round(worldSize / mpp);
		int h = w;
		if (w <= 0 || h <= 0)
		{
			w = 6400;
			h = 6400;
		}

		float hMin = DEFAULT_HMIN;
		float hMax = DEFAULT_HMAX;
		if (ctx.m_vBoundsMin[1] < ctx.m_vBoundsMax[1] && ctx.m_vBoundsMin[1] < 0)
		{
			hMin = ctx.m_vBoundsMin[1];
			hMax = ctx.m_vBoundsMax[1];
		}

		string outRaster = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_TerrainExport_heightmap.txt");
		string outMeta = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_TerrainExport_meta.json");

		Print(string.Format("%1 Exporting DEM %2x%3 (res %4 m/px, world %5 m) -> %6",
			TAG, w, h, mpp, worldSize, outRaster));

		// Probe anchors
		array<ref TBD_MapExportAnchor> anchors = BuildAnchors(worldSize);
		float pMin = 100000.0;
		float pMax = -100000.0;
		foreach (TBD_MapExportAnchor anc : anchors)
		{
			anc.surfaceY = ctx.m_API.GetTerrainSurfaceY(anc.x, anc.z);
			if (anc.surfaceY < pMin) pMin = anc.surfaceY;
			if (anc.surfaceY > pMax) pMax = anc.surfaceY;
		}

		FileHandle f = FileIO.OpenFile(outRaster, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Could not open " + outRaster + " for write", LogLevel.ERROR);
			return false;
		}

		float sampMin = 100000.0;
		float sampMax = -100000.0;
		string buf = "";
		float sx = worldSize / (w - 1);
		float sz = worldSize / (h - 1);
		bool writeOk = true;

		for (int py = 0; py < h; py++)
		{
			float wz = py * sz;
			for (int px = 0; px < w; px++)
			{
				float wx = px * sx;
				float y = ctx.m_API.GetTerrainSurfaceY(wx, wz);
				if (y < sampMin) sampMin = y;
				if (y > sampMax) sampMax = y;

				buf += EncodeU16(y, hMin, hMax).ToString();
				if (px < w - 1)
					buf += " ";

				if (buf.Length() > FLUSH)
				{
					writeOk = TBD_MapExportJson.Write(f, buf, TAG);
					if (!writeOk) break;
					buf = "";
				}
			}
			if (!writeOk)
				break;

			buf += "\n";
			if (buf.Length() > FLUSH)
			{
				writeOk = TBD_MapExportJson.Write(f, buf, TAG);
				if (!writeOk) break;
				buf = "";
			}

			if (py % 512 == 0)
				Print(string.Format("%1 DEM row %2 / %3", TAG, py, h));
		}

		if (writeOk && buf.Length() > 0)
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
		f.Close();

		if (!writeOk)
		{
			FileIO.DeleteFile(outRaster);
			Print(TAG + " ABORTED: raster write failed.", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-getsurfacey-resample\",\n";
			mj += "  \"widthPx\": " + w.ToString() + ",\n";
			mj += "  \"heightPx\": " + h.ToString() + ",\n";
			mj += "  \"planarResolutionM\": " + mpp.ToString() + ",\n";
			mj += "  \"heightRangeMinM\": " + hMin.ToString() + ",\n";
			mj += "  \"heightRangeMaxM\": " + hMax.ToString() + ",\n";
			mj += "  \"sampledMinM\": " + sampMin.ToString() + ",\n";
			mj += "  \"sampledMaxM\": " + sampMax.ToString() + ",\n";
			mj += "  \"boundsMin\": \"" + ctx.m_vBoundsMin.ToString() + "\",\n";
			mj += "  \"boundsMax\": \"" + ctx.m_vBoundsMax.ToString() + "\",\n";
			mj += "  \"rasterFile\": \"TBD_TerrainExport_heightmap.txt\",\n";
			mj += "  \"rasterFormat\": \"ascii-uint16-rows\",\n";
			mj += "  \"anchors\": [\n";
			for (int i = 0; i < anchors.Count(); i++)
			{
				TBD_MapExportAnchor a = anchors[i];
				mj += "    { \"id\": \"" + a.id + "\", \"x\": " + a.x.ToString() + ", \"z\": " + a.z.ToString() + ", \"surfaceYM\": " + a.surfaceY.ToString() + " }";
				if (i < anchors.Count() - 1) mj += ",";
				mj += "\n";
			}
			mj += "  ]\n";
			mj += "}\n";
			bool metaOk = TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
			if (!metaOk)
			{
				FileIO.DeleteFile(outMeta);
				Print(TAG + " Meta write failed.", LogLevel.ERROR);
				return false;
			}
		}

		Print(string.Format("%1 DONE DEM export (sampled [%2, %3] m) -> %4", TAG, sampMin, sampMax, outRaster));
		return true;
	}
}
