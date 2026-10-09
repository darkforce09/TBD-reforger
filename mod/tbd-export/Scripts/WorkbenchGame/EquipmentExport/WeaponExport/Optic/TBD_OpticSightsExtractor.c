/**
 * Reads each effective optic sight separately, preserving native FOV, zoom,
 * ordered zeroing tuples and reticle configuration without calculated specifications.
 */

//! Extracts configured sight values into the standard optic model.
class TBD_OpticSightsExtractor
{
	//! Preserve every native sight installation; no primary-sight precedence is imposed.
	static void ExtractSights(map<string, ref array<BaseContainer>> comps, TBD_OpticSightsInfo outSights)
	{
		outSights.m_aInstances.Clear();
		foreach (string className, array<BaseContainer> components : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(className, "BaseSightsComponent"))
				continue;
			foreach (BaseContainer source : components)
			{
				TBD_OpticSightInstance instance = new TBD_OpticSightInstance();
				instance.m_sInstanceId = TBD_EquipmentComponentGraph.InstanceId(source);
				instance.m_sClassName = source.GetClassName();
				instance.m_sInstanceName = source.GetName();
				string path = "/sights/" + outSights.m_aInstances.Count().ToString();
				instance.m_sSettingsJson = ReadSettings(source, path + "/settings");
				instance.m_sFieldOfViewJson = ReadFieldOfView(source, path + "/field_of_view");
				instance.m_sZeroingJson = ReadZeroing(source, path + "/zeroing");
				instance.m_sReticleJson = ReadReticle(source, path + "/reticle");
				instance.m_sAlignmentJson = ReadAlignment(source, path + "/alignment");
				outSights.m_aInstances.Insert(instance);
			}
		}
	}

	//! Bind only gameplay sight settings and preserve their native values and types.
	protected static string ReadSettings(BaseContainer source, string path)
	{
		string bindings = "enabled=Enabled|priority=SightsPriority|skip_switch=SightsSwitchSkip";
		bindings += "|ads_time=ADSTime|camera_recoil_amount=CameraRecoilAmount";
		if (source.GetVarIndex("m_fADSActivationPercentage") >= 0)
			bindings += "|ads_activation_percentage=m_fADSActivationPercentage|ads_deactivation_percentage=m_fADSDeactivationPercentage";
		else bindings += "|ads_activation_percentage=ADSActivationPercentage|ads_deactivation_percentage=ADSDeactivationPercentage";
		bindings += "|pip_ads_activation_percentage=m_fADSActivationPercentagePIP|pip_ads_deactivation_percentage=m_fADSDeactivationPercentagePIP";
		bindings += "|partial_hide_ads=SightsPartialHideADS|partial_freelook_factor=SightsPartialFreelookFactor";
		bindings += "|rangefinder=m_bHasRangefinder|movement_damping_speed=m_fMovementDampingSpeed";
		bindings += "|misalignment_scale=m_fMisalignmentScale";
		bindings += "|misalignment_damping_speed=m_fMisalignmentDampingSpeed|rotation_scale=m_fRotationScale|rotation_damping_speed=m_fRotationDampingSpeed";
		bindings += "|movement_scale=m_fMovementScale|roll_scale=m_fRollScale|roll_damping_speed=m_fRollDampingSpeed|reticle_offset_interpolation_speed=m_fReticleOffsetInterpSpeed";
		bindings += "|vehicle_sight=CollimatorIsVehicleSight|compartment_name=CollimatorCompartmentName";
		return TBD_EquipmentExportJson.Fields(source, bindings, path);
	}

	//! Keep objective, main-camera and configurable FOV measurements distinct.
	protected static string ReadFieldOfView(BaseContainer source, string path)
	{
		array<string> fields = {};
		fields.Insert("\"objective_fov\":" + TBD_EquipmentExportJson.Field(source, "m_fObjectiveFov", path + "/objective_fov"));
		fields.Insert("\"main_camera_fov\":" + TBD_EquipmentExportJson.Field(source, "m_fMainCameraFOV", path + "/main_camera_fov"));
		fields.Insert("\"magnification\":" + TBD_EquipmentExportJson.Field(source, "m_fMagnification", path + "/magnification"));
		BaseContainer fov = source.GetObject("SightsFOVInfo");
		if (fov)
		{
			fields.Insert("\"configuration_class\":" + TBD_EquipmentExportJson.Quote(fov.GetClassName()));
			string bindings = "field_of_view=m_fFieldOfView|base_zoom=m_fBaseZoom|maximum_zoom=m_fZoomMax";
			bindings += "|step_zoom_size=m_fStepZoomSize|fov_values=m_aFOVs|interpolation_speed=m_fInterpolationSpeed";
			fields.Insert("\"configuration\":" + TBD_EquipmentExportJson.Fields(fov, bindings, path + "/configuration"));
		}
		else
		{
			fields.Insert("\"configuration\":null");
		}
		return "{" + TBD_EquipmentExportJson.Join(fields) + "}";
	}

	//! Preserve complete native range tuples, their order and an explicit zero default index.
	protected static string ReadZeroing(BaseContainer source, string path)
	{
		array<string> fields = {};
		fields.Insert("\"default_index\":" + TBD_EquipmentExportJson.Field(source, "SightsRangesDefaultIndex", path + "/default_index"));
		fields.Insert("\"type\":" + TBD_EquipmentExportJson.Field(source, "m_eZeroingType", path + "/type"));
		BaseContainerList ranges = source.GetObjectArray("SightsRanges");
		array<string> entries = {};
		if (ranges)
		{
			for (int i = 0; i < ranges.Count(); i++)
			{
				BaseContainer range = ranges.Get(i);
				if (!range) { entries.Insert("null"); continue; }
				string rangePath = path + "/ranges/" + i.ToString();
				string entry = TBD_EquipmentExportJson.Fields(range, "range=Range", rangePath);
				entry = TBD_EquipmentExportJson.Member(entry, "source", TBD_EquipmentExportJson.Context(range));
				entries.Insert(TBD_EquipmentExportJson.Member(entry, "weapon_position", ReadPoint(range.GetObject("WeaponPosition"), rangePath + "/weapon_position")));
			}
		}
		string rangeJson = "null";
		if (ranges) rangeJson = "[" + TBD_EquipmentExportJson.Join(entries) + "]";
		fields.Insert("\"ranges\":" + rangeJson);
		return "{" + TBD_EquipmentExportJson.Join(fields) + "}";
	}

	//! Preserve configured reticle alternatives and colors without selecting the first entry.
	protected static string ReadReticle(BaseContainer source, string path)
	{
		string bindings = "texture=m_sReticleTexture|glow_texture=m_sReticleGlowTexture|color=m_ReticleColor";
		bindings += "|illumination_color=m_cReticleTextureIllumination|illumination=m_bHasIllumination";
		bindings += "|angular_size=m_fReticleAngularSize|texture_portion=m_fReticlePortion|base_zoom=m_fReticleBaseZoom";
		bindings += "|default_angular_size=ReticleDefaultAngularSize|default_texture_portion=ReticleDefaultTexturePortion";
		bindings += "|automatic_brightness=ReticleAutoBright|automatic_lower=ReticleAutoLower";
		bindings += "|automatic_toggle=ReticleAutoToggle|automatic_upper=ReticleAutoUpper|automatic_brightness_flip=ReticleAutoBrightFlip";
		bindings += "|default_color_index=DefaultReticleColor|default_reticle_index=DefaultReticleIndex|outline_color=m_ReticleOutlineColor|filter_texture=m_sFilterTexture";
		bindings += "|initial_horizontal_angular_correction=InitialHorziontalAngularCorrection|initial_vertical_angular_correction=InitialVerticalAngularCorrection";
		bindings += "|automatic_day_night_clamp=ReticleAutoClampDayNight|daylight_brightness=m_fDaylightBrightness|night_brightness=m_fNightBrightness";
		bindings += "|texture_glow_alpha=m_fReticleTextureGlowAlpha|offset_x=m_fReticleOffsetX|offset_y=m_fReticleOffsetY|pip_scale=m_fReticlePIPScale";
		string result = TBD_EquipmentExportJson.Fields(source, bindings, path);
		result = TBD_EquipmentExportJson.PreserveSubstring(result, 0, result.Length() - 1);
		result += ",\"colors\":" + TBD_EquipmentExportJson.ObjectArray(source, "ReticleColors", "reticle_color=ReticleColor|glow_color=GlowColor", path + "/colors");
		string reticleBindings = "index=ReticleIndex|override=DoOverride|angular_size=AngularSize|texture_portion=ReticlePortion";
		result += ",\"alternatives\":" + TBD_EquipmentExportJson.ObjectArray(source, "ReticleInfos", reticleBindings, path + "/alternatives");
		return result + "}";
	}

	//! Read authored alignment points without converting them to real-world optical dimensions.
	protected static string ReadAlignment(BaseContainer source, string path)
	{
		array<string> properties = {"SightsPosition", "SightsPointFront", "SightsPointRear", "CollimatorTopLeft", "CollimatorBottomRight", "CollimatorCenter"};
		array<string> names = {"position", "front", "rear", "collimator_top_left", "collimator_bottom_right", "collimator_center"};
		array<string> fields = {};
		for (int i = 0; i < properties.Count(); i++)
		{
			BaseContainer point = source.GetObject(properties[i]);
			string value = ReadPoint(point, path + "/" + names[i]);
			fields.Insert(TBD_EquipmentExportJson.Quote(names[i]) + ":" + value);
		}
		string bindings = "camera_offset=m_vCameraOffset|camera_angles=m_vCameraAngles|unfocused_camera_offset=m_vMainCameraOffsetUnfocused";
		bindings += "|center_distance=m_fCenterDistance|near_distance_factor=m_fDistanceMoveNear|far_distance_factor=m_fDistanceMoveFar|basic_parallax=m_fBasicParallax|maximum_parallax=m_fMaxParallax";
		bindings += "|center_offset_x=m_fCenterOffsetX|center_offset_y=m_fCenterOffsetY|vignette_parallax_scale=m_fVignetteParallaxScale|vignette_scale=m_fVignetteScale|vignette_move_speed=m_fVignetteMoveSpeed";
		bindings += "|objective_scale=m_fObjectiveScale|scope_radius=m_fScopeRadius|objective_pip_edge_minimum=m_fObjectivePIPEdgeMin|objective_pip_edge_maximum=m_fObjectivePIPEdgeMax";
		fields.Insert("\"optical_geometry\":" + TBD_EquipmentExportJson.Fields(source, bindings, path + "/optical_geometry"));
		return "{" + TBD_EquipmentExportJson.Join(fields) + "}";
	}

	//! Point subclasses retain their native bone or pivot binding without substituting one for the other.
	protected static string ReadPoint(BaseContainer point, string path)
	{
		if (!point) return "null";
		string bindings = "offset=Offset|angles=Angles";
		if (point.GetVarIndex("Bone") >= 0) bindings += "|bone=Bone";
		else bindings += "|pivot_id=PivotID";
		return TBD_EquipmentExportJson.Member(TBD_EquipmentExportJson.Fields(point, bindings, path), "source", TBD_EquipmentExportJson.Context(point));
	}
}
