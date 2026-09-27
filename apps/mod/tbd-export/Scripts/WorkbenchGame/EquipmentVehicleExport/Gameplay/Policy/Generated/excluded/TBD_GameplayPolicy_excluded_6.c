// Generated from the authoritative gameplay field-selection policy.
class TBD_GameplayPolicy_excluded_6
{
	static void Apply(TBD_GameplaySelectionPolicy policy)
	{
		policy.AddRule("SpawnParticleEffect", "AttachToParent\tBOOLEAN|Enabled\tBOOLEAN|FollowParent\tBOOLEAN|ParticleEffect\tRESOURCE_NAME|ShouldReplicate\tBOOLEAN|SoundEvent\tSTRING|SoundStopOffset\tINTEGER|StopSoundWithParticles\tBOOLEAN|TriggerName\tSTRING|TriggerOnce\tBOOLEAN|UseFrameEvent\tBOOLEAN", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("Sprocket", "excluded");
		policy.AddClass("SubDistanceParticleEffect", "excluded");
		policy.AddRule("SubDistanceParticleEffect", "ModuloSpawned\tINTEGER|ParticleEffect\tRESOURCE_NAME", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("SurfaceLightConfigItem", "excluded");
		policy.AddRule("SurfaceLightConfigItem", "EmissiveColorTint\tCOLOR|EmissiveMultiplier\tINTEGER|ForLightType\tINTEGER|Priority\tINTEGER", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("SurfaceProperties", "excluded");
		policy.AddClass("TagComponent", "excluded");
		policy.AddRule("TagComponent", "Enabled\tBOOLEAN|IsDynamic\tBOOLEAN|Tag categories\tFLAGS", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("Track", "excluded");
		policy.AddClass("UGLAnimationComponent", "excluded");
		policy.AddRule("UGLAnimationComponent", "AlwaysActive\tBOOLEAN|AnimCommandsToBind\tSTRING_ARRAY|AnimConstants\tOBJECT_ARRAY|AnimGraph\tRESOURCE_NAME|AnimIkpose\tRESOURCE_NAME|AnimInjection\tOBJECT|AnimInstance\tRESOURCE_NAME|AnimUpdate\tINTEGER|AnimVariables\tOBJECT_ARRAY|AnimVariablesToBind\tSTRING_ARRAY|AutoCommandBind\tBOOLEAN|AutoVariablesBind\tBOOLEAN|BindWithInjection\tBOOLEAN|DeactivationDelay\tSCALAR|Enabled\tBOOLEAN|MeshVisibilityConfigurations\tOBJECT_ARRAY|ResetOnDeactivation\tBOOLEAN|SimulateOnHeadless\tBOOLEAN|SimulationDistance\tSCALAR|SleepAfterInactivity\tBOOLEAN|SleepTimeout\tSCALAR|StartNode\tSTRING|components\tOBJECT_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("UniqueAttribute", "excluded");
		policy.AddClass("UnlinkingAction", "excluded");
		policy.AddRule("UnlinkingAction", "EjectWhenUnlinking\tBOOLEAN|RagdollWhenUnlinking\tBOOLEAN|UpSpeedWhenUnlinking\tSCALAR", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("UserActionContext", "excluded");
		policy.AddRule("UserActionContext", "ContextName\tSTRING|Display UI At Reference Point\tBOOLEAN|FilterActionsUsingCache\tBOOLEAN|Height\tSCALAR|LineOfSightCheck\tBOOLEAN|Omnidirectional\tBOOLEAN|Position\tOBJECT|Radius\tSCALAR|SkipCullingPlaneDetection\tBOOLEAN|UIInfo\tOBJECT|VisibilityAngle\tSCALAR", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VObject", "excluded");
		policy.AddClass("VariableMappingObject", "excluded");
		policy.AddClass("VehicleAnimation", "excluded");
		policy.AddRule("VehicleAnimation", "AnimGraph\tRESOURCE_NAME|AnimInstance\tRESOURCE_NAME|AnimVariables\tOBJECT_ARRAY|VehicleParts\tOBJECT_ARRAY|WakeCommand\tOBJECT", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VehicleAnimationComponent", "excluded");
		policy.AddRule("VehicleAnimationComponent", "AlwaysActive\tBOOLEAN|AnimCommandsToBind\tSTRING_ARRAY|AnimConstants\tOBJECT_ARRAY|AnimGraph\tRESOURCE_NAME|AnimIkpose\tRESOURCE_NAME|AnimInjection\tOBJECT|AnimInstance\tRESOURCE_NAME|AnimUpdate\tINTEGER|AnimVariables\tOBJECT_ARRAY|AnimVariablesToBind\tSTRING_ARRAY|AutoCommandBind\tBOOLEAN|AutoVariablesBind\tBOOLEAN|BindWithInjection\tBOOLEAN|DeactivationDelay\tSCALAR|Enabled\tBOOLEAN|MeshVisibilityConfigurations\tOBJECT_ARRAY|ResetOnDeactivation\tBOOLEAN|SimulateOnHeadless\tBOOLEAN|SimulationDistance\tSCALAR|SleepAfterInactivity\tBOOLEAN|SleepTimeout\tSCALAR|StartNode\tSTRING|components\tOBJECT_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VehicleBaseSimulation", "excluded");
		policy.AddClass("VehicleControllerComponent", "excluded");
		policy.AddClass("VehicleDoorUserAction", "excluded");
		policy.AddClass("VehicleGamepadEffectsManagerComponent", "excluded");
		policy.AddRule("VehicleGamepadEffectsManagerComponent", "EffectContexts\tOBJECT_ARRAY|Enabled\tBOOLEAN|OwnedEffects\tOBJECT_ARRAY|components\tOBJECT_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VehicleLightPortal", "excluded");
		policy.AddRule("VehicleLightPortal", "ContextName\tSTRING|MinIntensity\tINTEGER|PortalPositionInfo\tOBJECT", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VehicleLightPortalComponent", "excluded");
		policy.AddRule("VehicleLightPortalComponent", "Enabled\tBOOLEAN|PortalInfoList\tOBJECT_ARRAY|components\tOBJECT_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VehiclePartAnimation", "excluded");
		policy.AddRule("VehiclePartAnimation", "PartType\tINTEGER|SlotName\tSTRING|StartNode\tSTRING", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VehiclePerceivableComponent", "excluded");
		policy.AddClass("VehicleProcAnimComponent", "excluded");
		policy.AddRule("VehicleProcAnimComponent", "Enabled\tBOOLEAN|OnFrameUpdate\tBOOLEAN|Parameters\tOBJECT_ARRAY|components\tOBJECT_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VehicleSoundComponent", "excluded");
		policy.AddClass("VehicleWheelSound", "excluded");
		policy.AddRule("VehicleWheelSound", "Filename\tRESOURCE_NAME|Min step\tSCALAR|SoundPoint\tOBJECT", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VisualIdentity", "excluded");
		policy.AddRule("VisualIdentity", "Body\tRESOURCE_NAME|BodyMeshesConfig\tOBJECT|Head\tRESOURCE_NAME|Headcamo\tRESOURCE_NAME_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("VoNComponent", "excluded");
		policy.AddClass("WeaponAnimationComponent", "excluded");
		policy.AddRule("WeaponAnimationComponent", "AlwaysActive\tBOOLEAN|AnimCommandsToBind\tSTRING_ARRAY|AnimConstants\tOBJECT_ARRAY|AnimGraph\tRESOURCE_NAME|AnimIkpose\tRESOURCE_NAME|AnimInjection\tOBJECT|AnimInstance\tRESOURCE_NAME|AnimUpdate\tINTEGER|AnimVariables\tOBJECT_ARRAY|AnimVariablesToBind\tSTRING_ARRAY|AutoCommandBind\tBOOLEAN|AutoVariablesBind\tBOOLEAN|BindWithInjection\tBOOLEAN|DeactivationDelay\tSCALAR|Enabled\tBOOLEAN|MeshVisibilityConfigurations\tOBJECT_ARRAY|ResetOnDeactivation\tBOOLEAN|SimulateOnHeadless\tBOOLEAN|SimulationDistance\tSCALAR|SleepAfterInactivity\tBOOLEAN|SleepTimeout\tSCALAR|StartNode\tSTRING|components\tOBJECT_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("WeaponGamepadEffectsManagerComponent", "excluded");
		policy.AddRule("WeaponGamepadEffectsManagerComponent", "Default ADS Trigger effect\tOBJECT|Default Chambered Trigger effect\tOBJECT|Default Deployment effect\tOBJECT|Default Empty Fire Trigger effect\tOBJECT|Default Firing Trigger effect\tOBJECT|EffectContexts\tOBJECT_ARRAY|Enabled\tBOOLEAN|OwnedEffects\tOBJECT_ARRAY|components\tOBJECT_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("WeaponSoundComponent", "excluded");
		policy.AddRule("WeaponSoundComponent", "DistanceManagement\tBOOLEAN|Enabled\tBOOLEAN|Events\tOBJECT_ARRAY|Filenames\tRESOURCE_NAME_ARRAY|Immediate load\tBOOLEAN|IncludeInactive\tBOOLEAN|OnFrameUpdate\tBOOLEAN|ScriptCallbacks\tBOOLEAN|Sound Geometry Info\tOBJECT|SoundPoints\tOBJECT_ARRAY|components\tOBJECT_ARRAY", "exclude", "excluded", false, "No gameplay-catalog consumer: presentation, engine infrastructure, AI, or cosmetic configuration");
		policy.AddClass("WeaponsGroup", "excluded");
		policy.AddClass("WorldSubsceneComponent", "excluded");
		policy.AddClass("global_pointer", "excluded");
		policy.AddClass("pointer", "excluded");
	}
}
