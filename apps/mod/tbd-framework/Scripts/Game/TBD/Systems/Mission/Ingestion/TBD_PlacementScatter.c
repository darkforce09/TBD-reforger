/**
 * @file TBD_PlacementScatter.c
 * @brief Deterministic spawn offsets from `placementRadius` and `placementShape` on slots and groups.
 *
 * Role: a second `JsonLoadContext` pass over the held mission JSON whose root declares only the
 * scatter keys of `slots[]` and `orbat.*.groups[]`, and the scatter that jitters a slot's
 * authored spawn point.  Position: `ForSlot` runs from `TBD_SlotBodyMaterializer` for every slot
 * body it spawns, initial and respawn; reads `TBD_MissionJsonPass.LoadRoot` and
 * `TBD_MissionLoader.GetMissionId`.
 * State: the static parsed slot and group rows keyed to the mission id, server only.
 * Invariants: `Scatter` is deterministic, so a slot spawns at the same offset every time; radius 0
 * or absent returns the authored point; `square` spreads over the axis-aligned square and anything
 * else over a uniform disk; a group's offset is shared by every member (one seed per faction and
 * callsign) and adds to the slot's own; the offset is horizontal, the spawn owns height.
 */

//! One `slots[]` row's scatter fields. Field names are the JSON keys.
//! @contract mission.schema.json#/$defs/slot
class TBD_PlacementScatterSlotWireStruct
{
	static const float ABSENT = -1000000; //!< "`placementRadius` absent from JSON"

	string id; //!< `id`
	string uid; //!< `uid`, or empty
	float placementRadius = -1000000; //!< Metres. ABSENT when omitted. 0 is authored exact.
	string placementShape;             //!< circle|square. Empty when omitted.

	//! Whether the row authors `placementRadius`.
	bool HasRadius()
	{
		return placementRadius != ABSENT;
	}
}

//! One `$defs/group` object's scatter fields. Field names are the JSON keys.
//! @contract mission.schema.json#/$defs/group
class TBD_PlacementScatterGroupWireStruct
{
	static const float ABSENT = -1000000; //!< "`placementRadius` absent from JSON"

	string callsign; //!< `callsign`
	float placementRadius = -1000000; //!< Metres. ABSENT when omitted. 0 is authored exact.
	string placementShape;             //!< circle|square. Empty when omitted.

	//! Whether the row authors `placementRadius`.
	bool HasRadius()
	{
		return placementRadius != ABSENT;
	}
}

//! One ORBAT faction's groups, as far as scatter reads them.
//! @contract mission.schema.json#/$defs/orbatFaction
class TBD_PlacementScatterFactionWireStruct
{
	ref array<ref TBD_PlacementScatterGroupWireStruct> groups; //!< `groups[]`
}

//! Root of the second parse. Declares `slots` and `orbat` and nothing else.
//! @contract mission.schema.json#/properties/slots
//! @contract mission.schema.json#/properties/orbat
class TBD_PlacementScatterDocStruct
{
	ref array<ref TBD_PlacementScatterSlotWireStruct> slots; //!< `slots[]`
	ref map<string, ref TBD_PlacementScatterFactionWireStruct> orbat; //!< `orbat`, keyed by faction
}

//! Flattened group row after parse (faction key is the orbat map key, not a JSON field).
class TBD_PlacementScatterGroupRow
{
	string faction; //!< the `orbat` map key
	string callsign; //!< the group's `callsign`
	float placementRadius; //!< metres, or `ABSENT`
	string placementShape; //!< circle|square, or empty
}

//! Server-side reader: bind placementRadius/placementShape and scatter slot spawn points.
class TBD_PlacementScatter
{
	static const string CH = "Scatter"; //!< log channel
	static const string SHAPE_CIRCLE = "circle"; //!< `placementShape` for a disk, and the default
	static const string SHAPE_SQUARE = "square"; //!< `placementShape` for an axis-aligned square

	protected static ref array<ref TBD_PlacementScatterSlotWireStruct> s_aSlots; //!< parsed `slots[]` rows
	protected static ref array<ref TBD_PlacementScatterGroupRow> s_aGroups; //!< parsed group rows
	protected static bool s_bParsed; //!< the rows are current for `s_sParsedForMission`
	protected static string s_sParsedForMission; //!< mission id the rows were parsed for

	//! Deterministic scatter. Radius <= 0 returns center unchanged.
	//! @param radius metres
	//! @param shape `square`, else a disk
	//! @param seed the deterministic seed
	//! @return the scattered point; Y is `center`'s
	static vector Scatter(vector center, float radius, string shape, int seed)
	{
		if (radius <= 0)
			return center;

		if (shape == SHAPE_SQUARE)
		{
			float dx = (UnitFloat(seed, 1) * 2.0) - 1.0;
			float dz = (UnitFloat(seed, 2) * 2.0) - 1.0;
			return Vector(center[0] + (dx * radius), center[1], center[2] + (dz * radius));
		}

		float u = UnitFloat(seed, 1);
		float theta = UnitFloat(seed, 2) * Math.PI * 2.0;
		float r = Math.Sqrt(u) * radius;
		float cx = r * Math.Cos(theta);
		float cz = r * Math.Sin(theta);
		return Vector(center[0] + cx, center[1], center[2] + cz);
	}

	//! Slot + group scatter for one spawn. Call from SpawnSlotBody (initial and respawn).
	//! Zero / absent radius on both layers returns Vector(x, 0, z) -- the authored point.
	//! @return the scattered point with Y 0
	//! @authority server
	static vector ForSlot(string slotKey, string slotId, string faction, string groupCallsign, float x, float z)
	{
		vector center = Vector(x, 0, z);
		if (!EnsureParsed())
			return center;

		TBD_PlacementScatterSlotWireStruct slotWire = FindSlot(slotKey, slotId);
		float slotR = 0;
		string slotShape = SHAPE_CIRCLE;
		if (slotWire && slotWire.HasRadius())
		{
			slotR = slotWire.placementRadius;
			if (!slotWire.placementShape.IsEmpty())
				slotShape = slotWire.placementShape;
		}

		int slotSeed = SeedFromString(slotKey);
		if (slotKey.IsEmpty())
			slotSeed = SeedFromString(slotId);

		vector pos = Scatter(center, slotR, slotShape, slotSeed);

		TBD_PlacementScatterGroupRow groupRow = FindGroup(faction, groupCallsign);
		float groupR = 0;
		string groupShape = SHAPE_CIRCLE;
		if (groupRow && groupRow.placementRadius != TBD_PlacementScatterSlotWireStruct.ABSENT)
		{
			groupR = groupRow.placementRadius;
			if (!groupRow.placementShape.IsEmpty())
				groupShape = groupRow.placementShape;
		}

		int groupSeed = SeedFromString(faction + ":" + groupCallsign);
		vector origin = Vector(0, 0, 0);
		vector groupOffset = Scatter(origin, groupR, groupShape, groupSeed);
		vector outPos = Vector(pos[0] + groupOffset[0], 0, pos[2] + groupOffset[2]);

		if (slotR > 0 || groupR > 0)
		{
			Print(string.Format("[TBD][Scatter] slot=%1 from=%2,%3 to=%4,%5 slotR=%6 groupR=%7",
				slotKey, x, z, outPos[0], outPos[2], slotR, groupR));
		}

		return outPos;
	}

	//! Parse the scatter pass once per mission id.
	//! @return false when no mission text is held; true otherwise, with an ERROR line and no rows
	//! when the text or its root does not read
	protected static bool EnsureParsed()
	{
		string missionId = TBD_MissionLoader.GetMissionId();
		if (s_bParsed && missionId == s_sParsedForMission)
			return true;

		s_aSlots = new array<ref TBD_PlacementScatterSlotWireStruct>();
		s_aGroups = new array<ref TBD_PlacementScatterGroupRow>();

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (outcome == TBD_EMissionJsonPassOutcome.NO_DOCUMENT)
			return false;

		if (!ctx)
		{
			Print("[TBD][Scatter] the mission document did not parse as JSON on the scatter pass - spawn stays exact", LogLevel.ERROR);
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		TBD_PlacementScatterDocStruct doc = new TBD_PlacementScatterDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print("[TBD][Scatter] the mission document parsed but its root would not read on the scatter pass - spawn stays exact", LogLevel.ERROR);
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		if (doc.slots)
			s_aSlots = doc.slots;

		if (doc.orbat)
		{
			foreach (string factionKey, TBD_PlacementScatterFactionWireStruct faction : doc.orbat)
			{
				CollectFaction(factionKey, faction);
			}
		}

		s_bParsed = true;
		s_sParsedForMission = missionId;
		return true;
	}

	//! Flatten one faction's groups with a callsign into `s_aGroups`.
	protected static void CollectFaction(string factionKey, TBD_PlacementScatterFactionWireStruct faction)
	{
		if (!faction || !faction.groups)
			return;

		foreach (TBD_PlacementScatterGroupWireStruct group : faction.groups)
		{
			if (!group)
				continue;
			if (group.callsign.IsEmpty())
				continue;

			TBD_PlacementScatterGroupRow row = new TBD_PlacementScatterGroupRow();
			row.faction = factionKey;
			row.callsign = group.callsign;
			row.placementRadius = group.placementRadius;
			row.placementShape = group.placementShape;
			s_aGroups.Insert(row);
		}
	}

	//! The parsed row of a slot: by `uid` equal to `slotKey` first, else by `id`.
	//! @return the row, or null
	protected static TBD_PlacementScatterSlotWireStruct FindSlot(string slotKey, string slotId)
	{
		if (!s_aSlots)
			return null;

		foreach (TBD_PlacementScatterSlotWireStruct row : s_aSlots)
		{
			if (!row)
				continue;
			if (slotKey.IsEmpty())
				continue;
			if (row.uid.IsEmpty())
				continue;
			if (row.uid == slotKey)
				return row;
		}

		foreach (TBD_PlacementScatterSlotWireStruct row : s_aSlots)
		{
			if (!row)
				continue;
			if (!slotId.IsEmpty() && row.id == slotId)
				return row;
			if (!slotKey.IsEmpty() && row.id == slotKey)
				return row;
		}

		return null;
	}

	//! The parsed group row of a faction and callsign.
	//! @return the row, or null when either key is empty or unknown
	protected static TBD_PlacementScatterGroupRow FindGroup(string faction, string groupCallsign)
	{
		if (!s_aGroups)
			return null;
		if (faction.IsEmpty())
			return null;
		if (groupCallsign.IsEmpty())
			return null;

		foreach (TBD_PlacementScatterGroupRow row : s_aGroups)
		{
			if (!row)
				continue;
			if (row.faction != faction)
				continue;
			if (row.callsign != groupCallsign)
				continue;
			return row;
		}

		return null;
	}

	//! djb2 over bytes. Slot ids are ASCII (faction:callsign:role:n / uid).
	protected static int SeedFromString(string s)
	{
		int h = 5381;
		if (s.IsEmpty())
			return h;

		int n = s.Length();
		int i = 0;
		while (i < n)
		{
			int code = s.Get(i).ToAscii();
			h = (h * 33) + code;
			i++;
		}

		return h;
	}

	//! Deterministic unit float in [0, 1). Not Math.Random* (those are global / non-repeatable).
	protected static float UnitFloat(int seed, int salt)
	{
		int x = seed + (salt * 374761393);
		x = x * 1103515245 + 12345;
		int mag = x;
		if (mag < 0)
			mag = -1 - mag;
		int unit = mag % 1000000;
		return unit / 1000000.0;
	}
}
