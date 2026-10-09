/**
 * @file TBD_FrameworkRollCall.c
 * @brief Prints the one-line roll-call of the manager components on the framework game mode.
 *
 * Role: proves that every manager component listed on TBD_GameMode.et instantiated; a component
 * whose class fails to resolve is dropped from the prefab silently.  Position: called by
 * TBD_FrameworkManager one frame after OnPostInit; `cargo xtask mod world-boot` asserts the line.
 * State: none.  Invariants: the line reads `[TBD] roll-call:` followed by ` Label=ok` or
 * ` Label=MISSING` per component, printed with PrintFormat; any miss is logged at ERROR.
 */

//! Component roll-call of the framework game mode.
class TBD_FrameworkRollCall
{
	//! Print the roll-call line for `owner` and arm the briefing wire self-check, which a world boot
	//! with no players would otherwise never run.
	//! @param owner the game mode entity; null logs an ERROR and prints no roll-call
	static void Report(IEntity owner)
	{
		if (!owner)
		{
			Print("[TBD] roll-call: no owner entity -- cannot enumerate components.", LogLevel.ERROR);
			return;
		}

		array<string> missing = new array<string>();
		string line = "[TBD] roll-call:";
		line += Entry(owner, TBD_SpawnManager, "SpawnManager", missing);
		line += Entry(owner, TBD_SafestartManager, "Safestart", missing);
		line += Entry(owner, TBD_LoadoutEquipComponent, "LoadoutEquip", missing);
		line += Entry(owner, TBD_SpectatorComponent, "Spectator", missing);
		line += Entry(owner, TBD_LobbyComponent, "Lobby", missing);
		line += Entry(owner, TBD_PlayAreaComponent, "PlayArea", missing);
		line += Entry(owner, TBD_MarkerComponent, "Markers", missing);
		line += Entry(owner, TBD_RadioComponent, "Radio", missing);
		line += Entry(owner, TBD_ObjectivesComponent, "Objectives", missing);
		line += Entry(owner, TBD_MatchTelemetryComponent, "MatchTelemetry", missing);

		// Armed here because a world boot has no players, so Serialise never runs; once-only inside.
		TBD_BriefingWireSelfCheck.Run();

		// PrintFormat, not Print: Print of a local variable emits its declaration, not its value.
		if (missing.IsEmpty())
		{
			PrintFormat("%1", line);
			return;
		}

		PrintFormat("%1", line, level: LogLevel.ERROR);
		Print(string.Format("[TBD] roll-call: %1 component(s) declared on TBD_GameMode.et did not instantiate.",
			missing.Count()), LogLevel.ERROR);
	}

	//! One roll-call cell; records a miss in `missing`.
	//! @param owner the game mode entity
	//! @param componentType the component class to find on it
	//! @param label the cell name
	//! @param missing receives `label` when the component is absent
	//! @return ` <label>=ok` or ` <label>=MISSING`
	protected static string Entry(notnull IEntity owner, typename componentType, string label, notnull array<string> missing)
	{
		if (owner.FindComponent(componentType))
			return " " + label + "=ok";

		missing.Insert(label);
		return " " + label + "=MISSING";
	}
}
