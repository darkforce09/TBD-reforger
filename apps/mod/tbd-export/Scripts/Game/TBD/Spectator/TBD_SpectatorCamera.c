//! T-181.12 - the spectator camera. Under ONE LIFE this is where a dead player spends the rest
//! of the event, so it is a first-class view, not a fallback.
//!
//! -- Why this is a reimplementation and not `SCR_ManualCamera` --------------------------------
//! Read before assuming this is NIH. `SCR_ManualCamera` was read from real vanilla source
//! (`apps/mod/vanilla_reference/Source/SCR_ManualCamera.c`), and the answer is in its own
//! declaration:
//!
//!   1. **All of its behaviour lives in a prefab.** Movement, acceleration, terrain collision and
//!      UI are `[Attribute] array<ref SCR_BaseManualCameraComponent> m_aComponents` - authored in
//!      an `.et`, dispatched by `ProcessComponents()`. Spawn the class bare and you get an inert
//!      camera that never moves. Deriving therefore forces a NEW `.et`, and an `.et` is a
//!      non-script resource, which means the SAME `resourceDatabase.rdb` blocker that already
//!      holds the menu preset. That would be a second blocked resource bought for nothing.
//!   2. **It is the editor camera.** `ManualCameraContext` + `EManualCameraFlag`, save slots,
//!      entity attach, ATL/AGL toggles, and a destructor that calls `SwitchToPreviousCamera()`.
//!      None of that is spectator behaviour and all of it is surface we would have to fight.
//!   3. **It cannot follow anything.** Half this slice is follow / first-person on a living
//!      player, which `SCR_ManualCamera` has no concept of. CRF derived from it and *still* drove
//!      `SetTransform` itself every frame for both follow modes - i.e. the base class bought them
//!      nothing for the part that matters.
//!
//! What we DO reuse is the part that is load-bearing: `SCR_CameraBase : CameraBase` for the
//! camera itself, `CameraManager.SetCamera()` for activation, `Math3D` for the transform, and -
//! crucially - vanilla's own `ManualCameraContext` input actions, so free-flight uses the keys the
//! player has already bound and needs no new input resource at all.
//!
//! -- MEASURED (probe, `compile.sh --probe=/tmp/...`, 2026-07-25) -------------------------------
//!   * `GetGame().SpawnEntity(TBD_SpectatorCamera, world, EntitySpawnParams)` compiles - a
//!     scripted camera can be spawned BY TYPENAME with **no prefab**. That is what makes the
//!     rdb-free path possible.
//!   * `SetEventMask(EntityEvent.POSTFRAME)` + `override protected void EOnPostFrame(IEntity,
//!     float)` is the correct per-frame hook (same one `SCR_ManualCamera` uses) - POSTFRAME so a
//!     followed character has already moved this frame and the camera is not one frame behind.
//!   * `CameraManager.SetCamera(CameraBase)` / `CurrentCamera()` exist; `ChimeraWorld` has NO
//!     `GetCameraManager` - it is on `Game`.
//!   * `CharacterHeadAimingComponent.GetAimingDirectionWorld()` exists - real first person, not a
//!     body-yaw approximation.
//!
//! -- Not compile-provable, needs the operator's eyes -----------------------------------------
//! Nothing here returns a framebuffer. Feel - speed, acceleration ramp, orbit distance, mouse
//! sensitivity - is tuned from constants at the top of this file and can only be judged live.
//! MEASURED: this descriptor needs the trailing `;` that `SCR_BaseGameModeComponentClass`
//! descriptors elsewhere in the mod do without - omit it and the parser mis-associates the next
//! class ("Syntax error / Unexpected scope"). Vanilla's own SCR_ManualCameraClass writes it too.
[EntityEditorProps(category: "TBD/Spectator", description: "TBD spectator camera")]
class TBD_SpectatorCameraClass : SCR_CameraBaseClass {};

//! Free flight, third-person follow, or first person through a living player's eyes.
class TBD_SpectatorCamera : SCR_CameraBase
{
	// -- Vanilla input. These action names are NOT guesses: they are the retail
	// `ManualCameraContext` vocabulary, and every one of them is already bound in the player's
	// control settings because Game Master uses this camera. Using them means free flight needs
	// ZERO new input resources and therefore works before any Workbench pass.
	static const string CTX_CAMERA      = "ManualCameraContext";

	//! TBD's own accelerators (roster / cycle / view / free). Unlike the camera actions above these
	//! ARE new resources and share the menu preset's Workbench dependency - which is exactly why
	//! every one of them is also a click in the roster screen.
	static const string CTX_SPECTATOR   = "TBD_SpectatorContext";

	static const string ACT_LATERAL     = "ManualCameraMoveLateral";
	static const string ACT_LONGITUDINAL= "ManualCameraMoveLongitudinal";
	static const string ACT_VERTICAL    = "ManualCameraMoveVertical";
	static const string ACT_YAW         = "ManualCameraRotateYaw";
	static const string ACT_PITCH       = "ManualCameraRotatePitch";
	static const string ACT_SPEED       = "ManualCameraSpeedAdjust";

	// -- Feel. One place, so tuning after an operator pass is a constant edit, never a hunt. --
	static const float  BASE_SPEED_MS       = 18.0;  //!< m/s at speed scale 1.0
	static const float  SPEED_SCALE_MIN     = 0.15;
	static const float  SPEED_SCALE_MAX     = 12.0;
	static const float  SPEED_SCALE_STEP    = 0.15;  //!< per unit of scroll
	static const float  ACCEL_SECONDS       = 0.12;  //!< matches vanilla's acceleration component
	//! Degrees per unit of mouse delta. **SIGNED ON PURPOSE.** Whether
	//! `ManualCameraRotateYaw`/`Pitch` report positive for right/up is a runtime fact no compile
	//! can settle, and an inverted spectator camera is the first thing an operator will notice. So
	//! the correction is a single minus sign here rather than a hunt through the math - flip the
	//! sign, do not touch ReadLook().
	static const float  LOOK_SENSITIVITY_YAW   = -12.0;
	static const float  LOOK_SENSITIVITY_PITCH = -12.0;
	static const float  PITCH_LIMIT_DEG     = 88.0;

	//! Below this the camera is treated as stopped. Without it an exponential decay leaves a
	//! vanishing but non-zero velocity forever, and the view creeps for the rest of the event.
	static const float  STOP_EPSILON_MS     = 0.05;
	static const float  FLOOR_CLEARANCE_M   = 0.6;   //!< a FLOOR, not a collision jail (see Step)
	static const float  CEILING_AGL_M       = 2500.0;

	static const float  ORBIT_MIN_M         = 1.5;
	static const float  ORBIT_MAX_M         = 60.0;
	static const float  ORBIT_START_M       = 6.0;
	static const float  ORBIT_STEP_M        = 0.8;
	static const float  EYE_HEIGHT_M        = 1.62;  //!< standing eye height, first person
	static const float  FOLLOW_SMOOTH       = 14.0;  //!< higher = snappier follow

	protected TBD_ESpectatorCameraMode m_eMode = TBD_ESpectatorCameraMode.FREE;

	//! Weak - the world owns the followed entity, we only look at it. Cleared the moment it dies
	//! or leaves our streaming range, which is why every read is re-validated.
	protected IEntity m_Target;

	protected vector m_vPosition;
	protected float m_fYaw;
	protected float m_fPitch;

	protected vector m_vVelocity;
	protected float m_fSpeedScale = 1.0;
	protected float m_fOrbitDistance = ORBIT_START_M;

	//! Set false by the controller while a screen wants the keyboard for itself. Look/move are
	//! suppressed; the camera keeps rendering and keeps following, so nothing ever freezes.
	protected bool m_bInputEnabled = true;

	protected BaseWorld m_World;
	protected InputManager m_Input;

	//------------------------------------------------------------------------------------------------
	void TBD_SpectatorCamera(IEntitySource src, IEntity parent)
	{
		// POSTFRAME, not FRAME: a followed character has already been moved by the time this runs,
		// so follow mode does not trail a frame behind. Same event SCR_ManualCamera uses.
		SetEventMask(EntityEvent.POSTFRAME);
		SetFlags(EntityFlags.ACTIVE, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Place the camera and take over. Called once by TBD_SpectatorController straight after spawn;
	//! the entity is spawned by typename so there is no prefab to carry defaults.
	void Configure(vector position, vector angles)
	{
		m_World = GetGame().GetWorld();
		m_Input = GetGame().GetInputManager();

		m_vPosition = position;
		m_fYaw = angles[0];
		m_fPitch = Math.Clamp(angles[1], -PITCH_LIMIT_DEG, PITCH_LIMIT_DEG);
		m_vVelocity = vector.Zero;

		ApplyTransform();
	}

	// -- Modes -------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Fly the AO. Keeps the current position and heading, so leaving a follow does not teleport
	//! you - the camera simply stops being tethered (immediate feedback, no disorientation).
	void SetModeFree()
	{
		m_eMode = TBD_ESpectatorCameraMode.FREE;
		m_Target = null;
		m_vVelocity = vector.Zero;
	}

	//------------------------------------------------------------------------------------------------
	//! Follow an entity. `firstPerson` puts the camera at the target's eyes and aligns it with what
	//! they are actually aiming at; otherwise it orbits them at the current distance.
	//! Passing null is the same as SetModeFree() - a target that just died can never strand us.
	void SetModeFollow(IEntity target, bool firstPerson)
	{
		if (!target)
		{
			SetModeFree();
			return;
		}

		m_Target = target;

		if (firstPerson)
			m_eMode = TBD_ESpectatorCameraMode.FIRST_PERSON;
		else
			m_eMode = TBD_ESpectatorCameraMode.FOLLOW;

		m_vVelocity = vector.Zero;

		if (m_fOrbitDistance <= 0)
			m_fOrbitDistance = ORBIT_START_M;
	}

	//------------------------------------------------------------------------------------------------
	TBD_ESpectatorCameraMode GetMode()
	{
		return m_eMode;
	}

	//------------------------------------------------------------------------------------------------
	//! The entity being followed, or null in free flight (or once the target went away).
	IEntity GetTarget()
	{
		return m_Target;
	}

	//------------------------------------------------------------------------------------------------
	//! Suppress look/move without stopping the camera. Used while a text field owns the keyboard.
	void SetInputEnabled(bool enabled)
	{
		m_bInputEnabled = enabled;
		if (!enabled)
			m_vVelocity = vector.Zero;
	}

	//------------------------------------------------------------------------------------------------
	//! Current speed multiplier, for a status readout.
	float GetSpeedScale()
	{
		return m_fSpeedScale;
	}

	//------------------------------------------------------------------------------------------------
	vector GetPosition()
	{
		return m_vPosition;
	}

	// -- Frame -------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	override protected void EOnPostFrame(IEntity owner, float timeSlice)
	{
		if (timeSlice <= 0)
			return;

		// Input contexts DECAY: they must be re-armed every frame while they should be live
		// (the same measured fact TBD_MenuBase encodes). This is the only place that arms either
		// context, so the moment the camera is deleted the player gets their bindings back with
		// no explicit release to forget.
		//
		// The spectator accelerators are armed even when look/move are suppressed: a player who is
		// typing must still be able to press Escape-equivalents like "back to free camera".
		if (m_Input)
		{
			m_Input.ActivateContext(CTX_SPECTATOR);

			if (m_bInputEnabled)
				m_Input.ActivateContext(CTX_CAMERA);
		}

		// A followed entity can be destroyed or leave our streaming range between frames. Fall
		// back to free flight from wherever we are standing rather than snapping to the origin.
		if (m_eMode != TBD_ESpectatorCameraMode.FREE && !m_Target)
			SetModeFree();

		switch (m_eMode)
		{
			case TBD_ESpectatorCameraMode.FOLLOW:        StepFollow(timeSlice);      break;
			case TBD_ESpectatorCameraMode.FIRST_PERSON:  StepFirstPerson(timeSlice); break;
			default:                                     StepFree(timeSlice);        break;
		}

		ApplyTransform();
	}

	// -- Free flight -------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Fly the AO. Deliberately NO collision: a spectator that gets stuck inside a wall is worse
	//! than one that can pass through it. The one constraint is a FLOOR - the camera will not go
	//! below terrain, because under the map you cannot see anything and cannot tell which way is
	//! out. A floor you can slide along is not a jail.
	protected void StepFree(float timeSlice)
	{
		ReadLook(timeSlice);

		vector wish = vector.Zero;

		if (m_bInputEnabled && m_Input)
		{
			m_fSpeedScale = Math.Clamp(
				m_fSpeedScale + m_Input.GetActionValue(ACT_SPEED) * SPEED_SCALE_STEP,
				SPEED_SCALE_MIN, SPEED_SCALE_MAX);

			vector basis[4];
			Math3D.AnglesToMatrix(Vector(m_fYaw, m_fPitch, 0), basis);

			wish += basis[0] * m_Input.GetActionValue(ACT_LATERAL);
			wish += basis[2] * m_Input.GetActionValue(ACT_LONGITUDINAL);
			wish += vector.Up  * m_Input.GetActionValue(ACT_VERTICAL);

			if (wish.LengthSq() > 1)
				wish = wish.Normalized();
		}

		// Ramp toward the wish velocity instead of snapping to it. Same 0.12 s constant vanilla's
		// SCR_AccelerationManualCameraComponent uses, so the camera feels like the one operators
		// already know from Game Master.
		vector wishVelocity = wish * BASE_SPEED_MS * m_fSpeedScale;
		float blend = Math.Clamp(timeSlice / ACCEL_SECONDS, 0, 1);
		m_vVelocity = vector.Lerp(m_vVelocity, wishVelocity, blend);

		// The lerp decays toward zero but never reaches it, so without this the view creeps for
		// the rest of the event after the last key is released. Snap once it stops mattering.
		if (m_vVelocity.LengthSq() < STOP_EPSILON_MS * STOP_EPSILON_MS)
			m_vVelocity = vector.Zero;

		m_vPosition = m_vPosition + m_vVelocity * timeSlice;
		ClampToWorld();
	}

	// -- Follow ------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Orbit the target. Mouse drives yaw/pitch around them, scroll drives distance - the same two
	//! inputs free flight uses, so there is one thing to learn, not two.
	protected void StepFollow(float timeSlice)
	{
		ReadLook(timeSlice);

		if (m_bInputEnabled && m_Input)
		{
			m_fOrbitDistance = Math.Clamp(
				m_fOrbitDistance - m_Input.GetActionValue(ACT_SPEED) * ORBIT_STEP_M,
				ORBIT_MIN_M, ORBIT_MAX_M);
		}

		vector focus = m_Target.GetOrigin() + vector.Up * EYE_HEIGHT_M;

		vector basis[4];
		Math3D.AnglesToMatrix(Vector(m_fYaw, m_fPitch, 0), basis);

		// Sit behind the look direction, so the target stays centred no matter where you swing to.
		vector wanted = focus - basis[2] * m_fOrbitDistance;

		// Smooth in position only. Rotation is already smooth because it is raw mouse.
		float blend = Math.Clamp(timeSlice * FOLLOW_SMOOTH, 0, 1);
		m_vPosition = vector.Lerp(m_vPosition, wanted, blend);
		ClampToWorld();
	}

	//------------------------------------------------------------------------------------------------
	//! Through their eyes. `CharacterHeadAimingComponent.GetAimingDirectionWorld()` (probed) is the
	//! real look vector, so this is genuine first person and not a body-yaw approximation. If the
	//! target has no head-aiming component we fall back to its own facing, which is still correct
	//! for anything that is not a character.
	protected void StepFirstPerson(float timeSlice)
	{
		vector eye = m_Target.GetOrigin() + vector.Up * EYE_HEIGHT_M;
		m_vPosition = eye;

		vector angles;
		CharacterHeadAimingComponent aiming = CharacterHeadAimingComponent.Cast(m_Target.FindComponent(CharacterHeadAimingComponent));
		if (aiming)
		{
			angles = aiming.GetAimingDirectionWorld().VectorToAngles();
		}
		else
		{
			vector targetTransform[4];
			m_Target.GetWorldTransform(targetTransform);
			angles = targetTransform[2].VectorToAngles();
		}

		m_fYaw = angles[0];
		m_fPitch = Math.Clamp(angles[1], -PITCH_LIMIT_DEG, PITCH_LIMIT_DEG);
	}

	// -- Shared ------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Mouse look. Pitch is clamped short of vertical so the view can never invert - an inverted
	//! spectator camera is the single fastest way to lose a player's bearings.
	protected void ReadLook(float timeSlice)
	{
		if (!m_bInputEnabled || !m_Input)
			return;

		m_fYaw   += m_Input.GetActionValue(ACT_YAW)   * LOOK_SENSITIVITY_YAW;
		m_fPitch += m_Input.GetActionValue(ACT_PITCH) * LOOK_SENSITIVITY_PITCH;

		m_fPitch = Math.Clamp(m_fPitch, -PITCH_LIMIT_DEG, PITCH_LIMIT_DEG);

		// Keep yaw in a sane range so a long session cannot drift it into float mush.
		if (m_fYaw > 360)
			m_fYaw -= 360;
		else if (m_fYaw < -360)
			m_fYaw += 360;
	}

	//------------------------------------------------------------------------------------------------
	//! The floor and the ceiling. Not collision - see StepFree.
	protected void ClampToWorld()
	{
		if (!m_World)
			return;

		float ground = m_World.GetSurfaceY(m_vPosition[0], m_vPosition[2]);
		float low = ground + FLOOR_CLEARANCE_M;
		float high = ground + CEILING_AGL_M;

		if (m_vPosition[1] < low)
		{
			m_vPosition[1] = low;
			// Kill the downward component only, so sliding along the floor still works.
			if (m_vVelocity[1] < 0)
				m_vVelocity[1] = 0;
		}
		else if (m_vPosition[1] > high)
		{
			m_vPosition[1] = high;
			if (m_vVelocity[1] > 0)
				m_vVelocity[1] = 0;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void ApplyTransform()
	{
		vector transform[4];
		Math3D.AnglesToMatrix(Vector(m_fYaw, m_fPitch, 0), transform);
		transform[3] = m_vPosition;
		SetWorldTransform(transform);
	}
}

//! Which of the three views the spectator camera is running. Public because the roster screen
//! reports it and the input handler toggles it.
enum TBD_ESpectatorCameraMode
{
	FREE,          //!< fly the AO
	FOLLOW,        //!< orbit a living player
	FIRST_PERSON   //!< through their eyes
}
