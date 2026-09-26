/**
 * @file TBD_SpectatorCamera.c
 * @brief The spectator camera: free flight, third-person follow orbit and first person through a living player's eyes.
 *
 * Role: moves and orients itself each post-frame from vanilla ManualCameraContext input (free
 * flight and orbit) or from the followed entity (first person), keeping above a terrain floor and
 * under a ceiling.
 * Position: TBD_SpectatorController spawns it by type name, configures it and makes it current;
 * TBD_SpectatorTargeting switches its mode; TBD_SpectatorHostReporter reads its position.
 * State: mode, weak target, position, yaw, pitch, velocity, speed scale, orbit distance and the
 * input-enabled flag, per instance on the client.
 * Invariants: spawned with no prefab, because SCR_ManualCamera's behaviour lives in prefab
 * components (and it cannot follow); free flight uses the retail ManualCameraContext actions, so it
 * needs no new input resource; the input contexts are re-armed every frame, so deleting the camera
 * releases them; pitch stays within +-PITCH_LIMIT_DEG, so the view never inverts; a vanished target
 * falls back to free flight in place; free flight has a floor and a ceiling but no collision.
 */

//! Editor descriptor of TBD_SpectatorCamera. The trailing semicolon is required: without it the
//! parser fails on the next class with "Syntax error / Unexpected scope".
[EntityEditorProps(category: "TBD/Spectator", description: "TBD spectator camera")]
class TBD_SpectatorCameraClass : SCR_CameraBaseClass {};

//! Free flight, third-person follow, or first person through a living player's eyes.
class TBD_SpectatorCamera : SCR_CameraBase
{
	static const string CTX_CAMERA      = "ManualCameraContext"; //!< vanilla free-flight context; its actions are already bound because Game Master uses them
	static const string CTX_SPECTATOR   = "TBD_SpectatorContext"; //!< TBD accelerator context (roster, cycle, view, free); each is also a roster click

	static const string ACT_LATERAL     = "ManualCameraMoveLateral"; //!< vanilla action: strafe
	static const string ACT_LONGITUDINAL= "ManualCameraMoveLongitudinal"; //!< vanilla action: forward and back
	static const string ACT_VERTICAL    = "ManualCameraMoveVertical"; //!< vanilla action: up and down
	static const string ACT_YAW         = "ManualCameraRotateYaw"; //!< vanilla action: mouse yaw
	static const string ACT_PITCH       = "ManualCameraRotatePitch"; //!< vanilla action: mouse pitch
	static const string ACT_SPEED       = "ManualCameraSpeedAdjust"; //!< vanilla action: scroll; speed in free flight, distance in orbit

	static const float  BASE_SPEED_MS       = 18.0;  //!< m/s at speed scale 1.0
	static const float  SPEED_SCALE_MIN     = 0.15; //!< lowest speed multiplier
	static const float  SPEED_SCALE_MAX     = 12.0; //!< highest speed multiplier
	static const float  SPEED_SCALE_STEP    = 0.15;  //!< per unit of scroll
	static const float  ACCEL_SECONDS       = 0.12;  //!< matches vanilla's acceleration component
	static const float  LOOK_SENSITIVITY_YAW   = -12.0; //!< degrees per unit of mouse delta; the sign is the one place to correct an inverted axis
	static const float  LOOK_SENSITIVITY_PITCH = -12.0; //!< degrees per unit of mouse delta; the sign is the one place to correct an inverted axis
	static const float  PITCH_LIMIT_DEG     = 88.0; //!< degrees; pitch stays short of vertical so the view never inverts

	static const float  STOP_EPSILON_MS     = 0.05; //!< m/s; below this velocity snaps to zero, so the view never creeps
	static const float  FLOOR_CLEARANCE_M   = 0.6;   //!< a FLOOR, not a collision jail (see StepFree)
	static const float  CEILING_AGL_M       = 2500.0; //!< metres above ground the camera may climb

	static const float  ORBIT_MIN_M         = 1.5; //!< closest orbit distance, metres
	static const float  ORBIT_MAX_M         = 60.0; //!< farthest orbit distance, metres
	static const float  ORBIT_START_M       = 6.0; //!< first orbit distance, metres
	static const float  ORBIT_STEP_M        = 0.8; //!< metres per unit of scroll in orbit
	static const float  EYE_HEIGHT_M        = 1.62;  //!< standing eye height, first person
	static const float  FOLLOW_SMOOTH       = 14.0;  //!< higher = snappier follow

	protected TBD_ESpectatorCameraMode m_eMode = TBD_ESpectatorCameraMode.FREE; //!< current view; default FREE
	protected IEntity m_Target; //!< followed entity, weak (the world owns it); null in free flight

	protected vector m_vPosition; //!< world metres
	protected float m_fYaw; //!< degrees
	protected float m_fPitch; //!< degrees, within +-PITCH_LIMIT_DEG

	protected vector m_vVelocity; //!< m/s, free flight
	protected float m_fSpeedScale = 1.0; //!< free-flight speed multiplier; default 1.0
	protected float m_fOrbitDistance = ORBIT_START_M; //!< metres from the followed target

	protected bool m_bInputEnabled = true; //!< false while another screen owns the keyboard; look and move are suppressed, rendering and following continue

	protected BaseWorld m_World; //!< world for the terrain floor; set by Configure
	protected InputManager m_Input; //!< input source; set by Configure

	//! Subscribe to POSTFRAME and mark the entity ACTIVE.
	//! @param src the entity source, null when spawned by type name
	//! @param parent the parent entity, normally null
	void TBD_SpectatorCamera(IEntitySource src, IEntity parent)
	{
		// POSTFRAME, not FRAME: a followed character has already moved this frame, so follow mode
		// does not trail a frame behind.
		SetEventMask(EntityEvent.POSTFRAME);
		SetFlags(EntityFlags.ACTIVE, true);
	}

	//! Place the camera. TBD_SpectatorController calls it once straight after the spawn by type
	//! name, which carries no prefab defaults.
	//! @param position the start position, world metres
	//! @param angles the start angles (yaw, pitch, roll), degrees; pitch is clamped
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


	//! Switch to free flight, keeping the current position and heading, so leaving a follow does not
	//! teleport the view.
	void SetModeFree()
	{
		m_eMode = TBD_ESpectatorCameraMode.FREE;
		m_Target = null;
		m_vVelocity = vector.Zero;
	}

	//! Follow an entity: at its eyes along its aim in first person, otherwise orbiting at the
	//! current distance. A null target is SetModeFree, so a target that just died cannot strand the view.
	//! @param target the entity to follow, or null
	//! @param firstPerson first person when true
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

	//! @return the current view mode
	TBD_ESpectatorCameraMode GetMode()
	{
		return m_eMode;
	}

	//! The entity being followed, or null in free flight (or once the target went away).
	IEntity GetTarget()
	{
		return m_Target;
	}

	//! Suppress or restore look and move without stopping the camera; suppressing also stops the
	//! current motion. TBD_SpectatorController uses it while another screen owns the keyboard.
	//! @param enabled true to take input
	void SetInputEnabled(bool enabled)
	{
		m_bInputEnabled = enabled;
		if (!enabled)
			m_vVelocity = vector.Zero;
	}

	//! Current speed multiplier, for a status readout.
	float GetSpeedScale()
	{
		return m_fSpeedScale;
	}

	//! @return the camera position, world metres
	vector GetPosition()
	{
		return m_vPosition;
	}


	//! Per frame: re-arm the input contexts, fall back to free flight when the target vanished,
	//! step the current mode and apply the transform.
	//! @param owner this entity
	//! @param timeSlice seconds since the last frame; 0 or less does nothing
	//! @authority client
	override protected void EOnPostFrame(IEntity owner, float timeSlice)
	{
		if (timeSlice <= 0)
			return;

		// Input contexts decay and must be re-armed every frame (as TBD_MenuBase does). This is the
		// only place that arms either one, so deleting the camera gives the bindings back. The
		// accelerators stay armed while look and move are suppressed.
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


	//! Free-flight step: look, speed from the scroll, a wish velocity ramped over ACCEL_SECONDS,
	//! then the floor and ceiling. No collision, since a camera stuck in a wall is worse than one
	//! that passes through it; the floor keeps it above terrain and can be slid along.
	//! @param timeSlice seconds since the last frame
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

		// Ramp toward the wish velocity with vanilla's SCR_AccelerationManualCameraComponent
		// constant, so the camera feels like the Game Master one.
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


	//! Orbit step: the mouse drives yaw and pitch around the target, the scroll drives distance,
	//! and the position eases toward the point behind the look direction.
	//! @param timeSlice seconds since the last frame
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

	//! First-person step: at the target's eye height, looking along
	//! CharacterHeadAimingComponent.GetAimingDirectionWorld, or along the target's facing when it has
	//! no head-aiming component.
	//! @param timeSlice seconds since the last frame (unused; the view is copied from the target)
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


	//! Mouse look, when input is enabled. Pitch is clamped short of vertical so the view never
	//! inverts; yaw is kept within +-360 degrees.
	//! @param timeSlice seconds since the last frame (unused; mouse deltas are per frame)
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

	//! Hold the position between FLOOR_CLEARANCE_M and CEILING_AGL_M above the terrain, cancelling
	//! only the vertical velocity that pushes past the bound. Not collision; see StepFree.
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

	//! Write the position, yaw and pitch into the entity transform.
	protected void ApplyTransform()
	{
		vector transform[4];
		Math3D.AnglesToMatrix(Vector(m_fYaw, m_fPitch, 0), transform);
		transform[3] = m_vPosition;
		SetWorldTransform(transform);
	}
}
