/** Native vehicle AI navigation, control and usage settings in the standard vehicle catalog. */
class TBD_VehicleAIConfigurationExtractor
{
	//! Preserve each configured AI component without simulating movement or inventing capabilities.
	static string Extract(map<string, ref array<BaseContainer>> components)
	{
		array<string> movement = {};
		array<string> control = {};
		array<string> usage = {};
		foreach (string className, array<BaseContainer> instances : components)
		{
			foreach (BaseContainer component : instances)
			{
				if (TBD_EquipmentComponentGraph.IsA(className, "AICarMovementComponent"))
					movement.Insert(ReadMovement(component, "/operational_systems/ai/movement/" + movement.Count().ToString()));
				if (TBD_EquipmentComponentGraph.IsA(className, "ChimeraAIVehicleControlComponent"))
				{
					string bindings = "enabled=Enabled|enable_ai=EnableAI|agent_template=Agenttemplate|behavior_override=OverrideAIBehaviorData|parent_ai_group=ParentAIGroupName|physics_layer=PhysicsLayer|physics_layer_preset=PhysicsLayerPreset";
					control.Insert(Fields(component, bindings, "/operational_systems/ai/control/" + control.Count().ToString()));
				}
				if (TBD_EquipmentComponentGraph.IsA(className, "SCR_AIVehicleUsageComponent"))
					usage.Insert(Fields(component, "enabled=Enabled|vehicle_type=m_eVehicleType|can_be_piloted=m_bCanBePiloted", "/operational_systems/ai/usage/" + usage.Count().ToString()));
			}
		}
		return "{\"movement\":[" + TBD_EquipmentExportJson.Join(movement) + "],\"control\":[" + TBD_EquipmentExportJson.Join(control) + "],\"usage\":[" + TBD_EquipmentExportJson.Join(usage) + "]}";
	}

	//! Export native steering, avoidance and path-following configuration in its authored units.
	protected static string ReadMovement(BaseContainer source, string path)
	{
		string bindings = "enabled=Enabled|minimum_speed=Min Speed|maximum_speed=Max Speed|navlink_traversal_attempts=Navlink Traversal Attempts|max_stuck_time=Max Stuck Time";
		bindings += "|minimum_prediction_distance=Min Prediction Distance|maximum_prediction_distance=Max Prediction Distance|using_railroad=Using Railroad|maximum_distance_to_path=Max Distance to Path|search_box_area_distance=Search box area distance";
		bindings += "|maximum_simple_steering_distance=Max Simple Steering Distance|cruise_vehicle_speed=CruiseVehicleSpeedKmh|reverse_vehicle_speed=ReverseVehicleSpeedKmh|friction_coefficient=FrictionCoefficient|stop_distance_coefficient=StopDistanceCoefficient";
		bindings += "|maximum_reverse_travel_distance=MaxReverseTravelDistance|brake_threshold=BreakTreshold|maximum_brake_at=MaxBreakAt|maximum_bump_speed=MaxBumpSpeed|maximum_steering_change=MaxSteeringChangeS";
		bindings += "|steering_pid_on_water=SteeringPIDOnWater|steering_pid=SteeringPID|throttle_pid=ThrottlePID|use_navmesh=Use navmesh for pathfinding|collision_rays_origin_height=Collision rays origin height from pivot";
		bindings += "|collision_detection_layers=Collision detection layers|collision_detection_layers_preset=Collision detection layers preset|ai_lod_level=AILOD level|maximum_ai_lod=Max AILOD|lod_speed_coefficient=LOD speed coeficient";
		bindings += "|obstacle_avoidance_timer=ObstacleAvoidanceTimer|obstacle_avoidance_check_distance=ObstacleAvoidanceCheckDist|detection_angle=DetectionAngle|minimum_range_detection_angle=MinRangeDetectionAngle|minimum_detection_range=MinDetectionRange";
		bindings += "|minimum_horn_time=MinHornTimeS|maximum_horn_time=MaxHornTimeS|minimum_horn_pause=MinHornPauseS|maximum_horn_pause=MaxHornPauseS";
		return Fields(source, bindings, path);
	}

	//! Each installation carries independent source and field provenance.
	protected static string Fields(BaseContainer source, string bindings, string path)
	{
		return TBD_EquipmentExportJson.Member(TBD_EquipmentExportJson.Fields(source, bindings, path), "source", TBD_EquipmentExportJson.Context(source));
	}
}
