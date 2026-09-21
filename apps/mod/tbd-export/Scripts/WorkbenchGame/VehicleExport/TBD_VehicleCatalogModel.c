/**
 * TBD_VehicleCatalogModel.c
 *
 * Data models for the Universal Vehicle Platforms & Variants Catalog.
 * Groups individual vehicle variants under their parent platform / family.
 */

class TBD_VehicleVariantInfo
{
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
	string SerializeToJson()
	{
		string json = "        {\n";
		json += "          \"id\": \"" + m_sId + "\",\n";
		json += "          \"resourceName\": \"" + m_sResourceName + "\",\n";
		json += "          \"displayName\": \"" + TBD_VehicleExportJson.Escape(m_sDisplayName) + "\",\n";
		json += "          \"description\": \"" + TBD_VehicleExportJson.Escape(m_sDescription) + "\",\n";
		json += "          \"platform\": \"" + m_sPlatform + "\",\n";
		json += "          \"faction\": \"" + m_sFaction + "\",\n";
		json += "          \"vehicleDomain\": \"" + m_sVehicleDomain + "\",\n";
		json += "          \"role\": \"" + m_sRole + "\",\n";
		json += "          \"filePath\": \"" + m_sFilePath + "\",\n";
		json += "          \"addonId\": \"" + m_sAddonId + "\",\n";
		json += "          \"parentPrefab\": \"" + m_sParentPrefab + "\",\n";
		json += "          \"isAbstract\": " + m_bIsAbstract.ToString() + ",\n";
		json += "          \"isAmphibious\": " + m_bIsAmphibious.ToString() + ",\n";
		json += "          \"massKg\": " + m_fMassKg.ToString() + ",\n";
		json += "          \"supplyCapacity\": " + m_fSupplyCapacity.ToString() + ",\n";

		// Seating
		json += "          \"seating\": {\n";
		json += "            \"totalSeats\": " + m_iTotalSeats.ToString() + ",\n";
		json += "            \"driver\": " + m_iDriverSeats.ToString() + ",\n";
		json += "            \"crew\": " + m_iCrewSeats.ToString() + ",\n";
		json += "            \"passengers\": " + m_iPassengerSeats.ToString() + "\n";
		json += "          },\n";

		// Armament
		json += "          \"armament\": {\n";
		json += "            \"isArmed\": " + m_bIsArmed.ToString() + ",\n";
		json += "            \"weapons\": [";
		for (int w = 0; w < m_aMountedWeapons.Count(); w++)
		{
			json += "\"" + TBD_VehicleExportJson.Escape(m_aMountedWeapons[w]) + "\"";
			if (w < m_aMountedWeapons.Count() - 1) json += ", ";
		}
		json += "]\n";
		json += "          }\n";

		json += "        }";
		return json;
	}
}

class TBD_VehiclePlatformInfo
{
	string m_sPlatformId;
	string m_sDisplayName;
	string m_sVehicleDomain;
	string m_sPrimaryFaction;
	ref array<ref TBD_VehicleVariantInfo> m_aVariants = {};

	//------------------------------------------------------------------------------------------------
	string SerializeToJson()
	{
		string json = "    {\n";
		json += "      \"platformId\": \"" + m_sPlatformId + "\",\n";
		json += "      \"displayName\": \"" + TBD_VehicleExportJson.Escape(m_sDisplayName) + "\",\n";
		json += "      \"vehicleDomain\": \"" + m_sVehicleDomain + "\",\n";
		json += "      \"primaryFaction\": \"" + m_sPrimaryFaction + "\",\n";
		json += "      \"variantCount\": " + m_aVariants.Count().ToString() + ",\n";
		json += "      \"variants\": [\n";

		for (int i = 0, n = m_aVariants.Count(); i < n; i++)
		{
			json += m_aVariants[i].SerializeToJson();
			if (i < n - 1) json += ",";
			json += "\n";
		}

		json += "      ]\n";
		json += "    }";
		return json;
	}
}
