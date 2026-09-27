/**
 * Preserves magazine ammunition ordering and native projectile tracer settings.
 * Tracer ratios, expanded round counts and filename classifications are not source facts.
 */

//! Reads tracer configuration without deriving ammunition statistics.
class TBD_AmmoTracerExtractor
{
	//! Retain the exact configured ammunition mapping without generating a tracer cadence.
	static void ExtractTracers(map<string, ref array<BaseContainer>> comps, array<string> ammoResources, int roundCapacity, TBD_TracerRatioInfo outTracers, string displayName)
	{
		outTracers.m_aAmmoMapping.Clear();
		outTracers.m_sRatioString = string.Empty;
		outTracers.m_iTracerCount = -1;
		outTracers.m_iStandardCount = -1;
		outTracers.m_iInterval = -1;
		outTracers.m_iTerminalTracerCluster = -1;
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseMagazineComponent") && !TBD_EquipmentComponentGraph.IsA(cls, "MagazineComponent")) continue;
			foreach (BaseContainer component : bucket)
			{
				if (component.Get("AmmoMapping", outTracers.m_aAmmoMapping))
					return;
			}
		}
	}

	//! Read native burn duration from the projectile root; a delay is not a distance.
	static void ExtractTracerInfo(BaseContainer root, map<string, ref array<BaseContainer>> comps, TBD_ProjectileTracerInfo outTracer)
	{
		outTracer.m_fTracerStartDistance = -1;
		outTracer.m_fTracerBurnTime = -1;
		outTracer.m_sTracerColor = string.Empty;
		if (root)
			root.Get("Tracer Burn Time", outTracer.m_fTracerBurnTime);
	}

	//! Export source timing and visual references without converting delay into start distance.
	static string ExtractTracerConfiguration(BaseContainer root, string outputPath = "/tracer")
	{
		string bindings = "burn_start_after=Tracer Burn Start After|burn_time=Tracer Burn Time";
		bindings += "|projectile_visible_time_scale=ProjectileVisibleTimeScale|projectile_model=ProjectileModel";
		return TBD_EquipmentExportJson.Fields(root, bindings, outputPath);
	}
}
