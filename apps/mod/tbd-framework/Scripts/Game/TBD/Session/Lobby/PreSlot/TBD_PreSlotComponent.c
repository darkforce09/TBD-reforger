/**
 * @file TBD_PreSlotComponent.c
 * @brief Game mode component that arms the pre-slot overlook camera on machines with a screen.
 *
 * Role: the socket of TBD_PreSlotCameraArm: starts it START_DELAY_MS after init and shuts it down
 * with the world.  Position: attached by Prefabs/Systems/TBD_GameMode.et, so a green
 * `cargo xtask mod world-boot` also proves the class resolves; every decision lives in the arm.
 * State: the m_bPreSlotCamera switch.
 * Invariants: arms only with the switch on and on a machine with a screen (not a dedicated server,
 * and with a workspace), logging a line either way; teardown is unconditional, so a switch flipped
 * or a workspace gone at teardown never leaks a camera into the next world.
 */

//! Entity class of TBD_PreSlotComponent.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD pre-slot presence -- the overlook camera a connected player sees instead of a black screen while they have no body.")]
class TBD_PreSlotComponentClass : SCR_BaseGameModeComponentClass {}

//! Pre-slot camera socket on the game mode.
class TBD_PreSlotComponent : SCR_BaseGameModeComponent
{
	static const int START_DELAY_MS = 2000; //!< ms after init before the arm starts; the arm polls, so starting late misses nothing

	[Attribute("1", desc: "Show a slow overlook of the terrain to a local player who has no body yet. Off = a black screen behind the slot picker.")]
	protected bool m_bPreSlotCamera; //!< default 1; off leaves a bodyless player on a black screen and logs a WARNING

	//! Arm TBD_PreSlotCameraArm START_DELAY_MS later on a machine with a screen; log INERT and stop when switched off (WARNING) or screenless.
	//! @authority client
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		if (!m_bPreSlotCamera)
		{
			Print("[TBD][PreSlot] pre-slot component INERT -- the camera is switched off on this prefab, a player with no body gets a black screen.", LogLevel.WARNING);
			return;
		}

		// Both tests: the mode test is the authoritative "renders nothing" (a headless dedicated server
		// still has a workspace), and the workspace test catches a headless client, which is not
		// RplMode.Dedicated.
		string screenless = string.Empty;
		if (RplSession.Mode() == RplMode.Dedicated)
			screenless = "dedicated server";
		else if (!GetGame().GetWorkspace())
			screenless = "no workspace";

		// The INERT line is this component's evidence in a headless boot log, where nothing else runs.
		if (!screenless.IsEmpty())
		{
			Print(string.Format("[TBD][PreSlot] pre-slot component UP but INERT here (%1) -- client-only; nothing arms on this machine.", screenless));
			return;
		}

		GetGame().GetCallqueue().CallLater(TBD_PreSlotCameraArm.Start, START_DELAY_MS, false);
	}

	//! Cancel a pending start and shut the arm down, unconditionally; both are no-ops when nothing started.
	override void OnDelete(IEntity owner)
	{
		GetGame().GetCallqueue().Remove(TBD_PreSlotCameraArm.Start);
		TBD_PreSlotCameraArm.Shutdown();

		super.OnDelete(owner);
	}
}
