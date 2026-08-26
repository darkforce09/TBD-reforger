/**
 * TBD_MapExportContext.c
 *
 * Encapsulates editor world access, terrain bounds querying, and entity introspection.
 */

class TBD_MapExportContext
{
	WorldEditor m_WorldEditor;
	WorldEditorAPI m_API;
	BaseWorld m_World;
	vector m_vBoundsMin;
	vector m_vBoundsMax;
	float m_fWorldSize;
	bool m_bValid;

	//------------------------------------------------------------------------------------------------
	bool Init()
	{
		m_WorldEditor = Workbench.GetModule(WorldEditor);
		if (!m_WorldEditor)
		{
			Print("[TBD][MapExport] WorldEditor module not available", LogLevel.ERROR);
			return false;
		}
		m_API = m_WorldEditor.GetApi();
		if (!m_API)
		{
			Print("[TBD][MapExport] WorldEditorAPI not available", LogLevel.ERROR);
			return false;
		}

		m_World = ResolveBaseWorld();
		if (!m_World)
		{
			Print("[TBD][MapExport] Could not resolve BaseWorld from top-level editor entities", LogLevel.ERROR);
			return false;
		}

		if (!m_WorldEditor.GetTerrainBounds(m_vBoundsMin, m_vBoundsMax))
		{
			Print("[TBD][MapExport] Warning: GetTerrainBounds returned false; using default bounds.", LogLevel.WARNING);
			m_vBoundsMin = Vector(0, -204.78, 0);
			m_vBoundsMax = Vector(12800.0, 375.53, 12800.0);
		}

		m_fWorldSize = m_vBoundsMax[0];
		if (m_vBoundsMax[2] > m_fWorldSize)
			m_fWorldSize = m_vBoundsMax[2];

		if (m_fWorldSize <= 0)
			m_fWorldSize = 12800.0;

		m_bValid = true;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected BaseWorld ResolveBaseWorld()
	{
		int rootCount = m_API.GetEditorEntityCount();
		for (int i = 0; i < rootCount; i++)
		{
			IEntitySource s = m_API.GetEditorEntity(i);
			if (!s)
				continue;
			IEntity re = m_API.SourceToEntity(s);
			if (re)
			{
				BaseWorld w = re.GetWorld();
				if (w)
					return w;
			}
		}
		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Resolves the prefab resource name of an entity (via entity prefab data or editor API ancestor container).
	static string GetEntityResourceName(IEntity ent)
	{
		if (!ent)
			return string.Empty;

		EntityPrefabData pd = ent.GetPrefabData();
		if (pd)
		{
			ResourceName pName = pd.GetPrefabName();
			if (!pName.IsEmpty())
				return pName;
		}

		WorldEditor we = Workbench.GetModule(WorldEditor);
		if (we)
		{
			WorldEditorAPI api = we.GetApi();
			if (api)
			{
				IEntitySource src = api.EntityToSource(ent);
				if (src)
				{
					BaseContainer anc = src.GetAncestor();
					if (anc)
						return anc.GetResourceName();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	string ResolvePrefab(IEntity e)
	{
		return GetEntityResourceName(e);
	}
}
