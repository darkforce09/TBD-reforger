/** Attachment inventory, rigid-body measurements and item model references. */
class TBD_AttachmentExtractor
{
	//! Read effective inventory and body measurements without substituting one mass for another.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_AttachmentPhysicalInfo outPhys, string category, string filePath)
	{
		outPhys.m_sInventoryJson = TBD_ItemInventoryExtractor.Inventory(comps);
		outPhys.m_sPhysicsJson = TBD_ItemInventoryExtractor.Physics(comps);
	}

	//! Preserve the effective item model reference, including an explicit empty value.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_AttachmentVisualsInfo outVisuals)
	{
		foreach (string className, array<BaseContainer> instances : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(className, "MeshObject")) continue;
			foreach (BaseContainer instance : instances)
			{
				if (TBD_EquipmentComponentGraph.StructuralPath(instance).Contains("/children/")) continue;
				string model;
				if (instance.Get("Object", model))
				{
					outVisuals.m_sModelMesh = model;
					return;
				}
			}
		}
	}
}
