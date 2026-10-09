/**
 * TBD_M16Model.c
 *
 * Strong data models for the M16 weapon platform deep scan: rifle variants with their muzzles,
 * fire modes, and attachment slots, plus the candidate magazine and attachment pools the scanner
 * matches those slots against.
 *
 * Unlike the other weapon domains, these carriers hold resolved cross-product compatibility
 * arrays (m_aCompatibleMagResourceNames, m_aCompatibleItemResourceNames). The M16 export is a
 * compatibility matrix, not a catalog, so the pairing is computed at scan time rather than left
 * to the platform.
 */

class TBD_M16AttachmentSlotInfo
{
	string m_sSlotName;
	string m_sRequiredAttachType;
	string m_sDefaultAttachedPrefab;
	ref array<string> m_aCompatibleItemResourceNames = {};
	ref array<string> m_aCompatibleItemDisplayNames = {};
}

class TBD_M16MuzzleInfo
{
	int m_iIndex;
	string m_sMuzzleClass;
	ref array<string> m_aMagazineWells = {};
	string m_sDefaultMagazineTemplate;
	ref array<string> m_aFireModeNames = {};
	ref array<int> m_aFireModeBursts = {};
	ref array<int> m_aFireModeRpms = {};
	ref array<string> m_aCompatibleMagResourceNames = {};
	ref array<string> m_aCompatibleMagDisplayNames = {};
	ref array<int> m_aCompatibleMagCapacities = {};
}

class TBD_M16WeaponVariantInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sFilePath;
	string m_sAddonId;
	string m_sVariantOf;
	bool m_bIsAbstract;
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	int m_iCargoGridW = -1;
	int m_iCargoGridH = -1;
	ref array<string> m_aZeroingDistances = {};
	ref array<ref TBD_M16MuzzleInfo> m_aMuzzles = {};
	ref array<ref TBD_M16AttachmentSlotInfo> m_aAttachmentSlots = {};
}

class TBD_M16CandidateMagazine
{
	string m_sResourceName;
	string m_sDisplayName;
	string m_sMagWellClass;
	int m_iCapacity = 0;
	float m_fWeightKg = -1.0;
	string m_sAddonId;
}

class TBD_M16CandidateAttachment
{
	string m_sResourceName;
	string m_sDisplayName;
	string m_sAttachmentTypeClass;
	bool m_bIsOptic;
	float m_fWeightKg = -1.0;
	string m_sAddonId;
	string m_sFilePath;
}
