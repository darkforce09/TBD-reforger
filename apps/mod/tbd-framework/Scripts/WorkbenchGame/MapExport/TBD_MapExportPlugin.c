/**
 * TBD_MapExportPlugin.c
 *
 * Unified Workbench plugin for extracting all map data layers:
 *   - Layer 1: DEM 16-bit elevation heightmap matrix & metadata
 *   - Layer 2: Placed world objects (AABB spatial query chunked JSONL)
 *   - Layer 3: Named towns, villages, and landmarks (JSON)
 *   - Layer 4: Satellite / cartographic rasterization (.tga)
 *   - Layer 5: Water classification, 3D depth, and lake/pond catalog
 *   - Layer 6: Road network centerlines, widths & classes (JSON)
 *   - Layer 7: Tactical fences & stone walls micro-cover (JSONL)
 *   - Layer 8: Multi-level bridges & oriented pier decks (JSON)
 *   - Layer 9: Aviation infrastructure (Runways & Helipads) (JSON)
 *   - Layer 10: Electrical power grid & pylon graphs (JSON)
 *   - Layer 11: Prefab taxonomy, components & physical dimensions (JSON)
 *   - Layer 12: Arsenal, weapons & equipment compatibility registry (JSON)
 *   - Layer 13: Authoritative georeferencing anchor oracle (JSON)
 *
 * Prompts the user with an interactive dialog to choose the output destination
 * directory and select which layers to export.
 *
 * Menu: Workbench > Plugins > TBD > "Export TBD Map Data"
 */

[WorkbenchPluginAttribute(
	name: "Export All Map Data (Full Suite)",
	description: "Unified map exporter: extracts DEM, objects, locations, water, roads, fences, bridges, aviation, power grid, prefabs, arsenal, and anchors to a configurable destination directory.",
	category: "TBD"
)]
class TBD_MapExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportObjects m_ObjectsExporter;
	protected ref TBD_MapExportLocations m_LocationsExporter;
	protected ref TBD_MapExportRoads m_RoadsExporter;
	protected ref TBD_MapExportFences m_FencesExporter;
	protected ref TBD_MapExportBridges m_BridgesExporter;
	protected ref TBD_MapExportAviation m_AviationExporter;
	protected ref TBD_MapExportPowerlines m_PowerlinesExporter;
	protected ref TBD_MapExportPrefabs m_PrefabsExporter;
	protected ref TBD_MapExportArsenal m_ArsenalExporter;
	protected ref TBD_MapExportAnchors m_AnchorsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Print("[TBD][MapExport] Starting direct export from Workbench menu...", LogLevel.NORMAL);
		ExecuteExport(m_Config);
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure TBD Map Export",
			"Configure output destination directory and select map layers to export:",
			m_Config
		);
	}

	//------------------------------------------------------------------------------------------------
	//! Programmatic / automated execution entry point
	bool ExecuteExport(TBD_MapExportConfig config = null)
	{
		ref TBD_MapExportConfig activeConfig = config;
		if (!activeConfig)
		{
			if (m_Config)
				activeConfig = m_Config;
			else
				activeConfig = new TBD_MapExportConfig();
		}

		int tickStart = System.GetTickCount();
		TBD_MapExportPaths.EnsureDestinationDir(activeConfig.m_sDestinationDir);
		Print(string.Format("[TBD][MapExport] Starting unified map export to '%1'...", activeConfig.m_sDestinationDir));

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][MapExport] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return false;
		}

		bool allOk = true;

		// 1. DEM Heightmap
		if (activeConfig.m_bExportDEM)
		{
			Print("[TBD][MapExport] --- Running DEM Export ---");
			if (!TBD_MapExportDEM.Export(ctx, activeConfig))
				allOk = false;
		}

		// 2. Placed World Objects
		if (activeConfig.m_bExportObjects)
		{
			Print("[TBD][MapExport] --- Running World Objects Export ---");
			if (!m_ObjectsExporter)
				m_ObjectsExporter = new TBD_MapExportObjects();
			if (!m_ObjectsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 3. Named Locations & Towns
		if (activeConfig.m_bExportLocations)
		{
			Print("[TBD][MapExport] --- Running Locations Export ---");
			if (!m_LocationsExporter)
				m_LocationsExporter = new TBD_MapExportLocations();
			if (!m_LocationsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 4. Satellite / Cartographic Rasterization
		if (activeConfig.m_bExportSatellite)
		{
			Print("[TBD][MapExport] --- Running Satellite Rasterization ---");
			if (!TBD_MapExportSatellite.Export(ctx, activeConfig))
				allOk = false;
		}

		// 5. Water Surfaces & Masks (Ocean, Lakes, Rivers)
		if (activeConfig.m_bExportWater)
		{
			Print("[TBD][MapExport] --- Running Water Surface & Mask Export ---");
			if (!TBD_MapExportWater.Export(ctx, activeConfig))
				allOk = false;
		}

		// 6. Road Network (Centerlines, Widths, Classes)
		if (activeConfig.m_bExportRoads)
		{
			Print("[TBD][MapExport] --- Running Road Network Export ---");
			if (!m_RoadsExporter)
				m_RoadsExporter = new TBD_MapExportRoads();
			if (!m_RoadsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 7. Tactical Fences & Walls (Micro-Cover)
		if (activeConfig.m_bExportFences)
		{
			Print("[TBD][MapExport] --- Running Fences & Walls Export ---");
			if (!m_FencesExporter)
				m_FencesExporter = new TBD_MapExportFences();
			if (!m_FencesExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 8. Bridges, Viaducts & Oriented Pier Decks
		if (activeConfig.m_bExportBridges)
		{
			Print("[TBD][MapExport] --- Running Bridges & Piers Export ---");
			if (!m_BridgesExporter)
				m_BridgesExporter = new TBD_MapExportBridges();
			if (!m_BridgesExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 9. Aviation Infrastructure (Runways & Helipads)
		if (activeConfig.m_bExportAviation)
		{
			Print("[TBD][MapExport] --- Running Aviation Export ---");
			if (!m_AviationExporter)
				m_AviationExporter = new TBD_MapExportAviation();
			if (!m_AviationExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 10. Electrical Power Grid & Pylon Graphs
		if (activeConfig.m_bExportPowerlines)
		{
			Print("[TBD][MapExport] --- Running Powerlines & Grid Export ---");
			if (!m_PowerlinesExporter)
				m_PowerlinesExporter = new TBD_MapExportPowerlines();
			if (!m_PowerlinesExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 11. Prefab Taxonomy, Components & Dimensions
		if (activeConfig.m_bExportPrefabs)
		{
			Print("[TBD][MapExport] --- Running Prefabs & Taxonomy Export ---");
			if (!m_PrefabsExporter)
				m_PrefabsExporter = new TBD_MapExportPrefabs();
			if (!m_PrefabsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 12. Arsenal, Weapons & Equipment Compatibility Registry
		if (activeConfig.m_bExportArsenal)
		{
			Print("[TBD][MapExport] --- Running Arsenal Registry Export ---");
			if (!m_ArsenalExporter)
				m_ArsenalExporter = new TBD_MapExportArsenal();
			if (!m_ArsenalExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 13. Authoritative Georeferencing Anchor Oracle
		if (activeConfig.m_bExportAnchors)
		{
			Print("[TBD][MapExport] --- Running Terrain Anchors Export ---");
			if (!m_AnchorsExporter)
				m_AnchorsExporter = new TBD_MapExportAnchors();
			if (!m_AnchorsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][MapExport] All requested exports finished in %1 ms (success=%2) -> %3",
			elapsedMs, allOk, activeConfig.m_sDestinationDir));

		return allOk;
	}
}
