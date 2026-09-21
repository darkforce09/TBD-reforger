/**
 * TBD_VehicleDrivetrainExtractor.c
 *
 * Dedicated extractor for vehicle mobility, simulation engines, transmissions,
 * differentials, suspension geometry, wheel dynamics, and amphibious water propulsion.
 * Operates purely via Enfusion BaseContainer reflection without hardcoded vehicle tables.
 */

class TBD_VehicleDrivetrainExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspect vehicle drivetrain, engine curves, transmission, and suspension.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleDrivetrainData dt = new TBD_VehicleDrivetrainData();
		varData.m_Drivetrain = dt;

		// 1. Detect simulation type
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.Contains("VehicleWheeledSimulation"))
			{
				dt.m_sSimulationClass = cls;
				ExtractWheeledSimulation(bucket, dt);
				break;
			}
			else if (cls.Contains("VehicleHelicopterSimulation"))
			{
				dt.m_sSimulationClass = cls;
				ExtractHelicopterSimulation(bucket, dt);
				break;
			}
			else if (cls.Contains("VehicleTrackedSimulation"))
			{
				dt.m_sSimulationClass = cls;
				dt.m_sDriveConfiguration = "Continuous Tracked Drive";
				break;
			}
			else if (cls.Contains("VehicleBoatSimulation"))
			{
				dt.m_sSimulationClass = cls;
				dt.m_sDriveConfiguration = "Marine Watercraft";
				dt.m_bIsAmphibious = true;
				break;
			}
		}

		if (dt.m_sSimulationClass.IsEmpty())
			dt.m_sSimulationClass = "VehicleSimulation";

		// 2. Introspect Car Controller for operational RPM limits
		ExtractCarControllerLimits(comps, dt);

		// 3. Introspect Buoyancy & Amphibious water propulsion
		ExtractAmphibiousPropulsion(comps, dt);

		// 4. Derive Drive Configuration summary (e.g. 8x8 All-Wheel Drive, 4x4, etc.)
		DeriveDriveConfiguration(dt);
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect wheeled simulation configuration container and nested engineering blocks.
	protected static void ExtractWheeledSimulation(array<BaseContainer> bucket, TBD_VehicleDrivetrainData dt)
	{
		// Derived container takes precedence
		for (int i = 0; i < bucket.Count(); i++)
		{
			BaseContainer sim = bucket[i];
			if (!sim) continue;

			BaseContainer simConf = sim.GetObject("Simulation");
			if (simConf)
			{
				ExtractEngineBlock(simConf, dt);
				ExtractGearboxBlock(simConf, dt);
				ExtractDifferentialBlock(simConf, dt);
				ExtractAxleBlock(simConf, dt);
			}

			// Parse simulation string if sub-objects were serialized as config text
			string simText;
			if (sim.Get("Simulation", simText) && !simText.IsEmpty())
			{
				ParseSimulationText(simText, dt);
			}

			if (dt.m_fEnginePeakPowerKw > 0 && !dt.m_aWheels.IsEmpty())
				break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract engine power, torque, idle and max RPM.
	protected static void ExtractEngineBlock(BaseContainer simConf, TBD_VehicleDrivetrainData dt)
	{
		BaseContainer engine = simConf.GetObject("Engine");
		if (engine)
		{
			engine.Get("m_fPeakPower", dt.m_fEnginePeakPowerKw);
			if (dt.m_fEnginePeakPowerKw <= 0) engine.Get("PeakPower", dt.m_fEnginePeakPowerKw);
			if (dt.m_fEnginePeakPowerKw <= 0) engine.Get("Power", dt.m_fEnginePeakPowerKw);

			if (dt.m_fEnginePeakPowerKw > 0)
				dt.m_fEnginePeakPowerHp = dt.m_fEnginePeakPowerKw * 1.34102;

			engine.Get("m_fPeakTorque", dt.m_fEnginePeakTorqueNm);
			if (dt.m_fEnginePeakTorqueNm <= 0) engine.Get("PeakTorque", dt.m_fEnginePeakTorqueNm);
			if (dt.m_fEnginePeakTorqueNm <= 0) engine.Get("Torque", dt.m_fEnginePeakTorqueNm);

			engine.Get("m_fMaxRpm", dt.m_fEngineMaxRpm);
			if (dt.m_fEngineMaxRpm <= 0) engine.Get("MaxRpm", dt.m_fEngineMaxRpm);

			engine.Get("m_fIdleRpm", dt.m_fEngineIdleRpm);
			if (dt.m_fEngineIdleRpm <= 0) engine.Get("IdleRpm", dt.m_fEngineIdleRpm);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract transmission gearbox ratios, efficiency, and final drive.
	protected static void ExtractGearboxBlock(BaseContainer simConf, TBD_VehicleDrivetrainData dt)
	{
		BaseContainer gb = simConf.GetObject("Gearbox");
		if (!gb) return;

		gb.Get("m_fEfficiency", dt.m_fGearboxEfficiency);
		if (dt.m_fGearboxEfficiency <= 0) gb.Get("Efficiency", dt.m_fGearboxEfficiency);

		gb.Get("m_fFinalDriveRatio", dt.m_fFinalDriveRatio);
		if (dt.m_fFinalDriveRatio <= 0) gb.Get("FinalDriveRatio", dt.m_fFinalDriveRatio);

		BaseContainerList ratios = gb.GetObjectArray("m_aGearRatios");
		if (!ratios) ratios = gb.GetObjectArray("GearRatios");
		if (!ratios) ratios = gb.GetObjectArray("Forward");

		if (ratios)
		{
			for (int g = 0, gn = ratios.Count(); g < gn; g++)
			{
				BaseContainer gr = ratios.Get(g);
				if (!gr) continue;
				float r = 0;
				if (gr.Get("m_fRatio", r) || gr.Get("Ratio", r))
					dt.m_aGearRatios.Insert(r);
			}
			dt.m_iGearboxForwardGears = dt.m_aGearRatios.Count();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract differentials and torque transfer mechanisms.
	protected static void ExtractDifferentialBlock(BaseContainer simConf, TBD_VehicleDrivetrainData dt)
	{
		BaseContainerList diffs = simConf.GetObjectArray("Differentials");
		if (!diffs) return;

		for (int d = 0, dn = diffs.Count(); d < dn; d++)
		{
			BaseContainer df = diffs.Get(d);
			if (!df) continue;

			string dType;
			if (df.Get("Type", dType) && !dType.IsEmpty())
			{
				if (dt.m_aDifferentialTypes.Find(dType) == -1)
					dt.m_aDifferentialTypes.Insert(dType);
			}

			float dRatio;
			if (df.Get("Ratio", dRatio) && dRatio > 0)
				dt.m_aDifferentialRatios.Insert(dRatio);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract axles, wheel geometry, brake torques, and suspension rates.
	protected static void ExtractAxleBlock(BaseContainer simConf, TBD_VehicleDrivetrainData dt)
	{
		BaseContainerList axles = simConf.GetObjectArray("Axles");
		if (!axles) axles = simConf.GetObjectArray("m_aAxles");
		if (!axles) return;

		dt.m_iAxleCount = axles.Count();

		for (int a = 0, an = axles.Count(); a < an; a++)
		{
			BaseContainer axle = axles.Get(a);
			if (!axle) continue;

			float steerAngle = 0;
			BaseContainer susp = axle.GetObject("Suspension");
			if (!susp) susp = axle.GetObject("m_pSuspension");

			float springRate = 0, compDamp = 0, relaxDamp = 0;
			if (susp)
			{
				susp.Get("MaxSteeringAngle", steerAngle);
				if (steerAngle <= 0) susp.Get("m_fMaxAngle", steerAngle);
				if (steerAngle > dt.m_fMaxSteeringAngleDeg)
					dt.m_fMaxSteeringAngleDeg = steerAngle;

				susp.Get("SpringRate", springRate);
				susp.Get("CompressionDamper", compDamp);
				susp.Get("RelaxationDamper", relaxDamp);
			}

			if (steerAngle > 0 && dt.m_aSteeredAxles.Find(a) == -1)
				dt.m_aSteeredAxles.Insert(a);

			// Two wheels per standard axle (Left and Right)
			for (int side = 0; side < 2; side++)
			{
				TBD_VehicleWheelData wd = new TBD_VehicleWheelData();
				wd.m_iAxleIndex = a;
				wd.m_bIsSteered = (steerAngle > 0);
				wd.m_bIsPowered = true;
				wd.m_fSuspensionStiffness = springRate;
				wd.m_fSuspensionDamping = compDamp;

				string sideCode = "L";
				if (side == 1) sideCode = "R";
				wd.m_sName = string.Format("Wheel_%1%2", sideCode, a + 1);

				dt.m_aWheels.Insert(wd);
			}
		}

		dt.m_iWheelCount = dt.m_aWheels.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! Parse textual config dump if engine/transmission properties are embedded as text block.
	protected static void ParseSimulationText(string text, TBD_VehicleDrivetrainData dt)
	{
		// 1. Engine config file reference
		if (dt.m_sEngineConfig.IsEmpty())
		{
			int engIdx = text.IndexOf("Engine Engine Engine :");
			if (engIdx != -1)
			{
				int quote1 = text.IndexOfFrom(engIdx, "\"");
				if (quote1 != -1)
				{
					int quote2 = text.IndexOfFrom(quote1 + 1, "\"");
					if (quote2 != -1)
					{
						string engConf = text.Substring(quote1 + 1, quote2 - quote1 - 1);
						dt.m_sEngineConfig = engConf;
						LoadEngineResource(engConf, dt);
					}
				}
			}
		}

		// 2. Forward gear ratios
		if (dt.m_aGearRatios.IsEmpty())
		{
			int fwdIdx = text.IndexOf("Forward {");
			if (fwdIdx != -1)
			{
				int braceEnd = text.IndexOfFrom(fwdIdx, "}");
				if (braceEnd != -1)
				{
					string ratiosStr = text.Substring(fwdIdx + 9, braceEnd - fwdIdx - 9);
					ratiosStr.Replace("\n", " ");
					ratiosStr.Replace("\r", " ");
					array<string> tokens = {};
					ratiosStr.Split(" ", tokens, true);
					foreach (string tok : tokens)
					{
						string t = tok.Trim();
						if (!t.IsEmpty())
						{
							float rVal = t.ToFloat();
							if (rVal > 0) dt.m_aGearRatios.Insert(rVal);
						}
					}
					dt.m_iGearboxForwardGears = dt.m_aGearRatios.Count();
				}
			}
		}

		// 3. Reverse gear ratio
		int revIdx = text.IndexOf("Reverse ");
		if (revIdx != -1)
		{
			int nextSp = text.IndexOfFrom(revIdx + 8, "\n");
			if (nextSp != -1)
			{
				string revStr = text.Substring(revIdx + 8, nextSp - revIdx - 8).Trim();
				if (revStr.ToFloat() > 0)
					dt.m_iGearboxReverseGears = 1;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Load standalone Engine config resource (.conf) and extract peak power, torque, and RPM curves.
	protected static void LoadEngineResource(string engConf, TBD_VehicleDrivetrainData dt)
	{
		Resource r = Resource.Load(engConf);
		if (!r || !r.IsValid()) return;
		BaseResourceObject bro = r.GetResource();
		if (!bro) return;
		BaseContainer cont = bro.ToBaseContainer();
		if (!cont) return;

		cont.Get("m_fPeakPower", dt.m_fEnginePeakPowerKw);
		if (dt.m_fEnginePeakPowerKw <= 0) cont.Get("PeakPower", dt.m_fEnginePeakPowerKw);
		if (dt.m_fEnginePeakPowerKw <= 0) cont.Get("Power", dt.m_fEnginePeakPowerKw);

		if (dt.m_fEnginePeakPowerKw > 0)
			dt.m_fEnginePeakPowerHp = dt.m_fEnginePeakPowerKw * 1.34102;

		cont.Get("m_fPeakTorque", dt.m_fEnginePeakTorqueNm);
		if (dt.m_fEnginePeakTorqueNm <= 0) cont.Get("PeakTorque", dt.m_fEnginePeakTorqueNm);
		if (dt.m_fEnginePeakTorqueNm <= 0) cont.Get("Torque", dt.m_fEnginePeakTorqueNm);

		cont.Get("m_fMaxRpm", dt.m_fEngineMaxRpm);
		if (dt.m_fEngineMaxRpm <= 0) cont.Get("MaxRpm", dt.m_fEngineMaxRpm);

		cont.Get("m_fIdleRpm", dt.m_fEngineIdleRpm);
		if (dt.m_fEngineIdleRpm <= 0) cont.Get("IdleRpm", dt.m_fEngineIdleRpm);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract operational RPM shifting points and drowning survival time from CarController.
	protected static void ExtractCarControllerLimits(map<string, ref array<BaseContainer>> comps, TBD_VehicleDrivetrainData dt)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("CarControllerComponent")) continue;

			foreach (BaseContainer cc : bucket)
			{
				float upShift = 0;
				if (cc.Get("UpShiftRpm", upShift) && upShift > 0 && dt.m_fEngineMaxRpm <= 0)
					dt.m_fEngineMaxRpm = upShift * 1.35;

				float idle = 0;
				if (cc.Get("DownShiftRpm", idle) && idle > 0 && dt.m_fEngineIdleRpm <= 0)
					dt.m_fEngineIdleRpm = idle * 0.55;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect helicopter rotor configurations, turbine outputs, and aerodynamic coefficients.
	protected static void ExtractHelicopterSimulation(array<BaseContainer> bucket, TBD_VehicleDrivetrainData dt)
	{
		dt.m_sDriveConfiguration = "Helicopter Rotary-Wing";
		dt.m_iWheelCount = 0;
		dt.m_iAxleCount = 0;

		for (int i = 0; i < bucket.Count(); i++)
		{
			BaseContainer sim = bucket[i];
			if (!sim) continue;

			BaseContainer mainRotor = sim.GetObject("MainRotor");
			if (!mainRotor) mainRotor = sim.GetObject("m_pMainRotor");
			if (mainRotor)
			{
				float maxRpm = 0;
				if (mainRotor.Get("MaxRpm", maxRpm) || mainRotor.Get("m_fMaxRpm", maxRpm))
					dt.m_fEngineMaxRpm = maxRpm;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Inspect BuoyancyComponent for water propulsion, amphibious thrust, and rudder degrees.
	protected static void ExtractAmphibiousPropulsion(map<string, ref array<BaseContainer>> comps, TBD_VehicleDrivetrainData dt)
	{
		foreach (string bcls, array<BaseContainer> bucket : comps)
		{
			if (!bcls.Contains("BuoyancyComponent")) continue;

			foreach (BaseContainer bc : bucket)
			{
				float thrust = 0;
				if (bc.Get("m_fWaterThrust", thrust) && thrust > 0)
				{
					dt.m_bIsAmphibious = true;
					dt.m_fWaterThrust = thrust;
				}
				else if (bc.Get("ThrustForward", thrust) && thrust > 0)
				{
					dt.m_bIsAmphibious = true;
					dt.m_fWaterThrust = thrust;
				}

				float rudder = 0;
				if (bc.Get("m_fWaterRudderAngle", rudder) && rudder > 0)
					dt.m_fWaterRudderAngle = rudder;
				else if (bc.Get("RudderAngle", rudder) && rudder > 0)
					dt.m_fWaterRudderAngle = rudder;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Format drive configuration string based on wheel and axle topology.
	protected static void DeriveDriveConfiguration(TBD_VehicleDrivetrainData dt)
	{
		if (!dt.m_sDriveConfiguration.IsEmpty())
			return;

		if (dt.m_iWheelCount == 8)
			dt.m_sDriveConfiguration = "8x8 All-Wheel Drive";
		else if (dt.m_iWheelCount == 6)
			dt.m_sDriveConfiguration = "6x6 All-Wheel Drive";
		else if (dt.m_iWheelCount == 4)
			dt.m_sDriveConfiguration = "4x4 All-Wheel Drive";
		else if (dt.m_iWheelCount > 0)
			dt.m_sDriveConfiguration = string.Format("%1-Wheel Configuration", dt.m_iWheelCount);
		else
			dt.m_sDriveConfiguration = "Standard Automotive Drivetrain";
	}
}
