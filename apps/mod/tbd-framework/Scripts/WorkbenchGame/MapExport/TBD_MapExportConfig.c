/**
 * TBD_MapExportConfig.c
 *
 * User-configurable settings and dialog parameters for the TBD Workbench Map Data Exporter.
 */

class TBD_MapExportConfig
{
	[Attribute("$profile:TBD_Export/", UIWidgets.EditBox, "Destination directory (e.g. $profile:TBD_Export/ or custom subfolder)")]
	string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Export DEM 16-bit elevation heightmap matrix & metadata")]
	bool m_bExportDEM;

	[Attribute("1", UIWidgets.CheckBox, "Export Placed World Objects (Full AABB chunked JSONL)")]
	bool m_bExportObjects;

	[Attribute("1", UIWidgets.CheckBox, "Export Named Locations & Towns (JSON)")]
	bool m_bExportLocations;

	[Attribute("1", UIWidgets.CheckBox, "Export Satellite / Cartographic Rasterization (.tga)")]
	bool m_bExportSatellite;

	[Attribute("1", UIWidgets.CheckBox, "Export Water Surfaces & Masks (Ocean, Lakes, Rivers)")]
	bool m_bExportWater;

	[Attribute("1", UIWidgets.CheckBox, "Export Road & Spline Network (Centerlines, Widths, Classes)")]
	bool m_bExportRoads;

	[Attribute("1", UIWidgets.CheckBox, "Export Tactical Fences & Stone Walls (Micro-Cover)")]
	bool m_bExportFences;

	[Attribute("1", UIWidgets.CheckBox, "Export Bridges, Viaducts & Oriented Pier Decks")]
	bool m_bExportBridges;

	[Attribute("1", UIWidgets.CheckBox, "Export Aviation Infrastructure (Runways & Helipads)")]
	bool m_bExportAviation;

	[Attribute("1", UIWidgets.CheckBox, "Export Electrical Power Grid & Pylon Graphs")]
	bool m_bExportPowerlines;

	[Attribute("1", UIWidgets.CheckBox, "Export Prefab Taxonomy, Components & Dimensions")]
	bool m_bExportPrefabs;

	[Attribute("1", UIWidgets.CheckBox, "Export Arsenal, Weapons & Equipment Registry")]
	bool m_bExportArsenal;

	[Attribute("1", UIWidgets.CheckBox, "Export Authoritative Georeferencing Anchor Oracle")]
	bool m_bExportAnchors;

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
		m_bExportObjects = true;
		m_bExportLocations = true;
		m_bExportSatellite = true;
		m_bExportWater = true;
		m_bExportRoads = true;
		m_bExportFences = true;
		m_bExportBridges = true;
		m_bExportAviation = true;
		m_bExportPowerlines = true;
		m_bExportPrefabs = true;
		m_bExportArsenal = true;
		m_bExportAnchors = true;
		m_fDemMetersPerPixel = 2.0;
		m_fWaterMetersPerPixel = 1.0;
		m_fObjectChunkSizeM = 512.0;
	}
}
