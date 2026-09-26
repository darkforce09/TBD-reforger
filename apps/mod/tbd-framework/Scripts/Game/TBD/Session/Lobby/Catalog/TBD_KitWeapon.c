/**
 * @file TBD_KitWeapon.c
 * @brief One weapon card of a kit: slot label, name, mounted attachments and ammunition.
 *
 * Role: one weapon slot of a kit.  Position: built by TBD_LobbyMock into TBD_KitInfo.m_aWeapons;
 * drawn by TBD_KitInspectorCells.MountWeapon.
 * State: plain data.  Invariants: the arrays are never null.
 */

//! One WEAPON SLOT card: name, mounted attachments, ammunition rows and a summary.
class TBD_KitWeapon
{
	string m_sSlotLabel; //!< e.g. `WEAPON SLOT 1`
	string m_sName; //!< e.g. `AK-74`
	string m_sAmmoSummary; //!< e.g. `7 MAGS` or `4 ROCKETS`
	ref array<ref TBD_KitEntry> m_aAttachments; //!< mounted attachments
	ref array<ref TBD_KitEntry> m_aAmmo; //!< ammunition rows with counts

	//! Create a weapon card with no attachments or ammunition.
	void TBD_KitWeapon(string slotLabel, string name, string ammoSummary)
	{
		m_sSlotLabel = slotLabel;
		m_sName = name;
		m_sAmmoSummary = ammoSummary;
		m_aAttachments = {};
		m_aAmmo = {};
	}
}
