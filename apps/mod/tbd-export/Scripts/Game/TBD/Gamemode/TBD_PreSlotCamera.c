//! T-181.50 - THE PRE-SLOT CAMERA: what a player actually looks at while they wait for a slot.
//! T-181.53 - and, since the pre-slot ghost was deleted, the WHOLE of the pre-slot answer.
//!
//! -- THIS IS THE HALF THAT FIXES THE BLACK SCREEN --------------------------------------------
//! T-181.50 shipped two halves: a server-side ghost body (`TBD_PreSlotBody.c`) that gave a
//! connected-but-unslotted player something to control, and this camera. Only this one ever worked.
//! The reason it was always the load-bearing half is worth spelling out, because the two oracles
//! differ here and following either one literally would leave the defect in place:
//!
//!   * PlayableSelector's ghost is a real `SCR_ChimeraCharacter`
//!     (`PS_GameModeCoop.c:730` spawning `Prefabs/InitialPlayer_Version2.et`), so it arrives with
//!     vanilla's character camera handler attached and the player sees THROUGH IT for free. Their
//!     picker then draws an opaque background over the view
//!     (`UI/Lobby/CoopLobby.layout:5`), and `PS_PlayableControllerComponent.c:621-632` pins the
//!     current camera to the ghost's cell each frame - so what is behind their menu is empty sky
//!     100 km up, which is fine because nothing shows through. **PS GETS A CAMERA FOR FREE BECAUSE
//!     THEIR GHOST IS A CHARACTER.**
//!   * OURS was a bare `GenericEntity` (`TBD_SpectatorHostEntity`), chosen for ONE LIFE reasons
//!     argued at length in T-181.24's own header - a character can be shot, counted alive and
//!     killed, and none of those are things a lobby placeholder may be. It had NO camera handler.
//!     A player controlling it and nothing else has no view at all.
//!
//! So we owed the player a camera outright whether or not the ghost existed, and the ghost bought
//! only the server-side half of the problem. Having to supply a camera anyway, we point it somewhere
//! useful - an overlook of the terrain rather than empty sky - which is CRF's idea rather than
//! PlayableSelector's: `CRF_PlayerMenuManager.c:29-40` puts the briefing and slotting menus over a
//! slow automatic orbit of the AO centre (read as an oracle; the geometry constants below are ours).
//!
//! -- THERE IS NO PRE-SLOT BODY. IT WAS TRIED. READ THIS BEFORE BUILDING ANOTHER ONE ----------
//! A pre-slot body was written (T-181.50) and deleted (T-181.53) on the same day, on operator
//! instruction, after a live Workbench session settled it. Two facts from that run:
//!
//!   1. **THE GHOST NEVER WORKED.** It spawned its anchor 100 km up - an altitude lifted verbatim
//!      from PlayableSelector's lattice - which is OUTSIDE Everon's world bounds. The engine raised
//!      a MODAL ASSERTION DIALOG (`enf_entity.cpp:280`, "Entity out of world bounds. Type
//!      'GenericEntity'") from the ghost's spawn helper, and then refused the control transfer
//!      outright: `player=1 has NO pre-slot ghost - the engine did not transfer control to the ghost
//!      - rolled back, the player stays bodyless`. The verbatim assertion and stack are in the
//!      T-181.53 commit message; they are deliberately NOT reproduced here, so that a grep for the
//!      deleted spawn helper or its altitude constant stays a meaningful "is it really gone" check.
//!   2. **AND THE SESSION RAN END TO END ANYWAY, BODYLESS.** In the same log: `pre-slot camera UP -
//!      no body yet, overlooking <6120.51, 157.306, 6277.02>` -> picker OPEN -> `claim ok player=1
//!      slot=s1` -> `deploy player=1 result=DEPLOYED` -> `pre-slot camera DOWN - the player controls a
//!      body now` -> `[TBD][Spawn] deployed ... groundDelta=0.0114822`.
//!
//! So the ghost's value was defensive and unproven - nothing in this mod was ever observed to need
//! `GetPlayerControlledEntity` to be non-null - while its cost was a modal dialog in the middle of a
//! mission, which is fatal in a one-life event. The operator chose deletion over clamping the
//! altitude. **BODYLESS + THIS CAMERA IS THE CONFIGURATION THAT IS PROVEN TO WORK.**
//!
//! A later slice that genuinely needs a pre-slot body owes three things before writing one: place it
//! INSIDE the world bounds, show the assertion is gone in a live run rather than in a headless boot
//! (the zero-player harness never saw it), and say what it buys that this camera does not.
//!
//! -- WHY IT DEPENDS ON NOTHING ---------------------------------------------------------------
//! Deliberately: no mission data, no roster, no stage, no controlled entity, no new input resource
//! and no widget. Three reasons, all of them about not being fragile in the exact way the defect
//! was - and reason 2 is the one that made this file survive T-181.53 unchanged while the other
//! half was deleted out from under it:
//!   1. `Scripts/Game/TBD/UI/**` belongs to T-181.49 in parallel. A camera that needed the lobby to
//!      tell it anything would be a second thing that can silently fail to arm.
//!   2. IT NEVER READ THE PRE-SLOT BODY. A camera that resolved its position FROM the controlled
//!      entity would have been black on both counts: the ghost was typename-spawned and therefore
//!      server-only so a client could not see it, and since T-181.53 there is no controlled entity
//!      on any machine at all. This one does not care, and that is why deleting the server half cost
//!      it nothing.
//!   3. The terrain and the static world geometry on it are part of the world file, not network
//!      state, so an overlook renders with no replication at all. (Dynamic replicated entities near
//!      the focus may not be streamed to a client that holds no anchor entity. Stated as a known
//!      limitation, not discovered later: an empty-looking valley behind the picker is expected.)
//!
//! The single condition is "the local player controls nothing", which is precisely the state the
//! operator reported, is now the NORMAL pre-slot state rather than a defect, and needs neither
//! authority nor mission state to evaluate.
//!
//! -- WHAT A LATER SLICE WOULD CHANGE ---------------------------------------------------------
//! `ResolveFocus` uses the centre of the world bound box because that is the only focus point a
//! client can derive with zero new replication. Pointing it at the mission AO needs the slot
//! centroid on the client, which is roster data T-181.49's lobby already carries - when that lands,
//! `Configure` takes a focus point and nothing else has to move.
//!
//! -- MEASURED --------------------------------------------------------------------------------
//! Every API below is proved by `scripts/mod/compile.sh --probe` against a negative control that
//! FAILED on invented names (`BaseWorld.GetLobbyOverlookBox`, `Game.SpawnLobbyGhost`,
//! `SCR_PlayerController.ClearInitialMainEntity` - all three reported "Undefined function").
//! `BaseWorld.GetBoundBox(out vector, out vector)` is additionally already in production use in
//! `TBD_SpectatorHost.ClampToWorld`.
//!
//! MEASURED: this descriptor needs the trailing `;` - the same parser quirk `TBD_SpectatorCameraClass`
//! and `TBD_SpectatorHostEntityClass` both document. Omit it and the NEXT class fails with a
//! misleading "Syntax error / Unexpected scope".
[EntityEditorProps(category: "TBD/Framework", description: "TBD pre-slot camera - a slow overlook orbit shown to a player who is connected but has not picked a slot yet.")]
class TBD_PreSlotCameraClass : SCR_CameraBaseClass {};

//! A camera on rails. It takes NO input on purpose: the slot picker owns the keyboard and the mouse
//! while this is up, and a camera that armed an input context every frame (which is what
//! `TBD_SpectatorCamera` correctly does for free flight) would fight it. Everything it does is one
//! angle advanced per frame.
class TBD_PreSlotCamera : SCR_CameraBase
{
	//! Metres from the focus point. Far enough that a whole town reads as a place rather than as a
	//! wall of roof.
	static const float ORBIT_RADIUS_M = 800.0;

	//! Metres above the focus point. Gives a shallow-enough downward angle to keep the horizon in
	//! frame, which is what makes it read as terrain rather than as a map.
	static const float ORBIT_HEIGHT_M = 300.0;

	//! Degrees per second. Slow enough not to be a distraction behind a menu the player is reading,
	//! fast enough to prove the frame is live rather than frozen - which matters, because "frozen"
	//! and "broken" look identical and this screen exists to replace a black one.
	static const float ORBIT_DEG_PER_S = 1.5;

	//! Framing. Wider than the character default so the overlook takes in terrain rather than a
	//! hillside.
	static const float FOV_DEG = 70.0;

	protected vector m_vFocus;
	protected float m_fYaw;

	//------------------------------------------------------------------------------------------------
	void TBD_PreSlotCamera(IEntitySource src, IEntity parent)
	{
		// FRAME, not POSTFRAME. `TBD_SpectatorCamera` needs POSTFRAME because it can follow a
		// character that has already moved this frame; this one orbits a fixed point and has nothing
		// to be behind.
		SetEventMask(EntityEvent.FRAME);
		SetFlags(EntityFlags.ACTIVE, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Place the camera and start the orbit. Called once straight after spawn; the entity is spawned
	//! by typename so there is no prefab to carry defaults.
	void Configure(vector focus, float startYawDeg)
	{
		m_vFocus = focus;
		m_fYaw = startYawDeg;
		SetVerticalFOV(FOV_DEG);
		ApplyTransform();
	}

	//------------------------------------------------------------------------------------------------
	vector GetFocus()
	{
		return m_vFocus;
	}

	//------------------------------------------------------------------------------------------------
	override protected void EOnFrame(IEntity owner, float timeSlice)
	{
		if (timeSlice <= 0)
			return;

		m_fYaw = m_fYaw + ORBIT_DEG_PER_S * timeSlice;

		// Wrapped rather than left to grow: a float that has been accumulating degrees for a
		// three-hour event loses precision exactly where the orbit is smoothest.
		if (m_fYaw >= 360.0)
			m_fYaw = m_fYaw - 360.0;

		ApplyTransform();
	}

	//------------------------------------------------------------------------------------------------
	//! Sit on the orbit and look at the focus. The look angles are DERIVED from the position rather
	//! than tracked separately, so the focus can never drift out of frame no matter what the
	//! geometry constants above are set to.
	protected void ApplyTransform()
	{
		float radians = m_fYaw * Math.PI / 180.0;

		vector position;
		position[0] = m_vFocus[0] + Math.Cos(radians) * ORBIT_RADIUS_M;
		position[1] = m_vFocus[1] + ORBIT_HEIGHT_M;
		position[2] = m_vFocus[2] + Math.Sin(radians) * ORBIT_RADIUS_M;

		// `VectorToAngles` yields (yaw, pitch, 0) and `Math3D.AnglesToMatrix` consumes
		// (yaw, pitch, roll) - the exact pairing `TBD_SpectatorController.ResolveEntryView` and
		// `TBD_SpectatorCamera.ApplyTransform` already rely on, so this is the house idiom rather
		// than a fresh guess at the convention.
		vector angles = vector.Direction(position, m_vFocus).VectorToAngles();

		vector transform[4];
		Math3D.AnglesToMatrix(angles, transform);
		transform[3] = position;
		SetWorldTransform(transform);
	}
}

//! CLIENT - the lifecycle. Static for the same reason `TBD_SpectatorController` is: the thing that
//! owns it (`TBD_PreSlotComponent`) is created and destroyed with the world, and `Start`/`Shutdown`
//! are the two lines that tie this to that lifetime.
//!
//! **Statics outlive a world inside one process** (measured landmine in this codebase), which is why
//! `Shutdown` is not optional and why it hands the view back rather than just dropping the handle.
class TBD_PreSlotCameraArm
{
	//! Same cadence as `TBD_SpectatorController.POLL_MS`. One entity lookup per poll.
	static const int POLL_MS = 250;

	//! How long the local player must control NOTHING before we put a camera up.
	//!
	//! Not zero, and the reason is the ordinary case rather than the broken one: a deploy hands the
	//! body over asynchronously (vanilla's possess request -> preload -> finalize), and a world load
	//! has its own gap before the first camera exists. Entering instantly would flash an overlook
	//! into the middle of both. Three seconds is long enough to sit out a normal hand-off and short
	//! enough that a player who is genuinely waiting for the picker never reads it as a hang.
	//!
	//! Deliberately far shorter than `TBD_SpectatorController.NO_BODY_GRACE_MS` (20 s), which is
	//! answering a different question - "has this player's life quietly been spent" - and must be
	//! slow. See `Tick` for how the two are kept from fighting.
	static const int GRACE_MS = 3000;

	protected static TBD_PreSlotCamera s_Camera;
	protected static CameraBase s_PreviousCamera;
	protected static bool s_bActive;
	protected static int s_iNoBodyMs;

	//! One-shot latch so a world with a degenerate bound box reports once instead of once per poll.
	protected static bool s_bFocusFailureLogged;

	// -- Lifecycle ---------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! @authority client (and the listen host's own screen). Called by `TBD_PreSlotComponent`.
	static void Start()
	{
		if (s_bActive || s_Camera)
			return;

		s_iNoBodyMs = 0;
		s_bFocusFailureLogged = false;
		GetGame().GetCallqueue().CallLater(Tick, POLL_MS, true);
		Print("[TBD][PreSlot] pre-slot camera ARMED - a local player with no body gets an overlook instead of a black screen");
	}

	//------------------------------------------------------------------------------------------------
	static void Shutdown()
	{
		GetGame().GetCallqueue().Remove(Tick);
		Leave("shutdown");
		s_iNoBodyMs = 0;
		s_PreviousCamera = null;
		s_bFocusFailureLogged = false;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsActive()
	{
		return s_bActive;
	}

	// -- The poll ----------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! A poll rather than a hook, for the reason `TBD_SpectatorController.Tick` gives: "do I have a
	//! body" has one true answer and deriving it every quarter second cannot drift, cannot miss an
	//! edge and cannot be raced by ordering.
	static void Tick()
	{
		// Inert on any world that is not running the framework - the same guard the rest of the mod
		// uses, so a plain vanilla scenario never grows a lobby camera.
		if (!TBD_FrameworkManager.IsFrameworkWorld())
		{
			Leave("not a framework world");
			s_iNoBodyMs = 0;
			return;
		}

		// -- DO NOT FIGHT THE SPECTATOR ----------------------------------------------------------
		// `TBD_SpectatorController` owns the view of a player whose life is spent, and it reaches for
		// it via the same `CameraManager.SetCamera` seat. Two cameras taking turns once a second
		// would be worse than either alone. It wins outright: a spent life is a permanent state and
		// this one is a waiting room.
		//
		// Note the overlap is real, not theoretical - its `NO_BODY_GRACE_MS` path deliberately
		// enters spectator for a player who has had NO body in a live round, which describes a
		// slot-less player at LIVE as well as a reconnect onto a spent life. That path is gated to
		// SAFE_START and later (`IsStageSpectatable`), so in LOBBY and BRIEFING it cannot fire at
		// all; from SAFE_START on, a player who still has not picked after 20 s is handed to the
		// spectator and this camera steps aside. Not this slice's call to change - but it is why
		// this test exists rather than being assumed unnecessary.
		if (TBD_SpectatorController.IsActive())
		{
			Leave("the spectator owns the view now");
			s_iNoBodyMs = 0;
			return;
		}

		IEntity local = SCR_PlayerController.GetLocalControlledEntity();
		if (local)
		{
			// THE EXIT. A body arrived - from the picker, from an admin deploy, or from vanilla -
			// so the engine has a camera of its own again and this one must get out of the way.
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

	// -- Enter / Leave -----------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Take over the view.
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
			Print("[TBD][PreSlot] no CameraManager - cannot put up the pre-slot camera.", LogLevel.ERROR);
			return;
		}

		vector focus;
		if (!ResolveFocus(world, focus))
			return;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = focus;

		// Spawned BY TYPENAME - no prefab, therefore no `resourceDatabase.rdb` dependency, therefore
		// this works before the Workbench pass that this mod's stale rdb snapshot is waiting on.
		// Same route `TBD_SpectatorController.Enter` uses for `TBD_SpectatorCamera`.
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
		// an identical frame - cheap, and it makes a screenshot from a live test attributable.
		s_Camera.Configure(focus, LocalStartYaw());
		cameras.SetCamera(s_Camera);

		s_bActive = true;

		Print(string.Format("[TBD][PreSlot] pre-slot camera UP - no body yet, overlooking %1", focus.ToString()));
	}

	//------------------------------------------------------------------------------------------------
	//! Hand the view back. Safe to call when not active.
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

		// Switch away BEFORE deleting, never after - deleting the active camera leaves the engine
		// rendering from a dead entity.
		if (s_Camera)
		{
			SCR_EntityHelper.DeleteEntityAndChildren(s_Camera);
			s_Camera = null;
		}

		s_PreviousCamera = null;

		if (s_bActive)
			Print(string.Format("[TBD][PreSlot] pre-slot camera DOWN - %1", reason));

		s_bActive = false;
	}

	//------------------------------------------------------------------------------------------------
	//! The point the overlook orbits.
	//!
	//! The centre of the world bound box, at ground level. Not the mission AO, and not a guess at
	//! it: a client has no mission document, and inventing a focus that is wrong is worse than one
	//! that is merely generic. See the header for what a later slice would change.
	protected static bool ResolveFocus(notnull BaseWorld world, out vector focus)
	{
		vector mins;
		vector maxs;
		world.GetBoundBox(mins, maxs);

		// A degenerate box means the engine answered nothing. Refusing is right: the alternative is
		// a camera at the world origin, and `TBD_SpectatorHost.ResolveAnchor` already records why
		// 0,0,0 is never an acceptable answer (CRF shouts the same thing in
		// `CRF_EntityHelper.ZERO_SPAWN_VECTOR`).
		if (mins[0] >= maxs[0] || mins[2] >= maxs[2])
		{
			if (!s_bFocusFailureLogged)
			{
				s_bFocusFailureLogged = true;
				Print("[TBD][PreSlot] the world reported no bound box - cannot place the pre-slot camera, the player keeps whatever view they had.", LogLevel.WARNING);
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

	//------------------------------------------------------------------------------------------------
	//! A stable per-player starting bearing. Falls back to 0 when there is no controller yet, which
	//! cannot happen on the path that calls this but costs nothing to be right about.
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

	//------------------------------------------------------------------------------------------------
	protected static CameraBase LocalPlayerCamera()
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!controller)
			return null;

		return controller.GetPlayerCamera();
	}
}
