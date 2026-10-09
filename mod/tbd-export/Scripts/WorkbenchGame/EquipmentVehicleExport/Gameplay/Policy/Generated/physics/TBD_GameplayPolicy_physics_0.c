// Generated from the authoritative gameplay field-selection policy.
class TBD_GameplayPolicy_physics_0
{
	static void Apply(TBD_GameplaySelectionPolicy policy)
	{
		policy.AddClass("AABBBoundingVolume", "physics");
		policy.AddRule("AABBBoundingVolume", "Maxs\tVECTOR3|Mins\tVECTOR3", "retain_value", "physics", false, "Native configuration used by physics");
		policy.AddClass("BoundingVolume", "physics");
		policy.AddRule("BoundingVolume", "Size\tVECTOR3", "retain_value", "physics", false, "Native configuration used by physics");
		policy.AddClass("PhysicsModelGeometry", "physics");
		policy.AddRule("PhysicsModelGeometry", "SurfaceProperties\tRESOURCE_NAME", "retain_relationship", "physics", true, "Native configuration used by physics");
		policy.AddRule("PhysicsModelGeometry", "Collider\tSTRING|LayerPreset\tSTRING|Layers\tFLAGS|Mass\tSCALAR|Offset\tVECTOR3|Orientation\tVECTOR3", "retain_value", "physics", false, "Native configuration used by physics");
		policy.AddClass("RigidBody", "physics");
		policy.AddRule("RigidBody", "Active\tINTEGER|AngularDamping\tSCALAR|AngularSleepingThreashold\tSCALAR|CCDRadius\tSCALAR|Enabled\tBOOLEAN|Gravity\tBOOLEAN|Kinematic\tBOOLEAN|LinearDamping\tSCALAR|LinearSleepingThreashold\tSCALAR|MasslessInertia\tVECTOR3|ResponseIndex\tSTRING|SimState\tINTEGER|Static\tBOOLEAN", "exclude", "excluded", false, "Non-gameplay setting of a mixed-purpose RigidBody container");
		policy.AddRule("RigidBody", "Geometries\tOBJECT_ARRAY", "retain_relationship", "physics", false, "Native configuration used by physics");
		policy.AddRule("RigidBody", "CenterOfMass\tVECTOR3|LayerPreset\tSTRING|Layers\tFLAGS|Mass\tSCALAR|ModelGeometry\tBOOLEAN", "retain_value", "physics", false, "Native configuration used by physics");
	}
}
