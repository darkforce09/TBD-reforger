/**
 * @file TBD_EObjectiveRole.c
 * @brief Which side of one objective a viewer is on.
 *
 * Role: the per-viewer reading of one objective, chosen from its single `objectives[]` row.
 * Position: returned by `TBD_Objective.RoleOf`; read by the title and task-text lookups.
 * State: none.  Invariants: one objective carries both framings, never two rows; `NEUTRAL` is
 * what a viewer with no side, or an objective for no valid side, reads.
 */

//! Viewer role on one objective.
enum TBD_EObjectiveRole
{
	NEUTRAL, //!< no resolved side, or the objective names no valid side; reads the label
	ATTACKER, //!< reads `framing.attacker`
	DEFENDER //!< reads `framing.defender`
}
