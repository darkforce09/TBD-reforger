//------------------------------------------------------------------------------------------------
// TBD_AmmoTracerExtractor.c
//
// Reads tracer behaviour, which spans both halves of this domain: a magazine declares the ratio
// and pattern of tracer rounds it is loaded with, while a projectile declares whether it is itself
// a tracer and what colour it burns.
//
// Each side fills its own carrier - TBD_TracerRatioInfo for the magazine, TBD_ProjectileTracerInfo
// for the round - which is why both readers sit here rather than with either half.
//------------------------------------------------------------------------------------------------

class TBD_AmmoTracerExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Parse AmmoMapping and calculate exact tracer ratios, intervals, and clusters.
	static void ExtractTracers(map<string, ref array<BaseContainer>> comps, array<string> ammoResources, int roundCapacity, TBD_TracerRatioInfo outTracers, string displayName)
	{
		// 1. Read AmmoMapping from MagazineComponent
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MagazineComponent") && !cls.EndsWith("BaseMagazineComponent"))
				continue;

			foreach (BaseContainer magComp : bucket)
			{
				BaseContainer cur = magComp;
				while (cur && outTracers.m_aAmmoMapping.IsEmpty())
				{
					cur.Get("AmmoMapping", outTracers.m_aAmmoMapping);
					cur = cur.GetAncestor();
				}
			}
		}

		// 2. If AmmoMapping is empty: fallback based on name and capacity
		if (outTracers.m_aAmmoMapping.IsEmpty())
		{
			string dLower = displayName;
			dLower.ToLower();
			if (dLower.Contains("tracer") && !dLower.Contains("mixed") && !dLower.Contains("4ap"))
			{
				outTracers.m_bHasTracers = true;
				outTracers.m_iTracerCount = roundCapacity;
				outTracers.m_iStandardCount = 0;
				outTracers.m_sRatioString = "1:0";
				outTracers.m_iInterval = 1;
				outTracers.m_iTerminalTracerCluster = 0;
			}
			else
			{
				outTracers.m_bHasTracers = false;
				outTracers.m_iTracerCount = 0;
				outTracers.m_iStandardCount = roundCapacity;
				outTracers.m_sRatioString = "none";
				outTracers.m_iInterval = 0;
				outTracers.m_iTerminalTracerCluster = 0;
			}
			return;
		}

		// 3. Identify which resource indices are tracers
		array<int> tracerIndices = {};
		for (int ri = 0; ri < ammoResources.Count(); ri++)
		{
			string res = ammoResources[ri];
			string resLower = res;
			resLower.ToLower();

			bool isTracer = (resLower.Contains("tracer") || resLower.Contains("_t_") || resLower.EndsWith("_t.et") || resLower.Contains("-t."));
			if (!isTracer)
			{
				Resource r = Resource.Load(res);
				if (r && r.IsValid())
				{
					BaseResourceObject ro = r.GetResource();
					if (ro && ro.ToBaseContainer() && ro.ToBaseContainer().GetClassName() == "TracerProjectile")
						isTracer = true;
				}
			}

			if (isTracer)
				tracerIndices.Insert(ri);
		}

		// If no resource identified as tracer by string, but display name says tracer and we have > 1 resources:
		if (tracerIndices.IsEmpty() && ammoResources.Count() > 1)
		{
			string dLower2 = displayName;
			dLower2.ToLower();
			if (dLower2.Contains("tracer"))
				tracerIndices.Insert(1); // default secondary index in belt configs
		}

		// 4. Count rounds
		int totalRounds = outTracers.m_aAmmoMapping.Count();
		int tracerCount = 0;
		for (int m = 0; m < totalRounds; m++)
		{
			int idx = outTracers.m_aAmmoMapping[m];
			if (tracerIndices.Find(idx) != -1)
				tracerCount++;
		}

		outTracers.m_iTracerCount = tracerCount;
		outTracers.m_iStandardCount = totalRounds - tracerCount;
		outTracers.m_bHasTracers = (tracerCount > 0);

		if (!outTracers.m_bHasTracers)
		{
			outTracers.m_sRatioString = "none";
			outTracers.m_iInterval = 0;
			outTracers.m_iTerminalTracerCluster = 0;
			return;
		}

		if (outTracers.m_iStandardCount == 0)
		{
			outTracers.m_sRatioString = "1:0";
			outTracers.m_iInterval = 1;
			outTracers.m_iTerminalTracerCluster = 0;
			return;
		}

		// 5. Detect cadence / interval between tracers
		array<int> tracerPositions = {};
		for (int tp = 0; tp < totalRounds; tp++)
		{
			if (tracerIndices.Find(outTracers.m_aAmmoMapping[tp]) != -1)
				tracerPositions.Insert(tp);
		}

		if (tracerPositions.Count() >= 2)
		{
			int diff = tracerPositions[1] - tracerPositions[0];
			if (diff > 0)
				outTracers.m_iInterval = diff;
		}

		// 6. Detect terminal tracer cluster at the end of the belt
		int termCluster = 0;
		for (int back = totalRounds - 1; back >= 0; back--)
		{
			if (tracerIndices.Find(outTracers.m_aAmmoMapping[back]) != -1)
				termCluster++;
			else
				break;
		}
		outTracers.m_iTerminalTracerCluster = termCluster;

		// 7. Format ratio string
		if (outTracers.m_iInterval > 1)
		{
			int stdPerTracer = outTracers.m_iInterval - 1;
			outTracers.m_sRatioString = stdPerTracer.ToString() + ":1";
		}
		else if (outTracers.m_iTracerCount > 0 && outTracers.m_iStandardCount > 0)
		{
			int approxRatio = Math.Round((1.0 * outTracers.m_iStandardCount) / outTracers.m_iTracerCount);
			if (approxRatio > 0)
				outTracers.m_sRatioString = approxRatio.ToString() + ":1";
			else
				outTracers.m_sRatioString = "1:1";
		}
		else
		{
			outTracers.m_sRatioString = "1:1";
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract tracer rendering properties.
	static void ExtractTracerInfo(BaseContainer root, map<string, ref array<BaseContainer>> comps, TBD_ProjectileTracerInfo outTracer)
	{
		string rootCls = root.GetClassName();
		if (rootCls == "TracerProjectile" || rootCls.Contains("Tracer"))
			outTracer.m_bIsTracer = true;

		if (!outTracer.m_bIsTracer)
		{
			outTracer.m_bIsTracer = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "TracerComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "TracerMoveComponent");
		}

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("Tracer"))
				continue;

			foreach (BaseContainer tc : bucket)
			{
				BaseContainer cur = tc;
				while (cur)
				{
					if (outTracer.m_sTracerColor.IsEmpty())
						cur.Get("TracerColor", outTracer.m_sTracerColor);

					if (outTracer.m_fTracerStartDistance == 0)
						cur.Get("TracerStartDistance", outTracer.m_fTracerStartDistance);

					if (outTracer.m_fTracerBurnTime == 0)
						cur.Get("TracerBurnTime", outTracer.m_fTracerBurnTime);

					cur = cur.GetAncestor();
				}
			}
		}
	}
}
