/**
 * TBD_VehicleArmorExtractor.c
 *
 * Dedicated extractor for vehicle armor hit zones, health budgets, armor plate thickness,
 * damage multipliers, collision thresholds, and secondary explosion configurations.
 * Pure dynamic reflection over SCR_WheeledDamageManagerComponent, SCR_ArmorDamageManagerComponent, etc.
 */

class TBD_VehicleArmorExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspect hit zones, armor thickness, and damage thresholds for a vehicle variant.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleDamageThresholds thresholds = new TBD_VehicleDamageThresholds();
		varData.m_DamageThresholds = thresholds;

		ref array<string> recordedNames = {};

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("DamageManagerComponent"))
				continue;

			// Child containers take precedence over ancestors
			for (int i = 0; i < bucket.Count(); i++)
			{
				BaseContainer dm = bucket[i];
				if (!dm) continue;

				// 1. Extract vehicle-level destruction and impact thresholds
				ExtractThresholds(dm, thresholds);

				// 2. Extract primary default hit zone
				ExtractSingleHitZone(dm.GetObject("DefaultHitZone"), varData.m_aHitZones, recordedNames);
				ExtractSingleHitZone(dm.GetObject("m_DefaultHitZone"), varData.m_aHitZones, recordedNames);

				// 3. Extract array hit zones across supported container keys
				ExtractHitZoneList(dm.GetObjectArray("Additional hit zones"), varData.m_aHitZones, recordedNames);
				ExtractHitZoneList(dm.GetObjectArray("AdditionalHitZones"), varData.m_aHitZones, recordedNames);
				ExtractHitZoneList(dm.GetObjectArray("HitZones"), varData.m_aHitZones, recordedNames);
				ExtractHitZoneList(dm.GetObjectArray("m_aHitZones"), varData.m_aHitZones, recordedNames);
			}
			break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract damage multipliers, collision limits, and secondary blast presets from DamageManager.
	protected static void ExtractThresholds(BaseContainer dm, TBD_VehicleDamageThresholds thresholds)
	{
		if (thresholds.m_fVehicleDestroyDamage <= 0)
		{
			dm.Get("m_fVehicleDestroyDamage", thresholds.m_fVehicleDestroyDamage);
			if (thresholds.m_fVehicleDestroyDamage <= 0)
				dm.Get("VehicleDestroyDamage", thresholds.m_fVehicleDestroyDamage);
		}

		if (thresholds.m_fCollisionVelocityThreshold <= 0)
		{
			dm.Get("CollisionVelocityThreshold", thresholds.m_fCollisionVelocityThreshold);
			if (thresholds.m_fCollisionVelocityThreshold <= 0)
				dm.Get("m_fCollisionVelocityThreshold", thresholds.m_fCollisionVelocityThreshold);
		}

		if (thresholds.m_fHeavyDamageThreshold <= 0)
		{
			dm.Get("Heavy damage threshold", thresholds.m_fHeavyDamageThreshold);
			if (thresholds.m_fHeavyDamageThreshold <= 0)
				dm.Get("HeavyDamageThreshold", thresholds.m_fHeavyDamageThreshold);
		}

		if (thresholds.m_fOccupantsDamageSpeedThreshold <= 0)
			dm.Get("m_fOccupantsDamageSpeedThreshold", thresholds.m_fOccupantsDamageSpeedThreshold);

		if (thresholds.m_fOccupantsSpeedDeath <= 0)
			dm.Get("m_fOccupantsSpeedDeath", thresholds.m_fOccupantsSpeedDeath);

		// Directional multipliers
		float fMult = 1, rMult = 1, lMult = 1, riMult = 1, tMult = 1, bMult = 1;
		if (dm.Get("m_fFrontMultiplier", fMult)) thresholds.m_fFrontMultiplier = fMult;
		if (dm.Get("m_fRearMultiplier", rMult)) thresholds.m_fRearMultiplier = rMult;
		if (dm.Get("m_fLeftMultiplier", lMult)) thresholds.m_fLeftMultiplier = lMult;
		if (dm.Get("m_fRightMultiplier", riMult)) thresholds.m_fRightMultiplier = riMult;
		if (dm.Get("m_fTopMultiplier", tMult)) thresholds.m_fTopMultiplier = tMult;
		if (dm.Get("m_fBottomMultiplier", bMult)) thresholds.m_fBottomMultiplier = bMult;

		// Secondary blast presets
		if (thresholds.m_sSecondaryExplosionsConf.IsEmpty())
		{
			string secExpl;
			if (dm.Get("m_SecondaryExplosions", secExpl) && !secExpl.IsEmpty())
				thresholds.m_sSecondaryExplosionsConf = ExtractConfPath(secExpl);
		}

		if (thresholds.m_sSecondaryFiresConf.IsEmpty())
		{
			string secFire;
			if (dm.Get("m_SecondaryFires", secFire) && !secFire.IsEmpty())
				thresholds.m_sSecondaryFiresConf = ExtractConfPath(secFire);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Iterate a container array and extract individual hit zones without duplicate registration.
	protected static void ExtractHitZoneList(BaseContainerList list, array<ref TBD_VehicleHitZoneData> outZones, array<string> recorded)
	{
		if (!list) return;

		for (int i = 0, n = list.Count(); i < n; i++)
		{
			ExtractSingleHitZone(list.Get(i), outZones, recorded);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract name, group category, max health, and armor thickness from a hit zone container.
	protected static void ExtractSingleHitZone(BaseContainer hz, array<ref TBD_VehicleHitZoneData> outZones, array<string> recorded)
	{
		if (!hz) return;

		string name;
		if (hz.Get("m_sHitZoneName", name) && !name.IsEmpty())
		{
		}
		else if (hz.Get("m_sName", name) && !name.IsEmpty())
		{
		}
		else if (hz.Get("HitZoneName", name) && !name.IsEmpty())
		{
		}
		else
		{
			name = string.Format("HitZone_%1", outZones.Count() + 1);
		}

		if (recorded.Find(name) != -1)
			return;

		TBD_VehicleHitZoneData hd = new TBD_VehicleHitZoneData();
		hd.m_sName = name;
		recorded.Insert(name);

		// Group classification
		string group;
		if (hz.Get("m_eHitZoneGroup", group) && !group.IsEmpty())
			hd.m_sGroup = group;
		else if (hz.Get("HitZoneGroup", group) && !group.IsEmpty())
			hd.m_sGroup = group;
		else
			hd.m_sGroup = ClassifyGroupByStem(name);

		// Health budget
		float hp = 0;
		if (hz.Get("m_fMaxHealth", hp) && hp > 0)
			hd.m_fMaxHealth = hp;
		else if (hz.Get("MaxHealth", hp) && hp > 0)
			hd.m_fMaxHealth = hp;

		// Armor plate thickness (mm)
		float armor = 0;
		if (hz.Get("m_fArmorThickness", armor) && armor > 0)
			hd.m_fArmorThickness = armor;
		else if (hz.Get("ArmorThickness", armor) && armor > 0)
			hd.m_fArmorThickness = armor;

		// Damage multiplier
		float dmgMult = 1.0;
		if (hz.Get("m_fDamageMultiplier", dmgMult) && dmgMult > 0)
			hd.m_fDamageMultiplier = dmgMult;
		else if (hz.Get("DamageMultiplier", dmgMult) && dmgMult > 0)
			hd.m_fDamageMultiplier = dmgMult;

		outZones.Insert(hd);
	}

	//------------------------------------------------------------------------------------------------
	//! Derive tactical hit zone group category from name when not explicitly serialized.
	protected static string ClassifyGroupByStem(string name)
	{
		string lower = name;
		lower.ToLower();

		if (lower.Contains("hull") || lower.Contains("body")) return "HULL";
		if (lower.Contains("engine") || lower.Contains("motor")) return "ENGINE";
		if (lower.Contains("fuel") || lower.Contains("tank")) return "FUEL";
		if (lower.Contains("wheel") || lower.Contains("tire") || lower.Contains("tyre")) return "WHEELS";
		if (lower.Contains("turret") || lower.Contains("cupola") || lower.Contains("gun")) return "TURRET";
		if (lower.Contains("rotor") || lower.Contains("tail")) return "ROTOR";
		if (lower.Contains("glass") || lower.Contains("window")) return "WINDOW";
		if (lower.Contains("transmission") || lower.Contains("gearbox")) return "DRIVETRAIN";
		return "VIRTUAL";
	}

	//------------------------------------------------------------------------------------------------
	//! Extract configuration file path from Enfusion resource reference string.
	protected static string ExtractConfPath(string s)
	{
		int quote1 = s.IndexOf("\"");
		if (quote1 == -1) return s;
		int quote2 = s.IndexOfFrom(quote1 + 1, "\"");
		if (quote2 == -1) return s;
		return s.Substring(quote1 + 1, quote2 - quote1 - 1);
	}
}
