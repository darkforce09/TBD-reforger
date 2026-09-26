/**
 * @file TBD_PlayAreaViolation.c
 * @brief One player's out-of-bounds countdown while they are in violation.
 *
 * Role: the per-player warning, grace and penalty state of the play-area enforcer.
 * Position: created and dropped by `TBD_PlayAreaComponent`; updated by `TBD_PlayAreaPenalties.WarnPlayer`.
 * State: the fields below, owned by the server's play-area component, one row per player in
 * violation.  Invariants: a row exists only while its player is in violation, so "being warned"
 * is a null check; the penalty fires once per row.
 */

//! One player's open violation of a play-area zone.
class TBD_PlayAreaViolation
{
	float m_fSecondsOutside; //!< seconds counted since the violation started, in ticks
	float m_fSecondsSinceWarned; //!< seconds since the last warning message
	string m_sZoneKey; //!< `TBD_Zone.LogKey` of the violated zone; a change restarts the countdown
	bool m_bPenaltyApplied; //!< true once the penalty fired; it fires once per violation
	bool m_bWarned; //!< true once the player was told anything, so a return is announced
	EntityID m_LastBody; //!< body the violation was observed on; a new body restarts it (recycled ids)
}
