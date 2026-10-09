// Generated from the authoritative gameplay field-selection policy.
class TBD_GameplayPolicy_visuals_0
{
	static void Apply(TBD_GameplaySelectionPolicy policy)
	{
		policy.AddClass("MaterialAssignClass", "visuals");
		policy.AddRule("MaterialAssignClass", "AssignedMaterial\tRESOURCE_NAME", "retain_relationship", "visuals", false, "Native configuration used by visuals");
		policy.AddRule("MaterialAssignClass", "SourceMaterial\tSTRING", "retain_value", "visuals", false, "Native configuration used by visuals");
		policy.AddClass("MeshObject", "visuals");
		policy.AddRule("MeshObject", "BoundingBox\tOBJECT|CastFarShadow\tBOOLEAN|CastShadow\tBOOLEAN|ClutterOccluder\tBOOLEAN|ClutterOccluderClipPlane\tSCALAR|ClutterOccluderEnlarge\tSCALAR|Enabled\tBOOLEAN|InheritLOD\tBOOLEAN|InheritSkeleton\tBOOLEAN|InheritVisibility\tBOOLEAN|LODFactors\tSCALAR_ARRAY|ObjectHeightmapBias\tSCALAR|ObjectHeightmapTraceBias\tSCALAR|PixelSizeFilter\tBOOLEAN|PixelSizeScale\tSCALAR|RainOccluder\tBOOLEAN|ReceiveLight\tBOOLEAN|WetnessAdder\tBOOLEAN|WetnessOccluder\tBOOLEAN", "exclude", "excluded", false, "Non-gameplay setting of a mixed-purpose MeshObject container");
		policy.AddRule("MeshObject", "Materials\tOBJECT_ARRAY|Object\tRESOURCE_NAME", "retain_relationship", "visuals", false, "Native configuration used by visuals");
		policy.AddClass("PreviewRenderAttributes", "visuals");
		policy.AddRule("PreviewRenderAttributes", "AnimationInstance\tRESOURCE_NAME|AspectRatio\tSCALAR|CameraDistanceToItem\tSCALAR|CameraOffset\tVECTOR3|CameraOrbitAngles\tVECTOR3|CameraPreset\tINTEGER|FOV\tSCALAR|IsDynamicObject\tBOOLEAN|IsPerspectiveCamera\tBOOLEAN|ItemRenderRotation\tVECTOR3|LodModel\tINTEGER|ShowAllChildrens\tBOOLEAN|TakeVisibilityIntoAccount\tBOOLEAN", "exclude", "excluded", false, "Non-gameplay setting of a mixed-purpose PreviewRenderAttributes container");
		policy.AddRule("PreviewRenderAttributes", "PreviewModel\tRESOURCE_NAME|PreviewPrefab\tRESOURCE_NAME", "retain_relationship", "visuals", false, "Native configuration used by visuals");
		policy.AddRule("PreviewRenderAttributes", "PreviewWornModel\tBOOLEAN", "retain_value", "visuals", false, "Native configuration used by visuals");
		policy.AddClass("SCR_CharacterInventoryPreviewAttributes", "visuals");
		policy.AddRule("SCR_CharacterInventoryPreviewAttributes", "AnimationInstance\tRESOURCE_NAME|AspectRatio\tSCALAR|CameraDistanceToItem\tSCALAR|CameraOffset\tVECTOR3|CameraOrbitAngles\tVECTOR3|CameraPreset\tINTEGER|FOV\tSCALAR|IsDynamicObject\tBOOLEAN|IsPerspectiveCamera\tBOOLEAN|ItemRenderRotation\tVECTOR3|LodModel\tINTEGER|ShowAllChildrens\tBOOLEAN|TakeVisibilityIntoAccount\tBOOLEAN", "exclude", "excluded", false, "Non-gameplay setting of a mixed-purpose PreviewRenderAttributes container");
		policy.AddRule("SCR_CharacterInventoryPreviewAttributes", "PreviewModel\tRESOURCE_NAME|PreviewPrefab\tRESOURCE_NAME", "retain_relationship", "visuals", false, "Native configuration used by visuals");
		policy.AddRule("SCR_CharacterInventoryPreviewAttributes", "PreviewWornModel\tBOOLEAN", "retain_value", "visuals", false, "Native configuration used by visuals");
		policy.AddClass("SCR_VONPreviewAttributes", "visuals");
		policy.AddRule("SCR_VONPreviewAttributes", "AnimationInstance\tRESOURCE_NAME|AspectRatio\tSCALAR|CameraDistanceToItem\tSCALAR|CameraOffset\tVECTOR3|CameraOrbitAngles\tVECTOR3|CameraPreset\tINTEGER|FOV\tSCALAR|IsDynamicObject\tBOOLEAN|IsPerspectiveCamera\tBOOLEAN|ItemRenderRotation\tVECTOR3|LodModel\tINTEGER|ShowAllChildrens\tBOOLEAN|TakeVisibilityIntoAccount\tBOOLEAN", "exclude", "excluded", false, "Non-gameplay setting of a mixed-purpose PreviewRenderAttributes container");
		policy.AddRule("SCR_VONPreviewAttributes", "PreviewModel\tRESOURCE_NAME|PreviewPrefab\tRESOURCE_NAME", "retain_relationship", "visuals", false, "Native configuration used by visuals");
		policy.AddRule("SCR_VONPreviewAttributes", "PreviewWornModel\tBOOLEAN", "retain_value", "visuals", false, "Native configuration used by visuals");
	}
}
