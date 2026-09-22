//------------------------------------------------------------------------------------------------
// TBD_RifleModel.c
//
// Strong data models representing rifle weapons and their intrinsic firing configuration
// (muzzles, magazine wells, fire modes, attachment slots, zeroing distances, mass, and volume)
// for the TBD Reforger Universal Equipment Export System.
//
// Pure ground-truth extraction:
//   - Carriers hold only intrinsic properties declared on the prefab and its container ancestry.
//   - Cross-product compatibility arrays are absent by design: magazine wells and required
//     attachment types are exported as foreign keys so the platform links them against the
//     ammunition and attachment catalogs at query time.
//------------------------------------------------------------------------------------------------

class TBD_RifleFireModeInfo
{
	string m_sName;
	int m_iBurstCount = 1;
	int m_iRoundsPerMinute = 0;
}

class TBD_RifleMuzzleInfo
{
	int m_iIndex = 0;
	string m_sMuzzleClass;
	ref array<string> m_aMagazineWells = {};
	string m_sDefaultMagazineTemplate;
	ref array<ref TBD_RifleFireModeInfo> m_aFireModes = {};
}

class TBD_RifleAttachmentSlotInfo
{
	string m_sSlotName;
	string m_sRequiredAttachmentType;
	string m_sDefaultAttachedPrefab;
}

class TBD_RifleWeaponInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sFamily;
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;
	string m_sVariantOf;
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	ref array<string> m_aZeroingDistances = {};
	ref array<ref TBD_RifleMuzzleInfo> m_aMuzzles = {};
	ref array<ref TBD_RifleAttachmentSlotInfo> m_aAttachmentSlots = {};
}
