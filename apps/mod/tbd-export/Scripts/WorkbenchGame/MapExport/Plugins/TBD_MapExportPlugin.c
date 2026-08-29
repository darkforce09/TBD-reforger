/**
 * TBD_MapExportPlugin.c
 *
 * Unified Workbench plugin for extracting all map data layers across:
 *   - Terrain Domain: DEM Elevation, Satellite Cartography, Road Networks, Water Surfaces & Bathymetry
 *   - Vegetation Domain: Trees, Forest Canopies, Rock Formations, Bushes & Clutter
 *   - Objects Domain: Buildings, Tactical Props, Infrastructure (Fences, Bridges, Aviation, Powerlines)
 *   - Locations Domain: Named Landmarks, Towns, Settlements, and Georeferencing Anchors
 *   - Registry Domain: Prefab Taxonomies and Arsenal Compatibility Registries
 *
 * Prompts the user with an interactive dialog to choose the output destination
 * directory and select which layers to export.
 *
 * Menu: Workbench > Plugins > TBD > "Export All Map Data (Full Suite)"
 */

[WorkbenchPluginAttribute(
	name: "Export All Map Data (Full Suite)",
	description: "Unified map exporter: extracts terrain (DEM, sat, roads, water), vegetation, objects, infrastructure, locations, and registries to a configurable destination directory.",
	category: "TBD"
)]
class TBD_MapExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportVegetation m_VegetationExporter;
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
		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][MapExport] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return false;
		}

		string mapName = ctx.GetMapName(activeConfig);
		TBD_MapExportPaths.EnsureDirRecursive(TBD_MapExportPaths.GetCategoryDir(activeConfig.m_sDestinationDir, mapName));
		Print(string.Format("[TBD][MapExport] Starting unified map export for '%1' to '%2'...", mapName, activeConfig.m_sDestinationDir));

		bool allOk = true;

		// 1. DEM Heightmap (Terrain Domain)
		if (activeConfig.m_bExportDEM)
		{
			Print("[TBD][MapExport] --- Running DEM Export ---");
			if (!TBD_MapExportDEM.Export(ctx, activeConfig))
				allOk = false;
		}

		// 2. Satellite / Cartographic Rasterization (Terrain Domain)
		if (activeConfig.m_bExportSatellite)
		{
			Print("[TBD][MapExport] --- Running Satellite Rasterization ---");
			if (!TBD_MapExportSatellite.Export(ctx, activeConfig))
				allOk = false;
		}

		// 3. Road Network (Terrain Domain)
		if (activeConfig.m_bExportRoads)
		{
			Print("[TBD][MapExport] --- Running Road Network Export ---");
			if (!TBD_MapExportRoads.Export(ctx, activeConfig))
				allOk = false;
		}

		// 4. Water Surfaces & Masks (Terrain Domain)
		if (activeConfig.m_bExportWater)
		{
			Print("[TBD][MapExport] --- Running Water Surface & Mask Export ---");
			if (!TBD_MapExportWater.Export(ctx, activeConfig))
				allOk = false;
		}

		// 5. Vegetation & Natural Foliage (Vegetation Domain)
		if (activeConfig.m_bExportVegetation)
		{
			Print("[TBD][MapExport] --- Running Vegetation Export ---");
			if (!TBD_MapExportVegetation.Export(ctx, activeConfig))
				allOk = false;
		}

		// 6. Placed World Objects & Structures (Objects Domain)
		if (activeConfig.m_bExportObjects)
		{
			Print("[TBD][MapExport] --- Running World Objects Export ---");
			if (!m_ObjectsExporter)
				m_ObjectsExporter = new TBD_MapExportObjects();
			if (!m_ObjectsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 7. Tactical Fences & Walls (Objects/Infrastructure)
		if (activeConfig.m_bExportFences)
		{
			Print("[TBD][MapExport] --- Running Fences & Walls Export ---");
			if (!m_FencesExporter)
				m_FencesExporter = new TBD_MapExportFences();
			if (!m_FencesExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 8. Bridges, Viaducts & Oriented Pier Decks (Objects/Infrastructure)
		if (activeConfig.m_bExportBridges)
		{
			Print("[TBD][MapExport] --- Running Bridges & Piers Export ---");
			if (!m_BridgesExporter)
				m_BridgesExporter = new TBD_MapExportBridges();
			if (!m_BridgesExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 9. Aviation Infrastructure (Objects/Infrastructure)
		if (activeConfig.m_bExportAviation)
		{
			Print("[TBD][MapExport] --- Running Aviation Export ---");
			if (!m_AviationExporter)
				m_AviationExporter = new TBD_MapExportAviation();
			if (!m_AviationExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 10. Electrical Power Grid & Pylon Graphs (Objects/Infrastructure)
		if (activeConfig.m_bExportPowerlines)
		{
			Print("[TBD][MapExport] --- Running Powerlines & Grid Export ---");
			if (!m_PowerlinesExporter)
				m_PowerlinesExporter = new TBD_MapExportPowerlines();
			if (!m_PowerlinesExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 11. Named Locations & Towns (Locations Domain)
		if (activeConfig.m_bExportLocations)
		{
			Print("[TBD][MapExport] --- Running Locations Export ---");
			if (!m_LocationsExporter)
				m_LocationsExporter = new TBD_MapExportLocations();
			if (!m_LocationsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 12. Authoritative Georeferencing Anchor Oracle (Locations/Anchors)
		if (activeConfig.m_bExportAnchors)
		{
			Print("[TBD][MapExport] --- Running Terrain Anchors Export ---");
			if (!m_AnchorsExporter)
				m_AnchorsExporter = new TBD_MapExportAnchors();
			if (!m_AnchorsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 13. Prefab Taxonomy, Components & Dimensions (Registry Domain)
		if (activeConfig.m_bExportPrefabs)
		{
			Print("[TBD][MapExport] --- Running Prefabs & Taxonomy Export ---");
			if (!m_PrefabsExporter)
				m_PrefabsExporter = new TBD_MapExportPrefabs();
			if (!m_PrefabsExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		// 14. Arsenal, Weapons & Equipment Compatibility Registry (Registry Domain)
		if (activeConfig.m_bExportArsenal)
		{
			Print("[TBD][MapExport] --- Running Arsenal Registry Export ---");
			if (!m_ArsenalExporter)
				m_ArsenalExporter = new TBD_MapExportArsenal();
			if (!m_ArsenalExporter.Export(ctx, activeConfig))
				allOk = false;
		}

		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][MapExport] All requested exports finished in %1 ms (success=%2) -> %3",
			elapsedMs, allOk, activeConfig.m_sDestinationDir));

		return allOk;
	}
}
