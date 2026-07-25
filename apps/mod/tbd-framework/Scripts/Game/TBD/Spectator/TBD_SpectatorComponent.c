//! T-181.12 — where the spectator lifecycle is hosted.
//!
//! Spectator is a **client** feature: a camera, a roster screen, and a poll of the local player's
//! own body. It needs a place on the client that starts with the world and dies with it, and in
//! this codebase that place is a component on the game mode prefab — the same seat
//! `TBD_FrameworkManager`, `TBD_SpawnManager` and `TBD_LoadoutEquipComponent` already occupy.
//!
//! Deliberately NOT a `modded class SCR_PlayerController`: `TBD_MissionBrowser.c` already mods
//! that class for the admin keybinds, and a second `modded class` block for the same class in the
//! same addon is a collision waiting to happen. Deliberately NOT a bare `GameSystem` either —
//! auto-registration of a scripted system is not something the headless compile lane can prove,
//! and an unprovable lifecycle is exactly what this program refuses to ship.
//!
//! All this class does is start and stop `TBD_SpectatorController`. Every decision lives there;
//! this is the socket, not the logic.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD spectator — free camera, follow, and the unit list a dead player lives in.")]
class TBD_SpectatorComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_SpectatorComponent : SCR_BaseGameModeComponent
{
	//! The game mode component graph is up well before the local player has a controller, so the
	//! start is nudged past init rather than racing it. Nothing is lost by being late: the
	//! controller polls, so it cannot miss a death that happened while it was waiting.
	static const int START_DELAY_MS = 2000;

	//------------------------------------------------------------------------------------------------
	//! @authority client — the server has no camera and nothing to spectate with.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// A dedicated server has no workspace at all (measured — see TBD_UILayouts). That is the
		// cleanest available "am I a machine with a screen" test, and it is the one the rest of
		// the UI framework already trusts.
		if (!GetGame().GetWorkspace())
			return;

		GetGame().GetCallqueue().CallLater(TBD_SpectatorController.Start, START_DELAY_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (measured landmine in this codebase), so the
	//! controller MUST be torn down here or the next round starts holding a camera that belongs to
	//! a world that no longer exists.
	override void OnDelete(IEntity owner)
	{
		if (GetGame().GetWorkspace())
		{
			GetGame().GetCallqueue().Remove(TBD_SpectatorController.Start);
			TBD_SpectatorController.Shutdown();
		}

		super.OnDelete(owner);
	}
}
