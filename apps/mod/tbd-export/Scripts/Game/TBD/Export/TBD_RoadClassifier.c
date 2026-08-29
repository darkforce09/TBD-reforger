/**
 * TBD_RoadClassifier.c
 *
 * Deterministic road classification engine for tbd-export.
 * Categorizes road segments into:
 *   1. Highways & Major Arterials (highways.json)
 *   2. Secondary Paved Roads (roads_paved.json)
 *   3. Dirt & Gravel Roads (roads_dirt.json)
 *   4. Tracks & Tractor Trails (tracks.json)
 *   5. Footpaths & Hiking Trails (paths.json)
 *   6. Airfield Runways & Taxiways (runways.json)
 */

enum TBD_ERoadLayer
{
	NONE = 0,
	HIGHWAY,
	PAVED,
	DIRT,
	TRACK,
	PATH,
	RUNWAY
}

class TBD_RoadClassifier
{
	//------------------------------------------------------------------------------------------------
	//! Classify a road segment based on material, prefab/resource name, and width.
	static TBD_ERoadLayer Classify(string matName, string resName, float widthM)
	{
		string lowerMat = matName;
		lowerMat.ToLower();
		string lowerRes = resName;
		lowerRes.ToLower();

		// Filter out non-road items
		if (lowerRes.Contains("/vegetation/") || lowerRes.Contains("/tree/") || lowerRes.Contains("/rocks/") || lowerRes.Contains("/water/"))
			return TBD_ERoadLayer.NONE;
		if (lowerRes.Contains("/props/") || lowerRes.Contains("/signs/") || lowerRes.Contains("lamp") || lowerRes.Contains("barrier") || lowerRes.Contains("traffic_"))
			return TBD_ERoadLayer.NONE;
		if (lowerRes.Contains("/fence") || lowerRes.Contains("/powerline") || lowerRes.Contains("/pylon"))
			return TBD_ERoadLayer.NONE;
		if (lowerRes.Contains("flowerbed") || lowerRes.Contains("naturedebris") || lowerRes.Contains("concretepanelrow"))
			return TBD_ERoadLayer.NONE;

		// 1. Runways & Taxiways
		if (lowerMat.Contains("runwayconcrete") || lowerMat.Contains("runway") || lowerRes.Contains("runway") || lowerRes.Contains("airstrip") || lowerRes.Contains("taxiway") || (widthM >= 18.0 && lowerMat.Contains("asphalt")))
		{
			return TBD_ERoadLayer.RUNWAY;
		}

		// 2. Paths & Hiking Trails
		if (lowerMat.Contains("traildirt") || lowerMat.Contains("trailgravel") || lowerMat.Contains("trailforest") || lowerMat.Contains("trail_") ||
			lowerRes.Contains("path") || lowerRes.Contains("trail") || lowerRes.Contains("footpath") || lowerRes.Contains("pedestrian") || lowerRes.Contains("walkway") || lowerRes.Contains("hiking"))
		{
			return TBD_ERoadLayer.PATH;
		}

		// 3. Agricultural & Forestry Tracks
		if (lowerMat.Contains("road_forest") || lowerMat.Contains("dirttracks") || lowerMat.Contains("road_dirt_02") || lowerMat.Contains("road_concretepanel") ||
			lowerRes.Contains("tractor") || lowerRes.Contains("field_track") || lowerRes.Contains("forest_track") || lowerRes.Contains("two_track") || lowerRes.Contains("dirt_tracks"))
		{
			return TBD_ERoadLayer.TRACK;
		}

		// 4. Dirt Roads
		if (lowerMat.Contains("road_dirt_01") || lowerMat.Contains("dirt_01") || lowerRes.Contains("road_dirt_01") || lowerRes.Contains("road_dirt"))
		{
			return TBD_ERoadLayer.DIRT;
		}

		// 5. Highways & Major Arterials
		if (lowerMat.Contains("dashedline") || lowerMat.Contains("road_asphalt_e_01") ||
			(lowerMat.Contains("road_asphalt_e_02") && widthM >= 7.5) ||
			(lowerMat.Contains("road_asphalt_e_03") && widthM >= 8.0) ||
			lowerRes.Contains("highway") || lowerRes.Contains("mainroad") || lowerRes.Contains("asphalt_wide") || lowerRes.Contains("wide_8m"))
		{
			return TBD_ERoadLayer.HIGHWAY;
		}

		// 6. Secondary Paved Roads
		if (lowerMat.Contains("road_asphalt") || lowerMat.Contains("cobblestone") || lowerMat.Contains("asphalt") ||
			lowerRes.Contains("road_asphalt") || lowerRes.Contains("road_paved") || lowerRes.Contains("cobblestone"))
		{
			return TBD_ERoadLayer.PAVED;
		}

		// Default fallback based on width if material not definitive
		if (widthM >= 7.5)
			return TBD_ERoadLayer.HIGHWAY;
		if (widthM >= 5.0)
			return TBD_ERoadLayer.PAVED;
		if (widthM >= 3.5)
			return TBD_ERoadLayer.DIRT;
		if (widthM >= 2.0)
			return TBD_ERoadLayer.TRACK;

		return TBD_ERoadLayer.PATH;
	}

	//------------------------------------------------------------------------------------------------
	static string LayerToSlug(TBD_ERoadLayer layer)
	{
		switch (layer)
		{
			case TBD_ERoadLayer.HIGHWAY: return "highway_paved";
			case TBD_ERoadLayer.PAVED:   return "road_paved";
			case TBD_ERoadLayer.DIRT:    return "road_dirt";
			case TBD_ERoadLayer.TRACK:   return "track";
			case TBD_ERoadLayer.PATH:    return "path";
			case TBD_ERoadLayer.RUNWAY:  return "runway";
		}
		return "road_unknown";
	}

	//------------------------------------------------------------------------------------------------
	static string LayerToFilename(TBD_ERoadLayer layer)
	{
		switch (layer)
		{
			case TBD_ERoadLayer.HIGHWAY: return "highways.json";
			case TBD_ERoadLayer.PAVED:   return "roads_paved.json";
			case TBD_ERoadLayer.DIRT:    return "roads_dirt.json";
			case TBD_ERoadLayer.TRACK:   return "tracks.json";
			case TBD_ERoadLayer.PATH:    return "paths.json";
			case TBD_ERoadLayer.RUNWAY:  return "runways.json";
		}
		return "roads_unknown.json";
	}

	//------------------------------------------------------------------------------------------------
	static string LayerToPrefix(TBD_ERoadLayer layer)
	{
		switch (layer)
		{
			case TBD_ERoadLayer.HIGHWAY: return "highway";
			case TBD_ERoadLayer.PAVED:   return "road_paved";
			case TBD_ERoadLayer.DIRT:    return "road_dirt";
			case TBD_ERoadLayer.TRACK:   return "track";
			case TBD_ERoadLayer.PATH:    return "path";
			case TBD_ERoadLayer.RUNWAY:  return "runway";
		}
		return "road";
	}
}
