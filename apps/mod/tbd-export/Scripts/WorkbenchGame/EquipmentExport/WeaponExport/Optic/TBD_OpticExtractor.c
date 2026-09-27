/**
 * Reads effective optic mounting rules, inventory attributes and model references.
 * Native property reads retain explicit zero and empty overrides.
 */

//! Uses the standard attachment rules for optic mounting compatibility.
class TBD_OpticExtractor
{
	//! Share native attachment ancestry and restrictions with the standard attachment reader.
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_OpticMountingInfo outMounting)
	{
		TBD_AttachmentMountingInfo mounting = new TBD_AttachmentMountingInfo();
		TBD_AttachmentMountingExtractor.ExtractMounting(comps, mounting, string.Empty, string.Empty);
		outMounting.m_sAttachmentType = mounting.m_sAttachmentType;
		outMounting.m_aCompatibleAttachmentTypes = mounting.m_aCompatibleAttachmentTypes;
		outMounting.m_aNativeTypeHierarchy = mounting.m_aNativeTypeHierarchy;
		outMounting.m_aObstructedAttachmentTypes = mounting.m_aObstructedAttachmentTypes;
	}

	//! Read effective inventory measurements without treating zero as an absent override.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_OpticPhysicalInfo outPhys)
	{
		outPhys.m_sInventoryJson = TBD_ItemInventoryExtractor.Inventory(comps);
		outPhys.m_sPhysicsJson = TBD_ItemInventoryExtractor.Physics(comps);
	}

	//! Preserve the effective model reference, including an explicitly empty value.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_OpticVisualsInfo outVisuals)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls != "MeshObject")
				continue;
			foreach (BaseContainer mesh : bucket)
			{
				if (mesh.Get("Object", outVisuals.m_sModelMesh))
					return;
			}
		}
	}
}
