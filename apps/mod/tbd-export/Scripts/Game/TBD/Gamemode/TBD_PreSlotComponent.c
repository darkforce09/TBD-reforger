//! T-181.50 - where the pre-slot lifecycle is hosted.
//! T-181.53 - and, since the server half was deleted, that lifecycle is exactly ONE thing:
//! the overlook camera that replaces the operator's black screen (`TBD_PreSlotCamera` /
//! `TBD_PreSlotCameraArm`).
//!
//! T-181.50 shipped TWO halves here - a server-side ghost body (`TBD_PreSlotBody`) as well as the
//! camera. The ghost was deleted the same day: it spawned at an altitude outside the world bounds,
//! raised a MODAL engine assertion mid-mission, never transferred control, and the live session ran
//! to a successful deploy without it. The full evidence is written down in the header of
//! `TBD_PreSlotCamera.c` - read it there before reaching for a pre-slot body again.
//!
//! This class is the socket and nothing else; every decision lives in the camera.
//!
//! -- WHY A COMPONENT AND NOT A HOOK IN TBD_SpawnManager --------------------------------------
//! Two reasons, and the second is the one that changed the answer:
//!   1. `TBD_SpawnManager` is documented top-to-bottom as "@authority server - the whole manager
//!      runs server-side". What is left of this slice is a CLIENT camera and nothing else. Hanging
//!      it there would make that header a lie for the next reader - more so now than in T-181.50,
//!      when at least half of this belonged on the server.
//!   2. IT IS THE ONLY THING THE ZERO-PLAYER HARNESS CAN PROVE. `scripts/mod/world-boot.sh` boots
//!      the real scenario with no players, so it cannot exercise a single player-triggered path in
//!      this slice - but its check 2 (`WORLD (E): Unknown class`) DOES catch a component listed in
//!      `TBD_GameMode.et` whose class fails to resolve, which is otherwise dropped SILENTLY. Put the
//!      arm behind a prefab component and a green world-boot becomes real evidence that the arm
//!      exists and instantiates. Put it inside an existing class and the harness proves nothing at
//!      all about it. That is worth one small file.
//!
//! Deliberately NOT a `modded class SCR_PlayerController` lifecycle, for the reason
//! `TBD_SpectatorComponent`'s header sets out: those blocks exist in this addon purely as narrow RPC
//! transports, because the player controller is the only entity a client owns. A lifecycle on top of
//! that is a different and much wider thing and belongs on the game mode.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD pre-slot presence - the overlook camera a connected player sees instead of a black screen while they have no body.")]
class TBD_PreSlotComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_PreSlotComponent : SCR_BaseGameModeComponent
{
	//! The game mode component graph is up well before the local player has a controller, so the
	//! client arm is nudged past init rather than racing it. Nothing is lost by being late: the arm
	//! polls, so it cannot miss a player who was already bodyless when it started. Same constant and
	//! same reasoning as `TBD_SpectatorComponent.START_DELAY_MS`.
	static const int START_DELAY_MS = 2000;

	//! T-181.50 - the kill switch. One switch now, because there is one half: an operator turning
	//! this off should know they are choosing the black screen back, which is the defect this exists
	//! to fix.
	//!
	//! T-181.53 removed the second attribute (`m_bPreSlotGhost`) along with the ghost it disarmed.
	//! `TBD_GameMode.et` lists this component with an EMPTY attribute block, so it inherits both the
	//! old default and this one and needed no edit - do not read that as the prefab being unaware of
	//! the change.
	[Attribute("1", desc: "Show a slow overlook of the terrain to a local player who has no body yet. Off = a black screen behind the slot picker, which is the defect T-181.50 exists to fix.")]
	protected bool m_bPreSlotCamera;

	//------------------------------------------------------------------------------------------------
	//! @authority client (and the listen host's own screen) - see the guards below.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		if (!m_bPreSlotCamera)
		{
			Print("[TBD][PreSlot] pre-slot component INERT - the camera is switched off on this prefab, a player with no body gets a black screen.", LogLevel.WARNING);
			return;
		}

		// -- THE "AM I A MACHINE WITH A SCREEN" TEST, AND A CORRECTION --------------------------
		// MEASURED 2026-07-25 against `scripts/mod/world-boot.sh`, which boots the real scenario on
		// the native Linux dedicated server: `GetGame().GetWorkspace()` is NOT null there. The first
		// cut of this file used the workspace test alone - the idiom `TBD_SpectatorComponent` uses
		// and describes as "a dedicated server has no workspace at all (measured - see
		// TBD_UILayouts)" - and the headless boot log duly printed "pre-slot camera ARMED" on a
		// machine with no screen and no CameraManager. Adding the mode test removed that line; that
		// before/after IS the negative control for this guard.
		//
		// The claim in `TBD_SpectatorComponent` is therefore at best harness-dependent, and this is
		// the same correction T-181.49 is carrying for the lobby raise path ("replace the
		// GetWorkspace() authority test with RplSession.Mode()==RplMode.Dedicated"). Not fixed there
		// from here - that file belongs to another lane - but recorded, because the two must not end
		// up disagreeing about what "has a screen" means.
		//
		// BOTH tests, not one: `RplMode.Dedicated` is the authoritative "this build renders nothing",
		// and the workspace test still earns its place for a headless CLIENT, which is `RplMode.Client`
		// and would sail past the mode test alone.
		string screenless = string.Empty;
		if (RplSession.Mode() == RplMode.Dedicated)
			screenless = "dedicated server";
		else if (!GetGame().GetWorkspace())
			screenless = "no workspace";

		if (!screenless.IsEmpty())
		{
			// -- ONE LINE SO A SILENT COMPONENT IS NOT AN INVISIBLE ONE -------------------------
			// T-181.53 deleted the server half, and with it the `pre-slot ghost ARMED` line that
			// used to be this component's ONLY positive evidence in the headless boot log. Losing
			// it would leave a component on `TBD_GameMode.et` that, on the exact machine
			// `world-boot.sh` runs, prints nothing at all - indistinguishable from one that failed
			// to instantiate, which is precisely the failure mode reason 2 in the header says this
			// file exists to catch. `world-boot.sh`'s roll-call is a hand-maintained list and does
			// not include this component, so the log line is the evidence.
			//
			// It says what it does: nothing. A dedicated server has no screen, so there is nothing
			// for a camera to do there and no ghost to arm any more either.
			Print(string.Format("[TBD][PreSlot] pre-slot component UP but INERT here (%1) - client-only since T-181.53; nothing arms on this machine.", screenless));
			return;
		}

		GetGame().GetCallqueue().CallLater(TBD_PreSlotCameraArm.Start, START_DELAY_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (measured landmine in this codebase, and
	//! `TBD_FrameworkManager.SelectMissionByNumber` restarts the scenario in-process), so the arm
	//! MUST be torn down here or the next round starts holding a camera that belongs to a world that
	//! is gone.
	override void OnDelete(IEntity owner)
	{
		// Unconditional, unlike the arming above, and deliberately NOT re-testing the two guards:
		// a teardown that asked "did I have a screen" could answer differently from the arm (the
		// workspace can be gone by then) and would leak a camera into the next world. It is also
		// unconditional on the attribute, because that is a live switch and a shutdown that only ran
		// when it was set would leak everything if it were ever flipped off mid-session. Both calls
		// are no-ops when nothing was started.
		GetGame().GetCallqueue().Remove(TBD_PreSlotCameraArm.Start);
		TBD_PreSlotCameraArm.Shutdown();

		super.OnDelete(owner);
	}
}
