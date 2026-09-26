/**
 * @file TBD_VehicleSpawnDefaults.c
 * @brief Default cargo and full fuel for mission roster vehicles.
 *
 * Role: gives every roster vehicle its authored inventory or a per-class default cargo, and full
 * fuel when none is authored.  Position: TBD_SlotBodyMaterializer.MaterializeSlotBodies runs
 * ApplyCargo before TBD_VehicleState.ApplySpawned (so authored ammo scales the inserted magazines)
 * and ApplyDefaultFuel after it (so authored fuel wins).
 * State: the prefabs already warned about as missing (static, per run of ApplyCargo).
 * Invariants: a vehicle not found near its roster position is logged and skipped; a missing cargo
 * prefab is warned about once and the vehicle stays in the world.
 */

//! Default cargo and fuel for roster vehicles.
class TBD_VehicleSpawnDefaults
{
	protected static const string JERRYCAN = "{12D5AD21E383B768}Prefabs/Items/Fuel/Jerrycan_01/Jerrycan_01_item.et"; //!< jerrycan item prefab
	protected static const string DRESSING = "{A81F501D3EF6F38E}Prefabs/Items/Medicine/FieldDressing_01/FieldDressing_US_01.et"; //!< field dressing item prefab
	protected static const string REPAIR = "{4AF9664BEE9263A4}Prefabs/Items/Equipment/Kits/RepairKit_01/RepairKit_01_base.et"; //!< repair kit item prefab
	protected static const string MEDKIT = "{AE578EEA4244D41F}Prefabs/Items/Equipment/Kits/MedicalKit_01/MedicalKit_01_US.et"; //!< medical kit item prefab

	protected static ref array<string> s_aWarnedPrefabs; //!< cargo prefabs already reported missing

	//! Insert each roster vehicle's authored inventory, or its class default cargo.
	//! @authority server
	static void ApplyCargo()
	{
		array<ref TBD_VehicleSpawnStruct> rows = Parse();
		if (!rows || rows.Count() < 1)
			return;

		s_aWarnedPrefabs = new array<string>();

		int cargoed = 0;
		int missed = 0;

		foreach (TBD_VehicleSpawnStruct wire : rows)
		{
			if (!wire)
				continue;

			IEntity body = FindBody(wire);
			if (!body)
			{
				missed++;
				Print(string.Format("[TBD][Vehicles] spawn-cargo: no world vehicle for uid='%1' alias='%2' at %3,%4 -- cargo NOT applied",
					wire.uid, wire.alias, wire.x, wire.z), LogLevel.WARNING);
				continue;
			}

			array<ref TBD_VehicleSpawnInventoryStruct> cargo;
			if (wire.HasInventory())
				cargo = wire.inventory;
			else
				cargo = DefaultCargoFor(wire.alias);

			if (InsertCargo(body, cargo))
				cargoed++;
		}

		Print(string.Format("[TBD][Vehicles] spawn-cargo cargoed=%1 missed=%2", cargoed, missed));
	}

	//! Fill every roster vehicle that authored no `fuel` (an authored 0 counts as authored).
	//! @authority server
	static void ApplyDefaultFuel()
	{
		array<ref TBD_VehicleSpawnStruct> rows = Parse();
		if (!rows || rows.Count() < 1)
			return;

		int fueled = 0;
		int missed = 0;
		int skippedAuthored = 0;

		foreach (TBD_VehicleSpawnStruct wire : rows)
		{
			if (!wire)
				continue;

			IEntity body = FindBody(wire);
			if (!body)
			{
				missed++;
				Print(string.Format("[TBD][Vehicles] spawn-fuel: no world vehicle for uid='%1' alias='%2' at %3,%4 -- default fuel NOT applied",
					wire.uid, wire.alias, wire.x, wire.z), LogLevel.WARNING);
				continue;
			}

			if (wire.HasFuel())
			{
				skippedAuthored++;
				continue;
			}

			TBD_VehicleState.Apply(body, false, 1.0, TBD_VehicleStateWireStruct.ABSENT);
			fueled++;
		}

		Print(string.Format("[TBD][Vehicles] spawn-fuel defaultFull=%1 skippedAuthored=%2 missed=%3",
			fueled, skippedAuthored, missed));
	}

	//! The `vehicles[]` rows of the held mission JSON; empty without a document or rows, logged
	//! ERROR when the JSON does not parse or its root does not read.
	protected static array<ref TBD_VehicleSpawnStruct> Parse()
	{
		array<ref TBD_VehicleSpawnStruct> empty = {};

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (outcome == TBD_EMissionJsonPassOutcome.NO_DOCUMENT)
			return empty;

		if (!ctx)
		{
			Print("[TBD][Vehicles] spawn-defaults: mission JSON did not parse -- default fuel/cargo NOT applied", LogLevel.ERROR);
			return empty;
		}

		TBD_VehicleSpawnDocStruct doc = new TBD_VehicleSpawnDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print("[TBD][Vehicles] spawn-defaults: mission root would not read -- default fuel/cargo NOT applied", LogLevel.ERROR);
			return empty;
		}

		if (!doc.vehicles)
			return empty;
		return doc.vehicles;
	}

	//! The placed vehicle within the roster row's search box.
	protected static IEntity FindBody(TBD_VehicleSpawnStruct wire)
	{
		return TBD_EntityQuery.FirstVehicleNear(wire.x, wire.z, TBD_VehicleSpawnStruct.XZ_M, TBD_VehicleSpawnStruct.Y_M);
	}

	//! Coarse class of a `veh:` alias: helo, apc, truck, jeep or default.
	protected static string VehicleClassOf(string alias)
	{
		string key = alias;
		key.ToLower();
		if (key.Contains("uh1") || key.Contains("mi8") || key.Contains("heli"))
			return "helo";
		if (key.Contains("btr") || key.Contains("brdm") || key.Contains("lav"))
			return "apc";
		if (key.Contains("ural") || key.Contains("m923") || key.Contains("s1203") || key.Contains("truck"))
			return "truck";
		if (key.Contains("uaz") || key.Contains("m151") || key.Contains("m1025") || key.Contains("m998") || key.Contains("humr") || key.Contains("jeep"))
			return "jeep";
		return "default";
	}

	//! The default cargo rows of the alias's class.
	protected static array<ref TBD_VehicleSpawnInventoryStruct> DefaultCargoFor(string alias)
	{
		string cls = VehicleClassOf(alias);
		array<ref TBD_VehicleSpawnInventoryStruct> rows = {};
		if (cls == "helo")
		{
			AddCargoRow(rows, MEDKIT, 1);
			AddCargoRow(rows, DRESSING, 2);
			return rows;
		}
		if (cls == "apc")
		{
			AddCargoRow(rows, JERRYCAN, 1);
			AddCargoRow(rows, REPAIR, 1);
			AddCargoRow(rows, DRESSING, 2);
			return rows;
		}
		if (cls == "truck")
		{
			AddCargoRow(rows, JERRYCAN, 2);
			AddCargoRow(rows, REPAIR, 1);
			AddCargoRow(rows, DRESSING, 2);
			return rows;
		}
		if (cls == "jeep")
		{
			AddCargoRow(rows, JERRYCAN, 1);
			AddCargoRow(rows, DRESSING, 2);
			return rows;
		}
		AddCargoRow(rows, JERRYCAN, 1);
		AddCargoRow(rows, DRESSING, 1);
		return rows;
	}

	//! Append a cargo row.
	protected static void AddCargoRow(array<ref TBD_VehicleSpawnInventoryStruct> rows, string item, int qty)
	{
		TBD_VehicleSpawnInventoryStruct row = new TBD_VehicleSpawnInventoryStruct();
		row.item = item;
		row.qty = qty;
		rows.Insert(row);
	}

	//! Spawn and insert every row's items into the vehicle's storage; an item that does not fit is
	//! deleted.
	//! @return false without a vehicle, rows or storage manager
	protected static bool InsertCargo(IEntity vehicle, array<ref TBD_VehicleSpawnInventoryStruct> rows)
	{
		if (!vehicle)
			return false;
		if (!rows || rows.Count() < 1)
			return false;

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			vehicle.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			Print("[TBD][Vehicles] cargo skipped -- body has no SCR_InventoryStorageManagerComponent", LogLevel.WARNING);
			return false;
		}

		foreach (TBD_VehicleSpawnInventoryStruct row : rows)
		{
			if (!row)
				continue;
			if (row.item.IsEmpty() || row.qty < 1)
				continue;

			for (int u = 0; u < row.qty; u++)
			{
				IEntity item = SpawnItem(vehicle, row.item);
				if (!item)
				{
					WarnMissingOnce(row.item);
					break;
				}

				bool ok = false;
				if (mgr.CanInsertItem(item))
					ok = mgr.TryInsertItem(item);
				if (!ok)
					SCR_EntityHelper.DeleteEntityAndChildren(item);
			}
		}
		return true;
	}

	//! Spawn one item prefab at the vehicle, or null when it does not load.
	protected static IEntity SpawnItem(IEntity vehicle, string resName)
	{
		Resource resource = Resource.Load(resName);
		if (!resource || !resource.IsValid())
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = vehicle.GetOrigin();
		return GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
	}

	//! Warn once that a cargo prefab is missing.
	protected static void WarnMissingOnce(string prefab)
	{
		if (!s_aWarnedPrefabs)
			s_aWarnedPrefabs = new array<string>();
		if (s_aWarnedPrefabs.Contains(prefab))
			return;
		s_aWarnedPrefabs.Insert(prefab);
		Print(string.Format("[TBD][Vehicles] cargo prefab missing: %1 -- vehicle still spawned", prefab), LogLevel.WARNING);
	}
}
