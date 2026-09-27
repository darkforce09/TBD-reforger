/**
 * TBD_VehicleDrivetrainExtractor.c
 *
 * Dedicated extractor for vehicle mobility, simulation engines, transmissions,
 * differentials, suspension geometry, wheel positions, and native water propulsion.
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
		dt.m_iWheelCount = -1;
		dt.m_iAxleCount = -1;
		dt.m_iGearboxForwardGears = -1;
		dt.m_iGearboxReverseGears = -1;
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "VehicleBaseSimulation")) continue;
			if (!dt.m_sSimulationClass.IsEmpty() || bucket.Count() != 1)
			{
				continue;
			}
			dt.m_sSimulationClass = cls;
			if (TBD_EquipmentComponentGraph.IsA(cls, "VehicleWheeledSimulation"))
				ExtractWheeledSimulation(bucket, dt);
		}
		ExtractAmphibiousPropulsion(comps, dt);
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect wheeled simulation configuration container and nested engineering blocks.
	protected static void ExtractWheeledSimulation(array<BaseContainer> bucket, TBD_VehicleDrivetrainData dt)
	{
		if (bucket.IsEmpty() || !bucket[0]) return;
		BaseContainer simConf = bucket[0].GetObject("Simulation");
		if (!simConf) return;
		ExtractEngineBlock(simConf, dt);
		ExtractGearboxBlock(simConf, dt);
		ExtractDifferentialBlock(simConf, dt);
		ExtractAxleBlock(simConf, dt);
	}

	//------------------------------------------------------------------------------------------------
	//! Reads the effective engine and clutch configuration without unit conversions.
	protected static void ExtractEngineBlock(BaseContainer simConf, TBD_VehicleDrivetrainData dt)
	{
		BaseContainer engine = simConf.GetObject("Engine");
		if (engine)
		{
			dt.m_sEngineConfig = engine.GetResourceName();
			engine.Get("MaxPower", dt.m_fEnginePeakPowerKw);
			engine.Get("MaxTorque", dt.m_fEnginePeakTorqueNm);
			engine.Get("RpmMax", dt.m_fEngineMaxRpm);
			engine.Get("RpmIdle", dt.m_fEngineIdleRpm);
		}
		BaseContainer clutch = simConf.GetObject("Clutch");
		if (clutch) clutch.Get("MaxClutchTorque", dt.m_fClutchTorque);
	}

	//------------------------------------------------------------------------------------------------
	//! Preserves the ordered native forward-gear ratios, including explicit empty lists.
	protected static void ExtractGearboxBlock(BaseContainer simConf, TBD_VehicleDrivetrainData dt)
	{
		BaseContainer gearbox = simConf.GetObject("Gearbox");
		if (!gearbox) return;
		array<float> forward = {};
		if (gearbox.Get("Forward", forward))
		{
			foreach (float ratio : forward) dt.m_aGearRatios.Insert(ratio);
			dt.m_iGearboxForwardGears = forward.Count();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract differentials and torque transfer mechanisms.
	protected static void ExtractDifferentialBlock(BaseContainer simConf, TBD_VehicleDrivetrainData dt)
	{
		BaseContainerList differentials = simConf.GetObjectArray("Differentials");
		if (!differentials) return;
		for (int i = 0; i < differentials.Count(); i++)
		{
			BaseContainer differential = differentials.Get(i);
			if (!differential) continue;
			int nativeType;
			if (differential.Get("Type", nativeType))
				dt.m_aDifferentialTypes.Insert(nativeType.ToString());
			float ratio;
			if (differential.Get("Ratio", ratio)) dt.m_aDifferentialRatios.Insert(ratio);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract axles, wheel geometry, brake torques, and suspension rates.
	protected static void ExtractAxleBlock(BaseContainer simConf, TBD_VehicleDrivetrainData dt)
	{
		BaseContainerList axles = simConf.GetObjectArray("Axles");
		if (!axles) return;
		dt.m_iAxleCount = axles.Count();
		for (int a = 0; a < axles.Count(); a++)
		{
			BaseContainer axle = axles.Get(a);
			if (!axle) continue;
			BaseContainer suspension = axle.GetObject("Suspension");
			BaseContainer wheel = axle.GetObject("Wheel");
			BaseContainerList positions = axle.GetObjectArray("WheelPositions");
			if (!positions)
			{
				TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, "drivetrain/axles/" + a.ToString(), "WheelPositions is unreadable");
				continue;
			}
			for (int p = 0; p < positions.Count(); p++)
			{
				BaseContainer position = positions.Get(p);
				if (!position) continue;
				TBD_VehicleWheelData data = new TBD_VehicleWheelData();
				data.m_sName = position.GetName();
				data.m_iAxleIndex = a;
				data.m_fRadius = -1;
				data.m_fWidth = -1;
				data.m_fMass = -1;
				data.m_fBrakeTorque = -1;
				data.m_fHandbrakeTorque = -1;
				data.m_fSuspensionStiffness = -1;
				data.m_fSuspensionDamping = -1;
				if (wheel)
				{
					wheel.Get("Radius", data.m_fRadius);
					wheel.Get("Width", data.m_fWidth);
					wheel.Get("Mass", data.m_fMass);
					wheel.Get("BrakeTorque", data.m_fBrakeTorque);
				}
				if (suspension)
				{
					suspension.Get("SpringRate", data.m_fSuspensionStiffness);
					suspension.Get("CompressionDamper", data.m_fSuspensionDamping);
				}
				dt.m_aWheels.Insert(data);
			}
		}
		dt.m_iWheelCount = dt.m_aWheels.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! Reads native forward water thrust without deriving flotation or rudder angle.
	protected static void ExtractAmphibiousPropulsion(map<string, ref array<BaseContainer>> comps, TBD_VehicleDrivetrainData dt)
	{
		array<BaseContainer> buoyancy = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BuoyancyComponent")) continue;
			foreach (BaseContainer component : bucket)
				if (component) buoyancy.Insert(component);
		}
		if (buoyancy.Count() == 1) buoyancy[0].Get("ThurstForward", dt.m_fWaterThrust);
	}

	//! Emits native configurations per simulation and installation, without merging unlike values.
	static void ExtractSource(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		array<string> simulations = {};
		array<string> controllers = {};
		array<string> water = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer component : bucket)
			{
				if (!component) continue;
				if (TBD_EquipmentComponentGraph.IsA(cls, "VehicleBaseSimulation"))
				{
					string path = "/drivetrain/simulations/" + simulations.Count().ToString();
					BaseContainer simulation = component.GetObject("Simulation");
					if (!simulation)
					{
						TBD_EquipmentExportJson.ExtractionError(varData.m_sResourceName, path, "Vehicle simulation configuration is unreadable");
						continue;
					}
					string data = SimulationSource(component, simulation, path);
					simulations.Insert(data);
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "VehicleControllerComponent"))
				{
					string controllerPath = "/drivetrain/controllers/" + controllers.Count().ToString();
					string controllerBindings = "max_startup_time=MaxStartupTime|max_startup_attempts=MaxStartupAttempts|engine_startup_chance=EngineStartupChance";
					controllerBindings += "|max_wants_to_start_time=MaxWantsToStartTime|max_lights_time=MaxLightsTime|shutdown_time=ShutdownTime|drowning_time=DrowningTime";
					controllerBindings += "|enabled=Enabled|apply_controls=ApplyControls|transmission_rnd=TransmissionRND|steering_backward_speed=SteeringBackwardSpeed|throttle_curve=ThrottleCurve|reverse_curve=ReverseCurve|throttle_reaction_time=ThrottleReactionTime";
					controllerBindings += "|clutch_feedback_factor=ClutchFeedbackFactor|clutch_uncouple_factor=ClutchUncoupleFactor|clutch_couple_factor=ClutchCoupleFactor|clutch_minimum_position=ClutchMinimumPosition|clutch_minimum_factor=ClutchMinimumFactor|clutch_maximum_factor=ClutchMaximumFactor";
					controllerBindings += "|up_shift_rpm=UpShiftRpm|down_shift_rpm=DownShiftRpm|up_shift_factor=UpShiftFactor|down_shift_factor=DownShiftFactor";
					controllerBindings += "|clutch_couple_time=ClutchCoupleTime|clutch_feedback_rpm=ClutchFeedbackRpm|clutch_uncouple_rpm=ClutchUncoupleRpm|clutch_couple_rpm=ClutchCoupleRpm|clutch_maximum_position=ClutchMaximumPosition";
					controllerBindings += "|ground_assist_disable_time=GroundAssistDisableTime|ground_assist_enable_time=GroundAssistEnableTime|ground_assist_collective_limit=GroundAssistCollectiveLimit";
					controllerBindings += "|type=Type|steering_forward_speed=SteeringForwardSpeed|steering_center_speed=SteeringCenterSpeed|throttle_turbo_time=ThrottleTurboTime|throttle_turbo=ThrottleTurbo|throttle_reverse_target=ThrottleReverseTarget";
					controllerBindings += "|clutch_uncouple_time=ClutchUncoupleTime|clutch_couple_forward_rpm=ClutchCoupleForwardRpm|clutch_couple_reverse_rpm=ClutchCoupleReverseRpm|clutch_couple_uphill_factor=ClutchCoupleUphillFactor";
					controllerBindings += "|braking_curve=BrakingCurve|brake_turbo_time=BrakeTurboTime|rpm_smoothing=RpmSmoothing|slope_smoothing=SlopeSmoothing|latency=Latency";
					controllerBindings += "|up_shift_factor_downhill=UpShiftFactorDownhill|down_shift_factor_downhill=DownShiftFactorDownhill|steering_factor_upshift=SteeringFactorUpshift|steering_factor_downshift=SteeringFactorDownshift|turbo_shift_factor=TurboShiftFactor";
					controllerBindings += "|peak_torque_upshifting_hysteresis=PeakTorqueUpshiftingHysteresis|peak_torque_downshifting_hysteresis=PeakTorqueDownshiftingHysteresis|peak_power_upshifting_hysteresis=PeakPowerUpshiftingHysteresis|peak_power_downshifting_hysteresis=PeakPowerDownshiftingHysteresis";
					controllerBindings += "|peak_power_upshift_factor_uphill=PeakPowerUpShiftFactorUphill|peak_power_upshift_factor_downhill=PeakPowerUpShiftFactorDownhill|peak_power_downshift_factor_uphill=PeakPowerDownShiftFactorUphill|peak_power_downshift_factor_downhill=PeakPowerDownShiftFactorDownhill";
					string controller = SourceFields(component, controllerBindings, controllerPath);
					string intakes = SourceArray(component, "AirIntakes", "pivot_id=PivotID|offset=Offset|angles=Angles", controllerPath + "/air_intakes");
					controllers.Insert(TBD_EquipmentExportJson.Member(controller, "air_intakes", intakes));
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "BuoyancyComponent"))
				{
					string waterPath = "/water_interaction/instances/" + water.Count().ToString();
					string bindings = "buoyancy=Buoyancy|apply_distance_scale=BuoyancyApplyDistanceScale|depth_offset=BuoyancyDepthOffset";
					bindings += "|hydrodynamic_scale_linear=HydrodynamicScaleLinear|hydrodynamic_scale_angular=HydrodynamicScaleAngular";
					bindings += "|bounding_box_scale=BoundingBoxScale|bounding_box_offset=BoundingBoxOffset|thrust_points=ThrustPoints";
					bindings += "|thrust_forward=ThurstForward|thrust_reverse=ThurstReverse|thrust_steering=ThurstSteering|buoyancy_loss=BuoyancyLoss|buoyancy_gain=BuoyancyGain";
					water.Insert(SourceFields(component, bindings, waterPath));
				}
			}
		}
		if (!simulations.IsEmpty() || !controllers.IsEmpty())
		{
			varData.m_sDrivetrainJson = "{\"simulations\":[" + TBD_EquipmentExportJson.Join(simulations) + "]";
			varData.m_sDrivetrainJson += ",\"controllers\":[" + TBD_EquipmentExportJson.Join(controllers) + "]}";
		}
		if (!water.IsEmpty()) varData.m_sWaterJson = "{\"instances\":[" + TBD_EquipmentExportJson.Join(water) + "]}";
	}

	//! Keeps wheeled engine settings distinct from helicopter engine and rotor measurements.
	protected static string SimulationSource(BaseContainer owner, BaseContainer simulation, string path)
	{
		string data = SourceFields(simulation, "inertia_override_enabled=InertiaOverrideEnabled|inertia_override=InertiaOverride", path);
		data = TBD_EquipmentExportJson.Member(data, "installation", TBD_EquipmentExportJson.Context(owner));
		BaseContainer engine = simulation.GetObject("Engine");
		if (engine)
		{
			string engineBindings;
			if (TBD_EquipmentComponentGraph.IsA(owner.GetClassName(), "VehicleHelicopterSimulation"))
				engineBindings = "rpm_max=RPMMax|rpm_idle=RPMIdle|start_up_time=StartUpTime|shutdown_time=ShutdownTime";
			else
			{
				engineBindings = "max_power=MaxPower|max_torque=MaxTorque|rpm_max_power=RpmMaxPower|rpm_max_torque=RpmMaxTorque";
				engineBindings += "|rpm_idle=RpmIdle|rpm_redline=RpmRedline|rpm_max=RpmMax|inertia=Inertia|inertia_coupled=InertiaCoupled|friction=Friction|steepness=Steepness|output=Output";
			}
			data = TBD_EquipmentExportJson.Member(data, "engine", SourceFields(engine, engineBindings, path + "/engine"));
		}
		data = AddObject(data, simulation, "Clutch", "clutch", "max_clutch_torque=MaxClutchTorque|output=Output", path);
		data = AddObject(data, simulation, "Gearbox", "gearbox", "forward=Forward|reverse=Reverse|efficiency=Efficiency|output=Output", path);
		data = AddObject(data, simulation, "Aerodynamics", "aerodynamics", "drag_coefficient=DragCoefficient", path);
		string hullBindings = "tail_stabilization_x=TailStabilizationX|tail_stabilization_y=TailStabilizationY|tail_stabilization_force_x=TailStabilizationForceX|tail_stabilization_force_y=TailStabilizationForceY";
		hullBindings += "|tail_stabilization_speed_coefficient=TailStabilizationSpeedCoef|friction_coefficients_x=FrictionCoefsX|friction_coefficients_y=FrictionCoefsY|friction_coefficients_z=FrictionCoefsZ";
		hullBindings += "|friction_force_x=FrictionForceX|friction_force_y=FrictionForceY|friction_force_z=FrictionForceZ|angular_friction_x=AngularFrictionX|angular_friction_y=AngularFrictionY|angular_friction_z=AngularFrictionZ";
		hullBindings += "|angular_friction_speed_x=AngularFrictionSpeedX|angular_friction_speed_y=AngularFrictionSpeedY|angular_friction_speed_z=AngularFrictionSpeedZ";
		hullBindings += "|angular_friction_force_x=AngularFrictionForceX|angular_friction_force_y=AngularFrictionForceY|angular_friction_force_z=AngularFrictionForceZ";
		hullBindings += "|bank_turn_force=BankTurnForce|bank_speed_effect=BankSpeedEffect|override_inertia=OverrideInertia|inertia=Inertia";
		data = AddObject(data, simulation, "Hull", "hull", hullBindings, path);
		string differential = "type=Type|ratio=Ratio|strength=Strength|anti_slip=Anti slip|anti_slip_torque=Anti slip torque|output_0=Output0|output_1=Output1";
		if (simulation.GetVarIndex("Differentials") >= 0)
			data = TBD_EquipmentExportJson.Member(data, "differentials", SourceArray(simulation, "Differentials", differential, path + "/differentials"));
		BaseContainerList axles = simulation.GetObjectArray("Axles");
		if (axles) data = TBD_EquipmentExportJson.Member(data, "axles", AxleSources(axles, differential, path + "/axles"));
		BaseContainerList rotors = simulation.GetObjectArray("Rotors");
		if (rotors) data = TBD_EquipmentExportJson.Member(data, "rotors", RotorSources(rotors, path + "/rotors"));
		BaseContainer landingGear = simulation.GetObject("LandingGear");
		if (landingGear)
		{
			BaseContainerList configurations = landingGear.GetObjectArray("Configuration");
			if (configurations) data = TBD_EquipmentExportJson.Member(data, "landing_gear", LandingGearSources(configurations, path + "/landing_gear"));
		}
		return data;
	}

	//! Serializes an axle's shared configuration once and preserves every wheel position.
	protected static string AxleSources(BaseContainerList axles, string differential, string path)
	{
		array<string> entries = {};
		for (int i = 0; i < axles.Count(); i++)
		{
			BaseContainer axle = axles.Get(i);
			if (!axle) { entries.Insert("null"); continue; }
			string itemPath = path + "/" + i.ToString();
			string entry = SourceFields(axle, "torque_share=TorqueShare", itemPath);
			entry = AddObject(entry, axle, "Differential", "differential", differential, itemPath);
			entry = AddObject(entry, axle, "Suspension", "suspension", SuspensionBindings(), itemPath);
			entry = AddObject(entry, axle, "Wheel", "wheel", "radius=Radius|width=Width|mass=Mass|brake_torque=BrakeTorque|handbrake_torque_factor=HandbrakeTorqueFactor", itemPath);
			entry = AddObject(entry, axle, "Tyre", "tyre", "roughness=Roughness|longitudinal_friction=LongitudinalFriction|lateral_friction=LateralFriction|rolling_resistance=RollingResistance", itemPath);
			entry = AddObject(entry, axle, "Swaybar", "swaybar", "stiffness=Stiffness", itemPath);
			entry = TBD_EquipmentExportJson.Member(entry, "wheel_positions", SourceArray(axle, "WheelPositions", "pivot_id=PivotID|offset=Offset|angles=Angles", itemPath + "/wheel_positions"));
			entries.Insert(entry);
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Preserves independent main and tail rotor settings and their configured pivots.
	protected static string RotorSources(BaseContainerList rotors, string path)
	{
		array<string> entries = {};
		for (int i = 0; i < rotors.Count(); i++)
		{
			BaseContainer rotor = rotors.Get(i);
			if (!rotor) { entries.Insert("null"); continue; }
			string itemPath = path + "/" + i.ToString();
			string bindings = "force=Force|target_rpm=TargetRPM|rotor_diameter=RotorDiameter|clockwise=Clockwise|force_speed_coefficients=ForceSpeedCoefs";
			bindings += "|torque_force=TorqueForce|altitude_no_force=AltNoForce|lift_max_speed=LiftMaxSpeedMS|lift_speed_force=LiftSpeedForce";
			bindings += "|cyclic_forward_force=CyclicForwardForce|cyclic_aside_force=CyclicAsideForce|cyclic_torque_min=CyclicTorqueMin";
			bindings += "|cyclic_forward_speed_coefficient=CyclicForwardSpeedCoef|cyclic_aside_speed_coefficient=CyclicAsideSpeedCoef|collective_speed_coefficient=CollectiveSpeedCoef|anti_torque_speed_coefficient=AntiTorqueSpeedCoef";
			string entry = SourceFields(rotor, bindings, itemPath);
			entry = AddObject(entry, rotor, "Pivot", "pivot", "bone=Bone", itemPath);
			entries.Insert(entry);
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Preserves landing contacts separately from driven wheel installations.
	protected static string LandingGearSources(BaseContainerList configurations, string path)
	{
		array<string> entries = {};
		for (int i = 0; i < configurations.Count(); i++)
		{
			BaseContainer configuration = configurations.Get(i);
			if (!configuration) { entries.Insert("null"); continue; }
			string itemPath = path + "/" + i.ToString();
			string entry = "{\"source\":" + TBD_EquipmentExportJson.Context(configuration) + "}";
			entry = AddObject(entry, configuration, "Pivot", "pivot", "bone=Bone", itemPath);
			entry = AddObject(entry, configuration, "Suspension", "suspension", SuspensionBindings(), itemPath);
			entries.Insert(entry);
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Defines native suspension parameters shared by axle and landing-gear configurations.
	protected static string SuspensionBindings()
	{
		return "spring_rate=SpringRate|compression_damper=CompressionDamper|relaxation_damper=RelaxationDamper|max_steering_angle=MaxSteeringAngle|max_travel_up=MaxTravelUp|max_travel_down=MaxTravelDown|ray_start_offset_up=RayStartOffsetUp";
	}

	//! Adds an applicable native object while retaining the owning domain section.
	protected static string AddObject(string json, BaseContainer owner, string property, string alias, string bindings, string path)
	{
		BaseContainer child = owner.GetObject(property);
		if (!child) return json;
		return TBD_EquipmentExportJson.Member(json, alias, SourceFields(child, bindings, path + "/" + alias));
	}

	//! Adds exact native instance context to explicitly selected fields.
	protected static string SourceFields(BaseContainer source, string bindings, string path)
	{
		if (!source) return "null";
		string data = TBD_EquipmentExportJson.Fields(source, bindings, path);
		return TBD_EquipmentExportJson.Member(data, "source", TBD_EquipmentExportJson.Context(source));
	}

	//! Preserves the source ordering and explicit null entries of gameplay configurations.
	protected static string SourceArray(BaseContainer owner, string property, string bindings, string path)
	{
		if (owner.GetVarIndex(property) < 0) return "null";
		BaseContainerList entries = owner.GetObjectArray(property);
		if (!entries)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Cannot read " + property);
			return "null";
		}
		array<string> data = {};
		for (int i = 0; i < entries.Count(); i++)
			data.Insert(SourceFields(entries.Get(i), bindings, path + "/" + i.ToString()));
		return "[" + TBD_EquipmentExportJson.Join(data) + "]";
	}
}
