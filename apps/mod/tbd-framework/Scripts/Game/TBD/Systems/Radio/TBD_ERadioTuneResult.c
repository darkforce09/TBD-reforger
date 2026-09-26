/**
 * @file TBD_ERadioTuneResult.c
 * @brief Outcome of one attempt to tune a player's radio into their nets.
 *
 * Role: the tune outcome vocabulary.  Position: set by `TBD_RadioTuner.TunePlayer` on a
 * `TBD_RadioTuneReport`; crosses the wire by name in `TBD_RadioWire.m_sTuneResult` and is read by
 * `TBD_RadioClient.TuneLine`.
 * State: none.  Invariants: travels by name, never by value; ordered roughly worst to best.
 */

//! What happened when a player was put on their nets.
enum TBD_ERadioTuneResult
{
	NO_BACKBONE, //!< the world has no `RadioManagerEntity`; the fallback channel table tunes instead, so no attempt reports it
	NO_BODY, //!< the player has no controlled entity yet (lobby, dead, mid-possess); ordinary
	NO_GADGET_MANAGER, //!< the body has no gadget manager: not a character, or not fully built
	NO_RADIO, //!< the player carries no radio; the net list is still shown
	NO_TRANSCEIVER, //!< every carried radio has no free transceiver left
	READBACK_MISMATCH, //!< every attempted transceiver read back a different frequency; a failure
	NO_NETS, //!< nothing to tune: no nets for this player
	TUNED //!< at least one net read back correct on a transceiver
}
