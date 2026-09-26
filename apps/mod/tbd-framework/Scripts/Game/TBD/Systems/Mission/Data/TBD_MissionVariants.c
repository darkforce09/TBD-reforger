/**
 * @file TBD_MissionVariants.c
 * @brief Decides whether a `variantId`-gated mission row is in the active variant selection.
 *
 * Role: the variant gate rules shared by the mission loader's filter and the second-pass readers
 * (objectives, editor triggers).  Position: called by `TBD_MissionLoader`,
 * `TBD_ObjectiveRegistry` and `TBD_TriggerRuntime`; reads only its arguments.
 * State: none.  Invariants: a row with no `variantId` is always included; a dangling id (named by
 * a row, declared by no `variants[]` entry) is excluded with a warning, never silently kept.
 */

//! Stateless variant gate predicates.
class TBD_MissionVariants
{
	//! Whether a row gated on `variantId` runs under the selection `activeIds`.
	//! @param variantId the row's `variantId`; empty means unconditional
	//! @param activeIds the selected variant ids; may be null
	//! @param selectionInForce false makes every row active (selection is inert)
	//! @param missingSetKeepsRow the answer when selection is in force but `activeIds` is null:
	//! true where null means "the document declares no variants" (the loader's
	//! `GetActiveVariantIds`), false where null means "nothing selected"
	//! @return true when the row runs
	static bool IsActive(string variantId, array<string> activeIds, bool selectionInForce = true, bool missingSetKeepsRow = true)
	{
		if (variantId.IsEmpty())
			return true;

		if (!selectionInForce)
			return true;

		if (!activeIds)
			return missingSetKeepsRow;

		return activeIds.Find(variantId) != -1;
	}

	//! The loader's per-row filter over its declared and active sets. Declared-but-inactive rows
	//! drop silently; a dangling id drops with one WARNING naming the row.
	//! @param variantId the row's `variantId`; empty means unconditional
	//! @param declared every well-formed `variants[].id`
	//! @param active the selected ids
	//! @param collection the row's collection for the warning (`zone`, `slot`, ...)
	//! @param rowName the row's name for the warning
	//! @return true to keep the row
	static bool IsRowIncluded(string variantId, notnull map<string, bool> declared, notnull map<string, bool> active, string collection, string rowName)
	{
		if (variantId.IsEmpty())
			return true;

		if (active.Contains(variantId))
			return true;

		if (!declared.Contains(variantId))
		{
			Print(string.Format(
				"[TBD][Variants] EXCLUDING %1 '%2' -- variantId='%3' names no variants[] entry (dangling reference; fail-visible, not fail-open)",
				collection, rowName, variantId), LogLevel.WARNING);
		}

		return false;
	}
}
