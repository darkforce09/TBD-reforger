//! T-181.9.1 — where the lobby's client-side lifecycle is hosted.
//!
//! The slot picker is a **client** feature: a screen, a poll of the replicated game stage, and a
//! cached roster. It needs a place on the client that starts with the world and dies with it, and
//! in this codebase that place is a component on the game mode prefab — the same seat
//! `TBD_FrameworkManager`, `TBD_SpawnManager`, `TBD_LoadoutEquipComponent` and
//! `TBD_SpectatorComponent` already occupy (`Prefabs/Systems/TBD_GameMode.et`).
//!
//! Deliberately NOT a third `OnControlledEntityChanged` override on `SCR_PlayerController`: two
//! modded blocks already override it (`TBD_MissionBrowser.c`, `TBD_BriefingController.c`), and
//! stacking a third override of one vanilla method across three modded blocks in one addon is a
//! runtime behaviour this lane cannot prove. A game-mode component is a lifecycle the tree has
//! already shipped and the operator has already seen work.
//!
//! All this class does is start and stop `TBD_LobbyStage`. Every decision lives there; this is the
//! socket, not the logic.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD lobby — the side/group/slot picker a player takes their one seat in.")]
class TBD_LobbyComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_LobbyComponent : SCR_BaseGameModeComponent
{
	//! The game mode component graph is up well before the local player has a controller or the
	//! server has replicated a stage, so the start is nudged past init rather than racing it.
	//! Nothing is lost by being late: `TBD_LobbyStage` polls, so it cannot miss a transition that
	//! happened while it was waiting — LOBBY is a stage the round SITS in, not one it passes
	//! through.
	static const int START_DELAY_MS = 2000;

	//------------------------------------------------------------------------------------------------
	//! @authority client — the server has no workspace and nothing to draw a picker on.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// A dedicated server has no workspace at all (measured — see TBD_UILayouts). That is the
		// cleanest available "am I a machine with a screen" test, and it is the one the rest of the
		// UI framework already trusts.
		if (!GetGame().GetWorkspace())
			return;

		GetGame().GetCallqueue().CallLater(TBD_LobbyStage.Start, START_DELAY_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (measured landmine in this codebase), so the
	//! watcher MUST be torn down here or the next round starts polling a framework manager that
	//! belongs to a world that no longer exists.
	override void OnDelete(IEntity owner)
	{
		if (GetGame().GetWorkspace())
		{
			GetGame().GetCallqueue().Remove(TBD_LobbyStage.Start);
			TBD_LobbyStage.Shutdown();
		}

		super.OnDelete(owner);
	}
}
