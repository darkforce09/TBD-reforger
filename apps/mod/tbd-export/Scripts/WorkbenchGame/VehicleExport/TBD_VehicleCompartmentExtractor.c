/**
 * TBD_VehicleCompartmentExtractor.c
 *
 * Dedicated extractor for vehicle compartments, seating slots, tactical crew roles,
 * turnout capabilities, door interactions, and ingress/egress points.
 * Operates purely via Enfusion BaseContainer reflection without hardcoded vehicle tables.
 */

class TBD_VehicleCompartmentExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspect seating slots, crew roles, doors, and turnout abilities for a vehicle variant.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		ref map<int, ref TBD_VehicleSeatData> seatMap = new map<int, ref TBD_VehicleSeatData>();
		ref array<string> doorNames = {};

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("CompartmentManagerComponent"))
				continue;

			// Child container takes precedence
			for (int b = 0; b < bucket.Count(); b++)
			{
				BaseContainer cm = bucket[b];
				if (!cm) continue;

				// 1. Introspect DoorInfoList for valid door names
				ExtractDoors(cm, doorNames);

				// 2. Introspect CompartmentSlots array across supported keys
				ExtractSlotList(cm.GetObjectArray("CompartmentSlots"), seatMap, doorNames);
				ExtractSlotList(cm.GetObjectArray("Compartments"), seatMap, doorNames);
				ExtractSlotList(cm.GetObjectArray("m_aCompartments"), seatMap, doorNames);

				if (!seatMap.IsEmpty())
					break;
			}
			break;
		}

		// Sort seats by slot ID and insert into variant array
		for (int i = 0; i < seatMap.Count(); i++)
		{
			TBD_VehicleSeatData sd = seatMap.Get(i);
			if (sd)
				varData.m_aSeats.Insert(sd);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract door names and entrance interaction points from DoorInfoList.
	protected static void ExtractDoors(BaseContainer cm, array<string> doorNames)
	{
		BaseContainerList doors = cm.GetObjectArray("DoorInfoList");
		if (!doors) doors = cm.GetObjectArray("m_aDoorInfoList");
		if (!doors) return;

		for (int d = 0, dn = doors.Count(); d < dn; d++)
		{
			BaseContainer door = doors.Get(d);
			if (!door) continue;

			string dName;
			if (door.Get("m_sDoorName", dName) && !dName.IsEmpty())
			{
				if (doorNames.Find(dName) == -1)
					doorNames.Insert(dName);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract individual seating slots from a compartment container array.
	protected static void ExtractSlotList(BaseContainerList list, map<int, ref TBD_VehicleSeatData> seatMap, array<string> doors)
	{
		if (!list) return;

		int passengerIndex = 1;
		for (int i = 0, n = list.Count(); i < n; i++)
		{
			BaseContainer slot = list.Get(i);
			if (!slot) continue;

			// If slot already extracted by derived container, preserve override
			if (seatMap.Contains(i))
				continue;

			TBD_VehicleSeatData sd = new TBD_VehicleSeatData();
			sd.m_iSlotId = i;

			// Internal slot identifier
			string slotName;
			if (slot.Get("m_sName", slotName) && !slotName.IsEmpty())
				sd.m_sName = slotName;
			else if (slot.Get("CompartmentName", slotName) && !slotName.IsEmpty())
				sd.m_sName = slotName;
			else
				sd.m_sName = string.Format("Seat_%1", i);

			// Role classification
			string slotCls = slot.GetClassName();
			sd.m_sType = ClassifySeatRole(slotCls, sd.m_sName);

			// Turnout capability
			slot.Get("m_bCanTurnOut", sd.m_bCanTurnOut);
			if (!sd.m_bCanTurnOut) slot.Get("CanTurnOut", sd.m_bCanTurnOut);

			// Door association
			string doorStr;
			if (slot.Get("m_sDoor", doorStr) && !doorStr.IsEmpty())
				sd.m_sDoor = doorStr;
			else if (slot.Get("Door", doorStr) && !doorStr.IsEmpty())
				sd.m_sDoor = doorStr;
			else if (!doors.IsEmpty())
			{
				int doorIdx = i % doors.Count();
				sd.m_sDoor = doors[doorIdx];
			}

			// UI display name from m_UIInfo or derived role title
			BaseContainer ui = slot.GetObject("m_UIInfo");
			if (ui)
			{
				string uiName;
				if (ui.Get("Name", uiName) && !uiName.IsEmpty())
				{
					if (uiName.StartsWith("#"))
						sd.m_sUIName = TBD_VehicleExportNaming.CleanNameFromToken(uiName.Substring(1, uiName.Length() - 1));
					else
						sd.m_sUIName = uiName;
				}
			}

			if (sd.m_sUIName.IsEmpty())
			{
				sd.m_sUIName = FormatFallbackSeatName(sd.m_sType, sd.m_sName, passengerIndex);
				if (sd.m_sType == "CARGO")
					passengerIndex++;
			}

			seatMap.Insert(i, sd);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Classify seat role category (PILOT, TURRET, CARGO).
	protected static string ClassifySeatRole(string slotCls, string slotName)
	{
		string lower = slotName;
		lower.ToLower();

		if (slotCls.Contains("Pilot") || lower.Contains("pilot") || lower.Contains("driver"))
			return "PILOT";

		if (slotCls.Contains("Turret") || lower.Contains("turret") || lower.Contains("gunner") || lower.Contains("commander") || lower.Contains("cupola"))
			return "TURRET";

		return "CARGO";
	}

	//------------------------------------------------------------------------------------------------
	//! Derive clean user-facing seat title when UIInfo is absent.
	protected static string FormatFallbackSeatName(string role, string slotName, int passengerIndex)
	{
		string lower = slotName;
		lower.ToLower();

		if (lower.Contains("driver")) return "Driver";
		if (lower.Contains("pilot")) return "Pilot";
		if (lower.Contains("copilot") || lower.Contains("co-pilot")) return "Co-Pilot";
		if (lower.Contains("gunner")) return "Gunner";
		if (lower.Contains("commander")) return "Commander";
		if (lower.Contains("loader")) return "Loader";

		if (role == "PILOT") return "Driver";
		if (role == "TURRET") return "Gunner";

		return string.Format("Passenger %1", passengerIndex);
	}
}
