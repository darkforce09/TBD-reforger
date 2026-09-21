/**
 * TBD_VehicleDeepModel.c
 *
 * Typed data models for deep vehicle engineering specifications across all vehicle
 * platforms and variants. Captures mobility, drivetrain, armor hit zones, crew compartments,
 * weapons, optics, electrical, instrumentation, audio, storage, and comms.
 */

class TBD_VehicleSeatData
{
	int m_iSlotId;
	string m_sName;
	string m_sType; // PILOT, TURRET, CARGO
	string m_sUIName;
	string m_sDoor;
	bool m_bCanTurnOut;
}

class TBD_VehicleMountedWeaponData
{
	string m_sSlotName;
	string m_sWeaponPrefab;
	string m_sDisplayName;
	string m_sWeaponType;
	string m_sDefaultMagazinePrefab;
	string m_sDefaultMagazineDisplayName;
	int m_iMagazineCapacity;
	ref array<string> m_aFireModes = {};
	ref array<int> m_aFireModeRpms = {};
	ref array<int> m_aFireModeBursts = {};
	ref array<string> m_aZeroingDistances = {};
}

class TBD_VehicleTurretData
{
	string m_sSlotName;
	string m_sTurretPrefab;
	string m_sDisplayName;
	float m_fYawMin;
	float m_fYawMax;
	float m_fPitchMin;
	float m_fPitchMax;
	float m_fYawSpeed;
	float m_fPitchSpeed;
	ref array<ref TBD_VehicleMountedWeaponData> m_aWeapons = {};
	ref array<string> m_aOptics = {};
}

class TBD_VehicleWheelData
{
	string m_sName;
	int m_iAxleIndex;
	bool m_bIsPowered;
	bool m_bIsSteered;
	float m_fRadius;
	float m_fWidth;
	float m_fMass;
	float m_fBrakeTorque;
	float m_fHandbrakeTorque;
	float m_fSuspensionStiffness;
	float m_fSuspensionDamping;
}

class TBD_VehicleHitZoneData
{
	string m_sName;
	string m_sGroup;
	float m_fMaxHealth;
	float m_fArmorThickness;
	float m_fDamageMultiplier = 1.0;
}

class TBD_VehicleDamageThresholds
{
	float m_fVehicleDestroyDamage = -1.0;
	float m_fCollisionVelocityThreshold = -1.0;
	float m_fHeavyDamageThreshold = -1.0;
	float m_fOccupantsDamageSpeedThreshold = -1.0;
	float m_fOccupantsSpeedDeath = -1.0;
	float m_fFrontMultiplier = 1.0;
	float m_fRearMultiplier = 1.0;
	float m_fLeftMultiplier = 1.0;
	float m_fRightMultiplier = 1.0;
	float m_fTopMultiplier = 1.0;
	float m_fBottomMultiplier = 1.0;
	string m_sSecondaryExplosionsConf;
	string m_sSecondaryFiresConf;
}

class TBD_VehicleDrivetrainData
{
	string m_sSimulationClass;
	string m_sDriveConfiguration;
	int m_iWheelCount;
	int m_iAxleCount;
	string m_sEngineConfig;
	float m_fEnginePeakPowerKw = -1.0;
	float m_fEnginePeakPowerHp = -1.0;
	float m_fEnginePeakTorqueNm = -1.0;
	float m_fEngineMaxRpm = -1.0;
	float m_fEngineIdleRpm = -1.0;
	float m_fClutchTorque = -1.0;
	int m_iGearboxForwardGears;
	int m_iGearboxReverseGears = 1;
	ref array<float> m_aGearRatios = {};
	float m_fGearboxEfficiency = -1.0;
	float m_fFinalDriveRatio = -1.0;
	float m_fMaxSteeringAngleDeg = -1.0;
	ref array<int> m_aSteeredAxles = {};
	ref array<string> m_aDifferentialTypes = {};
	ref array<float> m_aDifferentialRatios = {};
	ref array<ref TBD_VehicleWheelData> m_aWheels = {};
	bool m_bIsAmphibious;
	float m_fWaterThrust = -1.0;
	float m_fWaterRudderAngle = -1.0;
}

class TBD_VehicleElectricalData
{
	int m_iTotalLightSlots;
	ref array<string> m_aLightSlotNames = {};
	ref array<string> m_aEmissiveSurfaces = {};
	bool m_bHighBeamTurnsOffHeadlights;
}

class TBD_VehicleInstrumentationData
{
	int m_iInstrumentParameterCount;
	ref array<string> m_aInstrumentParameters = {};
	string m_sVehicleAnimGraph;
	string m_sPlayerAnimGraph;
}

class TBD_VehicleAudioData
{
	int m_iSoundPointsCount;
	ref array<string> m_aSoundPoints = {};
	ref array<string> m_aSoundSignals = {};
}

class TBD_VehicleCargoItemData
{
	string m_sPrefab;
	string m_sDisplayName;
	int m_iCount = 1;
	string m_sTargetStorage;
}

class TBD_VehicleStorageData
{
	float m_fVolumeLiters = -1.0;
	float m_fMaxWeightKg = -1.0;
	float m_fSupplyCapacity = -1.0;
	ref array<ref TBD_VehicleCargoItemData> m_aDefaultCargo = {};
}

class TBD_VehicleCommsData
{
	string m_sRadioType;
	float m_fTransmitterPowerW = -1.0;
	float m_fRangeKm = -1.0;
}

class TBD_VehicleDeepVariant
{
	string m_sId;
	string m_sResourceName;
	string m_sDisplayName;
	string m_sDescription;
	string m_sPlatform;
	string m_sFaction;
	string m_sVehicleDomain;
	string m_sRole;
	string m_sFilePath;
	string m_sAddonId;
	string m_sParentPrefab;
	bool m_bIsAbstract;

	// Dimensions & Mass
	float m_fWeightKg = -1.0;
	float m_fLengthM = -1.0;
	float m_fWidthM = -1.0;
	float m_fHeightM = -1.0;
	vector m_vCenterOfMass;

	// Fuel
	float m_fFuelCapacityLiters = -1.0;
	float m_fFuelConsumptionRate = -1.0;

	// Subsystem Engineering Structures
	ref TBD_VehicleDrivetrainData m_Drivetrain;
	ref TBD_VehicleDamageThresholds m_DamageThresholds;
	ref array<ref TBD_VehicleHitZoneData> m_aHitZones = {};
	ref array<ref TBD_VehicleSeatData> m_aSeats = {};
	ref array<ref TBD_VehicleTurretData> m_aTurrets = {};
	ref TBD_VehicleElectricalData m_Electrical;
	ref TBD_VehicleInstrumentationData m_Instruments;
	ref TBD_VehicleAudioData m_Audio;
	ref TBD_VehicleStorageData m_Storage;
	ref TBD_VehicleCommsData m_Comms;

	// Raw Variable Reflection Dump (for 100% data preservation)
	ref map<string, ref array<string>> m_mRawVariables = new map<string, ref array<string>>();
}

class TBD_VehicleDeepPlatform
{
	string m_sPlatformId;
	string m_sDisplayName;
	string m_sVehicleDomain;
	string m_sPrimaryFaction;
	ref array<ref TBD_VehicleDeepVariant> m_aVariants = {};
}
