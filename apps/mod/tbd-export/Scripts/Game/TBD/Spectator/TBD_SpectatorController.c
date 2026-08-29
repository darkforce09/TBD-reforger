//! T-181.12 - the spectator lifecycle. Owns the camera entity, decides when a player is in
//! spectator, and is the single public entry point another slice binds to.
//!
//! -- HOW A DEAD PLAYER ENTERS, EXACTLY -------------------------------------------------------
//! ```
//!  server: character dies
//!    -> SCR_BaseGameMode.OnPlayerKilled
//!    -> TBD_SpawnManager.OnPlayerKilled  ->  MarkLifeSpent(playerId)      [ONE LIFE, terminal]
//!
//!  client: TBD_SpectatorComponent (on the game mode) is polling every 250 ms
//!    -> TBD_SpectatorController.Tick()
//!    -> SCR_PlayerController.GetLocalControlledEntity()  is dead
//!    -> Enter()
//!         spawn TBD_SpectatorCamera by typename at the corpse
//!         CameraManager.SetCamera(camera)
//!         camera.SetModeFree()
//!         TBD_MenuStack.Open(ChimeraMenuPreset.TBD_Spectator)
//! ```
//!
//! -- AND HOW THEY LEAVE ----------------------------------------------------------------------
//! ```
//!  admin: "#tbd respawn <id>"
//!    -> TBD_AdminCommands  ->  TBD_SpawnManager.AdminRespawn(playerId)
//!    -> DeployPlayerEx(forceFreshBody: true, adminOverride: true)  ->  possess a fresh body
//!    -> FinishAdminRespawn(DEPLOYED)  ->  ClearLifeSpent(playerId)
//!
//!  client: the next Tick() (<=250 ms) sees a LIVING local controlled entity
//!    -> Leave()
//!         TBD_MenuStack.Close(ChimeraMenuPreset.TBD_Spectator)
//!         CameraManager.SetCamera(the player's own camera)
//!         delete the spectator camera entity
//! ```
//!
//! -- Why a poll and not a hook into TBD_SpawnManager -----------------------------------------
//! Two reasons, and only one of them is that T-181.22 owns that file this wave.
//!
//! The real reason is that "am I in spectator?" is a **client** question with a **client** answer:
//! do I control a living character right now. A server-side death flag would have to be replicated
//! to be usable here, and it would still be a second source of truth that could disagree with what
//! the client can actually see. Polling the one thing that is locally authoritative cannot drift,
//! cannot miss an edge, and cannot be raced by replication order. At 250 ms the worst case from
//! death to camera is a quarter of a second - invisible next to the death animation - and the cost
//! is one entity lookup and one component lookup per tick.
//!
//! It is also the reason this survives cases a death hook would miss: a player who reconnects
//! after their life was already spent never receives a death event at all, but they still have no
//! living body, so the grace path below still puts them in spectator.
//!
//! -- LANDMINE: entity streaming follows the CONTROLLED ENTITY, not the camera ----------------
//! Under ONE LIFE the dead player still controls their corpse, so their replication origin stays
//! where they fell. Fly the camera far enough and the world will be empty - not because the
//! camera is broken, but because those entities were never sent to this machine. That is why
//! `TBD_SpectatorTargets` reports a "not in view" count instead of pretending the roster is short.
//!
//! **T-181.24 addresses this** with `TBD_SpectatorHost`: the SERVER gives a dead player an inert,
//! damage-free `TBD_SpectatorHostEntity` to possess and this class reports the camera position to
//! it (`ReportCameraToHost` below, ~2/s, unreliable, 12 bytes), so the streaming origin travels
//! with the view. CRF reaches the same place with a physics-disabled CHARACTER; TBD deliberately
//! does not, because a character can be killed and a killed character spends a life - the full
//! argument is in the `TBD_SpectatorHostEntity` header.
//!
//! Two consequences for the code below, both load-bearing:
//!   * `TBD_SpectatorTargets.IsAlive` returns FALSE for a streaming host. Without that, a
//!     spectator whose own controlled entity became the host would be read as ALIVE by `Tick`,
//!     `Leave()` would tear the camera down, and the player would be left driving an invisible
//!     dummy with no view and no way back. It is the single most important client-side guard here.
//!   * The "not in view" count is still honest and still needed: the host improves what is streamed
//!     around the CAMERA, it does not make everything on the server visible.
class TBD_SpectatorController
{
	//! Fast enough that death -> camera is imperceptible, slow enough to be free.
	static const int POLL_MS = 250;

	//! A player with no body at all (reconnected after a spent life, or refused a deploy) has no
	//! death event to react to. After this long in a live round with nothing to control, spectator
	//! is the only honest answer - a black screen is not.
	static const int NO_BODY_GRACE_MS = 20000;

	//! Where the camera starts relative to the corpse: up and back, so the first thing a player
	//! sees is what just happened to them rather than the inside of their own head.
	static const float ENTRY_HEIGHT_M = 2.2;
	static const float ENTRY_BACK_M   = 4.0;
	static const float ENTRY_PITCH_DEG = -18.0;

	//! T-181.24 - one camera report every N ticks. 2 x 250 ms = twice a second, which is fast enough
	//! that the streaming origin never falls far behind a camera doing 18 m/s and slow enough to be
	//! free. Sent unreliable: a dropped sample is corrected by the next one, whereas a reliable
	//! channel would queue and replay stale positions after a stall.
	static const int HOST_REPORT_EVERY_TICKS = 2;

	//! Do not spend a packet on jitter. Below this the server would refuse the move anyway
	//! (`TBD_SpectatorHost.MIN_MOVE_M`), so the message is not sent in the first place.
	static const float HOST_REPORT_MIN_MOVE_M = 2.0;

	protected static TBD_SpectatorCamera s_Camera;
	protected static CameraBase s_PreviousCamera;

	protected static bool s_bActive;
	protected static bool s_bHadLife;
	protected static bool s_bListenersRegistered;
	protected static int s_iNoBodyMs;

	//! T-181.24 - camera-report bookkeeping. `s_bHostReported` exists so the FIRST report is always
	//! sent: comparing against a zeroed `s_vHostReported` would silently swallow it for anyone who
	//! died near the map origin.
	protected static int s_iHostReportTicks;
	protected static bool s_bHostReported;
	protected static vector s_vHostReported;

	//! Who we are following, or -1. Kept as a player id rather than an entity so the follow
	//! survives the target's entity being re-created under it.
	protected static int s_iFollowPlayerId = -1;
	protected static bool s_bFirstPerson;

	//! Rebuilt on demand for cycling. Not the roster screen's copy - the screen owns its own.
	protected static ref array<ref TBD_SpectatorTarget> s_aCycleTargets;

	// -- Public surface. This is what another slice binds to. --------------------------------

	//------------------------------------------------------------------------------------------------
	//! Is the local player in spectator right now?
	static bool IsActive()
	{
		return s_bActive;
	}

	//------------------------------------------------------------------------------------------------
	//! The live camera, or null when not spectating.
	static TBD_SpectatorCamera GetCamera()
	{
		return s_Camera;
	}

	//------------------------------------------------------------------------------------------------
	//! Player id currently being followed, or -1 in free flight.
	static int GetFollowedPlayerId()
	{
		if (!s_bActive || !s_Camera || s_Camera.GetMode() == TBD_ESpectatorCameraMode.FREE)
			return -1;

		return s_iFollowPlayerId;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsFirstPerson()
	{
		return s_bActive && s_Camera && s_Camera.GetMode() == TBD_ESpectatorCameraMode.FIRST_PERSON;
	}

	//------------------------------------------------------------------------------------------------
	//! One line describing what the camera is doing, for the roster's status area. Non-blocking
	//! feedback is design law - the spectator should never have to guess which mode they are in.
	static string GetStatusLine()
	{
		if (!s_bActive || !s_Camera)
			return string.Empty;

		if (s_Camera.GetMode() == TBD_ESpectatorCameraMode.FREE)
		{
			// `/ 10.0`, not `/ 10` - an integer divisor here would round x1.5 down to x1 and the
			// readout would silently stop tracking the scroll wheel.
			float speed = Math.Round(s_Camera.GetSpeedScale() * 10) / 10.0;
			return string.Format("Free camera - speed x%1", speed);
		}

		string name = PlayerName(s_iFollowPlayerId);
		if (s_Camera.GetMode() == TBD_ESpectatorCameraMode.FIRST_PERSON)
			return string.Format("First person - %1", name);

		return string.Format("Following %1 - click again for first person", name);
	}

	// -- Lifecycle ---------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Start watching the local player. Called once by TBD_SpectatorComponent on a client.
	static void Start()
	{
		RegisterListeners();
		GetGame().GetCallqueue().Remove(Tick);
		GetGame().GetCallqueue().CallLater(Tick, POLL_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Mission teardown. Everything static must be put back or the next round inherits it - statics
	//! outlive a world inside one process, which is a measured landmine in this codebase.
	static void Shutdown()
	{
		GetGame().GetCallqueue().Remove(Tick);
		UnregisterListeners();
		Leave();

		s_bHadLife = false;
		s_iNoBodyMs = 0;
		s_iFollowPlayerId = -1;
		s_bFirstPerson = false;
		s_PreviousCamera = null;
		ResetHostReporting();

		if (s_aCycleTargets)
			s_aCycleTargets.Clear();

		TBD_SpectatorTargets.Reset();
	}

	//------------------------------------------------------------------------------------------------
	//! The poll. Cheap by construction: one entity lookup, one component lookup.
	static void Tick()
	{
		// Inert on any world that is not running the framework - same guard the rest of the mod
		// uses, so a plain vanilla scenario never grows a spectator camera.
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

			if (s_bActive)
				Leave();

			return;
		}

		if (s_bActive)
		{
			ValidateFollow();
			UpdateInputOwnership();
			ReportCameraToHost();
			return;
		}

		if (s_bHadLife)
		{
			Enter(local);
			return;
		}

		// Never had a body in a live round: reconnected on a spent life, or a refused deploy.
		s_iNoBodyMs += POLL_MS;
		if (s_iNoBodyMs >= NO_BODY_GRACE_MS)
			Enter(local);
	}

	//------------------------------------------------------------------------------------------------
	//! Take over the view. `body` is the corpse (may be null) and only supplies a starting point.
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
			Print("[TBD][spectator] no CameraManager - cannot enter spectator.", LogLevel.ERROR);
			return;
		}

		vector position;
		vector angles;
		ResolveEntryView(body, cameras, position, angles);

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = position;

		// Spawned BY TYPENAME - no prefab, therefore no resourceDatabase.rdb dependency, therefore
		// the camera works before the Workbench pass the menu preset is waiting on. (Probed.)
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
		s_iFollowPlayerId = -1;
		s_bFirstPerson = false;
		ResetHostReporting();

		Print("[TBD][spectator] entered - one life spent, free camera live.");

		OpenRoster();
	}

	//------------------------------------------------------------------------------------------------
	//! Hand the view back. Safe to call when not spectating.
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

		// Switch away BEFORE deleting, never after - deleting the active camera leaves the engine
		// rendering from a dead entity.
		if (s_Camera)
		{
			SCR_EntityHelper.DeleteEntityAndChildren(s_Camera);
			s_Camera = null;
		}

		s_PreviousCamera = null;
		s_bActive = false;
		s_iFollowPlayerId = -1;
		s_bFirstPerson = false;
		ResetHostReporting();

		Print("[TBD][spectator] left - back in the world.");
	}

	// -- View control. Every one of these is reachable from the roster AND from a key. -------

	//------------------------------------------------------------------------------------------------
	//! Fly the AO. The one obvious way out of any follow.
	static void SetFree()
	{
		if (!s_Camera)
			return;

		s_Camera.SetModeFree();
		s_iFollowPlayerId = -1;
		s_bFirstPerson = false;
	}

	//------------------------------------------------------------------------------------------------
	//! Follow a player. Re-resolves the entity first, so a row that went stale between the refresh
	//! and the click cannot point the camera at a corpse - it falls back to free flight and says so.
	//! Returns false when the target is gone.
	static bool FollowPlayer(int playerId, bool firstPerson)
	{
		if (!s_Camera)
			return false;

		IEntity entity = TBD_SpectatorTargets.ResolveLivingEntity(playerId);
		if (!entity)
		{
			SetFree();
			return false;
		}

		s_iFollowPlayerId = playerId;
		s_bFirstPerson = firstPerson;
		s_Camera.SetModeFollow(entity, firstPerson);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Third person <-> first person on the current target. No-op in free flight.
	static void ToggleFirstPerson()
	{
		if (!s_Camera || s_iFollowPlayerId <= 0)
			return;

		FollowPlayer(s_iFollowPlayerId, !s_bFirstPerson);
	}

	//------------------------------------------------------------------------------------------------
	//! Step through the valid targets. `delta` is +1 or -1. From free flight this picks the first
	//! target, so one key is enough to start watching somebody.
	static void CycleTarget(int delta)
	{
		if (!s_Camera)
			return;

		if (!s_aCycleTargets)
			s_aCycleTargets = {};

		int notInView;
		TBD_SpectatorTargets.Collect(s_aCycleTargets, notInView);

		int count = s_aCycleTargets.Count();
		if (count == 0)
		{
			SetFree();
			return;
		}

		int current = -1;
		for (int i = 0; i < count; i++)
		{
			if (s_aCycleTargets[i].m_iPlayerId == s_iFollowPlayerId)
			{
				current = i;
				break;
			}
		}

		int next;
		if (current < 0)
		{
			next = 0;
		}
		else
		{
			next = current + delta;
			// Wrap by hand: Enforce's % on a negative left operand is not worth trusting here.
			if (next >= count)
				next = 0;
			else if (next < 0)
				next = count - 1;
		}

		FollowPlayer(s_aCycleTargets[next].m_iPlayerId, s_bFirstPerson);
	}

	// -- Roster screen -----------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Open the unit list. Fails soft and loudly: until Workbench regenerates the addon's
	//! resourceDatabase.rdb the preset cannot resolve, and TBD_MenuStack already logs exactly why.
	//! The camera keeps working regardless - that is the whole point of keeping it prefab-free.
	static void OpenRoster()
	{
		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_Spectator);
	}

	//------------------------------------------------------------------------------------------------
	static void CloseRoster()
	{
		TBD_MenuStack.Close(ChimeraMenuPreset.TBD_Spectator);
	}

	//------------------------------------------------------------------------------------------------
	static void ToggleRoster()
	{
		if (!s_bActive)
			return;

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_Spectator))
			CloseRoster();
		else
			OpenRoster();
	}

	// -- Internals ---------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! A followed player can die without their entity going away - a ragdoll is still an entity, so
	//! the camera's own "target vanished" guard never fires and you end up orbiting a corpse. This
	//! is the check that notices, and it is here rather than in the camera because "is that player
	//! still alive" is a roster question, not a transform question.
	protected static void ValidateFollow()
	{
		if (!s_Camera || s_Camera.GetMode() == TBD_ESpectatorCameraMode.FREE)
			return;

		if (TBD_SpectatorTargets.ResolveLivingEntity(s_iFollowPlayerId))
			return;

		SetFree();
	}

	//------------------------------------------------------------------------------------------------
	//! The camera keeps flying while the roster is open - that is deliberate (nothing blocking).
	//! But if some OTHER TBD screen stacks on top of spectator, that screen owns the keyboard and
	//! the camera must stop moving under it, or a player will drive off into the AO while trying to
	//! use a menu.
	protected static void UpdateInputOwnership()
	{
		if (!s_Camera)
			return;

		int top = TBD_MenuStack.TopPreset();
		s_Camera.SetInputEnabled(top == -1 || top == ChimeraMenuPreset.TBD_Spectator);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.24 - tell the server where to keep our streaming origin.
	//!
	//! CLIENT ONLY, and deliberately fire-and-forget: this class never learns whether a host exists.
	//! That is not laziness, it is the same reasoning that made this a poll rather than a hook - the
	//! authority owns whether a dead player gets a host (`TBD_SpectatorHost.Tick`), it can withdraw
	//! one at any moment, and a client-side mirror of that decision would be a second source of
	//! truth that could disagree with it. A report that lands with no host is dropped by the
	//! authority in three lines and costs nothing.
	//!
	//! On a listen host `TBD_ReportSpectatorCamera` short-circuits straight into the authority
	//! rather than RPC-ing to itself, so both topologies run one code path.
	protected static void ReportCameraToHost()
	{
		if (!s_Camera)
			return;

		s_iHostReportTicks++;
		if (s_iHostReportTicks < HOST_REPORT_EVERY_TICKS)
			return;

		s_iHostReportTicks = 0;

		vector position = s_Camera.GetPosition();
		if (s_bHostReported && vector.Distance(position, s_vHostReported) < HOST_REPORT_MIN_MOVE_M)
			return;

		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return;

		s_bHostReported = true;
		s_vHostReported = position;
		controller.TBD_ReportSpectatorCamera(position);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.24 - forget where we last told the server we were, so the next entry always sends a
	//! first report instead of suppressing it as "no movement since last round".
	protected static void ResetHostReporting()
	{
		s_iHostReportTicks = 0;
		s_bHostReported = false;
		s_vHostReported = vector.Zero;
	}

	//------------------------------------------------------------------------------------------------
	//! Spectator engages from SAFE_START onward - a friendly-fire death during safe start spends a
	//! life exactly like any other, and the player must not be left staring at their own corpse.
	protected static bool IsStageSpectatable()
	{
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
			return false;

		return framework.GetStage() >= TBD_EGameStage.SAFE_START;
	}

	//------------------------------------------------------------------------------------------------
	//! Where the camera starts. Behind and above the corpse looking down at it if we have one;
	//! otherwise wherever the view already is, so entering can never black-screen a player.
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

	//------------------------------------------------------------------------------------------------
	protected static CameraBase LocalPlayerCamera()
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return null;

		return controller.GetPlayerCamera();
	}

	//------------------------------------------------------------------------------------------------
	protected static string PlayerName(int playerId)
	{
		if (playerId <= 0)
			return "nobody";

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return string.Format("Player %1", playerId);

		string name = players.GetPlayerName(playerId);
		if (name.IsEmpty())
			return string.Format("Player %1", playerId);

		return name;
	}

	// -- Keybinds ----------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Accelerators, not the only route: every one of these actions is also a click in the roster,
	//! so the feature is complete with a mouse alone. That matters because the action `.conf`s are
	//! non-script resources and share the menu preset's Workbench dependency, while free flight
	//! (vanilla's ManualCameraContext) does not.
	protected static void RegisterListeners()
	{
		if (s_bListenersRegistered)
			return;

		InputManager input = GetGame().GetInputManager();
		if (!input)
			return;

		input.AddActionListener("TBD_SpecRoster", EActionTrigger.DOWN, OnActionRoster);
		input.AddActionListener("TBD_SpecNext",   EActionTrigger.DOWN, OnActionNext);
		input.AddActionListener("TBD_SpecPrev",   EActionTrigger.DOWN, OnActionPrev);
		input.AddActionListener("TBD_SpecView",   EActionTrigger.DOWN, OnActionView);
		input.AddActionListener("TBD_SpecFree",   EActionTrigger.DOWN, OnActionFree);

		s_bListenersRegistered = true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void UnregisterListeners()
	{
		if (!s_bListenersRegistered)
			return;

		InputManager input = GetGame().GetInputManager();
		if (input)
		{
			input.RemoveActionListener("TBD_SpecRoster", EActionTrigger.DOWN, OnActionRoster);
			input.RemoveActionListener("TBD_SpecNext",   EActionTrigger.DOWN, OnActionNext);
			input.RemoveActionListener("TBD_SpecPrev",   EActionTrigger.DOWN, OnActionPrev);
			input.RemoveActionListener("TBD_SpecView",   EActionTrigger.DOWN, OnActionView);
			input.RemoveActionListener("TBD_SpecFree",   EActionTrigger.DOWN, OnActionFree);
		}

		s_bListenersRegistered = false;
	}

	//------------------------------------------------------------------------------------------------
	// The contexts these listeners live in are armed per-frame by TBD_SpectatorCamera.EOnPostFrame
	// - so they are live exactly while a spectator camera exists, and release themselves the
	// instant it is deleted. There is deliberately no explicit "disarm": an input context that
	// nobody re-arms is already gone.
	//------------------------------------------------------------------------------------------------
	protected static void OnActionRoster(float value, EActionTrigger trigger) { ToggleRoster(); }
	protected static void OnActionNext(float value, EActionTrigger trigger)   { if (s_bActive) CycleTarget(1); }
	protected static void OnActionPrev(float value, EActionTrigger trigger)   { if (s_bActive) CycleTarget(-1); }
	protected static void OnActionView(float value, EActionTrigger trigger)   { if (s_bActive) ToggleFirstPerson(); }
	protected static void OnActionFree(float value, EActionTrigger trigger)   { if (s_bActive) SetFree(); }
}
