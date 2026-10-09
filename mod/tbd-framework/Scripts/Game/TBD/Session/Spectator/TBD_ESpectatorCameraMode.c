/**
 * @file TBD_ESpectatorCameraMode.c
 * @brief The three views the spectator camera runs.
 *
 * Role: names the spectator camera's view mode.
 * Position: TBD_SpectatorCamera holds it; TBD_SpectatorTargeting and the roster screen read it.
 * State: none.
 * Invariants: FREE is the default and the fallback whenever a followed target goes away.
 */

//! Spectator camera view mode.
enum TBD_ESpectatorCameraMode
{
	FREE,          //!< fly the AO
	FOLLOW,        //!< orbit a living player
	FIRST_PERSON   //!< through their eyes
}
