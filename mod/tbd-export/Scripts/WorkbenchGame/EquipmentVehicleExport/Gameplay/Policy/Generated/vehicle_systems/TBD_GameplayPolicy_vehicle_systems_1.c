// Generated from the authoritative gameplay field-selection policy.
class TBD_GameplayPolicy_vehicle_systems_1
{
	static void Apply(TBD_GameplaySelectionPolicy policy)
	{
		policy.AddClass("TurretPart", "vehicle_systems");
		policy.AddRule("TurretPart", "SlotName\tSTRING", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddClass("TurretSlotComponent", "vehicle_systems");
		policy.AddRule("TurretSlotComponent", "AttachType\tOBJECT", "retain_relationship", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("TurretSlotComponent", "TurretTemplate\tRESOURCE_NAME", "retain_relationship", "vehicle_systems", true, "Native configuration used by vehicle systems");
		policy.AddRule("TurretSlotComponent", "Enabled\tBOOLEAN", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("TurretSlotComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddClass("Tyre", "vehicle_systems");
		policy.AddRule("Tyre", "LateralFriction\tSCALAR|LongitudinalFriction\tSCALAR|RollingDrag\tSCALAR|RollingResistance\tSCALAR|Roughness\tSCALAR|Tread\tSCALAR", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddClass("VehicleBuoyancyComponent", "vehicle_systems");
		policy.AddClass("VehicleHelicopterSimulation", "vehicle_systems");
		policy.AddRule("VehicleHelicopterSimulation", "Animation\tOBJECT|SignalsSourceAccess\tOBJECT", "exclude", "excluded", false, "Presentation, authoring, lifecycle, replication, animation, audio, or cosmetic effect setting");
		policy.AddRule("VehicleHelicopterSimulation", "RotorConfigs\tOBJECT_ARRAY|Simulation\tOBJECT", "retain_relationship", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("VehicleHelicopterSimulation", "Enabled\tBOOLEAN", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("VehicleHelicopterSimulation", "components\tOBJECT_ARRAY", "traverse_required_container", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddClass("VehicleTrackedSimulation", "vehicle_systems");
		policy.AddRule("VehicleTrackedSimulation", "Animation\tOBJECT|SignalsSourceAccess\tOBJECT", "exclude", "excluded", false, "Presentation, authoring, lifecycle, replication, animation, audio, or cosmetic effect setting");
		policy.AddRule("VehicleTrackedSimulation", "Simulation\tOBJECT", "retain_relationship", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("VehicleTrackedSimulation", "TrackSegment\tRESOURCE_NAME", "retain_relationship", "vehicle_systems", true, "Native configuration used by vehicle systems");
		policy.AddRule("VehicleTrackedSimulation", "Enabled\tBOOLEAN|TrackLength\tSCALAR|TrackOffset1\tVECTOR3|TrackOffset2\tVECTOR3|TrackPositions\tVECTOR2_ARRAY|TrackSegments\tINTEGER|TrackThickness\tSCALAR", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("VehicleTrackedSimulation", "components\tOBJECT_ARRAY", "traverse_required_container", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddClass("VehicleWheeledSimulation", "vehicle_systems");
		policy.AddRule("VehicleWheeledSimulation", "Animation\tOBJECT|SignalsSourceAccess\tOBJECT", "exclude", "excluded", false, "Presentation, authoring, lifecycle, replication, animation, audio, or cosmetic effect setting");
		policy.AddRule("VehicleWheeledSimulation", "Simulation\tOBJECT", "retain_relationship", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("VehicleWheeledSimulation", "Enabled\tBOOLEAN", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("VehicleWheeledSimulation", "components\tOBJECT_ARRAY", "traverse_required_container", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddClass("Wheel", "vehicle_systems");
		policy.AddRule("Wheel", "BrakeTorque\tSCALAR|HandbrakeTorqueFactor\tSCALAR|Mass\tSCALAR|Radius\tSCALAR|Ratio\tSCALAR", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddClass("WheelPosition", "vehicle_systems");
		policy.AddRule("WheelPosition", "Angles\tVECTOR3|Offset\tVECTOR3|PivotID\tSTRING", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddClass("Wheeled", "vehicle_systems");
		policy.AddRule("Wheeled", "Aerodynamics\tOBJECT|Axles\tOBJECT_ARRAY|Clutch\tOBJECT|Differentials\tOBJECT_ARRAY|Engine\tOBJECT|Gearbox\tOBJECT|Pacejka\tOBJECT", "retain_relationship", "vehicle_systems", false, "Native configuration used by vehicle systems");
		policy.AddRule("Wheeled", "InertiaOverride\tVECTOR3|InertiaOverrideEnabled\tBOOLEAN|LiquidsLayers\tSTRING|NoiseSteerSensitivity\tSCALAR|RaycastLayer\tSTRING|ResponseIndex\tSTRING|RoughnessSensitivity\tSCALAR|SolverUpdateRate\tINTEGER", "retain_value", "vehicle_systems", false, "Native configuration used by vehicle systems");
	}
}
