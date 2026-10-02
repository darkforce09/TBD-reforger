**Status:** live

# Template: Enfusion script header

**When to use:** the opening block of every Enfusion script (`.c`) under `apps/mod/`, and the
banners and tags that follow it through the file. `cargo xtask verify enfusion-comments` checks
every rule below as ECM-1 to ECM-9; `--path` narrows it to one folder or file under `apps/mod`.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. The header is
the first line of the file, with nothing above it.

````c
/**
 * @file <the exact file name, extension included>.c
 * @brief <one line: what the primary type is for>
 *
 * Role: <the responsibility of the file>  Position: <who feeds it, who consumes it>
 * State: <the state it owns and the machine that owns it, or "none">  Invariants: <the
 * guarantees it keeps and how it fails>
 */
````

## Rules

| Rule | Holds |
|---|---|
| ECM-1 | ASCII only, string literals included: write `--` and `->`. |
| ECM-2 | The header above is first and carries `@file` (exact name), `@brief`, `Role:`, `Position:`, `State:` and `Invariants:`, each with text. |
| ECM-3 | A `//!` banner sits directly above every class, modded class, enum and method, above its attributes; a method banner is its contract: what it does, its parameters, its return value, how it fails. |
| ECM-4 | A trailing `//!<` on every field and enum member gives its unit, default or JSON key. |
| ECM-5 | `//! @authority server\|client\|owner` in the banner of a method that depends on where it runs (its body asks `Replication` or `RplSession`) and of every method that calls `Rpc(`; `//! @rpc <Reliable\|Unreliable> <Server\|Owner\|Broadcast>` directly above `[RplRpc]`, matching it; `//! @replicated <prop>` directly above `[RplProp]`. |
| ECM-6 | `//! @route <METHOD> <path>` on a method that calls `TBD_GameRuntimeHttp.Post`/`Get` or uses `RestContext`; `//! @contract <schema>#<pointer>` on every `*Struct` class. `Struct` names schema-backed DTOs only; an internal wire class is named `*Wire`. |
| ECM-7 | `[Attribute]` has `desc:`; `[ComponentEditorProps]` has `description:`. |
| ECM-8 | Comments are present tense and context-free: no ticket ids, dates, history words, work markers, delivery vocabulary, commented-out code, separator lines, or file:line references (name the symbol). |
| ECM-9 | The file is named after its primary type; one primary type per file, with at most small companion types beside it. |

"Directly above" skips attribute-only lines and nothing else: a blank line, a plain `//` comment
or a separator between a banner and its declaration breaks the banner.

## Worked sample

Written for `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/TBD_TaskHud.c`, checked against its
code: the class keeps the last applied markers and snapshot signature on the client, the server
pushes the assigned-task snapshot through the player controller, and `PushToPlayers` returns at
once on a client. The sample is an excerpt: the header, the class banner, one field and one method
banner, with the rest of the class left out.

````c
/**
 * @file TBD_TaskHud.c
 * @brief Map markers for the tasks assigned to the local player.
 *
 * Role: turns the assigned-task snapshot into one static map marker per task.  Position: the
 * server builds the snapshot and pushes it through SCR_PlayerController owner RPCs; the client
 * applies it through TBD_MarkerIcons and SCR_MapMarkerManagerComponent.
 * State: s_aApplied and s_sLastSignature, on each client.  Invariants: a task that leaves
 * `assigned` is absent from the snapshot, so its marker is removed; an unchanged snapshot is not
 * re-applied.
 */

//! Draws and clears the map markers of assigned tasks.
class TBD_TaskHud
{
	static const string CH = "TaskHud"; //!< log channel name

	//! Pushes the current assigned-task snapshot to every connected player; returns at once on a
	//! client or when no player is connected.
	//! @authority server
	static void PushToPlayers()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;
	}
}
````

## Related documentation

- [Documentation standards](/documentation/standards/documentation_standards.md) — the
  in-code documentation rules this card applies to Enfusion scripts.
- [Mod script checks](/tools/xtask/src/verifications/mod_scripts/README.md) — the
  `enfusion-comments` gate that enforces the card.
