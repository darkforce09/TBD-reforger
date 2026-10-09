/**
 * @file TBD_MissionFactionSummary.c
 * @brief One faction's part of a mission: role, slots, vehicles and objectives.
 *
 * Role: one faction column of the inspector's ORBAT OVERVIEW and OBJECTIVES cards.  Position: TBD_MissionSelectorMock builds them through the chaining AddVehicle and AddObjective;
 * TBD_MissionInspectorCards renders them.
 * State: none beyond its fields.  Invariants: the vehicle and objective arrays are never null.
 */

//! Per-faction part of a mission: header (tag, role, slots) + vehicles + objectives.
class TBD_MissionFactionSummary
{
	string m_sKey;           //!< "BLUFOR"
	string m_sRole;          //!< "Defending" / "Attacking"
	int m_iSlots;            //!< playable slots of this faction
	TBD_EUITint m_eTint;     //!< BLUFOR / OPFOR
	ref array<ref TBD_MissionAsset> m_aVehicles; //!< counted vehicle lines
	ref array<ref TBD_MissionObjective> m_aObjectives; //!< objective lines

	//! Build a faction row with empty vehicle and objective lists.
	void TBD_MissionFactionSummary(string key, string role, int slots, TBD_EUITint tint)
	{
		m_sKey = key;
		m_sRole = role;
		m_iSlots = slots;
		m_eTint = tint;
		m_aVehicles = {};
		m_aObjectives = {};
	}

	//! Append a counted vehicle line.
	//! @return this row, for chaining
	TBD_MissionFactionSummary AddVehicle(string name, int count)
	{
		m_aVehicles.Insert(new TBD_MissionAsset(name, count));
		return this;
	}

	//! Append an objective line.
	//! @return this row, for chaining
	TBD_MissionFactionSummary AddObjective(string title, string icon)
	{
		m_aObjectives.Insert(new TBD_MissionObjective(title, icon));
		return this;
	}
}

//! A counted asset line in a faction's ORBAT column ("2x  BMP-2").
class TBD_MissionAsset
{
	string m_sName; //!< vehicle name ("BMP-2")
	int m_iCount; //!< how many

	//! Build a vehicle line.
	void TBD_MissionAsset(string name, int count)
	{
		m_sName = name;
		m_iCount = count;
	}
}

//! One objective line in a faction's OBJECTIVES column.
class TBD_MissionObjective
{
	string m_sTitle;         //!< "Defend Sector 1"
	string m_sIcon;          //!< "shield" / "flag"

	//! Build an objective line.
	void TBD_MissionObjective(string title, string icon)
	{
		m_sTitle = title;
		m_sIcon = icon;
	}
}
