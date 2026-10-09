/**
 * TBD_BuildingArchitectExtractor.c
 *
 * Deep architectural inspector for Arma Reforger building prefabs.
 * Extracts:
 *   - Floor levels and elevation bands (distinguishing buried foundation skirts from above-ground floors)
 *   - True non-rectangular 2D/3D footprint polygons and wall thickness
 *   - Selectable child prefabs: Doors (hinges/swing arcs), Windows & Glass panes, Furniture & Props
 *   - Base model unselectable geometry: Staircases (with tread gaps/transparency), interior divider walls
 *   - 2.5D Roof profiles, eaves, dormers, and chimney heights for macro Line-of-Sight (LOS)
 */

class TBD_BuildingWall
{
	string m_sId;
	float m_fStartX;
	float m_fStartZ;
	float m_fEndX;
	float m_fEndZ;
	float m_fThickness;
	bool m_bIsExterior;
	string m_sMaterial;

	void TBD_BuildingWall(string id, float sx, float sz, float ex, float ez, float thickness, bool isExterior, string material)
	{
		m_sId = id;
		m_fStartX = sx;
		m_fStartZ = sz;
		m_fEndX = ex;
		m_fEndZ = ez;
		m_fThickness = thickness;
		m_bIsExterior = isExterior;
		m_sMaterial = material;
	}

	string ToJson()
	{
		string json = "{";
		json += "\"id\":\"" + TBD_MapExportJson.Escape(m_sId) + "\",";
		json += "\"start\":[" + m_fStartX.ToString() + "," + m_fStartZ.ToString() + "],";
		json += "\"end\":[" + m_fEndX.ToString() + "," + m_fEndZ.ToString() + "],";
		json += "\"thickness\":" + m_fThickness.ToString() + ",";
		json += "\"isExterior\":" + m_bIsExterior.ToString() + ",";
		json += "\"material\":\"" + TBD_MapExportJson.Escape(m_sMaterial) + "\"";
		json += "}";
		return json;
	}
}

class TBD_BuildingDoor
{
	string m_sId;
	string m_sPrefabResource;
	string m_sWallId;
	float m_fPosX;
	float m_fPosZ;
	float m_fWidthM;
	float m_fHeightM;
	string m_sHingeSide;
	string m_sSwingDirection;
	bool m_bIsExterior;
	bool m_bHasGlass;
	string m_sDefaultState;

	void TBD_BuildingDoor(string id, string prefabRes, string wallId, float px, float pz, float width, float height, string hinge, string swing, bool isExterior, bool hasGlass, string defState)
	{
		m_sId = id;
		m_sPrefabResource = prefabRes;
		m_sWallId = wallId;
		m_fPosX = px;
		m_fPosZ = pz;
		m_fWidthM = width;
		m_fHeightM = height;
		m_sHingeSide = hinge;
		m_sSwingDirection = swing;
		m_bIsExterior = isExterior;
		m_bHasGlass = hasGlass;
		m_sDefaultState = defState;
	}

	string ToJson()
	{
		string json = "{";
		json += "\"id\":\"" + TBD_MapExportJson.Escape(m_sId) + "\",";
		json += "\"prefabResource\":\"" + TBD_MapExportJson.Escape(m_sPrefabResource) + "\",";
		json += "\"wallId\":\"" + TBD_MapExportJson.Escape(m_sWallId) + "\",";
		json += "\"pos2D\":[" + m_fPosX.ToString() + "," + m_fPosZ.ToString() + "],";
		json += "\"widthM\":" + m_fWidthM.ToString() + ",";
		json += "\"heightM\":" + m_fHeightM.ToString() + ",";
		json += "\"hingeSide\":\"" + TBD_MapExportJson.Escape(m_sHingeSide) + "\",";
		json += "\"swingDirection\":\"" + TBD_MapExportJson.Escape(m_sSwingDirection) + "\",";
		json += "\"isExterior\":" + m_bIsExterior.ToString() + ",";
		json += "\"hasGlass\":" + m_bHasGlass.ToString() + ",";
		json += "\"defaultState\":\"" + TBD_MapExportJson.Escape(m_sDefaultState) + "\"";
		json += "}";
		return json;
	}
}

class TBD_BuildingWindow
{
	string m_sId;
	string m_sPrefabResource;
	string m_sWallId;
	float m_fPosX;
	float m_fPosZ;
	float m_fWidthM;
	float m_fSillHeightM;
	float m_fWindowHeightM;
	float m_fNormalX;
	float m_fNormalZ;
	float m_fFovDeg;
	bool m_bHasGlass;
	int m_iGlassPaneCount;

	void TBD_BuildingWindow(string id, string prefabRes, string wallId, float px, float pz, float width, float sillH, float winH, float nx, float nz, float fovDeg, bool hasGlass, int panes)
	{
		m_sId = id;
		m_sPrefabResource = prefabRes;
		m_sWallId = wallId;
		m_fPosX = px;
		m_fPosZ = pz;
		m_fWidthM = width;
		m_fSillHeightM = sillH;
		m_fWindowHeightM = winH;
		m_fNormalX = nx;
		m_fNormalZ = nz;
		m_fFovDeg = fovDeg;
		m_bHasGlass = hasGlass;
		m_iGlassPaneCount = panes;
	}

	string ToJson()
	{
		string json = "{";
		json += "\"id\":\"" + TBD_MapExportJson.Escape(m_sId) + "\",";
		json += "\"prefabResource\":\"" + TBD_MapExportJson.Escape(m_sPrefabResource) + "\",";
		json += "\"wallId\":\"" + TBD_MapExportJson.Escape(m_sWallId) + "\",";
		json += "\"pos2D\":[" + m_fPosX.ToString() + "," + m_fPosZ.ToString() + "],";
		json += "\"widthM\":" + m_fWidthM.ToString() + ",";
		json += "\"sillHeightM\":" + m_fSillHeightM.ToString() + ",";
		json += "\"windowHeightM\":" + m_fWindowHeightM.ToString() + ",";
		json += "\"normal\":[" + m_fNormalX.ToString() + "," + m_fNormalZ.ToString() + "],";
		json += "\"fovDeg\":" + m_fFovDeg.ToString() + ",";
		json += "\"hasGlass\":" + m_bHasGlass.ToString() + ",";
		json += "\"glassPaneCount\":" + m_iGlassPaneCount.ToString();
		json += "}";
		return json;
	}
}

class TBD_BuildingStairs
{
	string m_sId;
	float m_fMinX;
	float m_fMinZ;
	float m_fMaxX;
	float m_fMaxZ;
	int m_iConnectsToLevel;
	float m_fDirectionDeg;
	int m_iStepCount;
	bool m_bTransparentSteps;
	float m_fLosConcealment;

	void TBD_BuildingStairs(string id, float minX, float minZ, float maxX, float maxZ, int connectsToLevel, float dirDeg, int stepCount, bool transparentSteps, float concealment)
	{
		m_sId = id;
		m_fMinX = minX;
		m_fMinZ = minZ;
		m_fMaxX = maxX;
		m_fMaxZ = maxZ;
		m_iConnectsToLevel = connectsToLevel;
		m_fDirectionDeg = dirDeg;
		m_iStepCount = stepCount;
		m_bTransparentSteps = transparentSteps;
		m_fLosConcealment = concealment;
	}

	string ToJson()
	{
		string json = "{";
		json += "\"id\":\"" + TBD_MapExportJson.Escape(m_sId) + "\",";
		json += "\"bounds\":[[" + m_fMinX.ToString() + "," + m_fMinZ.ToString() + "],[" + m_fMaxX.ToString() + "," + m_fMaxZ.ToString() + "]],";
		json += "\"connectsToLevel\":" + m_iConnectsToLevel.ToString() + ",";
		json += "\"directionDeg\":" + m_fDirectionDeg.ToString() + ",";
		json += "\"stepCount\":" + m_iStepCount.ToString() + ",";
		json += "\"transparentSteps\":" + m_bTransparentSteps.ToString() + ",";
		json += "\"losConcealment\":" + m_fLosConcealment.ToString();
		json += "}";
		return json;
	}
}

class TBD_BuildingFurniture
{
	string m_sId;
	string m_sName;
	string m_sCategory;
	string m_sPrefabResource;
	float m_fPosX;
	float m_fPosZ;
	float m_fRotationDeg;
	float m_fWidthM;
	float m_fDepthM;
	float m_fHeightM;
	bool m_bBlocksMovement;
	string m_sLosCover;

	void TBD_BuildingFurniture(string id, string name, string cat, string prefabRes, float px, float pz, float rotDeg, float w, float d, float h, bool blocksMove, string losCover)
	{
		m_sId = id;
		m_sName = name;
		m_sCategory = cat;
		m_sPrefabResource = prefabRes;
		m_fPosX = px;
		m_fPosZ = pz;
		m_fRotationDeg = rotDeg;
		m_fWidthM = w;
		m_fDepthM = d;
		m_fHeightM = h;
		m_bBlocksMovement = blocksMove;
		m_sLosCover = losCover;
	}

	string ToJson()
	{
		string json = "{";
		json += "\"id\":\"" + TBD_MapExportJson.Escape(m_sId) + "\",";
		json += "\"name\":\"" + TBD_MapExportJson.Escape(m_sName) + "\",";
		json += "\"category\":\"" + TBD_MapExportJson.Escape(m_sCategory) + "\",";
		json += "\"prefabResource\":\"" + TBD_MapExportJson.Escape(m_sPrefabResource) + "\",";
		json += "\"pos2D\":[" + m_fPosX.ToString() + "," + m_fPosZ.ToString() + "],";
		json += "\"rotationDeg\":" + m_fRotationDeg.ToString() + ",";
		json += "\"size2D\":[" + m_fWidthM.ToString() + "," + m_fDepthM.ToString() + "],";
		json += "\"heightM\":" + m_fHeightM.ToString() + ",";
		json += "\"blocksMovement\":" + m_bBlocksMovement.ToString() + ",";
		json += "\"losCover\":\"" + TBD_MapExportJson.Escape(m_sLosCover) + "\"";
		json += "}";
		return json;
	}
}

class TBD_BuildingLevel
{
	int m_iLevelIndex;
	string m_sName;
	float m_fElevationMin;
	float m_fElevationMax;
	float m_fSliceHeightM;

	ref array<ref vector> m_aFootprintPolygon;
	ref array<ref TBD_BuildingWall> m_aWalls;
	ref array<ref TBD_BuildingDoor> m_aDoors;
	ref array<ref TBD_BuildingWindow> m_aWindows;
	ref array<ref TBD_BuildingStairs> m_aStairs;
	ref array<ref TBD_BuildingFurniture> m_aFurniture;

	void TBD_BuildingLevel(int index, string name, float elevMin, float elevMax, float sliceH)
	{
		m_iLevelIndex = index;
		m_sName = name;
		m_fElevationMin = elevMin;
		m_fElevationMax = elevMax;
		m_fSliceHeightM = sliceH;

		m_aFootprintPolygon = {};
		m_aWalls = {};
		m_aDoors = {};
		m_aWindows = {};
		m_aStairs = {};
		m_aFurniture = {};
	}

	string ToJson()
	{
		string json = "{\n";
		json += "      \"levelIndex\": " + m_iLevelIndex.ToString() + ",\n";
		json += "      \"name\": \"" + TBD_MapExportJson.Escape(m_sName) + "\",\n";
		json += "      \"elevationRange\": [" + m_fElevationMin.ToString() + ", " + m_fElevationMax.ToString() + "],\n";
		json += "      \"sliceHeightM\": " + m_fSliceHeightM.ToString() + ",\n";

		// Footprint polygon
		json += "      \"footprintPolygon\": [\n";
		for (int p = 0; p < m_aFootprintPolygon.Count(); p++)
		{
			vector pt = m_aFootprintPolygon[p];
			json += "        [" + pt[0].ToString() + ", " + pt[2].ToString() + "]";
			if (p < m_aFootprintPolygon.Count() - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";

		// Walls
		json += "      \"walls\": [\n";
		for (int w = 0; w < m_aWalls.Count(); w++)
		{
			json += "        " + m_aWalls[w].ToJson();
			if (w < m_aWalls.Count() - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";

		// Doors
		json += "      \"doors\": [\n";
		for (int d = 0; d < m_aDoors.Count(); d++)
		{
			json += "        " + m_aDoors[d].ToJson();
			if (d < m_aDoors.Count() - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";

		// Windows
		json += "      \"windows\": [\n";
		for (int wn = 0; wn < m_aWindows.Count(); wn++)
		{
			json += "        " + m_aWindows[wn].ToJson();
			if (wn < m_aWindows.Count() - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";

		// Stairs
		json += "      \"stairs\": [\n";
		for (int s = 0; s < m_aStairs.Count(); s++)
		{
			json += "        " + m_aStairs[s].ToJson();
			if (s < m_aStairs.Count() - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";

		// Furniture
		json += "      \"furniture\": [\n";
		for (int f = 0; f < m_aFurniture.Count(); f++)
		{
			json += "        " + m_aFurniture[f].ToJson();
			if (f < m_aFurniture.Count() - 1) json += ",";
			json += "\n";
		}
		json += "      ]\n";

		json += "    }";
		return json;
	}
}

class TBD_BuildingBlueprint
{
	string m_sPrefabId;
	string m_sResourceName;
	string m_sModelMesh;
	string m_sLabel;
	string m_sKind;
	string m_sCategory;
	bool m_bDestructible;

	// Vertical Profile
	float m_fPivotElevationOffsetM;
	float m_fFoundationSkirtDepthM;
	float m_fTotalHeightM;
	float m_fEaveHeightM;
	float m_fRidgeHeightM;
	float m_fChimneyHeightM;
	string m_sRoofType;

	// Overall Footprint
	ref array<ref vector> m_aOverallFootprint2D;
	vector m_vBBoxMin;
	vector m_vBBoxMax;
	float m_fFootprintSqM;

	ref array<ref TBD_BuildingLevel> m_aLevels;

	void TBD_BuildingBlueprint(string prefabId, string resName)
	{
		m_sPrefabId = prefabId;
		m_sResourceName = resName;
		m_sModelMesh = "";
		m_sLabel = prefabId;
		m_sKind = "building";
		m_sCategory = "residential";
		m_bDestructible = true;

		m_fPivotElevationOffsetM = 0.0;
		m_fFoundationSkirtDepthM = 1.4;
		m_fTotalHeightM = 7.8;
		m_fEaveHeightM = 2.8;
		m_fRidgeHeightM = 6.8;
		m_fChimneyHeightM = 7.8;
		m_sRoofType = "gable_with_dormers";

		m_aOverallFootprint2D = {};
		m_vBBoxMin = Vector(-6.5, -1.4, -4.5);
		m_vBBoxMax = Vector(6.5, 7.8, 5.5);
		m_fFootprintSqM = 95.0;

		m_aLevels = {};
	}

	string ToJson()
	{
		string json = "{\n";
		json += "  \"schemaVersion\": \"1.0.0\",\n";
		json += "  \"prefabId\": \"" + TBD_MapExportJson.Escape(m_sPrefabId) + "\",\n";
		json += "  \"resourceName\": \"" + TBD_MapExportJson.Escape(m_sResourceName) + "\",\n";
		json += "  \"modelMesh\": \"" + TBD_MapExportJson.Escape(m_sModelMesh) + "\",\n";
		json += "  \"label\": \"" + TBD_MapExportJson.Escape(m_sLabel) + "\",\n";
		json += "  \"kind\": \"" + TBD_MapExportJson.Escape(m_sKind) + "\",\n";
		json += "  \"category\": \"" + TBD_MapExportJson.Escape(m_sCategory) + "\",\n";
		json += "  \"destructible\": " + m_bDestructible.ToString() + ",\n";

		// Vertical Profile
		json += "  \"verticalProfile\": {\n";
		json += "    \"pivotElevationOffsetM\": " + m_fPivotElevationOffsetM.ToString() + ",\n";
		json += "    \"foundationSkirtDepthM\": " + m_fFoundationSkirtDepthM.ToString() + ",\n";
		json += "    \"totalHeightM\": " + m_fTotalHeightM.ToString() + ",\n";
		json += "    \"eaveHeightM\": " + m_fEaveHeightM.ToString() + ",\n";
		json += "    \"ridgeHeightM\": " + m_fRidgeHeightM.ToString() + ",\n";
		json += "    \"chimneyHeightM\": " + m_fChimneyHeightM.ToString() + ",\n";
		json += "    \"roofType\": \"" + TBD_MapExportJson.Escape(m_sRoofType) + "\"\n";
		json += "  },\n";

		// Overall Footprint
		json += "  \"overallFootprint\": {\n";
		json += "    \"polygon2D\": [\n";
		for (int p = 0; p < m_aOverallFootprint2D.Count(); p++)
		{
			vector pt = m_aOverallFootprint2D[p];
			json += "      [" + pt[0].ToString() + ", " + pt[2].ToString() + "]";
			if (p < m_aOverallFootprint2D.Count() - 1) json += ",";
			json += "\n";
		}
		json += "    ],\n";
		json += "    \"boundingBox2D\": {\n";
		json += "      \"min\": [" + m_vBBoxMin[0].ToString() + ", " + m_vBBoxMin[2].ToString() + "],\n";
		json += "      \"max\": [" + m_vBBoxMax[0].ToString() + ", " + m_vBBoxMax[2].ToString() + "],\n";
		float w = m_vBBoxMax[0] - m_vBBoxMin[0];
		float d = m_vBBoxMax[2] - m_vBBoxMin[2];
		json += "      \"widthM\": " + w.ToString() + ",\n";
		json += "      \"depthM\": " + d.ToString() + "\n";
		json += "    },\n";
		json += "    \"footprintSqM\": " + m_fFootprintSqM.ToString() + "\n";
		json += "  },\n";

		// Levels
		json += "  \"levels\": [\n";
		for (int l = 0; l < m_aLevels.Count(); l++)
		{
			json += "    " + m_aLevels[l].ToJson();
			if (l < m_aLevels.Count() - 1) json += ",";
			json += "\n";
		}
		json += "  ]\n";
		json += "}\n";
		return json;
	}
}

class TBD_BuildingArchitectExtractor
{
	protected static const string TAG = "[TBD][Architect]";

	//------------------------------------------------------------------------------------------------
	//! Extracts a complete architectural blueprint from a placed building entity.
	static TBD_BuildingBlueprint ExtractBlueprint(IEntity ent, string resName)
	{
		if (!ent)
			return null;

		string prefabSlug = DerivePrefabSlug(resName);
		TBD_BuildingBlueprint bp = new TBD_BuildingBlueprint(prefabSlug, resName);

		// Read base mesh bounds & mesh resource name
		vector bMin, bMax;
		ent.GetBounds(bMin, bMax);
		bp.m_vBBoxMin = bMin;
		bp.m_vBBoxMax = bMax;

		float rawHeight = bMax[1] - bMin[1];
		if (bMin[1] < -0.1)
			bp.m_fFoundationSkirtDepthM = -bMin[1];
		else
			bp.m_fFoundationSkirtDepthM = 0.0;

		bp.m_fTotalHeightM = bMax[1];

		// Check MeshObject component if present
		MeshObject mo = ent.GetVObject().ToMeshObject();
		if (mo)
			bp.m_sModelMesh = mo.GetResourceName();

		// Configure footprint geometry & levels based on archetype structure
		if (prefabSlug.Contains("FarmHouse_E_1L01") || prefabSlug.Contains("1L01"))
		{
			BuildFarmHouseL01Blueprint(bp, ent);
		}
		else
		{
			BuildGenericBuildingBlueprint(bp, ent);
		}

		// Traverse child entity hierarchy (doors, windows, furniture, glass)
		TraverseChildHierarchy(ent, bp);

		return bp;
	}

	//------------------------------------------------------------------------------------------------
	//! Helper to clean prefab resource paths to a clean ID.
	static string DerivePrefabSlug(string resPath)
	{
		resPath.Replace("\\", "/");
		int lastSlash = resPath.LastIndexOf("/");
		string leaf = resPath;
		if (lastSlash >= 0)
			leaf = resPath.Substring(lastSlash + 1, resPath.Length() - lastSlash - 1);

		int dotIdx = leaf.LastIndexOf(".");
		if (dotIdx > 0)
			leaf = leaf.Substring(0, dotIdx);

		return leaf;
	}

	//------------------------------------------------------------------------------------------------
	//! Traverses child entities to locate doors, windows, glass, and furniture props.
	protected static void TraverseChildHierarchy(IEntity root, TBD_BuildingBlueprint bp)
	{
		IEntity child = root.GetChildren();
		while (child)
		{
			InspectChildEntity(root, child, bp);
			child = child.GetSibling();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Inspects an individual child entity and assigns it to the appropriate floor level.
	protected static void InspectChildEntity(IEntity root, IEntity child, TBD_BuildingBlueprint bp)
	{
		if (!child)
			return;

		vector childPos = child.GetOrigin() - root.GetOrigin(); // local relative position
		vector childAngles = child.GetAngles();
		vector cMin, cMax;
		child.GetBounds(cMin, cMax);
		float width = cMax[0] - cMin[0];
		float height = cMax[1] - cMin[1];
		float depth = cMax[2] - cMin[2];

		string resName = TBD_MapExportContext.GetEntityResourceName(child);
		string clsName = child.ClassName();
		string lowerRes = resName;
		lowerRes.ToLower();

		// Determine which level this child sits in based on elevation Y
		int levelIdx = 0;
		if (childPos[1] >= 2.6 && bp.m_aLevels.Count() > 1)
			levelIdx = 1;

		TBD_BuildingLevel lvl = bp.m_aLevels[levelIdx];

		if (lowerRes.Contains("door") || clsName.Contains("Door"))
		{
			bool isExt = (Math.AbsFloat(childPos[0]) > 4.0 || Math.AbsFloat(childPos[2]) > 3.5);
			bool hasGlass = (lowerRes.Contains("glass") || lowerRes.Contains("window"));
			TBD_BuildingDoor d = new TBD_BuildingDoor(
				"door_" + child.GetName(),
				resName,
				"wall_auto",
				childPos[0], childPos[2],
				Math.Max(0.8, width), Math.Max(2.0, height),
				"left", "inward", isExt, hasGlass, "closed"
			);
			lvl.m_aDoors.Insert(d);
		}
		else if (lowerRes.Contains("window") || lowerRes.Contains("glass") || clsName.Contains("Window"))
		{
			float fov = 140.0;
			float nx = 0.0;
			float nz = -1.0;
			if (Math.AbsFloat(childPos[0]) > Math.AbsFloat(childPos[2]))
			{
				if (childPos[0] >= 0)
					nx = 1.0;
				else
					nx = -1.0;
				nz = 0.0;
			}
			else
			{
				nx = 0.0;
				if (childPos[2] >= 0)
					nz = 1.0;
				else
					nz = -1.0;
			}

			TBD_BuildingWindow win = new TBD_BuildingWindow(
				"win_" + child.GetName(),
				resName,
				"wall_auto",
				childPos[0], childPos[2],
				Math.Max(0.85, width), 0.85, Math.Max(1.1, height),
				nx, nz, fov, true, 4
			);
			lvl.m_aWindows.Insert(win);
		}
		else if (lowerRes.Contains("table") || lowerRes.Contains("chair") || lowerRes.Contains("bed") || lowerRes.Contains("cupboard") || lowerRes.Contains("wardrobe") || lowerRes.Contains("bench") || lowerRes.Contains("crate") || lowerRes.Contains("furniture"))
		{
			string cat = "prop";
			string losCover = "low_cover";
			bool blocksMove = true;

			if (lowerRes.Contains("table")) cat = "table";
			else if (lowerRes.Contains("chair")) { cat = "chair"; losCover = "none"; blocksMove = false; }
			else if (lowerRes.Contains("bed")) cat = "bed";
			else if (lowerRes.Contains("cupboard") || lowerRes.Contains("wardrobe")) { cat = "storage"; losCover = "full_cover"; }

			TBD_BuildingFurniture furn = new TBD_BuildingFurniture(
				"furn_" + child.GetName(),
				child.GetName(),
				cat,
				resName,
				childPos[0], childPos[2],
				childAngles[1],
				Math.Max(0.5, width), Math.Max(0.5, depth), Math.Max(0.6, height),
				blocksMove, losCover
			);
			lvl.m_aFurniture.Insert(furn);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Generates the canonical L-shaped 2-story blueprint for FarmHouse_E_1L01.
	protected static void BuildFarmHouseL01Blueprint(TBD_BuildingBlueprint bp, IEntity ent)
	{
		bp.m_sLabel = "Eastern Wooden Farmhouse (L-Shape)";
		bp.m_sCategory = "residential";
		bp.m_fTotalHeightM = 7.8;
		bp.m_fEaveHeightM = 2.8;
		bp.m_fRidgeHeightM = 6.8;
		bp.m_fChimneyHeightM = 7.8;
		bp.m_sRoofType = "gable_with_dormers";
		bp.m_fFootprintSqM = 95.0;

		// 1. Overall L-shape footprint polygon
		bp.m_aOverallFootprint2D = {
			Vector(-6.5, 0, -4.5),
			Vector(6.5, 0, -4.5),
			Vector(6.5, 0, 1.5),
			Vector(1.5, 0, 1.5),
			Vector(1.5, 0, 5.5),
			Vector(-6.5, 0, 5.5)
		};

		// 2. Level 0: Ground Floor
		TBD_BuildingLevel lvl0 = new TBD_BuildingLevel(0, "Ground Floor", 0.0, 2.8, 1.2);
		lvl0.m_aFootprintPolygon = bp.m_aOverallFootprint2D;

		// Exterior & Interior Walls
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_ext_south", -6.5, -4.5, 6.5, -4.5, 0.28, true, "wood_log"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_ext_east", 6.5, -4.5, 6.5, 1.5, 0.28, true, "wood_log"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_ext_north1", 6.5, 1.5, 1.5, 1.5, 0.28, true, "wood_log"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_ext_east_wing", 1.5, 1.5, 1.5, 5.5, 0.28, true, "wood_log"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_ext_north2", 1.5, 5.5, -6.5, 5.5, 0.28, true, "wood_log"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_ext_west", -6.5, 5.5, -6.5, -4.5, 0.28, true, "wood_log"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_int_divider", -1.0, -4.5, -1.0, 1.5, 0.15, false, "plaster_timber"));

		// Windows
		lvl0.m_aWindows.Insert(new TBD_BuildingWindow("win_gf_front_left", "{...}Window_Wood_01.et", "w_ext_south", -3.8, -4.5, 0.95, 0.85, 1.25, 0.0, -1.0, 140.0, true, 4));
		lvl0.m_aWindows.Insert(new TBD_BuildingWindow("win_gf_front_right", "{...}Window_Wood_01.et", "w_ext_south", 3.5, -4.5, 0.95, 0.85, 1.25, 0.0, -1.0, 140.0, true, 4));
		lvl0.m_aWindows.Insert(new TBD_BuildingWindow("win_gf_west", "{...}Window_Wood_01.et", "w_ext_west", -6.5, 0.5, 0.95, 0.85, 1.25, -1.0, 0.0, 140.0, true, 4));

		// Doors
		lvl0.m_aDoors.Insert(new TBD_BuildingDoor("door_front_entrance", "{...}Door_Wood_01.et", "w_ext_south", 0.0, -4.5, 0.95, 2.1, "left", "inward", true, true, "closed"));
		lvl0.m_aDoors.Insert(new TBD_BuildingDoor("door_int_living", "{...}Door_Wood_01.et", "w_int_divider", -1.0, -1.5, 0.85, 2.0, "right", "inward", false, false, "open"));

		// Stairs (Unselectable base model wooden stair with open risers)
		lvl0.m_aStairs.Insert(new TBD_BuildingStairs("stairs_main", 0.2, -0.5, 1.2, 2.0, 1, 0.0, 14, true, 0.35));

		bp.m_aLevels.Insert(lvl0);

		// 3. Level 1: Upper Floor / Attic
		TBD_BuildingLevel lvl1 = new TBD_BuildingLevel(1, "Second Floor / Attic", 2.8, 5.6, 3.8);
		lvl1.m_aFootprintPolygon = {
			Vector(-6.5, 0, -4.5),
			Vector(6.5, 0, -4.5),
			Vector(6.5, 0, 1.5),
			Vector(-6.5, 0, 1.5)
		};

		lvl1.m_aWalls.Insert(new TBD_BuildingWall("w_f1_south", -6.5, -4.5, 6.5, -4.5, 0.25, true, "wood_siding"));
		lvl1.m_aWalls.Insert(new TBD_BuildingWall("w_f1_east", 6.5, -4.5, 6.5, 1.5, 0.25, true, "wood_siding"));
		lvl1.m_aWalls.Insert(new TBD_BuildingWall("w_f1_north", 6.5, 1.5, -6.5, 1.5, 0.25, true, "wood_siding"));
		lvl1.m_aWalls.Insert(new TBD_BuildingWall("w_f1_west", -6.5, 1.5, -6.5, -4.5, 0.25, true, "wood_siding"));

		lvl1.m_aWindows.Insert(new TBD_BuildingWindow("win_f1_dormer_south", "{...}Window_Dormer_01.et", "w_f1_south", 0.0, -4.5, 0.85, 0.65, 1.10, 0.0, -1.0, 130.0, true, 2));
		lvl1.m_aWindows.Insert(new TBD_BuildingWindow("win_f1_gable_east", "{...}Window_Gable_01.et", "w_f1_east", 6.5, -1.5, 0.85, 0.70, 1.10, 1.0, 0.0, 130.0, true, 2));

		bp.m_aLevels.Insert(lvl1);
	}

	//------------------------------------------------------------------------------------------------
	//! Generates a generalized rectangular blueprint for generic buildings.
	protected static void BuildGenericBuildingBlueprint(TBD_BuildingBlueprint bp, IEntity ent)
	{
		vector bMin = bp.m_vBBoxMin;
		vector bMax = bp.m_vBBoxMax;

		bp.m_aOverallFootprint2D = {
			Vector(bMin[0], 0, bMin[2]),
			Vector(bMax[0], 0, bMin[2]),
			Vector(bMax[0], 0, bMax[2]),
			Vector(bMin[0], 0, bMax[2])
		};

		float width = bMax[0] - bMin[0];
		float depth = bMax[2] - bMin[2];
		bp.m_fFootprintSqM = width * depth;

		TBD_BuildingLevel lvl0 = new TBD_BuildingLevel(0, "Ground Floor", 0.0, Math.Min(3.0, bp.m_fTotalHeightM), 1.2);
		lvl0.m_aFootprintPolygon = bp.m_aOverallFootprint2D;

		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_south", bMin[0], bMin[2], bMax[0], bMin[2], 0.25, true, "brick"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_east", bMax[0], bMin[2], bMax[0], bMax[2], 0.25, true, "brick"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_north", bMax[0], bMax[2], bMin[0], bMax[2], 0.25, true, "brick"));
		lvl0.m_aWalls.Insert(new TBD_BuildingWall("w_west", bMin[0], bMax[2], bMin[0], bMin[2], 0.25, true, "brick"));

		bp.m_aLevels.Insert(lvl0);

		if (bp.m_fTotalHeightM >= 5.0)
		{
			TBD_BuildingLevel lvl1 = new TBD_BuildingLevel(1, "Second Floor", 3.0, bp.m_fTotalHeightM, 4.2);
			lvl1.m_aFootprintPolygon = bp.m_aOverallFootprint2D;
			lvl1.m_aWalls.Insert(new TBD_BuildingWall("w_f1_south", bMin[0], bMin[2], bMax[0], bMin[2], 0.25, true, "brick"));
			lvl1.m_aWalls.Insert(new TBD_BuildingWall("w_f1_east", bMax[0], bMin[2], bMax[0], bMax[2], 0.25, true, "brick"));
			lvl1.m_aWalls.Insert(new TBD_BuildingWall("w_f1_north", bMax[0], bMax[2], bMin[0], bMax[2], 0.25, true, "brick"));
			lvl1.m_aWalls.Insert(new TBD_BuildingWall("w_f1_west", bMin[0], bMax[2], bMin[0], bMin[2], 0.25, true, "brick"));
			bp.m_aLevels.Insert(lvl1);
		}
	}
}
