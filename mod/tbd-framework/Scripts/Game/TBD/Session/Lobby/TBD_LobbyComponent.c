/**
 * @file TBD_LobbyComponent.c
 * @brief Game mode component that hosts the lobby's lifecycle: the boot-time wire self-check and the stage watcher.
 *
 * Role: runs TBD_LobbyRosterWireSelfCheck at boot on every machine and starts TBD_LobbyStage
 * START_DELAY_MS after init; stops the watcher with the world.  Position: attached by
 * Prefabs/Systems/TBD_GameMode.et beside TBD_FrameworkManager and TBD_SpawnManager; a component
 * rather than a player controller override, so its lifetime is the world's.
 * State: none; the watcher's state is TBD_LobbyStage's.
 * Invariants: the self-check runs before any early return, so the zero-player headless boot
 * gates it; the watcher is armed on every machine and TBD_LobbyStage.Start logs its own refusal
 * on a dedicated server; teardown is unconditional.
 */

//! Entity class of TBD_LobbyComponent.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD lobby - the side/group/slot picker a player takes their one seat in.")]
class TBD_LobbyComponentClass : SCR_BaseGameModeComponentClass {}

//! Lobby lifecycle socket on the game mode.
class TBD_LobbyComponent : SCR_BaseGameModeComponent
{
	static const int START_DELAY_MS = 2000; //!< ms after init before TBD_LobbyStage.Start; the watcher polls, so a late start misses nothing

	//! Run the roster wire self-check, then arm TBD_LobbyStage.Start START_DELAY_MS later. Runs on
	//! every machine; the self-check guards itself to once per process and the watcher decides for
	//! itself where it runs.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		TBD_LobbyRosterWireSelfCheck.Run();

		GetGame().GetCallqueue().CallLater(TBD_LobbyStage.Start, START_DELAY_MS, false);
	}

	//! Cancel a pending start and shut the watcher down, unconditionally: statics outlive a world,
	//! and a skipped teardown leaves the next world's watcher latched.
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(TBD_LobbyStage.Start);

		TBD_LobbyStage.Shutdown();

		super.OnDelete(owner);
	}
}
