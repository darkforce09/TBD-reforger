/**
 * TBD_MapExportConfig.c
 *
 * User-configurable settings and dialog parameters for the TBD Workbench Map Data Exporter.
 */

class TBD_MapExportConfig
{
	[Attribute("$profile:TBD_Export/", UIWidgets.EditBox, "Destination directory (e.g. $profile:TBD_Export/ or custom subfolder)")]
	string m_sDestinationDir;

	[Attribute("", UIWidgets.EditBox, "Map/Terrain name override (leave empty to auto-detect from active world)")]
	string m_sMapNameOverride;

	[Attribute("1", UIWidgets.CheckBox, "Export DEM 16-bit elevation heightmap matrix & metadata")]
	bool m_bExportDEM;

	[Attribute("1", UIWidgets.CheckBox, "Export Satellite / Cartographic Rasterization (.tga)")]
	bool m_bExportSatellite;

	[Attribute("1", UIWidgets.CheckBox, "Export Road & Spline Network (All Types)")]
	bool m_bExportRoads;

	[Attribute("1", UIWidgets.CheckBox, "Export Highways & Major Arterials exclusively to highways.json")]
	bool m_bExportHighways;

	[Attribute("1", UIWidgets.CheckBox, "Export Secondary Paved Roads exclusively to roads_paved.json")]
	bool m_bExportPavedRoads;

	[Attribute("1", UIWidgets.CheckBox, "Export Dirt & Gravel Roads exclusively to roads_dirt.json")]
	bool m_bExportDirtRoads;

	[Attribute("1", UIWidgets.CheckBox, "Export Forestry & Agricultural Tracks exclusively to tracks.json")]
	bool m_bExportTracks;

	[Attribute("1", UIWidgets.CheckBox, "Export Footpaths & Trails exclusively to paths.json")]
	bool m_bExportPaths;

	[Attribute("1", UIWidgets.CheckBox, "Export Airfield Runways & Taxiways exclusively to runways.json")]
	bool m_bExportRunways;

	[Attribute("1", UIWidgets.CheckBox, "Export Water Surfaces & Masks (Ocean, Lakes, Rivers)")]
	bool m_bExportWater;

	[Attribute("1", UIWidgets.CheckBox, "Export Vegetation & Natural Foliage (Trees, Rocks, Bushes)")]
	bool m_bExportVegetation;

	[Attribute("1", UIWidgets.CheckBox, "Export Living Trees exclusively to trees.json")]
	bool m_bExportTrees;

	[Attribute("1", UIWidgets.CheckBox, "Export Rocks & Cliff Formations exclusively to rocks.json")]
	bool m_bExportRocks;

	[Attribute("0", UIWidgets.CheckBox, "Cull 100% buried underground rock geometry from rocks.json")]
	bool m_bCullBuriedRocks;

	[Attribute("1", UIWidgets.CheckBox, "Export Bushes exclusively to bush.json")]
	bool m_bExportBushes;

	[Attribute("1", UIWidgets.CheckBox, "Export Wild Plants & Undergrowth exclusively to plants.json")]
	bool m_bExportPlants;

	[Attribute("1", UIWidgets.CheckBox, "Export Agricultural Crops & Vegetables exclusively to crops.json")]
	bool m_bExportCrops;

	[Attribute("1", UIWidgets.CheckBox, "Export Tree Stumps & Forestry Trunks exclusively to stumps.json")]
	bool m_bExportStumps;

	[Attribute("1", UIWidgets.CheckBox, "Export Placed World Objects (Buildings & Props)")]
	bool m_bExportObjects;

	[Attribute("1", UIWidgets.CheckBox, "Export Tactical Fences & Stone Walls (Micro-Cover)")]
	bool m_bExportFences;

	[Attribute("1", UIWidgets.CheckBox, "Export Bridges, Viaducts & Oriented Pier Decks")]
	bool m_bExportBridges;

	[Attribute("1", UIWidgets.CheckBox, "Export Aviation Infrastructure (Runways & Helipads)")]
	bool m_bExportAviation;

	[Attribute("1", UIWidgets.CheckBox, "Export Electrical Power Grid & Pylon Graphs")]
	bool m_bExportPowerlines;

	[Attribute("1", UIWidgets.CheckBox, "Export Named Locations & Towns (JSON)")]
	bool m_bExportLocations;

	[Attribute("1", UIWidgets.CheckBox, "Export Authoritative Georeferencing Anchor Oracle")]
	bool m_bExportAnchors;

	[Attribute("1", UIWidgets.CheckBox, "Export Prefab Taxonomy, Components & Dimensions")]
	bool m_bExportPrefabs;

	[Attribute("1", UIWidgets.CheckBox, "Export Arsenal, Weapons & Equipment Registry")]
	bool m_bExportArsenal;

	[Attribute("2.0", UIWidgets.EditBox, "DEM planar resolution in meters per pixel (default: 2.0 m/px)")]
	float m_fDemMetersPerPixel;

	[Attribute("1.0", UIWidgets.EditBox, "Water raster planar resolution in meters per pixel (default: 1.0 m/px, supports sub-meter e.g. 0.5)")]
	float m_fWaterMetersPerPixel;

	[Attribute("512.0", UIWidgets.EditBox, "World objects spatial query cell size in meters (default: 512.0 m)")]
	float m_fObjectChunkSizeM;

	//------------------------------------------------------------------------------------------------
	void TBD_MapExportConfig()
	{
		m_sDestinationDir = "$profile:TBD_Export/";
		m_bExportDEM = true;
		m_bExportSatellite = true;
		m_bExportRoads = true;
		m_bExportHighways = true;
		m_bExportPavedRoads = true;
		m_bExportDirtRoads = true;
		m_bExportTracks = true;
		m_bExportPaths = true;
		m_bExportRunways = true;
		m_bExportWater = true;
		m_bExportVegetation = true;
		m_bExportTrees = true;
		m_bExportRocks = true;
		m_bCullBuriedRocks = false;
		m_bExportBushes = true;
		m_bExportPlants = true;
		m_bExportCrops = true;
		m_bExportStumps = true;
		m_bExportObjects = true;
		m_bExportFences = true;
		m_bExportBridges = true;
		m_bExportAviation = true;
		m_bExportPowerlines = true;
		m_bExportLocations = true;
		m_bExportAnchors = true;
		m_bExportPrefabs = true;
		m_bExportArsenal = true;
		m_fDemMetersPerPixel = 2.0;
		m_fWaterMetersPerPixel = 1.0;
		m_fObjectChunkSizeM = 512.0;
	}
}
