/**
 * TBD_VehicleDeepSerializer.c
 *
 * Dedicated JSON serializer for the deep vehicle engineering platform.
 * Formats platforms, variants, drivetrain, armor hit zones, turrets, and reflection dumps
 * into clean, schema-compliant, indented JSON with zero mock data.
 */

class TBD_VehicleDeepSerializer
{
	//------------------------------------------------------------------------------------------------
	//! Serialize an entire platform and all of its deep variants to JSON.
	static string SerializePlatformToJson(TBD_VehicleDeepPlatform plat)
	{
		string json = "{\n";
		json += "  \"platform\": \"" + plat.m_sPlatformId + "\",\n";
		json += "  \"displayName\": \"" + TBD_VehicleExportJson.Escape(plat.m_sDisplayName) + "\",\n";
		json += "  \"vehicleDomain\": \"" + plat.m_sVehicleDomain + "\",\n";
		json += "  \"primaryFaction\": \"" + plat.m_sPrimaryFaction + "\",\n";
		json += "  \"generatedAt\": \"" + TBD_VehicleExportJson.IsoNowUtc() + "\",\n";
		json += "  \"variantsCount\": " + plat.m_aVariants.Count().ToString() + ",\n";
		json += "  \"variants\": [\n";

		for (int v = 0, vn = plat.m_aVariants.Count(); v < vn; v++)
		{
			json += SerializeVariantToJson(plat.m_aVariants[v]);
			if (v < vn - 1) json += ",";
			json += "\n";
		}

		json += "  ]\n";
		json += "}\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	//! Serialize a single variant's complete engineering telemetry to JSON.
	static string SerializeVariantToJson(TBD_VehicleDeepVariant v)
	{
		string json = "    {\n";
		json += "      \"id\": \"" + v.m_sId + "\",\n";
		json += "      \"resourceName\": \"" + v.m_sResourceName + "\",\n";
		json += "      \"displayName\": \"" + TBD_VehicleExportJson.Escape(v.m_sDisplayName) + "\",\n";
		json += "      \"description\": \"" + TBD_VehicleExportJson.Escape(v.m_sDescription) + "\",\n";
		json += "      \"platform\": \"" + v.m_sPlatform + "\",\n";
		json += "      \"faction\": \"" + v.m_sFaction + "\",\n";
		json += "      \"vehicleDomain\": \"" + v.m_sVehicleDomain + "\",\n";
		json += "      \"role\": \"" + v.m_sRole + "\",\n";
		json += "      \"filePath\": \"" + v.m_sFilePath + "\",\n";
		json += "      \"addonId\": \"" + v.m_sAddonId + "\",\n";
		json += "      \"parentPrefab\": \"" + v.m_sParentPrefab + "\",\n";
		json += "      \"isAbstract\": " + v.m_bIsAbstract.ToString() + ",\n";

		// Physical
		json += "      \"physical\": {\n";
		json += "        \"weightKg\": " + v.m_fWeightKg.ToString() + ",\n";
		json += "        \"lengthM\": " + v.m_fLengthM.ToString() + ",\n";
		json += "        \"widthM\": " + v.m_fWidthM.ToString() + ",\n";
		json += "        \"heightM\": " + v.m_fHeightM.ToString() + ",\n";
		json += "        \"centerOfMass\": [" + v.m_vCenterOfMass[0].ToString() + ", " + v.m_vCenterOfMass[1].ToString() + ", " + v.m_vCenterOfMass[2].ToString() + "]\n";
		json += "      },\n";

		// Drivetrain
		json += SerializeDrivetrainBlock(v.m_Drivetrain);

		// Fuel & Amphibious
		json += "      \"fuel\": {\n";
		json += "        \"capacityLiters\": " + v.m_fFuelCapacityLiters.ToString() + ",\n";
		json += "        \"consumptionRate\": " + v.m_fFuelConsumptionRate.ToString() + "\n";
		json += "      },\n";

		bool isAmphi = false;
		float wThrust = -1, rAngle = -1;
		if (v.m_Drivetrain)
		{
			isAmphi = v.m_Drivetrain.m_bIsAmphibious;
			wThrust = v.m_Drivetrain.m_fWaterThrust;
			rAngle = v.m_Drivetrain.m_fWaterRudderAngle;
		}
		json += "      \"amphibious\": {\n";
		json += "        \"isAmphibious\": " + isAmphi.ToString() + ",\n";
		json += "        \"waterThrust\": " + wThrust.ToString() + ",\n";
		json += "        \"rudderAngleDeg\": " + rAngle.ToString() + "\n";
		json += "      },\n";

		// Storage & Comms
		json += SerializeStorageBlock(v.m_Storage);
		json += SerializeCommsBlock(v.m_Comms);

		// Seats
		json += SerializeSeatsBlock(v.m_aSeats);

		// Turrets & Weapons
		json += SerializeTurretsBlock(v.m_aTurrets);

		// HitZones & Damage Thresholds
		json += SerializeHitZonesBlock(v.m_aHitZones);
		json += SerializeThresholdsBlock(v.m_DamageThresholds);

		// Electrical & Instrumentation & Audio
		json += SerializeElectricalBlock(v.m_Electrical);
		json += SerializeInstrumentsBlock(v.m_Instruments);
		json += SerializeAudioBlock(v.m_Audio);

		// Default Cargo
		json += "      \"defaultCargo\": [],\n";

		// Raw Component Variables Dump
		json += SerializeRawVariablesBlock(v.m_mRawVariables);

		json += "    }";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeDrivetrainBlock(TBD_VehicleDrivetrainData drivetrainData)
	{
		if (!drivetrainData) return "      \"drivetrain\": null,\n";

		string json = "      \"drivetrain\": {\n";
		json += "        \"simulationClass\": \"" + drivetrainData.m_sSimulationClass + "\",\n";
		json += "        \"driveConfiguration\": \"" + drivetrainData.m_sDriveConfiguration + "\",\n";
		json += "        \"wheelCount\": " + drivetrainData.m_iWheelCount.ToString() + ",\n";
		json += "        \"axleCount\": " + drivetrainData.m_iAxleCount.ToString() + ",\n";
		json += "        \"engine\": {\n";
		json += "          \"config\": \"" + drivetrainData.m_sEngineConfig + "\",\n";
		json += "          \"peakPowerKw\": " + drivetrainData.m_fEnginePeakPowerKw.ToString() + ",\n";
		json += "          \"peakPowerHp\": " + drivetrainData.m_fEnginePeakPowerHp.ToString() + ",\n";
		json += "          \"peakTorqueNm\": " + drivetrainData.m_fEnginePeakTorqueNm.ToString() + ",\n";
		json += "          \"maxRpm\": " + drivetrainData.m_fEngineMaxRpm.ToString() + ",\n";
		json += "          \"idleRpm\": " + drivetrainData.m_fEngineIdleRpm.ToString() + "\n";
		json += "        },\n";
		json += "        \"transmission\": {\n";
		json += "          \"forwardGears\": " + drivetrainData.m_iGearboxForwardGears.ToString() + ",\n";
		json += "          \"reverseGears\": " + drivetrainData.m_iGearboxReverseGears.ToString() + ",\n";
		json += "          \"gearboxEfficiency\": " + drivetrainData.m_fGearboxEfficiency.ToString() + ",\n";
		json += "          \"finalDriveRatio\": " + drivetrainData.m_fFinalDriveRatio.ToString() + ",\n";
		json += "          \"gearRatios\": [";
		for (int g = 0; g < drivetrainData.m_aGearRatios.Count(); g++)
		{
			json += drivetrainData.m_aGearRatios[g].ToString();
			if (g < drivetrainData.m_aGearRatios.Count() - 1) json += ", ";
		}
		json += "]\n";
		json += "        },\n";
		json += "        \"steering\": {\n";
		json += "          \"maxSteeringAngleDeg\": " + drivetrainData.m_fMaxSteeringAngleDeg.ToString() + ",\n";
		json += "          \"steeredAxles\": [";
		for (int a = 0; a < drivetrainData.m_aSteeredAxles.Count(); a++)
		{
			json += drivetrainData.m_aSteeredAxles[a].ToString();
			if (a < drivetrainData.m_aSteeredAxles.Count() - 1) json += ", ";
		}
		json += "]\n";
		json += "        }\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeSeatsBlock(array<ref TBD_VehicleSeatData> seats)
	{
		string json = "      \"seats\": [\n";
		for (int s = 0, sn = seats.Count(); s < sn; s++)
		{
			TBD_VehicleSeatData seat = seats[s];
			json += "        {\n";
			json += "          \"slotId\": " + seat.m_iSlotId.ToString() + ",\n";
			json += "          \"name\": \"" + seat.m_sName + "\",\n";
			json += "          \"type\": \"" + seat.m_sType + "\",\n";
			json += "          \"uiName\": \"" + TBD_VehicleExportJson.Escape(seat.m_sUIName) + "\",\n";
			json += "          \"door\": \"" + seat.m_sDoor + "\",\n";
			json += "          \"canTurnOut\": " + seat.m_bCanTurnOut.ToString() + "\n";
			json += "        }";
			if (s < sn - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeTurretsBlock(array<ref TBD_VehicleTurretData> turrets)
	{
		string json = "      \"turrets\": [\n";
		for (int t = 0, tn = turrets.Count(); t < tn; t++)
		{
			TBD_VehicleTurretData tur = turrets[t];
			json += "        {\n";
			json += "          \"slotName\": \"" + tur.m_sSlotName + "\",\n";
			json += "          \"turretPrefab\": \"" + tur.m_sTurretPrefab + "\",\n";
			json += "          \"displayName\": \"" + TBD_VehicleExportJson.Escape(tur.m_sDisplayName) + "\",\n";
			json += "          \"limits\": {\n";
			json += "            \"yawMin\": " + tur.m_fYawMin.ToString() + ",\n";
			json += "            \"yawMax\": " + tur.m_fYawMax.ToString() + ",\n";
			json += "            \"pitchMin\": " + tur.m_fPitchMin.ToString() + ",\n";
			json += "            \"pitchMax\": " + tur.m_fPitchMax.ToString() + ",\n";
			json += "            \"yawSpeedDegPerSec\": " + tur.m_fYawSpeed.ToString() + ",\n";
			json += "            \"pitchSpeedDegPerSec\": " + tur.m_fPitchSpeed.ToString() + "\n";
			json += "          },\n";
			json += "          \"optics\": [";
			for (int op = 0; op < tur.m_aOptics.Count(); op++)
			{
				json += "\"" + TBD_VehicleExportJson.Escape(tur.m_aOptics[op]) + "\"";
				if (op < tur.m_aOptics.Count() - 1) json += ", ";
			}
			json += "],\n";
			json += "          \"weapons\": [\n";
			for (int w = 0, wn = tur.m_aWeapons.Count(); w < wn; w++)
			{
				TBD_VehicleMountedWeaponData wep = tur.m_aWeapons[w];
				json += "            {\n";
				json += "              \"slotName\": \"" + wep.m_sSlotName + "\",\n";
				json += "              \"displayName\": \"" + TBD_VehicleExportJson.Escape(wep.m_sDisplayName) + "\",\n";
				json += "              \"weaponPrefab\": \"" + wep.m_sWeaponPrefab + "\",\n";
				json += "              \"weaponType\": \"" + wep.m_sWeaponType + "\",\n";
				json += "              \"defaultMagazine\": {\n";
				json += "                \"prefab\": \"" + wep.m_sDefaultMagazinePrefab + "\",\n";
				json += "                \"displayName\": \"" + TBD_VehicleExportJson.Escape(wep.m_sDefaultMagazineDisplayName) + "\",\n";
				json += "                \"capacity\": " + wep.m_iMagazineCapacity.ToString() + "\n";
				json += "              },\n";
				json += "              \"fireModes\": [";
				for (int fm = 0; fm < wep.m_aFireModes.Count(); fm++)
				{
					json += "\"" + wep.m_aFireModes[fm] + "\"";
					if (fm < wep.m_aFireModes.Count() - 1) json += ", ";
				}
				json += "],\n";
				json += "              \"zeroingDistances\": [";
				for (int zd = 0; zd < wep.m_aZeroingDistances.Count(); zd++)
				{
					json += "\"" + wep.m_aZeroingDistances[zd] + "\"";
					if (zd < wep.m_aZeroingDistances.Count() - 1) json += ", ";
				}
				json += "]\n";
				json += "            }";
				if (w < wn - 1) json += ",";
				json += "\n";
			}
			json += "          ]\n";
			json += "        }";
			if (t < tn - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeHitZonesBlock(array<ref TBD_VehicleHitZoneData> hitZones)
	{
		string json = "      \"hitZones\": [\n";
		for (int hz = 0, hzn = hitZones.Count(); hz < hzn; hz++)
		{
			TBD_VehicleHitZoneData hzd = hitZones[hz];
			json += "        {\n";
			json += "          \"name\": \"" + hzd.m_sName + "\",\n";
			json += "          \"group\": \"" + hzd.m_sGroup + "\",\n";
			json += "          \"maxHealth\": " + hzd.m_fMaxHealth.ToString() + ",\n";
			json += "          \"armorThicknessMm\": " + hzd.m_fArmorThickness.ToString() + "\n";
			json += "        }";
			if (hz < hzn - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeThresholdsBlock(TBD_VehicleDamageThresholds thresholdsData)
	{
		if (!thresholdsData) return "      \"damageThresholds\": null,\n";
		string json = "      \"damageThresholds\": {\n";
		json += "        \"destroyDamage\": " + thresholdsData.m_fVehicleDestroyDamage.ToString() + ",\n";
		json += "        \"collisionVelocity\": " + thresholdsData.m_fCollisionVelocityThreshold.ToString() + ",\n";
		json += "        \"heavyDamage\": " + thresholdsData.m_fHeavyDamageThreshold.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeStorageBlock(TBD_VehicleStorageData storageData)
	{
		if (!storageData) return "      \"storage\": null,\n";
		string json = "      \"storage\": {\n";
		json += "        \"volumeLiters\": " + storageData.m_fVolumeLiters.ToString() + ",\n";
		json += "        \"maxWeightKg\": " + storageData.m_fMaxWeightKg.ToString() + ",\n";
		json += "        \"supplyCapacity\": " + storageData.m_fSupplyCapacity.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeCommsBlock(TBD_VehicleCommsData commsData)
	{
		if (!commsData) return "      \"comms\": null,\n";
		string json = "      \"comms\": {\n";
		json += "        \"radio\": \"" + commsData.m_sRadioType + "\",\n";
		json += "        \"powerW\": " + commsData.m_fTransmitterPowerW.ToString() + ",\n";
		json += "        \"rangeKm\": " + commsData.m_fRangeKm.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeElectricalBlock(TBD_VehicleElectricalData electricalData)
	{
		if (!electricalData) return "      \"electrical\": null,\n";
		string json = "      \"electrical\": {\n";
		json += "        \"totalLightSlots\": " + electricalData.m_iTotalLightSlots.ToString() + ",\n";
		json += "        \"highBeamTurnsOffHeadlights\": " + electricalData.m_bHighBeamTurnsOffHeadlights.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeInstrumentsBlock(TBD_VehicleInstrumentationData instrumentsData)
	{
		if (!instrumentsData) return "      \"instrumentation\": null,\n";
		string json = "      \"instrumentation\": {\n";
		json += "        \"parameterCount\": " + instrumentsData.m_iInstrumentParameterCount.ToString() + ",\n";
		json += "        \"vehicleAnimGraph\": \"" + instrumentsData.m_sVehicleAnimGraph + "\"\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeAudioBlock(TBD_VehicleAudioData audioData)
	{
		if (!audioData) return "      \"audio\": null,\n";
		string json = "      \"audio\": {\n";
		json += "        \"soundPointsCount\": " + audioData.m_iSoundPointsCount.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeRawVariablesBlock(map<string, ref array<string>> rawVars)
	{
		string json = "      \"rawComponentsDump\": {\n";
		int cIdx = 0;
		int total = rawVars.Count();
		foreach (string cName, array<string> cLines : rawVars)
		{
			json += "        \"" + cName + "\": [\n";
			for (int l = 0, ln = cLines.Count(); l < ln; l++)
			{
				json += "          \"" + TBD_VehicleExportJson.Escape(cLines[l]) + "\"";
				if (l < ln - 1) json += ",";
				json += "\n";
			}
			json += "        ]";
			if (cIdx < total - 1) json += ",";
			json += "\n";
			cIdx++;
		}
		json += "      }\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	//! Serialize summary matrix index to JSON.
	static string SerializeSummaryToJson(array<ref TBD_VehicleDeepPlatform> plats, int totalVariants)
	{
		string json = "{\n";
		json += "  \"generatedAt\": \"" + TBD_VehicleExportJson.IsoNowUtc() + "\",\n";
		json += "  \"totalPlatforms\": " + plats.Count().ToString() + ",\n";
		json += "  \"totalVariants\": " + totalVariants.ToString() + ",\n";
		json += "  \"summary\": [\n";

		for (int i = 0, n = plats.Count(); i < n; i++)
		{
			TBD_VehicleDeepPlatform p = plats[i];
			json += "    {\n";
			json += "      \"platformId\": \"" + p.m_sPlatformId + "\",\n";
			json += "      \"displayName\": \"" + TBD_VehicleExportJson.Escape(p.m_sDisplayName) + "\",\n";
			json += "      \"domain\": \"" + p.m_sVehicleDomain + "\",\n";
			json += "      \"primaryFaction\": \"" + p.m_sPrimaryFaction + "\",\n";
			json += "      \"variantCount\": " + p.m_aVariants.Count().ToString() + "\n";
			json += "    }";
			if (i < n - 1) json += ",";
			json += "\n";
		}

		json += "  ]\n";
		json += "}\n";
		return json;
	}
}
