/**
 * @file TBD_SpectatorController.c
 * @brief Client spectator lifecycle: decides when the local player is in spectator and owns the camera entity.
 *
 * Role: polls the local player's controlled entity every 250 ms; enters spectator (spawns
 * TBD_SpectatorCamera by type name, opens the TBD_Spectator screen) once a life is spent, and
 * leaves (restores the player's camera, deletes the spectator camera) when a living body returns,
 * applying the mission's spectatorPolicy and the spectator input accelerators.
 * Position: TBD_SpectatorComponent calls Start and Shutdown on a machine with a workspace;
 * TBD_FrameworkManager supplies the stage and the replicated spectatorPolicy; TBD_SpectatorTargets
 * answers "is this alive"; TBD_SpectatorTargeting drives follow and cycling;
 * TBD_SpectatorHostReporter sends the camera position to the server; TBD_PreSlotCamera reads
 * IsActive.
 * State: static client state: the camera, the previous camera, the active, had-life and listener
 * flags, the no-body and policy wait timers; Shutdown clears them because statics outlive a world.
 * Invariants: "am I in spectator" is answered by polling the locally authoritative controlled
 * entity, never by a replicated death flag, so it cannot drift or miss an edge and covers a player
 * who reconnects onto a spent life (NO_BODY_GRACE_MS); a streaming host reads as not alive, so
 * possessing one never makes Tick leave; the view is switched away before the camera is deleted;
 * a plain vanilla world and a stage before SAFE_START never hold a spectator camera. Under one life
 * streaming follows the controlled entity, so entities far from the host are not on this machine.
 */

//! Client spectator lifecycle. Static; one local player per process.
class TBD_SpectatorController
{
	static const int POLL_MS = 250; //!< poll period in ms; death to camera is imperceptible and the poll is cheap
	static const int NO_BODY_GRACE_MS = 20000; //!< ms in a live round with no body before spectator engages without a death seen
	static const float ENTRY_HEIGHT_M = 2.2; //!< metres above the corpse the camera starts
	static const float ENTRY_BACK_M   = 4.0; //!< metres behind the corpse the camera starts
	static const float ENTRY_PITCH_DEG = -18.0; //!< degrees; starting pitch, looking down at the corpse
	static const int OWN_SIDE_DELAY_MS = 60000; //!< ms after death before the camera appears under own_side_delayed_60s

	static const string POLICY_NONE = "none"; //!< spectatorPolicy value: stay on the death view, no camera
	static const string POLICY_OWN_SIDE = "own_side_delayed_60s"; //!< spectatorPolicy value: own side only, after OWN_SIDE_DELAY_MS
	static const string POLICY_FREE = "free"; //!< spectatorPolicy value: every side, at once

	protected static TBD_SpectatorCamera s_Camera; //!< the live spectator camera; null when not spectating
	protected static CameraBase s_PreviousCamera; //!< camera current at entry; the fallback restored on leave

	protected static bool s_bActive; //!< the local player is in spectator; default false
	protected static bool s_bHadLife; //!< a living body was seen this round, so losing it means a spent life
	protected static int s_iNoBodyMs; //!< ms spent with no body and no life seen, toward NO_BODY_GRACE_MS
	protected static int s_iPolicyWaitMs; //!< ms spent dead and not yet in spectator, toward OWN_SIDE_DELAY_MS
	protected static bool s_bLoggedNone; //!< latch: the spectatorPolicy=none line is written once

	//! Is the local player in spectator right now?
	//! @return true between Enter and Leave
	static bool IsActive()
	{
		return s_bActive;
	}

	//! @return the live spectator camera, or null when not spectating
	static TBD_SpectatorCamera GetCamera()
	{
		return s_Camera;
	}

	//! Start watching the local player: register the input accelerators and arm the repeating poll.
	//! TBD_SpectatorComponent calls it once, 2 s after init, on a machine with a workspace.
	//! @authority client
	static void Start()
	{
		TBD_SpectatorInputActions.Register();
		GetGame().GetCallqueue().Remove(Tick);
		GetGame().GetCallqueue().CallLater(Tick, POLL_MS, true);
	}

	//! Mission teardown: cancel the poll, unregister the listeners, leave spectator and clear every
	//! static, the follow state, the host reports and the target cache, because statics outlive a
	//! world inside one process.
	//! @authority client
	static void Shutdown()
	{
		GetGame().GetCallqueue().Remove(Tick);
		TBD_SpectatorInputActions.Unregister();
		Leave();

		s_bHadLife = false;
		s_iNoBodyMs = 0;
		s_iPolicyWaitMs = 0;
		s_bLoggedNone = false;
		s_PreviousCamera = null;
		TBD_SpectatorTargeting.Reset();
		TBD_SpectatorHostReporter.Reset();

		TBD_SpectatorTargets.Reset();
	}

	//! The poll: leave on a non-framework world or a stage before SAFE_START; leave when the local
	//! controlled entity is alive; otherwise apply the spectatorPolicy, keep an active spectator's
	//! follow, input ownership and host reports current, or enter once a life was spent (or after
	//! NO_BODY_GRACE_MS with no body) and the policy delay has passed. One entity lookup and one
	//! component lookup per call.
	//! @authority client
	static void Tick()
	{
		// Inert on any world that is not running the framework, so a plain vanilla world never
		// grows a spectator camera.
		if (!TBD_FrameworkManager.IsFrameworkWorld())
		{
			if (s_bActive)
				Leave();

			return;
		}

		if (!IsStageSpectatable())
		{
			if (s_bActive)
				Leave();

			s_iNoBodyMs = 0;
			return;
		}

		IEntity local = SCR_PlayerController.GetLocalControlledEntity();
		bool alive = TBD_SpectatorTargets.IsAlive(local);

		if (alive)
		{
			// Seen a living body: from here on, losing it means the life was spent.
			s_bHadLife = true;
			s_iNoBodyMs = 0;
			s_iPolicyWaitMs = 0;

			if (s_bActive)
				Leave();

			return;
		}

		SyncSpectatorPolicy();

		string policy = AuthoredPolicy();
		if (policy == POLICY_NONE)
		{
			if (s_bActive)
				Leave();

			if (!s_bLoggedNone)
			{
				Print("[TBD][spectator] spectatorPolicy=none -- staying on the death view (no spectator camera).", LogLevel.NORMAL);
				s_bLoggedNone = true;
			}

			return;
		}

		if (s_bActive)
		{
			TBD_SpectatorTargeting.ValidateFollow();
			UpdateInputOwnership();
			TBD_SpectatorHostReporter.Report(s_Camera);
			return;
		}

		bool wantsEnter = false;
		if (s_bHadLife)
		{
			wantsEnter = true;
		}
		else
		{
			s_iNoBodyMs += POLL_MS;
			if (s_iNoBodyMs >= NO_BODY_GRACE_MS)
				wantsEnter = true;
		}

		if (!wantsEnter)
			return;

		if (policy == POLICY_OWN_SIDE)
		{
			s_iPolicyWaitMs += POLL_MS;
			if (s_iPolicyWaitMs < OWN_SIDE_DELAY_MS)
				return;
		}

		Enter(local);
	}

	//! The authored spectatorPolicy, replicated onto TBD_FrameworkManager.
	//! @return the policy, or empty when the mission omits the key or the value has not arrived
	protected static string AuthoredPolicy()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return string.Empty;

		return fm.GetSpectatorPolicy();
	}

	//! Apply the policy's faction restriction to TBD_SpectatorTargets on this client: free lifts
	//! it, own side and none set it, an empty policy leaves it. The server-side mission settings do
	//! not reach this machine, because TBD_SpectatorTargets is a process-local static.
	//! @authority client
	protected static void SyncSpectatorPolicy()
	{
		string policy = AuthoredPolicy();
		if (policy == POLICY_FREE)
			TBD_SpectatorTargets.SetFactionRestricted(false);
		else if (policy == POLICY_OWN_SIDE || policy == POLICY_NONE)
			TBD_SpectatorTargets.SetFactionRestricted(true);
	}

	//! Take over the view: spawn TBD_SpectatorCamera by type name at the entry view, make it the
	//! current camera in free flight and open the roster. No-op when already active; logs an ERROR
	//! and stays out when there is no CameraManager or the camera will not spawn.
	//! @param body the corpse, or null; it only supplies the starting point
	//! @authority client
	static void Enter(IEntity body)
	{
		if (s_bActive)
			return;

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return;

		CameraManager cameras = GetGame().GetCameraManager();
		if (!cameras)
		{
			Print("[TBD][spectator] no CameraManager -- cannot enter spectator.", LogLevel.ERROR);
			return;
		}

		vector position;
		vector angles;
		ResolveEntryView(body, cameras, position, angles);

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = position;

		// Spawned by type name: no prefab, so no resourceDatabase.rdb dependency.
		IEntity spawned = GetGame().SpawnEntity(TBD_SpectatorCamera, world, params);
		s_Camera = TBD_SpectatorCamera.Cast(spawned);
		if (!s_Camera)
		{
			Print("[TBD][spectator] could not spawn TBD_SpectatorCamera.", LogLevel.ERROR);
			if (spawned)
				SCR_EntityHelper.DeleteEntityAndChildren(spawned);

			return;
		}

		s_PreviousCamera = cameras.CurrentCamera();

		s_Camera.Configure(position, angles);
		s_Camera.SetModeFree();
		cameras.SetCamera(s_Camera);

		s_bActive = true;
		TBD_SpectatorTargeting.ClearFollow();
		TBD_SpectatorHostReporter.Reset();

		Print("[TBD][spectator] entered -- one life spent, free camera live.");

		OpenRoster();
	}

	//! Hand the view back: close the roster, restore the player's own camera (else the one current
	//! at entry), then delete the spectator camera and clear the follow and host reports. Safe to
	//! call when not spectating.
	//! @authority client
	static void Leave()
	{
		if (!s_bActive && !s_Camera)
			return;

		CloseRoster();

		CameraManager cameras = GetGame().GetCameraManager();
		if (cameras)
		{
			// Restore the player's OWN camera in preference to whatever was current when we
			// entered: the camera we captured then belonged to a body that is now a corpse, and
			// after an admin respawn the player has a brand new one that is the correct answer.
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
		s_bActive = false;
		TBD_SpectatorTargeting.ClearFollow();
		TBD_SpectatorHostReporter.Reset();

		Print("[TBD][spectator] left -- back in the world.");
	}


	//! Open the TBD_Spectator roster screen. When the preset cannot resolve, TBD_MenuStack logs why
	//! and the prefab-free camera keeps working.
	static void OpenRoster()
	{
		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_Spectator);
	}

	//! Close the TBD_Spectator roster screen.
	static void CloseRoster()
	{
		TBD_MenuStack.Close(ChimeraMenuPreset.TBD_Spectator);
	}

	//! Open or close the roster; no-op when not spectating.
	static void ToggleRoster()
	{
		if (!s_bActive)
			return;

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_Spectator))
			CloseRoster();
		else
			OpenRoster();
	}


	//! Camera input stays on while the roster is open (nothing blocks), and goes off while another
	//! TBD screen stacks on top of spectator, which then owns the keyboard.
	protected static void UpdateInputOwnership()
	{
		if (!s_Camera)
			return;

		int top = TBD_MenuStack.TopPreset();
		s_Camera.SetInputEnabled(top == -1 || top == ChimeraMenuPreset.TBD_Spectator);
	}

	//! Spectator engages from SAFE_START onward: a friendly-fire death during safe start spends a
	//! life like any other. TBD_SpectatorHostLifecycle.IsStageHostable matches it.
	//! @return true when TBD_FrameworkManager exists and its stage is SAFE_START or later
	protected static bool IsStageSpectatable()
	{
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
			return false;

		return framework.GetStage() >= TBD_EGameStage.SAFE_START;
	}

	//! Where the camera starts: behind and above the corpse looking down at it when there is one,
	//! else the current camera's view, else the world origin.
	//! @param body the corpse, or null
	//! @param cameras the camera manager
	//! @param position receives the start position, world metres
	//! @param angles receives the start angles (yaw, pitch, roll), degrees
	protected static void ResolveEntryView(IEntity body, CameraManager cameras, out vector position, out vector angles)
	{
		if (body)
		{
			vector bodyTransform[4];
			body.GetWorldTransform(bodyTransform);

			position = bodyTransform[3];
			position[1] = position[1] + ENTRY_HEIGHT_M;
			position = position - bodyTransform[2] * ENTRY_BACK_M;

			angles = bodyTransform[2].VectorToAngles();
			angles[1] = ENTRY_PITCH_DEG;
			return;
		}

		CameraBase current = cameras.CurrentCamera();
		if (current)
		{
			vector cameraTransform[4];
			current.GetWorldTransform(cameraTransform);

			position = cameraTransform[3];
			angles = cameraTransform[2].VectorToAngles();
			return;
		}

		position = vector.Zero;
		angles = vector.Zero;
	}

	//! @return the local player controller's own camera, or null
	protected static CameraBase LocalPlayerCamera()
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return null;

		return controller.GetPlayerCamera();
	}
}
