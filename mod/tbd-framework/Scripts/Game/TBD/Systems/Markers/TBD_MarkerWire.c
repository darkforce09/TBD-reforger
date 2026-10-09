/**
 * @file TBD_MarkerWire.c
 * @brief One player's marker set in the exact shape the marker RPC takes.
 *
 * Role: parallel per-marker columns plus the answer's side, mission and served flag.
 * Position: built by `TBD_MarkerService.BuildForPlayer`; the style columns are packed into `m_aX`
 * by `TBD_MarkerStyleCodec.PackIntoX`; sent by the modded `SCR_PlayerController`.
 * State: none beyond its fields.  Invariants: element i of every column is marker i; there is no
 * delimited string, so an empty label is an empty element and a label may contain any character.
 */

//! A marker answer, isomorphic to the RPC parameter list.
class TBD_MarkerWire
{
	bool m_bServed; //!< false while the server has no authoritative answer (no slot, no mission); the client keeps asking
	string m_sFactionKey; //!< side the rows belong to; diagnostic only, never trusted back from a client
	string m_sMissionId; //!< mission the rows belong to; tells a repeat from a mission switch

	ref array<int> m_aX = {}; //!< world X in metres per marker, rounded
	ref array<int> m_aZ = {}; //!< world Z in metres per marker, rounded
	ref array<string> m_aIcon = {}; //!< authored icon per marker
	ref array<string> m_aLabel = {}; //!< caption per marker, cut to `TBD_MarkerService.MAX_LABEL_CHARS`

	ref array<int> m_aSizeFp = {}; //!< icon scale x100 per marker; -1 absent
	ref array<int> m_aRotationFp = {}; //!< rotation in degrees x100 per marker; -1 absent
	ref array<string> m_aShape = {}; //!< authored shape per marker; empty is `icon`
	ref array<string> m_aBrush = {}; //!< authored brush per marker; empty is absent
	ref array<string> m_aColorHex = {}; //!< authored `#rrggbb` per marker; empty is absent
	ref array<int> m_aAlpha255 = {}; //!< opacity 0..255 per marker; -1 absent

	string m_sRefusal; //!< why the server declined; logged, never shown to the player

	//! @return the number of markers
	int Count()
	{
		return m_aX.Count();
	}
}
