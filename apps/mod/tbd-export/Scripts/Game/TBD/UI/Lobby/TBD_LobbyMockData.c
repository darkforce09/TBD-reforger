//! TBD Lobby Mock Data Layer -- 1:1 Parity with Stitch "Reforger Dark Tactical" Workstation Mockup.
//! Provides structured mock scenario data, factions, squad cards, slot roles, detailed kit specs,
//! and voice/TFAR comms hierarchy for the Lobby UI.

class TBD_MockRoleLoadout
{
	string m_sKey;
	string m_sRoleTitle;
	string m_sRoleCategory;       //!< OFFICER, NCO, MED, ENG, CREW, etc.
	string m_sClearance;          //!< "Full Command", "Squad Command", "Crew Authorization", etc.
	string m_sRank;               //!< "Captain (O-3)", "Staff Sergeant (E-6)", "Corporal (E-4)", etc.
	string m_sSquadAssignment;    //!< "Assigned to: A1-1 Company HQ"

	string m_sVehicleName;        //!< "M113A3 MEV", "M60A1 Patton MBT"
	string m_sVehicleStatus;      //!< "Operable", "Reserve", "Standby"
	string m_sVehicleDesc;        //!< Vehicle features and capability text

	string m_sPrimaryWeapon;      //!< "M16A2 Carbine / Rifle"
	string m_sPrimaryCaliber;     //!< "5.56x45mm NATO - STANAG"
	string m_sPrimaryAmmo;        //!< "7x 30rnd"
	ref array<string> m_aPrimaryAttachments; //!< "Aimpoint Optic", "3-Point Sling", "Condition: Clean"

	string m_sSidearm;            //!< "M9 Beretta 9x19mm"
	string m_sSidearmAmmo;        //!< "3x 15rnd"

	string m_sHelmetVest;         //!< "PASGT Helmet (Kevlar) & Body Armor"
	string m_sHelmetVestRating;   //!< "Level III-A"
	string m_sRig;                //!< "ALICE LBE Rig (Woodland BDU)"
	string m_sRigLoad;            //!< "Load: 18.4 kg"
	string m_sRadio;              //!< "AN/PRC-77 Backpack Radio"
	string m_sRadioFreq;          //!< "VHF Long-Range - Ch 01 / 48.50 MHz"
	string m_sRadioStatus;        //!< "Active Link"

	ref array<string> m_aInventoryItems; //!< "Vector 21 / M22", "MicroDAGR & Lensatic", etc.
	string m_sIfakStatus;         //!< "Complete"
	ref array<string> m_aIfakPills; //!< "Tourniquet", "QuikClot", "Field Bandages (4x)", "Morphine (2x)"

	string m_sAiStatus;           //!< "AI Ready" or "AI"
	string m_sClaimStatus;        //!< "Claimable" or "Restricted" or "Occupied"
	bool m_bIsClaimable;
	bool m_bIsClaimed;
	string m_sOccupantName;

	void TBD_MockRoleLoadout()
	{
		m_aPrimaryAttachments = {};
		m_aInventoryItems = {};
		m_aIfakPills = {};
	}
}

class TBD_MockSquad
{
	string m_sCallsign;           //!< "A1-1 Company HQ"
	string m_sVehicleSummary;     //!< "- M113A3 MEV"
	string m_sCategory;           //!< "COMMAND & MEDICAL", "ARMOR LEAD", etc.
	string m_sUnitType;           //!< "HQ", "ARMOR", "MECH"
	bool m_bIsCommand;
	ref array<ref TBD_MockRoleLoadout> m_aRoles;

	void TBD_MockSquad()
	{
		m_aRoles = {};
	}
}

class TBD_MockFaction
{
	string m_sKey;                //!< "BLUFOR", "OPFOR"
	string m_sName;               //!< "BLUFOR", "OPFOR"
	int m_iClaimed;               //!< 0
	int m_iTotalSlots;            //!< 92
	int m_iColor;
	ref array<ref TBD_MockSquad> m_aSquads;

	void TBD_MockFaction()
	{
		m_aSquads = {};
	}
}

class TBD_MockVoiceMember
{
	string m_sName;               //!< "Mission Maker (You)"
	string m_sRoleBadge;          //!< "HOST", "2IC", "MEDIC", "SL"
	bool m_bIsSpeaking;
	bool m_bIsMuted;
	bool m_bHasHeadphones;

	void TBD_MockVoiceMember(string name, string roleBadge, bool speaking, bool muted, bool headphones = true)
	{
		m_sName = name;
		m_sRoleBadge = roleBadge;
		m_bIsSpeaking = speaking;
		m_bIsMuted = muted;
		m_bHasHeadphones = headphones;
	}
}

class TBD_MockVoiceChannel
{
	string m_sName;               //!< "HQ & Command Planning"
	int m_iConnected;             //!< 3
	int m_iCapacity;              //!< 6
	bool m_bActive;
	bool m_bExpanded;
	ref array<ref TBD_MockVoiceMember> m_aMembers;

	void TBD_MockVoiceChannel(string name, int connected, int capacity, bool active = false, bool expanded = false)
	{
		m_sName = name;
		m_iConnected = connected;
		m_iCapacity = capacity;
		m_bActive = active;
		m_bExpanded = expanded;
		m_aMembers = {};
	}
}

class TBD_LobbyMockData
{
	static ref array<ref TBD_MockFaction> GetMockFactions()
	{
		array<ref TBD_MockFaction> factions = {};

		// == BLUFOR ==
		TBD_MockFaction blufor = new TBD_MockFaction();
		blufor.m_sKey = "BLUFOR";
		blufor.m_sName = "BLUFOR";
		blufor.m_iClaimed = 0;
		blufor.m_iTotalSlots = 92;
		blufor.m_iColor = 0xFF38BDF8; // Cyan

		// Squad 1: A1-1 Company HQ
		TBD_MockSquad sq1 = new TBD_MockSquad();
		sq1.m_sCallsign = "A1-1 Company HQ";
		sq1.m_sVehicleSummary = "- M113A3 MEV";
		sq1.m_sCategory = "COMMAND & MEDICAL";
		sq1.m_sUnitType = "HQ";
		sq1.m_bIsCommand = true;

		// Role 1: Company Commander (Selected by default)
		TBD_MockRoleLoadout r1 = new TBD_MockRoleLoadout();
		r1.m_sKey = "blufor:hq:commander";
		r1.m_sRoleTitle = "Company Commander";
		r1.m_sRoleCategory = "OFFICER";
		r1.m_sClearance = "Full Command";
		r1.m_sRank = "Captain (O-3)";
		r1.m_sSquadAssignment = "Assigned to: A1-1 Company HQ";
		r1.m_sVehicleName = "M113A3 MEV";
		r1.m_sVehicleStatus = "Operable";
		r1.m_sVehicleDesc = "Equipped with medical litter stations, smoke countermeasure launchers, and upgraded armor kit.";
		r1.m_sPrimaryWeapon = "M16A2 Carbine / Rifle";
		r1.m_sPrimaryCaliber = "5.56x45mm NATO - STANAG";
		r1.m_sPrimaryAmmo = "7x 30rnd";
		r1.m_aPrimaryAttachments.Insert("Aimpoint Optic");
		r1.m_aPrimaryAttachments.Insert("3-Point Sling");
		r1.m_aPrimaryAttachments.Insert("Condition: Clean");
		r1.m_sSidearm = "M9 Beretta 9x19mm";
		r1.m_sSidearmAmmo = "3x 15rnd";
		r1.m_sHelmetVest = "PASGT Helmet (Kevlar) & Body Armor";
		r1.m_sHelmetVestRating = "Level III-A";
		r1.m_sRig = "ALICE LBE Rig (Woodland BDU)";
		r1.m_sRigLoad = "Load: 18.4 kg";
		r1.m_sRadio = "AN/PRC-77 Backpack Radio";
		r1.m_sRadioFreq = "VHF Long-Range - Ch 01 / 48.50 MHz";
		r1.m_sRadioStatus = "Active Link";
		r1.m_aInventoryItems.Insert("Vector 21 / M22");
		r1.m_aInventoryItems.Insert("MicroDAGR & Lensatic");
		r1.m_aInventoryItems.Insert("2x M18 White, 1x Grn");
		r1.m_aInventoryItems.Insert("2x M67 Frag Grenade");
		r1.m_sIfakStatus = "Complete";
		r1.m_aIfakPills.Insert("Tourniquet");
		r1.m_aIfakPills.Insert("QuikClot");
		r1.m_aIfakPills.Insert("Field Bandages (4x)");
		r1.m_aIfakPills.Insert("Morphine (2x)");
		r1.m_sAiStatus = "AI Ready";
		r1.m_sClaimStatus = "Claimable";
		r1.m_bIsClaimable = true;
		r1.m_bIsClaimed = false;
		sq1.m_aRoles.Insert(r1);

		// Role 2: 2ic (Executive Officer)
		TBD_MockRoleLoadout r2 = new TBD_MockRoleLoadout();
		r2.m_sKey = "blufor:hq:2ic";
		r2.m_sRoleTitle = "2ic (Executive Officer)";
		r2.m_sRoleCategory = "NCO";
		r2.m_sClearance = "Deputy Command";
		r2.m_sRank = "1st Lieutenant (O-2)";
		r2.m_sSquadAssignment = "Assigned to: A1-1 Company HQ";
		r2.m_sVehicleName = "M113A3 MEV";
		r2.m_sVehicleStatus = "Operable";
		r2.m_sVehicleDesc = "Second-in-command vehicle station with backup tactical radio harness.";
		r2.m_sPrimaryWeapon = "M16A2 Carbine";
		r2.m_sPrimaryCaliber = "5.56x45mm NATO";
		r2.m_sPrimaryAmmo = "6x 30rnd";
		r2.m_aPrimaryAttachments.Insert("Iron Sights");
		r2.m_aPrimaryAttachments.Insert("Standard Sling");
		r2.m_sSidearm = "M9 Beretta 9x19mm";
		r2.m_sSidearmAmmo = "3x 15rnd";
		r2.m_sHelmetVest = "PASGT Helmet & Vest";
		r2.m_sHelmetVestRating = "Level III-A";
		r2.m_sRig = "ALICE LBE Rig";
		r2.m_sRigLoad = "Load: 17.1 kg";
		r2.m_sRadio = "AN/PRC-68 Squad Radio";
		r2.m_sRadioFreq = "UHF Short-Range - Ch 02";
		r2.m_sRadioStatus = "Standby";
		r2.m_aInventoryItems.Insert("Binoculars M22");
		r2.m_aInventoryItems.Insert("Lensatic Compass");
		r2.m_aInventoryItems.Insert("2x M18 Purple Smoke");
		r2.m_aInventoryItems.Insert("1x M67 Frag Grenade");
		r2.m_sIfakStatus = "Complete";
		r2.m_aIfakPills.Insert("Tourniquet");
		r2.m_aIfakPills.Insert("Field Bandages (4x)");
		r2.m_aIfakPills.Insert("Morphine (1x)");
		r2.m_sAiStatus = "AI";
		r2.m_sClaimStatus = "Restricted";
		r2.m_bIsClaimable = false;
		sq1.m_aRoles.Insert(r2);

		// Role 3: Senior Medical Specialist
		TBD_MockRoleLoadout r3 = new TBD_MockRoleLoadout();
		r3.m_sKey = "blufor:hq:medic";
		r3.m_sRoleTitle = "Senior Medical Specialist";
		r3.m_sRoleCategory = "MED";
		r3.m_sClearance = "Medical Authority";
		r3.m_sRank = "Staff Sergeant (E-6)";
		r3.m_sSquadAssignment = "Assigned to: A1-1 Company HQ";
		r3.m_sVehicleName = "M113A3 MEV";
		r3.m_sVehicleStatus = "Operable";
		r3.m_sVehicleDesc = "Direct operation of casualty collection stations inside the MEV.";
		r3.m_sPrimaryWeapon = "M16A2 Rifle";
		r3.m_sPrimaryCaliber = "5.56x45mm NATO";
		r3.m_sPrimaryAmmo = "5x 30rnd";
		r3.m_sSidearm = "M9 Beretta 9x19mm";
		r3.m_sSidearmAmmo = "2x 15rnd";
		r3.m_sHelmetVest = "PASGT Helmet & Vest (Red Cross)";
		r3.m_sHelmetVestRating = "Level III-A";
		r3.m_sRig = "Medical Aid Bag Rig";
		r3.m_sRigLoad = "Load: 21.2 kg";
		r3.m_sRadio = "AN/PRC-68 Squad Radio";
		r3.m_sRadioFreq = "UHF Short-Range - Ch 03 / Med Net";
		r3.m_sRadioStatus = "Standby";
		r3.m_aInventoryItems.Insert("Surgical Kit");
		r3.m_aInventoryItems.Insert("Saline IV (4x)");
		r3.m_aInventoryItems.Insert("Plasma Bags (2x)");
		r3.m_aInventoryItems.Insert("4x M18 Red Smoke");
		r3.m_sIfakStatus = "Complete";
		r3.m_aIfakPills.Insert("Tourniquet (4x)");
		r3.m_aIfakPills.Insert("QuikClot (4x)");
		r3.m_aIfakPills.Insert("Field Bandages (12x)");
		r3.m_aIfakPills.Insert("Morphine (6x)");
		r3.m_sAiStatus = "AI";
		r3.m_sClaimStatus = "Restricted";
		r3.m_bIsClaimable = false;
		sq1.m_aRoles.Insert(r3);

		blufor.m_aSquads.Insert(sq1);

		// Squad 2: A1-2 M60A1 Patton (Armor Section #1)
		TBD_MockSquad sq2 = new TBD_MockSquad();
		sq2.m_sCallsign = "A1-2 M60A1 Patton";
		sq2.m_sVehicleSummary = "- Main Battle Tank";
		sq2.m_sCategory = "ARMOR LEAD";
		sq2.m_sUnitType = "ARMOR";

		TBD_MockRoleLoadout r4 = new TBD_MockRoleLoadout();
		r4.m_sKey = "blufor:armor1:tc";
		r4.m_sRoleTitle = "Tank Commander";
		r4.m_sRoleCategory = "OFFICER";
		r4.m_sClearance = "Armor Platoon Command";
		r4.m_sRank = "1st Lieutenant (O-2)";
		r4.m_sSquadAssignment = "Assigned to: A1-2 M60A1 Patton";
		r4.m_sVehicleName = "M60A1 Patton MBT";
		r4.m_sVehicleStatus = "Operable";
		r4.m_sVehicleDesc = "Main battle tank with 105mm M68 rifled gun and commander cupola .50 cal.";
		r4.m_sPrimaryWeapon = "M3A1 Grease Gun";
		r4.m_sPrimaryCaliber = ".45 ACP";
		r4.m_sPrimaryAmmo = "4x 30rnd";
		r4.m_sSidearm = "M9 Beretta 9x19mm";
		r4.m_sSidearmAmmo = "3x 15rnd";
		r4.m_sHelmetVest = "DH-132 Tanker Helmet & CVC Vest";
		r4.m_sHelmetVestRating = "Level II";
		r4.m_sRig = "Tanker Coveralls";
		r4.m_sRigLoad = "Load: 12.5 kg";
		r4.m_sRadio = "VIC-1 Vehicle Intercom";
		r4.m_sRadioFreq = "VHF Net - Ch 04 / Armor Lead";
		r4.m_sRadioStatus = "Active Link";
		r4.m_aInventoryItems.Insert("Tank Periscope Tool");
		r4.m_aInventoryItems.Insert("Range Card");
		r4.m_aInventoryItems.Insert("2x M18 Violet Smoke");
		r4.m_aInventoryItems.Insert("Signal Flares");
		r4.m_sIfakStatus = "Complete";
		r4.m_aIfakPills.Insert("Tourniquet");
		r4.m_aIfakPills.Insert("Bandages (2x)");
		r4.m_sAiStatus = "AI";
		r4.m_sClaimStatus = "Restricted";
		sq2.m_aRoles.Insert(r4);

		TBD_MockRoleLoadout r5 = new TBD_MockRoleLoadout();
		r5.m_sKey = "blufor:armor1:driver";
		r5.m_sRoleTitle = "Driver";
		r5.m_sRoleCategory = "ENG";
		r5.m_sClearance = "Vehicle Mechanics";
		r5.m_sRank = "Specialist (E-4)";
		r5.m_sSquadAssignment = "Assigned to: A1-2 M60A1 Patton";
		r5.m_sVehicleName = "M60A1 Patton MBT";
		r5.m_sVehicleStatus = "Operable";
		r5.m_sVehicleDesc = "Certified for track tensioning, powerplant emergency overhaul and field repair.";
		r5.m_sPrimaryWeapon = "M3A1 Grease Gun";
		r5.m_sPrimaryCaliber = ".45 ACP";
		r5.m_sPrimaryAmmo = "3x 30rnd";
		r5.m_sSidearm = "M9 Beretta";
		r5.m_sSidearmAmmo = "2x 15rnd";
		r5.m_sHelmetVest = "DH-132 Tanker Helmet";
		r5.m_sHelmetVestRating = "Level II";
		r5.m_sRig = "Vehicle Tool Rig";
		r5.m_sRigLoad = "Load: 19.8 kg";
		r5.m_sRadio = "VIC-1 Intercom Station";
		r5.m_sRadioFreq = "Internal Crew Intercom";
		r5.m_sRadioStatus = "Connected";
		r5.m_aInventoryItems.Insert("Heavy Tool Kit");
		r5.m_aInventoryItems.Insert("Track Pin Set");
		r5.m_aInventoryItems.Insert("Extinguisher");
		r5.m_aInventoryItems.Insert("Smoke Grenade");
		r5.m_sIfakStatus = "Complete";
		r5.m_aIfakPills.Insert("Tourniquet");
		r5.m_aIfakPills.Insert("Field Bandage");
		r5.m_sAiStatus = "AI";
		r5.m_sClaimStatus = "Restricted";
		sq2.m_aRoles.Insert(r5);

		TBD_MockRoleLoadout r6 = new TBD_MockRoleLoadout();
		r6.m_sKey = "blufor:armor1:gunner";
		r6.m_sRoleTitle = "Gunner";
		r6.m_sRoleCategory = "CREW";
		r6.m_sClearance = "Weapons Station";
		r6.m_sRank = "Corporal (E-4)";
		r6.m_sSquadAssignment = "Assigned to: A1-2 M60A1 Patton";
		r6.m_sVehicleName = "M60A1 Patton MBT";
		r6.m_sVehicleStatus = "Operable";
		r6.m_sVehicleDesc = "105mm ballistic computer and coincidence rangefinder operation.";
		r6.m_sPrimaryWeapon = "M3A1 Grease Gun";
		r6.m_sPrimaryCaliber = ".45 ACP";
		r6.m_sPrimaryAmmo = "3x 30rnd";
		r6.m_sSidearm = "M9 Beretta";
		r6.m_sSidearmAmmo = "2x 15rnd";
		r6.m_sHelmetVest = "DH-132 Tanker Helmet";
		r6.m_sHelmetVestRating = "Level II";
		r6.m_sRig = "Tanker Coveralls";
		r6.m_sRigLoad = "Load: 11.2 kg";
		r6.m_sRadio = "VIC-1 Intercom Station";
		r6.m_sRadioFreq = "Internal Crew Intercom";
		r6.m_sRadioStatus = "Connected";
		r6.m_aInventoryItems.Insert("Boresight Device");
		r6.m_aInventoryItems.Insert("Optic Wipe Cloth");
		r6.m_aInventoryItems.Insert("Thermal Sight Manual");
		r6.m_aInventoryItems.Insert("Smoke Grenade");
		r6.m_sIfakStatus = "Complete";
		r6.m_aIfakPills.Insert("Tourniquet");
		r6.m_aIfakPills.Insert("Field Bandage");
		r6.m_sAiStatus = "AI";
		r6.m_sClaimStatus = "Restricted";
		sq2.m_aRoles.Insert(r6);

		blufor.m_aSquads.Insert(sq2);

		// Squad 3: A1-3 M60A1 Patton
		TBD_MockSquad sq3 = new TBD_MockSquad();
		sq3.m_sCallsign = "A1-3 M60A1 Patton";
		sq3.m_sVehicleSummary = "- Main Battle Tank";
		sq3.m_sCategory = "ARMOR WING";
		sq3.m_sUnitType = "ARMOR";

		TBD_MockRoleLoadout r7 = new TBD_MockRoleLoadout();
		r7.m_sKey = "blufor:armor2:tc";
		r7.m_sRoleTitle = "Tank Commander";
		r7.m_sRoleCategory = "NCO";
		r7.m_sClearance = "Section Command";
		r7.m_sRank = "Staff Sergeant (E-6)";
		r7.m_sSquadAssignment = "Assigned to: A1-3 M60A1 Patton";
		r7.m_sVehicleName = "M60A1 Patton MBT";
		r7.m_sVehicleStatus = "Operable";
		r7.m_sVehicleDesc = "Wing tank in 1st Platoon.";
		r7.m_sPrimaryWeapon = "M3A1 Grease Gun";
		r7.m_sPrimaryCaliber = ".45 ACP";
		r7.m_sPrimaryAmmo = "4x 30rnd";
		r7.m_sSidearm = "M9 Beretta";
		r7.m_sSidearmAmmo = "2x 15rnd";
		r7.m_sHelmetVest = "DH-132 Tanker Helmet";
		r7.m_sHelmetVestRating = "Level II";
		r7.m_sRig = "Tanker Coveralls";
		r7.m_sRigLoad = "Load: 12.0 kg";
		r7.m_sRadio = "VIC-1 Vehicle Intercom";
		r7.m_sRadioFreq = "VHF Net - Ch 04";
		r7.m_sRadioStatus = "Active Link";
		r7.m_aInventoryItems.Insert("Binoculars");
		r7.m_aInventoryItems.Insert("Compass");
		r7.m_aInventoryItems.Insert("Smoke Grenade");
		r7.m_aInventoryItems.Insert("Flare Gun");
		r7.m_sIfakStatus = "Complete";
		r7.m_aIfakPills.Insert("Tourniquet");
		r7.m_aIfakPills.Insert("Field Bandages");
		r7.m_sAiStatus = "AI";
		r7.m_sClaimStatus = "Restricted";
		sq3.m_aRoles.Insert(r7);

		TBD_MockRoleLoadout r8 = new TBD_MockRoleLoadout();
		r8.m_sKey = "blufor:armor2:driver";
		r8.m_sRoleTitle = "Driver";
		r8.m_sRoleCategory = "ENG";
		r8.m_sClearance = "Vehicle Mechanics";
		r8.m_sRank = "Private First Class (E-3)";
		r8.m_sSquadAssignment = "Assigned to: A1-3 M60A1 Patton";
		r8.m_sVehicleName = "M60A1 Patton MBT";
		r8.m_sVehicleStatus = "Operable";
		r8.m_sVehicleDesc = "Field certified driver.";
		r8.m_sPrimaryWeapon = "M3A1 Grease Gun";
		r8.m_sPrimaryCaliber = ".45 ACP";
		r8.m_sPrimaryAmmo = "3x 30rnd";
		r8.m_sSidearm = "M9 Beretta";
		r8.m_sSidearmAmmo = "2x 15rnd";
		r8.m_sHelmetVest = "DH-132 Tanker Helmet";
		r8.m_sHelmetVestRating = "Level II";
		r8.m_sRig = "Vehicle Tool Rig";
		r8.m_sRigLoad = "Load: 18.0 kg";
		r8.m_sRadio = "VIC-1 Intercom";
		r8.m_sRadioFreq = "Internal Intercom";
		r8.m_sRadioStatus = "Connected";
		r8.m_aInventoryItems.Insert("Tool Kit");
		r8.m_aInventoryItems.Insert("Extinguisher");
		r8.m_aInventoryItems.Insert("Smoke Grenade");
		r8.m_aInventoryItems.Insert("Flashlight");
		r8.m_sIfakStatus = "Complete";
		r8.m_aIfakPills.Insert("Tourniquet");
		r8.m_aIfakPills.Insert("Field Bandage");
		r8.m_sAiStatus = "AI";
		r8.m_sClaimStatus = "Restricted";
		sq3.m_aRoles.Insert(r8);

		TBD_MockRoleLoadout r9 = new TBD_MockRoleLoadout();
		r9.m_sKey = "blufor:armor2:gunner";
		r9.m_sRoleTitle = "Gunner";
		r9.m_sRoleCategory = "CREW";
		r9.m_sClearance = "Weapons Station";
		r9.m_sRank = "Specialist (E-4)";
		r9.m_sSquadAssignment = "Assigned to: A1-3 M60A1 Patton";
		r9.m_sVehicleName = "M60A1 Patton MBT";
		r9.m_sVehicleStatus = "Operable";
		r9.m_sVehicleDesc = "Main gunner station.";
		r9.m_sPrimaryWeapon = "M3A1 Grease Gun";
		r9.m_sPrimaryCaliber = ".45 ACP";
		r9.m_sPrimaryAmmo = "3x 30rnd";
		r9.m_sSidearm = "M9 Beretta";
		r9.m_sSidearmAmmo = "2x 15rnd";
		r9.m_sHelmetVest = "DH-132 Tanker Helmet";
		r9.m_sHelmetVestRating = "Level II";
		r9.m_sRig = "Tanker Coveralls";
		r9.m_sRigLoad = "Load: 11.5 kg";
		r9.m_sRadio = "VIC-1 Intercom";
		r9.m_sRadioFreq = "Internal Intercom";
		r9.m_sRadioStatus = "Connected";
		r9.m_aInventoryItems.Insert("Range Card");
		r9.m_aInventoryItems.Insert("Optic Cloth");
		r9.m_aInventoryItems.Insert("Smoke Grenade");
		r9.m_aInventoryItems.Insert("Spare Batteries");
		r9.m_sIfakStatus = "Complete";
		r9.m_aIfakPills.Insert("Tourniquet");
		r9.m_aIfakPills.Insert("Field Bandage");
		r9.m_sAiStatus = "AI";
		r9.m_sClaimStatus = "Restricted";
		sq3.m_aRoles.Insert(r9);

		blufor.m_aSquads.Insert(sq3);

		// Squad 4: A1-4 M60A1 Patton
		TBD_MockSquad sq4 = new TBD_MockSquad();
		sq4.m_sCallsign = "A1-4 M60A1 Patton";
		sq4.m_sVehicleSummary = "- Main Battle Tank";
		sq4.m_sCategory = "ARMOR RESERVE";
		sq4.m_sUnitType = "ARMOR";

		TBD_MockRoleLoadout r10 = new TBD_MockRoleLoadout();
		r10.m_sKey = "blufor:armor3:tc";
		r10.m_sRoleTitle = "Tank Commander";
		r10.m_sRoleCategory = "NCO";
		r10.m_sClearance = "Section Command";
		r10.m_sRank = "Sergeant (E-5)";
		r10.m_sSquadAssignment = "Assigned to: A1-4 M60A1 Patton";
		r10.m_sVehicleName = "M60A1 Patton MBT";
		r10.m_sVehicleStatus = "Operable";
		r10.m_sVehicleDesc = "Reserve section tank.";
		r10.m_sPrimaryWeapon = "M3A1 Grease Gun";
		r10.m_sPrimaryCaliber = ".45 ACP";
		r10.m_sPrimaryAmmo = "4x 30rnd";
		r10.m_sSidearm = "M9 Beretta";
		r10.m_sSidearmAmmo = "2x 15rnd";
		r10.m_sHelmetVest = "DH-132 Tanker Helmet";
		r10.m_sHelmetVestRating = "Level II";
		r10.m_sRig = "Tanker Coveralls";
		r10.m_sRigLoad = "Load: 12.0 kg";
		r10.m_sRadio = "VIC-1 Vehicle Intercom";
		r10.m_sRadioFreq = "VHF Net - Ch 04";
		r10.m_sRadioStatus = "Active Link";
		r10.m_aInventoryItems.Insert("Binoculars");
		r10.m_aInventoryItems.Insert("Compass");
		r10.m_aInventoryItems.Insert("Smoke Grenade");
		r10.m_aInventoryItems.Insert("Earplugs");
		r10.m_sIfakStatus = "Complete";
		r10.m_aIfakPills.Insert("Tourniquet");
		r10.m_aIfakPills.Insert("Field Bandages");
		r10.m_sAiStatus = "AI";
		r10.m_sClaimStatus = "Restricted";
		sq4.m_aRoles.Insert(r10);

		TBD_MockRoleLoadout r11 = new TBD_MockRoleLoadout();
		r11.m_sKey = "blufor:armor3:driver";
		r11.m_sRoleTitle = "Driver";
		r11.m_sRoleCategory = "ENG";
		r11.m_sClearance = "Vehicle Mechanics";
		r11.m_sRank = "Specialist (E-4)";
		r11.m_sSquadAssignment = "Assigned to: A1-4 M60A1 Patton";
		r11.m_sVehicleName = "M60A1 Patton MBT";
		r11.m_sVehicleStatus = "Operable";
		r11.m_sVehicleDesc = "Trained driver.";
		r11.m_sPrimaryWeapon = "M3A1 Grease Gun";
		r11.m_sPrimaryCaliber = ".45 ACP";
		r11.m_sPrimaryAmmo = "3x 30rnd";
		r11.m_sSidearm = "M9 Beretta";
		r11.m_sSidearmAmmo = "2x 15rnd";
		r11.m_sHelmetVest = "DH-132 Tanker Helmet";
		r11.m_sHelmetVestRating = "Level II";
		r11.m_sRig = "Vehicle Tool Rig";
		r11.m_sRigLoad = "Load: 17.5 kg";
		r11.m_sRadio = "VIC-1 Intercom";
		r11.m_sRadioFreq = "Internal Intercom";
		r11.m_sRadioStatus = "Connected";
		r11.m_aInventoryItems.Insert("Tool Kit");
		r11.m_aInventoryItems.Insert("Extinguisher");
		r11.m_aInventoryItems.Insert("Smoke Grenade");
		r11.m_aInventoryItems.Insert("Wrench");
		r11.m_sIfakStatus = "Complete";
		r11.m_aIfakPills.Insert("Tourniquet");
		r11.m_aIfakPills.Insert("Field Bandage");
		r11.m_sAiStatus = "AI";
		r11.m_sClaimStatus = "Restricted";
		sq4.m_aRoles.Insert(r11);

		blufor.m_aSquads.Insert(sq4);
		factions.Insert(blufor);

		// == OPFOR ==
		TBD_MockFaction opfor = new TBD_MockFaction();
		opfor.m_sKey = "OPFOR";
		opfor.m_sName = "OPFOR";
		opfor.m_iClaimed = 0;
		opfor.m_iTotalSlots = 95;
		opfor.m_iColor = 0xFFF87171; // Red

		TBD_MockSquad opSq1 = new TBD_MockSquad();
		opSq1.m_sCallsign = "101-1 Command HQ";
		opSq1.m_sVehicleSummary = "- BTR-70";
		opSq1.m_sCategory = "COMMAND & RECON";
		opSq1.m_sUnitType = "HQ";
		opSq1.m_bIsCommand = true;

		TBD_MockRoleLoadout opR1 = new TBD_MockRoleLoadout();
		opR1.m_sKey = "opfor:hq:commander";
		opR1.m_sRoleTitle = "Battalion Commander";
		opR1.m_sRoleCategory = "OFFICER";
		opR1.m_sClearance = "Supreme Command";
		opR1.m_sRank = "Major (OF-3)";
		opR1.m_sSquadAssignment = "Assigned to: 101-1 Command HQ";
		opR1.m_sVehicleName = "BTR-70 Command Variant";
		opR1.m_sVehicleStatus = "Operable";
		opR1.m_sVehicleDesc = "Armored command personnel carrier equipped with high-power VHF radio mast.";
		opR1.m_sPrimaryWeapon = "AK-74N Rifle";
		opR1.m_sPrimaryCaliber = "5.45x39mm Soviet";
		opR1.m_sPrimaryAmmo = "6x 30rnd";
		opR1.m_aPrimaryAttachments.Insert("1P29 Sight");
		opR1.m_aPrimaryAttachments.Insert("Canvas Sling");
		opR1.m_sSidearm = "Makarov PM 9x18mm";
		opR1.m_sSidearmAmmo = "3x 8rnd";
		opR1.m_sHelmetVest = "SSh-68 Helmet & 6B2 Vest";
		opR1.m_sHelmetVestRating = "GOST 2";
		opR1.m_sRig = "Lifchik Chest Rig";
		opR1.m_sRigLoad = "Load: 17.8 kg";
		opR1.m_sRadio = "R-107M Backpack Radio";
		opR1.m_sRadioFreq = "VHF Net - Ch 10 / 42.00 MHz";
		opR1.m_sRadioStatus = "Active Link";
		opR1.m_aInventoryItems.Insert("B-8x30 Binoculars");
		opR1.m_aInventoryItems.Insert("Adrianov Compass");
		opR1.m_aInventoryItems.Insert("2x RDG-2 Smoke (White)");
		opR1.m_aInventoryItems.Insert("2x RGD-5 Grenade");
		opR1.m_sIfakStatus = "Complete";
		opR1.m_aIfakPills.Insert("Esmarch Tourniquet");
		opR1.m_aIfakPills.Insert("Individual Dressing (4x)");
		opR1.m_aIfakPills.Insert("Promedol (2x)");
		opR1.m_sAiStatus = "AI Ready";
		opR1.m_sClaimStatus = "Claimable";
		opR1.m_bIsClaimable = true;
		opSq1.m_aRoles.Insert(opR1);

		opfor.m_aSquads.Insert(opSq1);
		factions.Insert(opfor);

		return factions;
	}

	static ref array<ref TBD_MockVoiceChannel> GetMockVoiceChannels()
	{
		array<ref TBD_MockVoiceChannel> channels = {};

		// Channel 1: In-Game Net
		TBD_MockVoiceChannel ch1 = new TBD_MockVoiceChannel("Arma 3 Comms Net", 1, 32, false, false);
		ch1.m_aMembers.Insert(new TBD_MockVoiceMember("Ghost-Actual", "SL", false, false, true));
		channels.Insert(ch1);

		// Channel 2: HQ & Command Planning (Active & Expanded in Mockup)
		TBD_MockVoiceChannel ch2 = new TBD_MockVoiceChannel("HQ & Command Planning", 3, 6, true, true);
		ch2.m_aMembers.Insert(new TBD_MockVoiceMember("Mission Maker (You)", "HOST", true, false, true));
		ch2.m_aMembers.Insert(new TBD_MockVoiceMember("Viper-1 (XO)", "2IC", false, false, true));
		ch2.m_aMembers.Insert(new TBD_MockVoiceMember("Doc_Hawkins", "MEDIC", false, true, true));
		channels.Insert(ch2);

		// Channel 3: Armor Platoon Net
		TBD_MockVoiceChannel ch3 = new TBD_MockVoiceChannel("Armor Platoon Net", 4, 8, false, false);
		ch3.m_aMembers.Insert(new TBD_MockVoiceMember("Tanker_Ace", "A1-2 TC", true, false, true));
		ch3.m_aMembers.Insert(new TBD_MockVoiceMember("SteelTrack", "DRIVER", false, false, true));
		ch3.m_aMembers.Insert(new TBD_MockVoiceMember("SabotLoader", "GUNNER", false, false, true));
		ch3.m_aMembers.Insert(new TBD_MockVoiceMember("IronClad_9", "A1-3 TC", false, true, false));
		channels.Insert(ch3);

		// Channel 4: Common Room
		TBD_MockVoiceChannel ch4 = new TBD_MockVoiceChannel("Lobby / Common Room", 14, 64, false, false);
		channels.Insert(ch4);

		return channels;
	}
}
