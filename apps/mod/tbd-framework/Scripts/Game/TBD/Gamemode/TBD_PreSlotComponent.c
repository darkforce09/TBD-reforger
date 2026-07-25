//! T-181.50 — where the pre-slot lifecycle is hosted.
//!
//! Two halves, and this component owns both:
//!   * SERVER — the inert ghost a connected-but-unslotted player controls so the engine is never
//!     asked to run a player who controls nothing (`TBD_PreSlotBody`).
//!   * CLIENT — the overlook camera that replaces the operator's black screen
//!     (`TBD_PreSlotCamera` / `TBD_PreSlotCameraArm`).
//!
//! Both need a place that starts with the world and dies with it, and in this codebase that place is
//! a component on the game mode prefab — the seat `TBD_FrameworkManager`, `TBD_SpawnManager`,
//! `TBD_SpectatorComponent` and `TBD_LoadoutEquipComponent` already occupy. This class is the socket
//! and nothing else; every decision lives in the two managers.
//!
//! ── WHY A COMPONENT AND NOT A HOOK IN TBD_SpawnManager ──────────────────────────────────────
//! Two reasons, and the second is the one that changed the answer:
//!   1. `TBD_SpawnManager` is documented top-to-bottom as "@authority server — the whole manager
//!      runs server-side". Half of this slice is a client camera. Hanging it there would make that
//!      header a lie for the next reader.
//!   2. IT IS THE ONLY THING THE ZERO-PLAYER HARNESS CAN PROVE. `scripts/mod/world-boot.sh` boots
//!      the real scenario with no players, so it cannot exercise a single player-triggered path in
//!      this slice — but its check 2 (`WORLD (E): Unknown class`) DOES catch a component listed in
//!      `TBD_GameMode.et` whose class fails to resolve, which is otherwise dropped SILENTLY. Put the
//!      arm behind a prefab component and a green world-boot becomes real evidence that the arm
//!      exists and instantiates. Put it inside an existing class and the harness proves nothing at
//!      all about it. That is worth one small file.
//!
//! Deliberately NOT a `modded class SCR_PlayerController` lifecycle, for the reason
//! `TBD_SpectatorComponent`'s header sets out: those blocks exist in this addon purely as narrow RPC
//! transports, because the player controller is the only entity a client owns. A lifecycle on top of
//! that is a different and much wider thing and belongs on the game mode.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD pre-slot presence — an inert anchor for a player who has not picked a slot yet, and the overlook camera they see instead of a black screen.")]
class TBD_PreSlotComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_PreSlotComponent : SCR_BaseGameModeComponent
{
	//! The game mode component graph is up well before the local player has a controller, so the
	//! client arm is nudged past init rather than racing it. Nothing is lost by being late: the arm
	//! polls, so it cannot miss a player who was already bodyless when it started. Same constant and
	//! same reasoning as `TBD_SpectatorComponent.START_DELAY_MS`.
	static const int START_DELAY_MS = 2000;

	//! T-181.50 — the kill switch for the SERVER half. Assigning a main entity is an invasive thing
	//! to do to a player controller, so there is exactly one attribute that makes the ghost stand
	//! down and leaves the server behaving as it did before this slice (a waiting player controls
	//! nothing, which is what T-181.48 shipped).
	//!
	//! Note that turning this off does NOT bring the black screen back: the camera is the half that
	//! fixes that, and it has its own switch below.
	[Attribute("1", desc: "Give a connected player with no slot an inert entity to control so the server never runs a bodyless player. Off = the pre-T-181.50 behaviour; the overlook camera is unaffected.")]
	protected bool m_bPreSlotGhost;

	//! T-181.50 — the kill switch for the CLIENT half. This is the one that fixes the reported
	//! black screen, so it defaults on and an operator turning it off should know they are choosing
	//! the black screen back.
	[Attribute("1", desc: "Show a slow overlook of the terrain to a local player who has no body yet. Off = a black screen behind the slot picker, which is the defect T-181.50 exists to fix.")]
	protected bool m_bPreSlotCamera;

	//------------------------------------------------------------------------------------------------
	//! Two halves, two guards, and on a LISTEN HOST both of them fire — which is the point of
	//! testing them separately rather than with one `if/else`.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// SERVER half. Authority is the only place that may spawn or assign anything, and a
		// dedicated server reaches this line while a client never does.
		if (m_bPreSlotGhost && RplSession.Mode() != RplMode.Client)
			TBD_PreSlotBody.Start();

		if (!m_bPreSlotCamera)
			return;

		// ── CLIENT half. THE "AM I A MACHINE WITH A SCREEN" TEST, AND A CORRECTION ─────────────
		// MEASURED 2026-07-25 against `scripts/mod/world-boot.sh`, which boots the real scenario on
		// the native Linux dedicated server: `GetGame().GetWorkspace()` is NOT null there. The first
		// cut of this file used the workspace test alone — the idiom `TBD_SpectatorComponent` uses
		// and describes as "a dedicated server has no workspace at all (measured — see
		// TBD_UILayouts)" — and the headless boot log duly printed "pre-slot camera ARMED" on a
		// machine with no screen and no CameraManager. Adding the mode test removed that line; that
		// before/after IS the negative control for this guard.
		//
		// The claim in `TBD_SpectatorComponent` is therefore at best harness-dependent, and this is
		// the same correction T-181.49 is carrying for the lobby raise path ("replace the
		// GetWorkspace() authority test with RplSession.Mode()==RplMode.Dedicated"). Not fixed there
		// from here — that file belongs to another lane — but recorded, because the two must not end
		// up disagreeing about what "has a screen" means.
		//
		// BOTH tests, not one: `RplMode.Dedicated` is the authoritative "this build renders nothing",
		// and the workspace test still earns its place for a headless CLIENT, which is `RplMode.Client`
		// and would sail past the mode test alone.
		if (RplSession.Mode() == RplMode.Dedicated)
			return;

		if (!GetGame().GetWorkspace())
			return;

		GetGame().GetCallqueue().CallLater(TBD_PreSlotCameraArm.Start, START_DELAY_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (measured landmine in this codebase, and
	//! `TBD_FrameworkManager.SelectMissionByNumber` restarts the scenario in-process), so both
	//! managers MUST be torn down here or the next round starts holding a camera — and a ghost the
	//! engine still lists as somebody's controlled entity — that belong to a world that is gone.
	override void OnDelete(IEntity owner)
	{
		// Unconditional, unlike the arming above: both flags are live attributes, and a shutdown
		// that only ran when its flag was set would leak everything if one were ever flipped off
		// mid-session. Both `Shutdown`s are no-ops when nothing was started.
		TBD_PreSlotBody.Shutdown();

		// Unconditional on the client side too, and deliberately NOT re-testing the two guards above:
		// a teardown that asked "did I have a screen" could answer differently from the arm (the
		// workspace can be gone by then) and would leak a camera into the next world. Both calls are
		// no-ops when nothing was started.
		GetGame().GetCallqueue().Remove(TBD_PreSlotCameraArm.Start);
		TBD_PreSlotCameraArm.Shutdown();

		super.OnDelete(owner);
	}
}
