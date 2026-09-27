/**
 * TBD_VehicleCatalogModel.c
 *
 * Data models for the Universal Vehicle Platforms & Variants Catalog.
 * Groups individual vehicle variants under their parent platform / family.
 */

class TBD_VehicleVariantInfo
{
	ref TBD_VehicleDeepVariant m_Resolved; //!< Shared source of all serialized vehicle facts.
	string m_sId;
	string m_sResourceName;
	string m_sDisplayName;
	string m_sDescription;
	string m_sPlatform;
	string m_sFaction;
	string m_sVehicleDomain; // Wheeled, Helicopter, Tracked, Boat, Plane
	string m_sRole;          // APC, Transport, Tanker, Repair, Ambulance, Armed, etc.
	string m_sFilePath;
	string m_sAddonId;
	string m_sParentPrefab;
	bool m_bIsAbstract;

	// Seating
	int m_iTotalSeats;
	int m_iDriverSeats;
	int m_iCrewSeats;
	int m_iPassengerSeats;

	// Armament
	bool m_bIsArmed;
	ref array<string> m_aMountedWeapons = {};

	// Key specs
	bool m_bIsAmphibious;
	float m_fMassKg = -1.0;
	float m_fSupplyCapacity = -1.0;

	//------------------------------------------------------------------------------------------------
	//! Serializes the same vehicle record used by its platform detail file.
	string SerializeToJson()
	{
		if (!m_Resolved)
		{
			TBD_EquipmentExportJson.ExtractionError(m_sResourceName, "/vehicles", "Catalog variant has no resolved vehicle record");
			return "null";
		}
		return TBD_VehicleDeepSerializer.SerializeVariantToJson(m_Resolved);
	}

}

class TBD_VehiclePlatformInfo
{
	ref TBD_VehicleDeepPlatform m_Resolved; //!< Shared platform membership and resolved variants.
	string m_sPlatformId;
	string m_sDisplayName;
	string m_sVehicleDomain;
	string m_sPrimaryFaction;
	ref array<ref TBD_VehicleVariantInfo> m_aVariants = {};

	//------------------------------------------------------------------------------------------------
	//! Serializes the resolved platform without re-extracting or reclassifying its vehicles.
	string SerializeToJson()
	{
		if (!m_Resolved)
		{
			TBD_EquipmentExportJson.ExtractionError(m_sPlatformId, "/platforms", "Catalog platform has no resolved platform record");
			return "null";
		}
		return TBD_VehicleDeepSerializer.SerializePlatformToJson(m_Resolved);
	}
}
