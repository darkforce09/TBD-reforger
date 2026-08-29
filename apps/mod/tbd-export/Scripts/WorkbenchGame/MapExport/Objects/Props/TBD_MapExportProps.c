/**
 * TBD_MapExportProps.c
 *
 * Tactical props, cover, and clutter extraction shell:
 *   - Containers, barriers, crates, sandbags, street furniture, and clutter
 *
 * Outputs:
 *   - props.jsonl
 *   - props_meta.json
 */

class TBD_MapExportProps
{
	protected static const string TAG = "[TBD][Objects][Props]";

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		string mapName = ctx.GetMapName(cfg);
		string outProps = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/props", "props.jsonl");
		Print(string.Format("%1 Props export shell initialized for '%2' -> %3", TAG, mapName, outProps), LogLevel.NORMAL);
		return true;
	}
}
