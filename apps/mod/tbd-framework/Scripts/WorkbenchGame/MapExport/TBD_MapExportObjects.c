/**
 * TBD_MapExportObjects.c
 *
 * Full-world entity extractor: iterates the map in spatial cell passes (default 512 m),
 * queries BaseWorld.QueryEntitiesByAABB, resolves prefabs, transforms, headings and bounds,
 * and writes TBD_WorldExport_full.jsonl + completion sentinel metadata JSON to the destination path.
 */

class TBD_MapExportObjects
{
	protected static const string TAG = "[TBD][WorldObjects]";
	protected static const float Y_MIN = -1000.0;
	protected static const float Y_MAX = 2000.0;
	protected static const int FLUSH = 8000;

	protected ref array<IEntity> m_aHits;

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected int CellIndex(float coord, float cellM, int cells)
	{
		int c = Math.Floor(coord / cellM);
		if (c < 0) c = 0;
		if (c > cells - 1) c = cells - 1;
		return c;
	}

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		float worldSize = ctx.m_fWorldSize;
		float cellM = cfg.m_fObjectChunkSizeM;
		if (cellM <= 10.0)
			cellM = 512.0;

		int cells = Math.Ceil(worldSize / cellM);
		string outJsonl = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_WorldExport_full.jsonl");
		string outMeta = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_WorldExport_full_meta.json");

		Print(string.Format("%1 Exporting world objects across %2 m (%3x%3 cells) -> %4",
			TAG, worldSize, cells, outJsonl));

		// Stale sentinel deleted before writing
		FileIO.DeleteFile(outMeta);

		FileHandle f = FileIO.OpenFile(outJsonl, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Cannot open " + outJsonl + " for write", LogLevel.ERROR);
			return false;
		}

		int tick0 = System.GetTickCount();
		int aabbHits = 0;
		int kept = 0;
		int withPrefab = 0;
		int outOfBounds = 0;
		string buf = "";
		bool writeOk = true;

		for (int iz = 0; iz < cells; iz++)
		{
			for (int ix = 0; ix < cells; ix++)
			{
				float x0 = ix * cellM;
				float z0 = iz * cellM;
				m_aHits = {};
				vector mins = Vector(x0, Y_MIN, z0);
				vector maxs = Vector(x0 + cellM, Y_MAX, z0 + cellM);
				ctx.m_World.QueryEntitiesByAABB(mins, maxs, CollectEntity);
				aabbHits += m_aHits.Count();

				int cellKept = 0;
				foreach (IEntity e : m_aHits)
				{
					vector pos = e.GetOrigin();
					if (pos[0] < 0 || pos[0] > worldSize || pos[2] < 0 || pos[2] > worldSize)
					{
						if (ix == 0 && iz == 0)
							outOfBounds++;
						continue;
					}
					if (CellIndex(pos[0], cellM, cells) != ix || CellIndex(pos[2], cellM, cells) != iz)
						continue;

					// S6 standard: GetAngles() = (pitch, heading/yaw, roll)
					vector ang = e.GetAngles();
					vector bmin, bmax;
					e.GetWorldBounds(bmin, bmax);
					float hx = (bmax[0] - bmin[0]) * 0.5;
					float hy = (bmax[1] - bmin[1]) * 0.5;
					float hz = (bmax[2] - bmin[2]) * 0.5;

					string rn = ctx.ResolvePrefab(e);
					if (rn != "")
						withPrefab++;

					string row = "{";
					row += "\"resourceName\":\"" + TBD_MapExportJson.Escape(rn) + "\",";
					row += "\"className\":\"" + TBD_MapExportJson.Escape(e.ClassName()) + "\",";
					row += "\"x\":" + pos[0].ToString() + ",";
					row += "\"y\":" + pos[1].ToString() + ",";
					row += "\"z\":" + pos[2].ToString() + ",";
					row += "\"headingDeg\":" + ang[1].ToString() + ",";
					row += "\"pitchDeg\":" + ang[0].ToString() + ",";
					row += "\"rollDeg\":" + ang[2].ToString() + ",";
					row += "\"halfExtentsM\":[" + hx.ToString() + "," + hy.ToString() + "," + hz.ToString() + "]";
					row += "}\n";

					buf += row;
					kept++;
					cellKept++;

					if (buf.Length() > FLUSH)
					{
						writeOk = TBD_MapExportJson.Write(f, buf, TAG);
						if (!writeOk) break;
						buf = "";
					}
				}
				if (!writeOk)
					break;
			}
			if (!writeOk)
				break;
			Print(string.Format("%1 Row %2/%3 done (total kept %4)", TAG, iz + 1, cells, kept));
		}

		if (writeOk && buf.Length() > 0)
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
		f.Close();

		if (!writeOk)
		{
			FileIO.DeleteFile(outJsonl);
			Print(TAG + " ABORTED: JSONL write failed — partial file deleted.", LogLevel.ERROR);
			return false;
		}

		int elapsedMs = System.GetTickCount() - tick0;

		// Completion sentinel metadata JSON written LAST
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (!mh)
		{
			Print(TAG + " Cannot open meta " + outMeta + " — export unsealed.", LogLevel.ERROR);
			return false;
		}

		string mj = "{\n";
		mj += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		mj += "  \"cellSizeM\": " + cellM.ToString() + ",\n";
		mj += "  \"cells\": " + cells.ToString() + ",\n";
		mj += "  \"aabbHitCount\": " + aabbHits.ToString() + ",\n";
		mj += "  \"keptCount\": " + kept.ToString() + ",\n";
		mj += "  \"withPrefab\": " + withPrefab.ToString() + ",\n";
		mj += "  \"outOfBounds\": " + outOfBounds.ToString() + ",\n";
		mj += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
		mj += "  \"anglesRule\": \"headingDeg=GetAngles()[1]; pitch=[0], roll=[2]\",\n";
		mj += "  \"partitionRule\": \"clamp(floor(coord/cellSize), 0, cells-1) on entity origin\"\n";
		mj += "}\n";

		bool metaOk = TBD_MapExportJson.Write(mh, mj, TAG);
		mh.Close();
		if (!metaOk)
		{
			FileIO.DeleteFile(outMeta);
			Print(TAG + " Meta write failed.", LogLevel.ERROR);
			return false;
		}

		Print(string.Format("%1 DONE — kept %2 objects (withPrefab %3, elapsed %4 ms) -> %5",
			TAG, kept, withPrefab, elapsedMs, outJsonl));
		return true;
	}
}
