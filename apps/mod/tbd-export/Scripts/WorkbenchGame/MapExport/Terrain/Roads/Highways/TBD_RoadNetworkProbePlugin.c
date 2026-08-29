/**
 * TBD_RoadNetworkProbePlugin.c
 *
 * Dedicated single-segment diagnostic test plugin for Reforger road network and AIWorld.
 *
 * Menu: Workbench > Plugins > TBD > "TEST: Probe Road Network & AIWorld"
 */

[WorkbenchPluginAttribute(
	name: "TEST: Probe Road Network & AIWorld",
	description: "Diagnostic probe: Tests ChimeraAIWorld, RoadNetworkManager, and extracts ONE single continuous road curve.",
	category: "TBD"
)]
class TBD_RoadNetworkProbePlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected static const string TAG = "[TBD-TEST]";
	protected ref array<IEntity> m_aProbeEntities;

	//------------------------------------------------------------------------------------------------
	protected bool CollectProbeEntity(IEntity e)
	{
		if (e && e.ClassName() == "RoadEntity")
		{
			m_aProbeEntities.Insert(e);
		}
		return true;
	}

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		Print("==================================================================", LogLevel.NORMAL);
		Print(TAG + " >>> STARTING ROAD NETWORK FORENSIC PROBE <<<", LogLevel.NORMAL);
		Print("==================================================================", LogLevel.NORMAL);

		// STEP 1: Export Context & World
		Print(TAG + " --- STEP 1: Checking World and Game context ---", LogLevel.NORMAL);
		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print(TAG + " ERROR: Failed to initialize TBD_MapExportContext!", LogLevel.ERROR);
			return;
		}

		Print(string.Format("%1 Map Name: '%2', World Size: %3 m", TAG, ctx.m_sMapName, ctx.m_fWorldSize), LogLevel.NORMAL);

		if (GetGame())
		{
			Print(TAG + " GetGame() is ACTIVE", LogLevel.NORMAL);
			AIWorld gwAI = GetGame().GetAIWorld();
			if (gwAI)
				Print(string.Format("%1 GetGame().GetAIWorld() found: %2", TAG, gwAI.ClassName()), LogLevel.NORMAL);
			else
				Print(TAG + " GetGame().GetAIWorld() returned NULL (normal if simulation not running).", LogLevel.NORMAL);
		}
		else
		{
			Print(TAG + " GetGame() is NULL (running in pure editor mode).", LogLevel.NORMAL);
		}

		if (ctx.m_World)
		{
			Print(TAG + " BaseWorld is ACTIVE", LogLevel.NORMAL);
		}

		// STEP 2: Recursive Editor Search for ChimeraAIWorld / SCR_AIWorld
		Print(TAG + " --- STEP 2: Searching editor entity hierarchy for AIWorld ---", LogLevel.NORMAL);
		ChimeraAIWorld aiWorld = null;
		if (GetGame() && GetGame().GetAIWorld())
		{
			aiWorld = ChimeraAIWorld.Cast(GetGame().GetAIWorld());
		}

		if (!aiWorld && ctx.m_API)
		{
			int rootCount = ctx.m_API.GetEditorEntityCount();
			Print(string.Format("%1 Searching through %2 root layer entities...", TAG, rootCount), LogLevel.NORMAL);
			for (int i = 0; i < rootCount; i++)
			{
				IEntitySource rootSrc = ctx.m_API.GetEditorEntity(i);
				if (!rootSrc)
					continue;
				aiWorld = SearchForAIWorld(rootSrc, ctx.m_API, 0);
				if (aiWorld)
					break;
			}
		}

		if (aiWorld)
			Print(string.Format("%1 SUCCESS: Resolved ChimeraAIWorld entity! (Class: %2)", TAG, aiWorld.ClassName()), LogLevel.NORMAL);
		else
			Print(TAG + " WARNING: Could not find ChimeraAIWorld in editor hierarchy.", LogLevel.WARNING);

		// STEP 3: Test RoadNetworkManager
		Print(TAG + " --- STEP 3: Testing RoadNetworkManager ---", LogLevel.NORMAL);
		RoadNetworkManager rnm = null;
		if (aiWorld)
		{
			rnm = aiWorld.GetRoadNetworkManager();
			if (rnm)
				Print(TAG + " SUCCESS: Obtained RoadNetworkManager instance!", LogLevel.NORMAL);
			else
				Print(TAG + " aiWorld.GetRoadNetworkManager() returned NULL.", LogLevel.WARNING);
		}

		// STEP 4: Query RoadEntity Waypoints from Scene
		Print(TAG + " --- STEP 4: Querying authored RoadEntity objects in scene ---", LogLevel.NORMAL);
		m_aProbeEntities = new array<IEntity>();
		ctx.m_World.QueryEntitiesByAABB(Vector(0, -500, 0), Vector(ctx.m_fWorldSize, 1500, ctx.m_fWorldSize), CollectProbeEntity);

		Print(string.Format("%1 Found %2 total RoadEntity objects in world.", TAG, m_aProbeEntities.Count()), LogLevel.NORMAL);

		// Inspect first 3 RoadEntity details
		int inspectCount = Math.Min(m_aProbeEntities.Count(), 3);
		for (int r = 0; r < inspectCount; r++)
		{
			IEntity rent = m_aProbeEntities[r];
			vector rPos = rent.GetOrigin();
			IEntitySource rSrc = ctx.m_API.EntityToSource(rent);
			string rMat = "";
			float rWidth = 0.0;
			if (rSrc)
			{
				rSrc.Get("Material", rMat);
				rSrc.Get("Width", rWidth);
			}
			Print(string.Format("%1 RoadEntity #%2: Pos=[%3, %4, %5], Width=%6m, Material='%7'",
				TAG, r + 1, rPos[0].ToString(1), rPos[1].ToString(1), rPos[2].ToString(1), rWidth, rMat), LogLevel.NORMAL);

			// If RNM is available, probe closest road for this exact entity!
			if (rnm)
			{
				BaseRoad roadSeg = null;
				float distToRoad = 0.0;
				int qRes = rnm.GetClosestRoad(rPos, roadSeg, distToRoad, true);
				Print(string.Format("%1   -> RNM.GetClosestRoad result=%2, distance=%3m", TAG, qRes, distToRoad.ToString(2)), LogLevel.NORMAL);

				if (roadSeg)
				{
					ref array<vector> pts = {};
					int numPts = roadSeg.GetPoints(pts);
					float segWidth = roadSeg.GetWidth();
					Print(string.Format("%1   -> [BASE ROAD FOUND] Width=%2m, Points=%3", TAG, segWidth, numPts), LogLevel.NORMAL);
					for (int p = 0; p < Math.Min(pts.Count(), 5); p++)
					{
						vector pt = pts[p];
						Print(string.Format("%1        Pt[%2] = [%3, %4, %5]", TAG, p, pt[0].ToString(2), pt[1].ToString(2), pt[2].ToString(2)), LogLevel.NORMAL);
					}
					if (pts.Count() > 5)
						Print(string.Format("%1        ... and %2 more vertices", TAG, pts.Count() - 5), LogLevel.NORMAL);
				}
			}
		}

		Print("==================================================================", LogLevel.NORMAL);
		Print(TAG + " >>> FORENSIC PROBE COMPLETE <<<", LogLevel.NORMAL);
		Print("==================================================================", LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected ChimeraAIWorld SearchForAIWorld(IEntitySource src, WorldEditorAPI api, int depth)
	{
		if (!src || depth > 12)
			return null;

		string cls = src.GetClassName();
		if (cls == "SCR_AIWorld" || cls == "ChimeraAIWorld" || cls == "AIWorld" || cls.Contains("AIWorld"))
		{
			IEntity ent = api.SourceToEntity(src);
			if (ent)
			{
				ChimeraAIWorld cw = ChimeraAIWorld.Cast(ent);
				if (cw)
				{
					Print(string.Format("%1 Found %2 entity in editor hierarchy (depth=%3, ID=%4)", TAG, cls, depth, src.GetID()), LogLevel.NORMAL);
					return cw;
				}
			}
		}

		int numChildren = src.GetNumChildren();
		for (int i = 0; i < numChildren; i++)
		{
			IEntitySource child = src.GetChild(i);
			ChimeraAIWorld found = SearchForAIWorld(child, api, depth + 1);
			if (found)
				return found;
		}

		return null;
	}
}
