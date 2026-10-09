/**
 * @file TBD_KitEntry.c
 * @brief One labelled line of a kit: a label, a value, an optional count and tint.
 *
 * Role: one gear, attachment, ammunition or consumable line.  Position: built by TBD_LobbyMock into
 * TBD_KitInfo and TBD_KitWeapon; drawn by TBD_KitInspectorPanel.
 * State: plain data.  Invariants: an empty value or `None` reads as none (dimmed); a count of 0 shows
 * no count.
 */

//! One kit line, e.g. `Helmet` / `SSh-68 Steel Helmet` or `Bandages` x4.
class TBD_KitEntry
{
	string m_sLabel; //!< e.g. `Helmet`
	string m_sValue; //!< e.g. `SSh-68 Steel Helmet`; `None` reads dim
	int m_iCount; //!< 0 = no count shown
	TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL; //!< WARNING paints the value amber (e.g. `PG-7VL HEAT`); default NEUTRAL

	//! Create a kit line.
	void TBD_KitEntry(string label, string value, int count = 0, TBD_EUITint tint = TBD_EUITint.NEUTRAL)
	{
		m_sLabel = label;
		m_sValue = value;
		m_iCount = count;
		m_eTint = tint;
	}

	//! @return true when the value is empty or `None`
	bool IsNone()
	{
		return m_sValue.IsEmpty() || m_sValue == "None";
	}
}
