/**
 * @file TBD_PreSlotCameraArm.c
 * @brief Puts the pre-slot overlook camera up while the local player controls nothing, and takes it down.
 *
 * Role: polls every POLL_MS; once the local player has controlled nothing for GRACE_MS it spawns
 * TBD_PreSlotCamera over the centre of the world bound box; it hands the view back when the player
 * controls a body, when TBD_SpectatorController takes the view, or off a framework world.
 * Position: started and shut down by TBD_PreSlotComponent on the game mode; the camera it spawns
 * reads no mission data, roster or controlled entity, so it needs no replication.
 * State: the camera, the camera it replaced, the active flag, the bodyless time and a focus-failure
 * latch, all process-wide statics; client only.
 * Invariants: statics outlive a world, so Shutdown runs on every world teardown; the view is
 * switched away before the camera is deleted; the spectator always wins the view; a world with a
 * degenerate bound box logs one WARNING and keeps the player's view.
 */

//! Client lifecycle of the pre-slot camera; static because its owner lives and dies with the world.
class TBD_PreSlotCameraArm
{
	static const int POLL_MS = 250; //!< ms between polls, the cadence of TBD_SpectatorController.POLL_MS; one entity lookup per poll
	static const int GRACE_MS = 3000; //!< ms bodyless before the camera goes up: sits out a deploy hand-off and a world load's camera gap; far shorter than TBD_SpectatorController.NO_BODY_GRACE_MS (20 s)
	protected static TBD_PreSlotCamera s_Camera; //!< the live camera; null when down
	protected static CameraBase s_PreviousCamera; //!< the camera current at Enter; the fallback restore
	protected static bool s_bActive; //!< the camera owns the view
	protected static int s_iNoBodyMs; //!< ms the local player has controlled nothing
	protected static bool s_bFocusFailureLogged; //!< the degenerate-bound-box WARNING was logged for this arm

	//! Start polling and log `pre-slot camera ARMED`; no-op while a camera is up.
	//! @authority client
	static void Start()
	{
		if (s_bActive || s_Camera)
			return;

		s_iNoBodyMs = 0;
		s_bFocusFailureLogged = false;
		GetGame().GetCallqueue().CallLater(Tick, POLL_MS, true);
		Print("[TBD][PreSlot] pre-slot camera ARMED -- a local player with no body gets an overlook instead of a black screen");
	}

	//! Stop polling, hand the view back and clear every static; called on world teardown.
	//! @authority client
	static void Shutdown()
	{
		GetGame().GetCallqueue().Remove(Tick);
		Leave("shutdown");
		s_iNoBodyMs = 0;
		s_PreviousCamera = null;
		s_bFocusFailureLogged = false;
	}

	//! @return true while the pre-slot camera owns the view
	static bool IsActive()
	{
		return s_bActive;
	}


	//! Poll: step aside off a framework world, for the spectator, or once the player controls a body; otherwise count bodyless time and Enter after GRACE_MS. A poll cannot miss an edge or be raced by ordering.
	//! @authority client
	static void Tick()
	{
		// Inert on any world that is not running the framework -- the same guard the rest of the mod
		// uses, so a plain vanilla scenario never grows a lobby camera.
		if (!TBD_FrameworkManager.IsFrameworkWorld())
		{
			Leave("not a framework world");
			s_iNoBodyMs = 0;
			return;
		}

		// `TBD_SpectatorController` owns the view of a player whose life is spent through the same
		// `CameraManager.SetCamera` seat, and it wins outright: a spent life is permanent and this is
		// a waiting room. The overlap is real: from SAFE_START on, its `NO_BODY_GRACE_MS` path hands a
		// player who has had no body for 20 s to the spectator, and this camera steps aside.
		if (TBD_SpectatorController.IsActive())
		{
			Leave("the spectator owns the view now");
			s_iNoBodyMs = 0;
			return;
		}

		IEntity local = SCR_PlayerController.GetLocalControlledEntity();
		if (local)
		{
			// A body arrived (the picker, an admin deploy or vanilla), so the engine has a camera of
			// its own again and this one steps aside.
			Leave("the player controls a body now");
			s_iNoBodyMs = 0;
			return;
		}

		if (s_bActive)
			return;

		s_iNoBodyMs = s_iNoBodyMs + POLL_MS;
		if (s_iNoBodyMs >= GRACE_MS)
			Enter();
	}


	//! Spawn the camera over the world's centre, remember the current camera and take the view; logs `pre-slot camera UP`. Does nothing without a world, CameraManager (an ERROR) or focus.
	protected static void Enter()
	{
		if (s_bActive || s_Camera)
			return;

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return;

		CameraManager cameras = GetGame().GetCameraManager();
		if (!cameras)
		{
			Print("[TBD][PreSlot] no CameraManager -- cannot put up the pre-slot camera.", LogLevel.ERROR);
			return;
		}

		vector focus;
		if (!ResolveFocus(world, focus))
			return;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = focus;

		// Spawned by typename: no prefab, so no `resourceDatabase.rdb` entry is needed. Same route
		// `TBD_SpectatorController.Enter` uses for `TBD_SpectatorCamera`.
		IEntity spawned = GetGame().SpawnEntity(TBD_PreSlotCamera, world, params);
		s_Camera = TBD_PreSlotCamera.Cast(spawned);
		if (!s_Camera)
		{
			Print("[TBD][PreSlot] could not spawn TBD_PreSlotCamera.", LogLevel.ERROR);
			if (spawned)
				SCR_EntityHelper.DeleteEntityAndChildren(spawned);

			return;
		}

		s_PreviousCamera = cameras.CurrentCamera();

		// A per-player starting angle, so two people sitting in the same lobby are not looking at
		// an identical frame -- cheap, and it makes a screenshot from a live test attributable.
		s_Camera.Configure(focus, LocalStartYaw());
		cameras.SetCamera(s_Camera);

		s_bActive = true;

		Print(string.Format("[TBD][PreSlot] pre-slot camera UP -- no body yet, overlooking %1", focus.ToString()));
	}

	//! Hand the view back, preferring the player's own camera, then delete the camera; logs `pre-slot camera DOWN -- <reason>` when it was up. Safe to call when down.
	protected static void Leave(string reason)
	{
		if (!s_bActive && !s_Camera)
			return;

		CameraManager cameras = GetGame().GetCameraManager();
		if (cameras)
		{
			// Prefer the player's OWN camera over whatever was current when we entered: the camera
			// we captured then belonged to a player with no body, and the whole reason we are
			// leaving is usually that they now have one. Same preference and same reasoning as
			// `TBD_SpectatorController.Leave`.
			CameraBase restore = LocalPlayerCamera();
			if (!restore)
				restore = s_PreviousCamera;

			if (restore)
				cameras.SetCamera(restore);
		}

		// Switch away BEFORE deleting, never after -- deleting the active camera leaves the engine
		// rendering from a dead entity.
		if (s_Camera)
		{
			SCR_EntityHelper.DeleteEntityAndChildren(s_Camera);
			s_Camera = null;
		}

		s_PreviousCamera = null;

		if (s_bActive)
			Print(string.Format("[TBD][PreSlot] pre-slot camera DOWN -- %1", reason));

		s_bActive = false;
	}

	//! The point the overlook orbits: the centre of the world bound box at ground level, the one point a client derives without the mission document.
	//! @return false, with one WARNING, when the bound box is degenerate
	protected static bool ResolveFocus(notnull BaseWorld world, out vector focus)
	{
		vector mins;
		vector maxs;
		world.GetBoundBox(mins, maxs);

		// A degenerate box means the engine answered nothing. Refusing is right: the alternative is
		// a camera at the world origin, and `TBD_SpectatorHostLifecycle.ResolveAnchor` already records why
		// 0,0,0 is never an acceptable answer (CRF shouts the same thing in
		// `CRF_EntityHelper.ZERO_SPAWN_VECTOR`).
		if (mins[0] >= maxs[0] || mins[2] >= maxs[2])
		{
			if (!s_bFocusFailureLogged)
			{
				s_bFocusFailureLogged = true;
				Print("[TBD][PreSlot] the world reported no bound box -- cannot place the pre-slot camera, the player keeps whatever view they had.", LogLevel.WARNING);
			}

			return false;
		}

		float centreX = (mins[0] + maxs[0]) * 0.5;
		float centreZ = (mins[2] + maxs[2]) * 0.5;

		focus[0] = centreX;
		focus[1] = world.GetSurfaceY(centreX, centreZ);
		focus[2] = centreZ;
		return true;
	}

	//! @return a stable per-player starting bearing, degrees: player id times 37 modulo 360; 0 without a controller
	protected static float LocalStartYaw()
	{
		PlayerController controller = GetGame().GetPlayerController();
		if (!controller)
			return 0;

		// MEASURED: `%` is int-only in this dialect and the compiler types the expression from the
		// return type, so returning it directly from a `float` function fails with the unhelpful
		// "Unknown operator '%'". Resolve as an int, then widen.
		int yaw = (controller.GetPlayerId() * 37) % 360;
		return yaw;
	}

	//! @return the local player's own camera, or null without a controller
	protected static CameraBase LocalPlayerCamera()
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return null;

		return controller.GetPlayerCamera();
	}
}
