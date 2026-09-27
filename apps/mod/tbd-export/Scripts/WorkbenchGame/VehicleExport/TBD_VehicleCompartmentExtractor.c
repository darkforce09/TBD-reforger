/**
 * TBD_VehicleCompartmentExtractor.c
 *
 * Dedicated extractor for vehicle compartments, seating slots, tactical crew roles,
 * turnout capabilities, door interactions, and ingress/egress points.
 * Operates purely via Enfusion BaseContainer reflection without hardcoded vehicle tables.
 */

class TBD_VehicleCompartmentExtractor
{
	//! Inspect native door resolution on initialized vehicle installations without advancing gameplay.
	static string VerifyInitializedDoors(string resourceName)
	{
		Workbench.OpenModule(WorldEditor);
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi() || !GetGame()) return "{\"status\":\"no_preview_world\"}";
		Resource resource = Resource.Load(resourceName);
		if (!resource || !resource.IsValid()) return "{\"status\":\"load_failed\"}";
		IEntity entity = GetGame().SpawnEntityPrefab(resource, editor.GetApi().GetWorld());
		if (!entity) return "{\"status\":\"spawn_failed\"}";
		array<string> entries = {};
		ReadInitializedManagers(entity, "root", 0, entries);
		SCR_EntityHelper.DeleteEntityAndChildren(entity);
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Compare authored station door indices with initialized manager lookup results for every child entity.
	protected static void ReadInitializedManagers(IEntity entity, string entityPath, int depth, array<string> entries)
	{
		if (depth > 128)
		{
			entries.Insert("{\"status\":\"preview_child_depth_limit\"}");
			return;
		}
		array<Managed> components = {};
		entity.FindComponents(BaseCompartmentManagerComponent, components);
		foreach (Managed component : components)
		{
			BaseCompartmentManagerComponent manager = BaseCompartmentManagerComponent.Cast(component);
			if (!manager) continue;
			string entry = TBD_EquipmentExportJson.Context(manager.GetComponentSource(entity));
			entry = TBD_EquipmentExportJson.Member(entry, "entity_path", TBD_EquipmentExportJson.Quote(entityPath));
			if (entity.GetPrefabData()) entry = TBD_EquipmentExportJson.Member(entry, "resource_name", TBD_EquipmentExportJson.Quote(entity.GetPrefabData().GetPrefabName()));
			array<BaseCompartmentSlot> slots = {};
			manager.GetCompartments(slots);
			array<string> stations = {};
			foreach (BaseCompartmentSlot slot : slots)
			{
				if (!slot) { stations.Insert("null"); continue; }
				string station = "{\"name\":" + TBD_EquipmentExportJson.Quote(slot.GetCompartmentName()) + "}";
				array<int> indices = {};
				slot.GetAvailableDoorIndices(indices);
				array<string> doors = {};
				foreach (int index : indices) doors.Insert(ReadInitializedDoor(manager, index));
				station = TBD_EquipmentExportJson.Member(station, "doors", "[" + TBD_EquipmentExportJson.Join(doors) + "]");
				stations.Insert(station);
			}
			entries.Insert(TBD_EquipmentExportJson.Member(entry, "stations", "[" + TBD_EquipmentExportJson.Join(stations) + "]"));
		}
		IEntity child = entity.GetChildren();
		int childIndex = 0;
		while (child)
		{
			ReadInitializedManagers(child, entityPath + "/children/" + childIndex.ToString(), depth + 1, entries);
			child = child.GetSibling();
			childIndex++;
		}
	}

	//! Preserve native lookup outcomes without assuming an invalid local index refers to a parent door.
	protected static string ReadInitializedDoor(BaseCompartmentManagerComponent manager, int index)
	{
		string json = "{\"index\":" + index.ToString() + "}";
		BaseCompartmentDoor door = manager.GetDoor(index);
		string doorClass = "null";
		if (door) doorClass = TBD_EquipmentExportJson.Quote(door.Type().ToString());
		json = TBD_EquipmentExportJson.Member(json, "door_class", doorClass);
		if (door) json = TBD_EquipmentExportJson.Member(json, "local_animation_index", door.GetAnimDoorIndex().ToString());
		CompartmentDoorInfo info = manager.GetDoorInfo(index);
		string infoClass = "null";
		if (info) infoClass = TBD_EquipmentExportJson.Quote(info.Type().ToString());
		json = TBD_EquipmentExportJson.Member(json, "resolved_info_class", infoClass);
		CompartmentDoorReference nativeDoorReference = manager.GetDoorReference(index);
		BaseCompartmentManagerComponent referencedManager;
		if (nativeDoorReference) referencedManager = nativeDoorReference.GetReferencedDoorCompartmentManager();
		if (nativeDoorReference) json = TBD_EquipmentExportJson.Member(json, "reference_animation_index", nativeDoorReference.GetAnimDoorIndex().ToString());
		string target = "null";
		if (referencedManager && referencedManager.GetOwner() && referencedManager.GetOwner().GetPrefabData())
			target = TBD_EquipmentExportJson.Quote(referencedManager.GetOwner().GetPrefabData().GetPrefabName());
		json = TBD_EquipmentExportJson.Member(json, "reference_by_animation_index_manager_resource", target);
		return json;
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect seating slots, crew roles, doors, and turnout abilities for a vehicle variant.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData, string installationId = "root")
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseCompartmentManagerComponent")) continue;
			foreach (BaseContainer manager : bucket)
			{
				string managerInstallation = ManagerInstallationId(manager, installationId);
				BaseContainerList slots = manager.GetObjectArray("CompartmentSlots");
				if (!slots) continue;
				for (int i = 0; i < slots.Count(); i++)
				{
					BaseContainer slot = slots.Get(i);
					if (!slot) continue;
					string path = "/crew_stations/" + varData.m_aCrewEntries.Count().ToString();
					string bindings = "seat_type=SeatType|doors=DoorInfoList|can_turn_out=m_bCanTurnOut|context_name=ContextName";
					bindings += "|compartment_section=m_iCompartmentSection|water_tight=m_bIsWaterTight|base_coverage=BaseCoverage";
					bindings += "|forced_free_look=ForcedFreeLook|character_weapon_handling=CharacterWeaponHandling|can_equip_gadget=CanOccupantEquipGadget";
					string json = TBD_EquipmentExportJson.Fields(slot, bindings, path);
					json = TBD_EquipmentExportJson.Member(json, "installation_id", TBD_EquipmentExportJson.Quote(managerInstallation));
					string stationId = managerInstallation + "/" + TBD_EquipmentComponentGraph.InstanceId(manager) + "/slots/" + i.ToString();
					json = TBD_EquipmentExportJson.Member(json, "station_id", TBD_EquipmentExportJson.Quote(stationId));
					json = TBD_EquipmentExportJson.Member(json, "manager_slot_index", i.ToString());
					json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(slot));
					json = TBD_EquipmentExportJson.Member(json, "manager", TBD_EquipmentExportJson.Context(manager));
					BaseContainer position = slot.GetObject("PassengerPositionInfo");
					if (position) json = TBD_EquipmentExportJson.Member(json, "position", TBD_EquipmentExportJson.Fields(position, "pivot_id=PivotID|offset=Offset|angles=Angles", path + "/position"));
					BaseContainer occupant = slot.GetObject("m_DefaultOccupantData");
					if (occupant) json = TBD_EquipmentExportJson.Member(json, "default_occupant", TBD_EquipmentExportJson.Fields(occupant, "prefab=m_sDefaultOccupantPrefab", path + "/default_occupant"));
					BaseContainer ui = slot.GetObject("UIInfo");
					if (ui) json = TBD_EquipmentExportJson.Member(json, "name", TBD_EquipmentExportJson.Field(ui, "Name", path + "/name"));
					json = TBD_EquipmentExportJson.Member(json, "door_definitions", ReadStationDoors(slot, manager, path + "/door_definitions"));
					json = TBD_EquipmentExportJson.Member(json, "access_contexts", ReadAccessContexts(slot, path + "/access_contexts"));
					varData.m_aCrewEntries.Insert(json);
					TBD_VehicleSeatData seat = new TBD_VehicleSeatData();
					seat.m_iSlotId = varData.m_aSeats.Count();
					seat.m_sName = slot.GetName();
					seat.m_sType = slot.GetClassName();
					varData.m_aSeats.Insert(seat);
				}
			}
		}
		varData.m_sCrewJson = "[" + TBD_EquipmentExportJson.Join(varData.m_aCrewEntries) + "]";
	}

	//! Resolve only the station's authored door indices, preserving their order and reference classes.
	protected static string ReadStationDoors(BaseContainer slot, BaseContainer manager, string path)
	{
		array<int> indices = {};
		if (!slot.Get("DoorInfoList", indices)) return "null";
		BaseContainerList configured = manager.GetObjectArray("DoorInfoList");
		array<string> entries = {};
		foreach (int index : indices)
		{
			string location = path + "/" + entries.Count().ToString();
			string json = "{\"manager_door_index\":" + index.ToString() + "}";
			if (!configured || index < 0 || index >= configured.Count() || !configured.Get(index))
			{
				VerifyMissingDoorDefinition(manager, index, location + "/definition");
				entries.Insert(TBD_EquipmentExportJson.Member(json, "definition", "null"));
				continue;
			}
			BaseContainer door = configured.Get(index);
			string definitionPath = location + "/definition";
			string definition = TBD_EquipmentExportJson.Fields(door, "animation_door_index=AnimDoorIndex", definitionPath);
			definition = TBD_EquipmentExportJson.Member(definition, "source", TBD_EquipmentExportJson.Context(door));
			if (TBD_EquipmentComponentGraph.IsA(door.GetClassName(), "CompartmentDoorInfo"))
			{
				string bindings = "context_name=ContextName|door_type=m_eDoorType|fake_door=FakeDoor";
				bindings += "|align_during_get_out=AlignDuringGetOut|get_in_aligning_teleport=GetInAligningTeleport";
				bindings += "|get_out_aligning_teleport=GetOutAligningTeleport|get_in_seat_aligning_on_event=GetInSeatAligningOnEvent";
				definition = TBD_EquipmentExportJson.Member(definition, "access", TBD_EquipmentExportJson.Fields(door, bindings, definitionPath + "/access"));
				array<string> pointProperties = {"EntryPositionInfo", "ExitPositionInfo", "ExitTeleportPositionInfo", "PortalPositionInfo"};
				array<string> pointNames = {"entry", "exit", "exit_teleport", "portal"};
				for (int pointIndex = 0; pointIndex < pointProperties.Count(); pointIndex++)
				{
					BaseContainer point = door.GetObject(pointProperties[pointIndex]);
					if (!point) continue;
					string pointPath = definitionPath + "/" + pointNames[pointIndex];
					string pointJson = TBD_EquipmentExportJson.Fields(point, "pivot_id=PivotID|offset=Offset|angles=Angles", pointPath);
					pointJson = TBD_EquipmentExportJson.Member(pointJson, "source", TBD_EquipmentExportJson.Context(point));
					definition = TBD_EquipmentExportJson.Member(definition, pointNames[pointIndex], pointJson);
				}
				BaseContainer accessibility = door.GetObject("AccessibilitySettings");
				if (accessibility)
				{
					string access = TBD_EquipmentExportJson.Fields(accessibility, "half_extents=BBHalfExtents|trace_offset=TraceOffset|excluded_physics_layers=ExcludePhysicsLayer", definitionPath + "/accessibility");
					definition = TBD_EquipmentExportJson.Member(definition, "accessibility", access);
				}
			}
			entries.Insert(TBD_EquipmentExportJson.Member(json, "definition", definition));
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! A missing configured local door is a source defect only when the initialized native manager agrees.
	protected static void VerifyMissingDoorDefinition(BaseContainer source, int index, string path)
	{
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		string status = "error";
		string explanation = "Configured compartment door index has no definition; initialized manager verification is unavailable";
		if (editor && editor.GetApi() && GetGame())
		{
			Resource resource = Resource.Load(TBD_EquipmentComponentGraph.m_CurrentResource);
			if (resource && resource.IsValid())
			{
				IEntity entity = GetGame().SpawnEntityPrefab(resource, editor.GetApi().GetWorld());
				if (entity)
				{
					array<BaseCompartmentManagerComponent> matches = {};
					FindInitializedManagers(entity, source.GetResourceName(), matches, 0);
					if (matches.Count() == 1)
					{
						BaseCompartmentManagerComponent manager = matches[0];
						if (!manager.GetDoor(index) && !manager.GetDoorInfo(index))
						{
							status = "source_reference_missing";
							explanation = "Authored local DoorInfoList index has no configured or initialized definition. GetDoorReference searches animation indices, not local array indices. Native evidence: " + ReadInitializedDoor(manager, index);
						}
						else explanation = "Native door resolves but its configured definition is missing; extraction requires the native relationship";
					}
					else explanation = "Cannot uniquely match the configured compartment manager to its initialized source";
					SCR_EntityHelper.DeleteEntityAndChildren(entity);
				}
			}
		}
		TBD_EquipmentExportJson.RecordNativeField(source, path, status, "BaseCompartmentManagerComponent.GetDoor/GetDoorInfo/GetDoorReference", explanation, string.Empty, "OBJECT");
		if (status == "error") TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, explanation);
	}

	//! Match exact native component identities and reject ambiguity across repeated child installations.
	protected static void FindInitializedManagers(IEntity entity, string sourceIdentity, array<BaseCompartmentManagerComponent> matches, int depth)
	{
		if (depth > 128 || sourceIdentity.IsEmpty()) return;
		array<Managed> components = {};
		entity.FindComponents(BaseCompartmentManagerComponent, components);
		foreach (Managed component : components)
		{
			BaseCompartmentManagerComponent manager = BaseCompartmentManagerComponent.Cast(component);
			if (!manager) continue;
			BaseContainer source = manager.GetComponentSource(entity);
			if (source && source.GetResourceName() == sourceIdentity) matches.Insert(manager);
		}
		IEntity child = entity.GetChildren();
		while (child)
		{
			FindInitializedManagers(child, sourceIdentity, matches, depth + 1);
			child = child.GetSibling();
		}
	}

	//! Keep authored access context relationships without action presentation or menu configuration.
	protected static string ReadAccessContexts(BaseContainer slot, string path)
	{
		array<string> properties = {"CompartmentAction", "GetOutAction", "JumpOutAction", "SwitchSeatAction"};
		array<string> names = {"get_in", "get_out", "jump_out", "switch_seat"};
		string json = "{}";
		for (int i = 0; i < properties.Count(); i++)
		{
			BaseContainer action = slot.GetObject(properties[i]);
			if (!action) continue;
			string location = path + "/" + names[i];
			string contexts = TBD_EquipmentExportJson.Fields(action, "parent_contexts=ParentContextList", location);
			contexts = TBD_EquipmentExportJson.Member(contexts, "source", TBD_EquipmentExportJson.Context(action));
			json = TBD_EquipmentExportJson.Member(json, names[i], contexts);
		}
		return json;
	}

	//! Qualify actual child entities while retaining the supplied prefab installation identity.
	protected static string ManagerInstallationId(BaseContainer manager, string installation)
	{
		string path = TBD_EquipmentComponentGraph.StructuralPath(manager);
		int child = path.IndexOf("/children/");
		string entityPath;
		while (child >= 0)
		{
			int end = path.IndexOfFrom(child + 10, "/");
			if (end < 0) { entityPath = path; break; }
			entityPath = TBD_EquipmentExportJson.PreserveSubstring(path, 0, end);
			child = path.IndexOfFrom(end, "/children/");
		}
		if (entityPath.IsEmpty()) return installation;
		return installation + "/" + entityPath;
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
				// No source relationship is available for this slot.
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
		return slotCls;
	}

	protected static string FormatFallbackSeatName(string role, string slotName, int passengerIndex)
	{
		return slotName;
	}
}
