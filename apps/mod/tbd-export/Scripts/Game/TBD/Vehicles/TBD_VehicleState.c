//! T-680 - vehicle states: lock, fuel, ammo.
//!
//! == What was missing ========================================================================
//! T-706 put `lock` / `fuel` / `ammo` on `$defs/vehicle` (and the vehicle-shaped `$defs/entity`).
//! `TBD_MissionVehicleStruct` does not declare those members, so the primary parse cannot see
//! them. Spawned vehicles kept engine defaults regardless of the authored values. This file is
//! the reader. Editor UI for the three attrs is NOT this slice.
//!
//! == Why a second JsonLoadContext pass =======================================================
//! Same pattern as `TBD_WaypointRuntime.c` (T-677): a second pass over
//! `TBD_MissionLoader.GetRawJson()` with a root that declares `vehicles[]` lock/fuel/ammo and
//! nothing else. Backend/TBD_MissionLoader.c and TBD_MissionVehicleStruct.c stay out of this
//! slice's owns list.
//!
//! == Presence, and the nested-ref landmine ===================================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key is
//! ABSENT. `vehicles` is an ARRAY, so presence is a null-or-Count() test. Numeric fields that
//! can be authored as 0 (`fuel`, `ammo`) carry an ABSENT sentinel. Bools cannot: `lock` false
//! and an omitted `lock` are the same bound value (T-676 `repeat`). Apply lock only when the
//! bound value is true; authored false and absent both leave the engine default (unlocked).
//!
//! == ATTR-FIELD-OBJ-LOCK / -FUEL / -AMMO =====================================================
//!   lock  -> VehicleControllerComponent.LockPilotControls (pilot controls, not door locks).
//!   fuel  -> FuelManagerComponent nodes via BaseFuelNode.SetFuel (fraction 0..1 of max).
//!            Slotted tanks (trailers / extra nodes) are included via SlotManagerComponent.
//!   ammo  -> TURRET: BaseWeaponManagerComponent.GetWeapons -> GetCurrentMagazine
//!            SetAmmoCount. CARGO: BaseInventoryStorageComponent.GetAll (child components
//!            included) items that carry BaseMagazineComponent. Same fraction on both.
//!            Spare magazines in cargo AND the currently loaded turret magazine are in
//!            scope. Loose world magazines that are not in this vehicle's inventory are not.
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile`. It cannot run a round. Whether a locked half-fuel
//! vehicle actually spawns locked with half fuel is a human checklist item.
//! @contract mission.schema.json#/$defs/vehicle

//------------------------------------------------------------------------------------------------
//! One `vehicles[]` row's state fields. Field names are the JSON keys.
class TBD_VehicleStateWireStruct
{
	static const float ABSENT = -1000000;
	static const float XZ_M = 3.0;
	static const float Y_M = 300.0;

	string uid;
	string alias;
	float x;
	float z;
	bool lock;                 //!< Schema boolean. Absent and authored-false bind the same.
	float fuel = ABSENT;       //!< Fraction 0..1. ABSENT when the key was omitted. 0 is authored.
	float ammo = ABSENT;       //!< Fraction 0..1. ABSENT when the key was omitted. 0 is authored.

	//------------------------------------------------------------------------------------------------
	bool HasFuel()
	{
		return fuel != ABSENT;
	}

	//------------------------------------------------------------------------------------------------
	bool HasAmmo()
	{
		return ammo != ABSENT;
	}

	//------------------------------------------------------------------------------------------------
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

//------------------------------------------------------------------------------------------------
//! Root of the second parse. Declares `vehicles` and nothing else.
class TBD_VehicleStateDocStruct
{
	ref array<ref TBD_VehicleStateWireStruct> vehicles;
}

//------------------------------------------------------------------------------------------------
//! Server-side reader: bind vehicles[] lock/fuel/ammo and apply them to the spawned body.
class TBD_VehicleState
{
	//! Scratch for the AABB query. Static because QueryEntitiesByAABB takes a function.
	protected static IEntity s_QueryHit;
	protected static ref array<ref TBD_VehicleStateWireStruct> s_aRows;

	//------------------------------------------------------------------------------------------------
	//! Called from `TBD_SpawnManager.MaterializeSlotBodies` AFTER `SeatAuthoredCrews`, so every
	//! roster vehicle has either joined its entities[] twin or been spawned. No-ops when the
	//! document has no vehicles[] or no authored lock/fuel/ammo, so a rosterless mission boots
	//! unchanged.
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

	//------------------------------------------------------------------------------------------------
	//! Apply authored lock / fuel / ammo to one spawned vehicle. Unset numerics are ABSENT and
	//! leave engine defaults. Lock applies only when the bound bool is true (see header).
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

	//------------------------------------------------------------------------------------------------
	protected static bool Parse()
	{
		s_aRows = new array<ref TBD_VehicleStateWireStruct>();

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
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

	//------------------------------------------------------------------------------------------------
	protected static IEntity FindBody(TBD_VehicleStateWireStruct wire)
	{
		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return null;

		s_QueryHit = null;

		float xz = TBD_VehicleStateWireStruct.XZ_M;
		float y = TBD_VehicleStateWireStruct.Y_M;
		vector mins = Vector(wire.x - xz, -y, wire.z - xz);
		vector maxs = Vector(wire.x + xz, y, wire.z + xz);
		world.QueryEntitiesByAABB(mins, maxs, OnQuery);

		IEntity hit = s_QueryHit;
		s_QueryHit = null;
		return hit;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool OnQuery(IEntity entity)
	{
		if (!entity)
			return true;
		if (ChimeraCharacter.Cast(entity))
			return true;
		if (!Vehicle.Cast(entity))
			return true;

		s_QueryHit = entity;
		return false;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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
