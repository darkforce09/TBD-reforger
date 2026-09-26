/**
 * @file TBD_VehicleState.c
 * @brief Applies authored `vehicles[]` lock, fuel and ammo to the placed vehicles.
 *
 * Role: a second `JsonLoadContext` pass over the held mission JSON whose root declares only
 * `vehicles[]` lock, fuel and ammo, and the engine calls that apply them:
 *   lock -> `VehicleControllerComponent.LockPilotControls` (pilot controls, not door locks);
 *   fuel -> every `BaseFuelNode.SetFuel` as a fraction of its maximum, slotted tanks included;
 *   ammo -> the loaded magazine of each turret weapon and every magazine in cargo, as a fraction.
 * Position: `ApplySpawned` runs from `TBD_SlotBodyMaterializer` after
 * `TBD_MissionVehicleRoster.SeatAuthoredCrews`; `Apply` is also called by
 * `TBD_VehicleSpawnDefaults`. Reads `TBD_MissionJsonPass.LoadRoot`.
 * State: the rows of the last pass (static), server only.  Invariants: an absent fuel or ammo key
 * reads `ABSENT` and leaves the engine default; `lock` applies only when true, because an absent
 * bool and an authored false bind the same; a fraction outside 0..1 is logged and skipped.
 */

//! One `vehicles[]` row's state fields. Field names are the JSON keys.
//! @contract mission.schema.json#/$defs/vehicle
class TBD_VehicleStateWireStruct
{
	static const float ABSENT = -1000000; //!< "key absent from JSON" sentinel for `fuel` and `ammo`
	static const float XZ_M = 3.0; //!< metres, half-extent in X and Z of the box that finds the vehicle
	static const float Y_M = 300.0; //!< metres, half-extent in Y of that box

	string uid; //!< `uid`, or empty
	string alias; //!< `alias`
	float x; //!< `x`, world metres
	float z; //!< `z`, world metres
	bool lock;                 //!< Schema boolean. Absent and authored-false bind the same.
	float fuel = ABSENT;       //!< Fraction 0..1. ABSENT when the key was omitted. 0 is authored.
	float ammo = ABSENT;       //!< Fraction 0..1. ABSENT when the key was omitted. 0 is authored.

	//! Whether the row authors `fuel`.
	bool HasFuel()
	{
		return fuel != ABSENT;
	}

	//! Whether the row authors `ammo`.
	bool HasAmmo()
	{
		return ammo != ABSENT;
	}

	//! Whether the row authors anything this reader applies: `lock` true, `fuel` or `ammo`.
	bool HasAny()
	{
		if (lock)
			return true;
		if (fuel != ABSENT)
			return true;
		if (ammo != ABSENT)
			return true;
		return false;
	}
}

//! Root of the second parse. Declares `vehicles` and nothing else.
//! @contract mission.schema.json#/properties/vehicles
class TBD_VehicleStateDocStruct
{
	ref array<ref TBD_VehicleStateWireStruct> vehicles; //!< `vehicles[]`
}

//! Server-side reader: bind vehicles[] lock/fuel/ammo and apply them to the spawned body.
class TBD_VehicleState
{
	protected static ref array<ref TBD_VehicleStateWireStruct> s_aRows; //!< rows of the last pass; empty when none

	//! Apply every authored row to the vehicle standing at its position. Runs after
	//! `SeatAuthoredCrews`, so every roster vehicle has either joined its `entities[]` twin or
	//! been spawned. Does nothing when the document has no `vehicles[]` or no authored
	//! lock/fuel/ammo; a row with no vehicle at its position logs a WARNING and is skipped.
	//! @authority server
	static void ApplySpawned()
	{
		if (!Parse())
			return;
		if (!s_aRows)
			return;
		if (s_aRows.Count() < 1)
			return;

		int applied = 0;
		int locked = 0;
		int fueled = 0;
		int ammoed = 0;
		int missed = 0;

		foreach (TBD_VehicleStateWireStruct wire : s_aRows)
		{
			if (!wire)
				continue;
			if (!wire.HasAny())
				continue;

			IEntity body = FindBody(wire);
			if (!body)
			{
				missed++;
				Print(string.Format("[TBD][VehicleState] no world vehicle for uid='%1' alias='%2' at %3,%4 -- state NOT applied",
					wire.uid, wire.alias, wire.x, wire.z), LogLevel.WARNING);
				continue;
			}

			Apply(body, wire.lock, wire.fuel, wire.ammo);
			applied++;
			if (wire.lock)
				locked++;
			if (wire.HasFuel())
				fueled++;
			if (wire.HasAmmo())
				ammoed++;
		}

		if (applied < 1 && missed < 1)
			return;

		Print(string.Format("[TBD][VehicleState] applied=%1 locked=%2 fueled=%3 ammoed=%4 missed=%5",
			applied, locked, fueled, ammoed, missed));
	}

	//! Apply authored lock / fuel / ammo to one spawned vehicle. Unset numerics are ABSENT and
	//! leave engine defaults. Lock applies only when the bound bool is true (see header).
	//! @param vehicle the vehicle entity; null does nothing
	//! @param fuel fraction 0..1, or `TBD_VehicleStateWireStruct.ABSENT`
	//! @param ammo fraction 0..1, or `TBD_VehicleStateWireStruct.ABSENT`
	//! @authority server
	static void Apply(IEntity vehicle, bool lock, float fuel, float ammo)
	{
		if (!vehicle)
			return;

		if (lock)
			ApplyLock(vehicle);

		if (fuel != TBD_VehicleStateWireStruct.ABSENT)
			ApplyFuel(vehicle, fuel);

		if (ammo != TBD_VehicleStateWireStruct.ABSENT)
			ApplyAmmo(vehicle, ammo);
	}

	//! Run the vehicle-state pass into `s_aRows`.
	//! @return false when no mission text is held; true otherwise, with an ERROR line and no rows
	//! when the text or its root does not read
	protected static bool Parse()
	{
		s_aRows = new array<ref TBD_VehicleStateWireStruct>();

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (outcome == TBD_EMissionJsonPassOutcome.NO_DOCUMENT)
			return false;

		if (!ctx)
		{
			Print("[TBD][VehicleState] the mission document did not parse as JSON on the vehicle-state pass - no lock/fuel/ammo applied this round", LogLevel.ERROR);
			return true;
		}

		TBD_VehicleStateDocStruct doc = new TBD_VehicleStateDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print("[TBD][VehicleState] the mission document parsed but its root would not read on the vehicle-state pass - no lock/fuel/ammo applied this round", LogLevel.ERROR);
			return true;
		}

		if (!doc.vehicles)
			return true;
		s_aRows = doc.vehicles;
		return true;
	}

	//! The first vehicle standing within the row's box.
	//! @return the vehicle, or null when none is there
	protected static IEntity FindBody(TBD_VehicleStateWireStruct wire)
	{
		return TBD_EntityQuery.FirstVehicleNear(wire.x, wire.z, TBD_VehicleStateWireStruct.XZ_M, TBD_VehicleStateWireStruct.Y_M);
	}

	//! Lock the pilot controls; logs a WARNING when the body has no `VehicleControllerComponent`.
	protected static void ApplyLock(IEntity vehicle)
	{
		VehicleControllerComponent controller = VehicleControllerComponent.Cast(vehicle.FindComponent(VehicleControllerComponent));
		if (!controller)
		{
			Print("[TBD][VehicleState] authored lock=true but the body has no VehicleControllerComponent -- lock NOT applied", LogLevel.WARNING);
			return;
		}

		controller.LockPilotControls(true);
	}

	//! Set every fuel node of the vehicle and of its slotted entities to `fuel` of its maximum.
	//! A fraction outside 0..1 logs a WARNING and applies nothing.
	protected static void ApplyFuel(IEntity vehicle, float fuel)
	{
		if (fuel < 0 || fuel > 1)
		{
			Print(string.Format("[TBD][VehicleState] fuel=%1 outside 0..1 -- not applied", fuel), LogLevel.WARNING);
			return;
		}

		ApplyFuelOnEntity(vehicle, fuel);

		SlotManagerComponent slots = SlotManagerComponent.Cast(vehicle.FindComponent(SlotManagerComponent));
		if (!slots)
			return;

		array<EntitySlotInfo> infos = {};
		slots.GetSlotInfos(infos);
		foreach (EntitySlotInfo info : infos)
		{
			if (!info)
				continue;
			IEntity attached = info.GetAttachedEntity();
			if (!attached)
				continue;
			ApplyFuelOnEntity(attached, fuel);
		}
	}

	//! Set every fuel node of one entity's `FuelManagerComponent`; no component does nothing.
	protected static void ApplyFuelOnEntity(IEntity entity, float fuel)
	{
		FuelManagerComponent fm = FuelManagerComponent.Cast(entity.FindComponent(FuelManagerComponent));
		if (!fm)
			return;

		array<BaseFuelNode> nodes = {};
		fm.GetFuelNodesList(nodes);
		foreach (BaseFuelNode node : nodes)
		{
			if (!node)
				continue;
			float maxFuel = node.GetMaxFuel();
			node.SetFuel(maxFuel * fuel);
		}
	}

	//! Scale the magazines of the vehicle and of its slotted entities to `ammo` of their capacity.
	//! A fraction outside 0..1 logs a WARNING and applies nothing.
	protected static void ApplyAmmo(IEntity vehicle, float ammo)
	{
		if (ammo < 0 || ammo > 1)
		{
			Print(string.Format("[TBD][VehicleState] ammo=%1 outside 0..1 -- not applied", ammo), LogLevel.WARNING);
			return;
		}

		ApplyAmmoOnEntity(vehicle, ammo);

		SlotManagerComponent slots = SlotManagerComponent.Cast(vehicle.FindComponent(SlotManagerComponent));
		if (!slots)
			return;

		array<EntitySlotInfo> infos = {};
		slots.GetSlotInfos(infos);
		foreach (EntitySlotInfo info : infos)
		{
			if (!info)
				continue;
			IEntity attached = info.GetAttachedEntity();
			if (!attached)
				continue;
			ApplyAmmoOnEntity(attached, ammo);
		}
	}

	//! Turret / hull weapons: currently loaded magazine. Cargo: every inventory item that is a
	//! magazine (includeChildComponents so a mag sitting in a weapon well still counts).
	protected static void ApplyAmmoOnEntity(IEntity entity, float ammo)
	{
		BaseWeaponManagerComponent weapons = BaseWeaponManagerComponent.Cast(entity.FindComponent(BaseWeaponManagerComponent));
		if (weapons)
		{
			array<BaseWeaponComponent> list = {};
			weapons.GetWeapons(list);
			foreach (BaseWeaponComponent weapon : list)
			{
				if (!weapon)
					continue;
				ScaleMagazine(weapon.GetCurrentMagazine(), ammo);
			}
		}

		BaseInventoryStorageComponent storage = BaseInventoryStorageComponent.Cast(entity.FindComponent(BaseInventoryStorageComponent));
		if (!storage)
			return;

		array<IEntity> items = {};
		storage.GetAll(items, true);
		foreach (IEntity item : items)
		{
			if (!item)
				continue;
			BaseMagazineComponent mag = BaseMagazineComponent.Cast(item.FindComponent(BaseMagazineComponent));
			ScaleMagazine(mag, ammo);
		}
	}

	//! Set one magazine to `ammo` of its capacity, rounded to the nearest round; null or an
	//! empty-capacity magazine does nothing.
	protected static void ScaleMagazine(BaseMagazineComponent mag, float ammo)
	{
		if (!mag)
			return;

		int maxCount = mag.GetMaxAmmoCount();
		if (maxCount < 1)
			return;

		float scaled = maxCount * ammo;
		int wanted = scaled + 0.5;
		if (wanted < 0)
			wanted = 0;
		if (wanted > maxCount)
			wanted = maxCount;
		mag.SetAmmoCount(wanted);
	}
}
