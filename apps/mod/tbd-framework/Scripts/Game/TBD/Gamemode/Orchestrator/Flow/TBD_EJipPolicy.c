/**
 * @file TBD_EJipPolicy.c
 * @brief The join-in-progress policies a mission's `flow.jip` can name.
 *
 * Role: the vocabulary of `flow.jip`, ordered by how much of the round each lets a latecomer into.
 * Position: produced by TBD_JipPolicy.FromString; read by TBD_JipPolicy and the join door in
 * TBD_SpawnManager.  State: none.
 * Invariants: no code switches on this enum (duplicate switch labels compile clean in Enfusion);
 * resolvers compare against named members and named stages only.
 */

//! Join-in-progress policy; each member closes the roster one stage earlier than the last.
enum TBD_EJipPolicy
{
	ALWAYS, //!< `"always"`: a join is permitted at every deployable stage; the default when `flow.jip` is absent
	UNTIL_SAFESTART_END, //!< `"until_safestart_end"`: open through LOBBY, BRIEFING and SAFE_START, closed from LIVE
	DISABLED //!< `"disabled"`: open during LOBBY only
}
