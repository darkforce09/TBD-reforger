/**
 * @file TBD_BriefingMapLauncher.c
 * @brief Opens the full-screen map under the Briefing screen, frames the mission and pans to Locate targets.
 *
 * Role: opens SCR_MapEntity in FULLSCREEN mode under the screen root over four chained call-queue
 * frames, zooms out and pans to the mission centre, closes it again, and pans for Locate buttons.
 * Position: TBD_BriefingScreen creates it in OnMenuInit, calls `Open` on screen open and `Close` on
 * screen close; TBD_BriefingPage calls `LocateOnMap`.
 * State: the map entity and a weak screen reference on the client, owned by the screen.
 * Invariants: `Close` removes every queued open step before closing the map, so no step runs after
 * the screen closes; the mission centre is the controlled entity, else the game mode's origin, else
 * no pan; a Locate while the map is closed only logs.
 */

//! Full-screen map lifecycle of the Briefing screen.
class TBD_BriefingMapLauncher : Managed
{
	protected TBD_BriefingScreen m_Screen; //!< weak; the screen owns the launcher
	protected SCR_MapEntity m_MapEntity; //!< the map instance; null until one exists

	//! Bind to the screen and pick up the map instance.
	//! @param screen the owning screen, whose root hosts the map
	void TBD_BriefingMapLauncher(TBD_BriefingScreen screen)
	{
		m_Screen = screen;
		if (!m_MapEntity)
			m_MapEntity = SCR_MapEntity.GetMapInstance();
	}

	//! Queue the map open when a map instance exists.
	void Open()
	{
		if (!m_MapEntity)
			m_MapEntity = SCR_MapEntity.GetMapInstance();

		if (m_MapEntity)
			GetGame().GetCallqueue().Call(OpenMap);
	}

	//! Cancel every queued open step and close the map if it is open.
	void Close()
	{
		GetGame().GetCallqueue().Remove(OpenMap);
		GetGame().GetCallqueue().Remove(OpenMapWrap);
		GetGame().GetCallqueue().Remove(OpenMapWrapZoomChange);
		GetGame().GetCallqueue().Remove(OpenMapWrapZoomChangeWrap);

		if (m_MapEntity && m_MapEntity.IsOpen())
			m_MapEntity.CloseMap();
	}

	//! Pan the map to a world position; logs and does nothing while the map is closed.
	//! @param x world X, metres
	//! @param z world Z, metres
	void LocateOnMap(float x, float z)
	{
		if (!m_MapEntity || !m_MapEntity.IsOpen())
		{
			Print(string.Format("[TBD][briefing] locate %1 %2 -- map not open", x, z));
			return;
		}

		m_MapEntity.ZoomPanSmooth(0.3, x, z);
		Print(string.Format("[TBD][briefing] locate %1 %2", x, z));
	}

	//! First open step: defer one more frame.
	void OpenMap()
	{
		GetGame().GetCallqueue().Call(OpenMapWrap);
	}

	//! Second open step: open the map in FULLSCREEN mode under the screen root, then queue the zoom.
	void OpenMapWrap()
	{
		if (!m_MapEntity)
			m_MapEntity = SCR_MapEntity.GetMapInstance();

		if (!m_MapEntity)
			return;

		MapConfiguration mapConfigFullscreen = m_MapEntity.SetupMapConfig(
			EMapEntityMode.FULLSCREEN,
			"{1B8AC767E06A0ACD}Configs/Map/MapFullscreen.conf",
			m_Screen.GetRoot()
		);

		if (mapConfigFullscreen)
			m_MapEntity.OpenMap(mapConfigFullscreen);

		GetGame().GetCallqueue().Call(OpenMapWrapZoomChange);
	}

	//! Third open step: defer one more frame.
	void OpenMapWrapZoomChange()
	{
		GetGame().GetCallqueue().Call(OpenMapWrapZoomChangeWrap);
	}

	//! Last open step: zoom out and pan to the mission centre when there is one.
	void OpenMapWrapZoomChangeWrap()
	{
		if (!m_MapEntity)
			return;

		m_MapEntity.ZoomOut();

		vector center = GetMissionCenter();
		if (center != vector.Zero)
			m_MapEntity.ZoomPanSmooth(0.3, center[0], center[2]);
	}

	//! @return the controlled entity's origin, else the game mode's origin, else vector.Zero
	protected vector GetMissionCenter()
	{
		PlayerController controller = GetGame().GetPlayerController();
		if (controller)
		{
			IEntity playerEnt = controller.GetControlledEntity();
			if (playerEnt)
				return playerEnt.GetOrigin();
		}

		BaseGameMode gm = GetGame().GetGameMode();
		if (gm)
			return gm.GetOrigin();

		return vector.Zero;
	}
}
