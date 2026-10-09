/**
 * @file TBD_DropdownItem.c
 * @brief One entry of a TBD dropdown: label, trailing badge, caller tag and check state.
 *
 * Role: carries the data of one dropdown row between a screen and `TBD_DropdownComponent`.
 * Position: built by the screens that fill a dropdown (`SetItems`); read by
 * `TBD_DropdownComponent` and painted as a pooled `TBD_ListBox` row by `TBD_DropdownMenu`.
 * State: the four fields, owned by the dropdown's item array on the client.
 * Invariants: `m_iTag` is the caller's id and is never interpreted by the dropdown.
 */

//! One entry of a dropdown. `m_iTag` is the caller's id; `m_sBadge` is the trailing mono text
//! (`LATEST`, `STABLE`, a count).
class TBD_DropdownItem
{
	string m_sLabel; //!< the row title; empty draws an empty title
	string m_sBadge; //!< trailing mono text (LATEST, STABLE, a count); empty hides the badge
	int m_iTag; //!< the caller's id, reported by OnChanged
	bool m_bChecked; //!< multi-select check state; default false

	//! Build an item.
	//! @param label the row title
	//! @param tag the caller's id
	//! @param badge the trailing text; empty for none
	//! @param checked the initial multi-select check state
	void TBD_DropdownItem(string label, int tag, string badge = "", bool checked = false)
	{
		m_sLabel = label;
		m_iTag = tag;
		m_sBadge = badge;
		m_bChecked = checked;
	}
}
