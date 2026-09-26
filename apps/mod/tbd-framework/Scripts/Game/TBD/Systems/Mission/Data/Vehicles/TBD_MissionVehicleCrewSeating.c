/**
 * @file TBD_MissionVehicleCrewSeating.c
 * @brief Seats the authored crew of one roster vehicle and verifies the seating a second later.
 *
 * Role: resolves each `seats[]` entry to a slot body and a compartment, force-teleports the body
 * in, and asks the engine afterwards which bodies are inside.  Position: called by
 * `TBD_MissionVehicleRoster.SeatAuthoredCrews`; reads `TBD_MissionLoader.GetSlotById` and
 * `TBD_SpawnManager.GetSlotBody`.
 * State: the static pending-seat list of the running pass, server only.  Invariants: a seat is
 * reported accepted, never seated, until `VerifySeatedCrews` sees the body in that vehicle; a
 * failing seat skips only itself.
 */

//! One seat whose `GetInVehicle` was accepted, held until the deferred pass can ask the engine
//! whether the body actually ended up inside. See `TBD_MissionVehicleCrewSeating.VerifySeatedCrews`.
class TBD_MissionVehiclePendingSeat
{
	IEntity body;      //!< The slot body that was told to get in.
	IEntity vehicle;   //!< The vehicle it was told to get into.
	string slotKey;    //!< The slot's durable key, for the log line.
	string vehicleLabel; //!< The roster row's label, for the log line.
	string role;       //!< The authored seat role, for the log line.
}

//! Seats one roster row's authored crew and verifies the result.
class TBD_MissionVehicleCrewSeating
{
	//! How long to wait before asking the engine whether the accepted seats actually took.
	//!
	//! MEASURED, not guessed: `GetInVehicle` returns true in the requesting frame but
	//! `GetVehicleIn` still answers null there, so a same-frame check reports every seat as
	//! deferred (four of four on the roster fixture). One second is ~4x `TBD_SpawnManager`'s own
	//! 250 ms settle tick and well inside the headless boot's settle window, so the verdict lands in
	//! the same boot log as the request.
	protected static const int SEAT_VERIFY_DELAY_MS = 1000; //!< milliseconds from the seat requests to the verification pass

	protected static ref array<ref TBD_MissionVehiclePendingSeat> s_aPendingSeats; //!< seats accepted by the running pass, awaiting verification; null after it

	//! Start a roster pass: drop any pending seats of an earlier pass.
	//! @authority server
	static void BeginPass()
	{
		s_aPendingSeats = new array<ref TBD_MissionVehiclePendingSeat>();
	}

	//! Arm `VerifySeatedCrews` `SEAT_VERIFY_DELAY_MS` from now when this pass accepted any seat;
	//! does nothing otherwise.
	//! @authority server
	static void ArmVerification()
	{
		if (!s_aPendingSeats || s_aPendingSeats.Count() == 0)
			return;

		GetGame().GetCallqueue().CallLater(VerifySeatedCrews, SEAT_VERIFY_DELAY_MS, false);
	}

	//! Seat every slot this roster row names.
	//!
	//! Returns how many seats the engine ACCEPTED; `skipped` counts the ones that could not be
	//! requested at all, each of which has logged its own reason. Accepted is deliberately not
	//! called seated: only `VerifySeatedCrews`, a second later, can say a body is in the vehicle.
	//! @param spawner the spawn manager that owns the slot bodies
	//! @param veh the roster row
	//! @param vehicle the row's world vehicle
	//! @param skipped set to the number of seats that could not be requested
	//! @return the number of seats the engine accepted
	//! @authority server
	static int SeatCrew(TBD_SpawnManager spawner, TBD_MissionVehicleStruct veh, IEntity vehicle, out int skipped)
	{
		skipped = 0;
		int accepted = 0;
		if (veh.CrewCount() == 0)
			return 0;

		foreach (TBD_MissionVehicleSeatStruct seat : veh.seats)
		{
			if (!seat)
			{
				skipped++;
				continue;
			}

			if (SeatOne(spawner, veh, vehicle, seat))
				accepted++;
			else
				skipped++;
		}

		return accepted;
	}

	//! Move one authored slot's body into one authored crew station.
	//!
	//! Every failure is REPORTED and skips only that seat: a seat that cannot be filled must never
	//! cost the rest of the crew their places, and a crew plan is never dropped silently.
	//! @return whether the engine ACCEPTED the seat -- never whether the body ended up in the
	//! vehicle. Only the deferred `VerifySeatedCrews` pass can answer that.
	protected static bool SeatOne(TBD_SpawnManager spawner, TBD_MissionVehicleStruct veh, IEntity vehicle, TBD_MissionVehicleSeatStruct seat)
	{
		if (seat.slotId.IsEmpty())
		{
			Print(string.Format("[TBD][Vehicles] %1 seat role='%2' names no slot -- skipped", veh.Label(), seat.role), LogLevel.WARNING);
			return false;
		}

		// `seats[].slotId` carries a slot UID; GetSlotById is uid-aware, so this is the one correct
		// resolver. flatten refuses to emit a seat naming a slot it did not emit, but this build
		// parses documents it did not compile, so a dangling reference is reported rather than
		// resolved to whichever seat happens to answer.
		TBD_MissionSlotStruct slot = TBD_MissionLoader.GetSlotById(seat.slotId);
		if (!slot)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat role='%2' slotId='%3' resolves to no slot -- skipped",
				veh.Label(), seat.role, seat.slotId), LogLevel.WARNING);
			return false;
		}

		IEntity body = spawner.GetSlotBody(slot.Key());
		if (!body)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat role='%2' slot=%3 has no materialized body -- skipped",
				veh.Label(), seat.role, slot.Key()), LogLevel.WARNING);
			return false;
		}

		bool knownRole;
		ECompartmentType type = CompartmentTypeFor(seat.role, knownRole);
		if (!knownRole)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat slot=%2 role='%3' is not one of the seven authored roles -- skipped",
				veh.Label(), slot.Key(), seat.role), LogLevel.WARNING);
			return false;
		}

		BaseCompartmentSlot compartment = ResolveCompartment(veh, vehicle, seat, type);
		if (!compartment)
			return false;

		ChimeraCharacter character = ChimeraCharacter.Cast(body);
		if (!character)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat slot=%2 body is not a character -- skipped", veh.Label(), slot.Key()), LogLevel.WARNING);
			return false;
		}

		SCR_CompartmentAccessComponent access = SCR_CompartmentAccessComponent.Cast(character.GetCompartmentAccessComponent());
		if (!access)
		{
			Print(string.Format("[TBD][Vehicles] %1 seat slot=%2 body has no SCR_CompartmentAccessComponent -- NOT seated",
				veh.Label(), slot.Key()), LogLevel.WARNING);
			return false;
		}

		// FORCE TELEPORT, AND THIS WAS MEASURED, NOT ASSUMED.
		//
		// `MoveInVehicle` was the obvious call and it is the WRONG one here: it returns true for a
		// request it has merely accepted, and the character then has to WALK to the door and play the
		// get-in animation. Slot bodies have their AI disabled at spawn (`DisableBodyAI`), so that
		// walk never happens and the crew would stand beside the vehicle forever while the log said
		// "seated". Measured on a headless boot of the roster fixture: four seats accepted, zero of
		// them inside the vehicle.
		//
		// `GetInVehicle(..., forceTeleport = true, ...)` is the immediate form and is what an
		// AUTHORED crew plan means: the crew START in the vehicle, they do not run to it at mission
		// load. `doorInfoIndex` is ignored under force teleport (engine doc), so -1 is passed to say
		// "no door", and the door is left as authored rather than animated shut behind a teleport.
		// The resolved compartment is passed explicitly, so the authored station is the one taken.
		if (!access.GetInVehicle(vehicle, compartment, true, -1, ECloseDoorAfterActions.LEAVE_OPEN, false))
		{
			Print(string.Format("[TBD][Vehicles] %1 seat slot=%2 role='%3' -- GetInVehicle refused the compartment",
				veh.Label(), slot.Key(), seat.role), LogLevel.WARNING);
			return false;
		}

		// A true from the call above says the REQUEST was ACCEPTED. It does not say the body is in the
		// vehicle, and reporting the first as the second is exactly how a dead mechanism reads as a
		// working one. Measured on the headless roster fixture: `GetVehicleIn` answers null in the
		// requesting frame for every one of four accepted seats, force teleport included -- the
		// engine attaches the occupant later. So the seat is queued for the deferred pass, which asks
		// the engine itself, and nothing calls it seated until the engine says so.
		QueuePendingSeat(body, vehicle, slot.Key(), veh.Label(), seat.role);

		Print(string.Format("[TBD][Vehicles] %1 seat accepted slot=%2 role='%3' index=%4 compartmentType=%5 -- awaiting engine confirmation",
			veh.Label(), slot.Key(), seat.role, seat.index, type));
		return true;
	}

	//! Hold one accepted seat for the deferred verification pass; the strings are for its log line.
	protected static void QueuePendingSeat(IEntity body, IEntity vehicle, string slotKey, string vehicleLabel, string role)
	{
		if (!s_aPendingSeats)
			s_aPendingSeats = new array<ref TBD_MissionVehiclePendingSeat>();

		TBD_MissionVehiclePendingSeat pending = new TBD_MissionVehiclePendingSeat();
		pending.body = body;
		pending.vehicle = vehicle;
		pending.slotKey = slotKey;
		pending.vehicleLabel = vehicleLabel;
		pending.role = role;
		s_aPendingSeats.Insert(pending);
	}

	//! THE SEATING PROOF. Ask the engine, one second after the requests, which bodies are actually in
	//! the vehicle they were told to get into.
	//!
	//! `GetVehicleIn` is the engine's own answer and it is compared against the SPECIFIC vehicle the
	//! seat named, not merely "some vehicle" -- a crewman in the wrong vehicle is the failure this is
	//! for, and "is he in a vehicle at all" could not see it. A seat that never confirms is reported
	//! at WARNING with its slot and role, so an authored crew plan never silently does nothing.
	//! Clears the pending list when done.
	//! @authority server
	static void VerifySeatedCrews()
	{
		if (!s_aPendingSeats || s_aPendingSeats.Count() == 0)
			return;

		int confirmed = 0;
		int unconfirmed = 0;
		foreach (TBD_MissionVehiclePendingSeat pending : s_aPendingSeats)
		{
			if (!pending || !pending.body || !pending.vehicle)
			{
				unconfirmed++;
				continue;
			}

			if (SCR_CompartmentAccessComponent.GetVehicleIn(pending.body) == pending.vehicle)
			{
				confirmed++;
				continue;
			}

			unconfirmed++;
			Print(string.Format("[TBD][Vehicles] %1 slot=%2 role='%3' -- accepted but the engine STILL does not report the body in this vehicle after %4 ms; the authored crew plan did not take effect for this seat",
				pending.vehicleLabel, pending.slotKey, pending.role, SEAT_VERIFY_DELAY_MS), LogLevel.WARNING);
		}

		LogLevel level = LogLevel.NORMAL;
		if (unconfirmed > 0)
			level = LogLevel.WARNING;

		Print(string.Format("[TBD][Vehicles] seating verified inVehicle=%1 of %2 accepted (%3 unconfirmed after %4 ms)",
			confirmed, s_aPendingSeats.Count(), unconfirmed, SEAT_VERIFY_DELAY_MS), level);

		s_aPendingSeats = null;
	}

	//! Which compartment of the resolved type this seat takes.
	//!
	//! An AUTHORED `index` is EXACT: if that ordinal does not exist or is already taken, the seat is
	//! reported and skipped rather than moved somewhere the author did not choose. An ABSENT index
	//! means "a station of this role", so it starts at the role's natural ordinal and may fall
	//! forward to the next free station of the same type -- which is what lets a commander and a
	//! gunner, both TURRET stations, resolve to different turrets. Every fall-forward is logged;
	//! none of it is silent.
	protected static BaseCompartmentSlot ResolveCompartment(TBD_MissionVehicleStruct veh, IEntity vehicle, TBD_MissionVehicleSeatStruct seat, ECompartmentType type)
	{
		SCR_BaseCompartmentManagerComponent manager = SCR_BaseCompartmentManagerComponent.Cast(vehicle.FindComponent(SCR_BaseCompartmentManagerComponent));
		if (!manager)
		{
			Print(string.Format("[TBD][Vehicles] %1 alias='%2' has no compartment manager -- role='%3' NOT seated",
				veh.Label(), veh.alias, seat.role), LogLevel.WARNING);
			return null;
		}

		array<BaseCompartmentSlot> compartments = {};
		manager.GetCompartmentsOfType(compartments, type);
		if (compartments.IsEmpty())
		{
			Print(string.Format("[TBD][Vehicles] %1 alias='%2' has no compartment of the type role='%3' needs -- NOT seated",
				veh.Label(), veh.alias, seat.role), LogLevel.WARNING);
			return null;
		}

		int ordinal = DefaultOrdinalFor(seat.role);
		if (seat.HasIndex())
			ordinal = seat.index;

		if (ordinal >= 0 && ordinal < compartments.Count())
		{
			BaseCompartmentSlot exact = compartments[ordinal];
			if (exact && !exact.IsOccupied())
				return exact;
		}

		if (seat.HasIndex())
		{
			Print(string.Format("[TBD][Vehicles] %1 role='%2' authored index=%3 -- that station is missing or already occupied (%4 of this type) -- NOT seated",
				veh.Label(), seat.role, seat.index, compartments.Count()), LogLevel.WARNING);
			return null;
		}

		foreach (BaseCompartmentSlot free : compartments)
		{
			if (!free || free.IsOccupied())
				continue;

			Print(string.Format("[TBD][Vehicles] %1 role='%2' authored no index -- station %3 was taken, using the next free one of the same type",
				veh.Label(), seat.role, ordinal));
			return free;
		}

		Print(string.Format("[TBD][Vehicles] %1 role='%2' -- every station of this type (%3) is occupied -- NOT seated",
			veh.Label(), seat.role, compartments.Count()), LogLevel.WARNING);
		return null;
	}

	//! `$defs/vehicle.seats[].role` to the engine's compartment type.
	//!
	//! The engine models crew stations as three types only (PILOT / TURRET / CARGO), so this is the
	//! whole of what a prefab-independent mapping can say. `commander`, `gunner` and `turret` all map
	//! to TURRET because Reforger models every crewed station that is not the driver as a turret
	//! compartment; a prefab that has none reports and skips the seat rather than dropping the crew
	//! member into cargo, which would look seated and be wrong.
	//!
	//! `known` is an out flag rather than a sentinel because ECompartmentType has no "unset" member
	//! to borrow -- all three of its values are real answers.
	protected static ECompartmentType CompartmentTypeFor(string role, out bool known)
	{
		known = true;

		if (role == "driver")
			return ECompartmentType.PILOT;
		if (role == "pilot")
			return ECompartmentType.PILOT;
		if (role == "copilot")
			return ECompartmentType.PILOT;
		if (role == "commander")
			return ECompartmentType.TURRET;
		if (role == "gunner")
			return ECompartmentType.TURRET;
		if (role == "turret")
			return ECompartmentType.TURRET;
		if (role == "cargo")
			return ECompartmentType.CARGO;

		known = false;
		return ECompartmentType.CARGO;
	}

	//! The station ordinal a role takes when the author gave no `index`.
	//!
	//! Only `copilot` is not 0, and that is what the word means: the second PILOT station. A vehicle
	//! with one pilot station therefore reports and skips an authored copilot instead of putting them
	//! in the pilot's seat.
	protected static int DefaultOrdinalFor(string role)
	{
		if (role == "copilot")
			return 1;
		return 0;
	}
}
