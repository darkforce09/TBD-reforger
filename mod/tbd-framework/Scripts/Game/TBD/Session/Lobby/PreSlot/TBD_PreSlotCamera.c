/**
 * @file TBD_PreSlotCamera.c
 * @brief A camera on rails: a slow overlook orbit of the terrain for a player who has no body yet.
 *
 * Role: orbits a focus point at ORBIT_RADIUS_M and ORBIT_HEIGHT_M, advancing ORBIT_DEG_PER_S, always
 * looking at the focus.  Position: spawned by typename and configured by TBD_PreSlotCameraArm, which
 * hands it to the CameraManager and deletes it when the player gets a body.
 * State: the focus point and the current yaw, per entity; client only.
 * Invariants: takes no input, so the slot picker keeps keyboard and mouse; the look angles are
 * derived from the position, so the focus never leaves the frame; the yaw wraps at 360 degrees.
 * The entity class descriptor needs its trailing `;`, or the next class fails to parse.
 */

//! Entity class of TBD_PreSlotCamera.
[EntityEditorProps(category: "TBD/Framework", description: "TBD pre-slot camera -- a slow overlook orbit shown to a player who is connected but has not picked a slot yet.")]
class TBD_PreSlotCameraClass : SCR_CameraBaseClass {};

//! Overlook camera shown behind the slot picker.
class TBD_PreSlotCamera : SCR_CameraBase
{
	static const float ORBIT_RADIUS_M = 800.0; //!< metres from the focus: a whole town reads as a place, not a wall of roof
	static const float ORBIT_HEIGHT_M = 300.0; //!< metres above the focus: a shallow downward angle keeps the horizon in frame
	static const float ORBIT_DEG_PER_S = 1.5; //!< degrees per second: slow behind a menu, fast enough to show the frame is live
	static const float FOV_DEG = 70.0; //!< vertical field of view, degrees; wider than the character default
	protected vector m_vFocus; //!< world point the orbit circles
	protected float m_fYaw; //!< current orbit angle, degrees in [0, 360)

	//! Subscribe to the per-frame event.
	void TBD_PreSlotCamera(IEntitySource src, IEntity parent)
	{
		// FRAME, not POSTFRAME. `TBD_SpectatorCamera` needs POSTFRAME because it can follow a
		// character that has already moved this frame; this one orbits a fixed point and has nothing
		// to be behind.
		SetEventMask(EntityEvent.FRAME);
		SetFlags(EntityFlags.ACTIVE, true);
	}

	//! Place the camera and start the orbit; called once straight after the typename spawn, which carries no prefab defaults.
	//! @param focus the world point to orbit
	//! @param startYawDeg the starting bearing, degrees
	void Configure(vector focus, float startYawDeg)
	{
		m_vFocus = focus;
		m_fYaw = startYawDeg;
		SetVerticalFOV(FOV_DEG);
		ApplyTransform();
	}

	//! @return the world point the orbit circles
	vector GetFocus()
	{
		return m_vFocus;
	}

	//! Advance the orbit by ORBIT_DEG_PER_S times `timeSlice` seconds and re-place the camera.
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

	//! Sit on the orbit and look at the focus; the look angles are derived from the position.
	protected void ApplyTransform()
	{
		float radians = m_fYaw * Math.PI / 180.0;

		vector position;
		position[0] = m_vFocus[0] + Math.Cos(radians) * ORBIT_RADIUS_M;
		position[1] = m_vFocus[1] + ORBIT_HEIGHT_M;
		position[2] = m_vFocus[2] + Math.Sin(radians) * ORBIT_RADIUS_M;

		// `VectorToAngles` yields (yaw, pitch, 0) and `Math3D.AnglesToMatrix` consumes
		// (yaw, pitch, roll) -- the exact pairing `TBD_SpectatorController.ResolveEntryView` and
		// `TBD_SpectatorCamera.ApplyTransform` already rely on, so this is the house idiom rather
		// than a fresh guess at the convention.
		vector angles = vector.Direction(position, m_vFocus).VectorToAngles();

		vector transform[4];
		Math3D.AnglesToMatrix(angles, transform);
		transform[3] = position;
		SetWorldTransform(transform);
	}
}
