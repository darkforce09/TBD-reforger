// Capability membership follows native class inheritance; filenames never supply facts.
class TBD_SourceCapabilityRules
{
	static bool IsA(string name, string baseName)
	{
		if (name == baseName) return true;
		typename actual = name.ToType();
		typename baseType = baseName.ToType();
		return actual && baseType && actual.IsInherited(baseType);
	}

	static string Capability(string name)
	{
		if (IsA(name, "BaseInventoryStorageComponent") || IsA(name, "InventoryStorageSlot")) return "storage";
		if (IsA(name, "BaseLoadoutClothComponent") || IsA(name, "InventoryItemComponent") || IsA(name, "ItemPhysicalAttributes") || IsA(name, "SCR_ItemAttributeCollection")) return "inventory";
		if (IsA(name, "BaseWeaponComponent") || IsA(name, "WeaponComponent") || IsA(name, "BaseMuzzleComponent") || IsA(name, "MuzzleComponent") || IsA(name, "BaseFireMode")) return "weapon";
		if (IsA(name, "BaseMagazineComponent") || IsA(name, "MagazineComponent") || IsA(name, "BaseMagazineWell")) return "magazine";
		if (IsA(name, "Projectile") || IsA(name, "ProjectileMoveComponent") || IsA(name, "BaseProjectileComponent") || IsA(name, "BallisticTable")) return "projectile";
		if (IsA(name, "BaseSlotComponent") || IsA(name, "AttachmentSlotComponent") || IsA(name, "WeaponAttachmentAttributes") || IsA(name, "BaseAttachmentType") || IsA(name, "SlotManagerComponent")) return "attachment";
		if (IsA(name, "SightsComponent") || IsA(name, "SightsFOVInfo") || IsA(name, "SCR_BinocularsComponent")) return "sights";
		if (IsA(name, "BaseDamageManagerComponent") || IsA(name, "HitZone") || IsA(name, "GameMaterial")) return "protection";
		if (IsA(name, "SCR_ConsumableItemComponent") || IsA(name, "SCR_ConsumableEffectBase") || IsA(name, "SCR_HealingEffect")) return "medical_effects";
		if (IsA(name, "BaseRadioComponent") || IsA(name, "RadioTransceiver") || IsA(name, "SCR_RadioComponent")) return "communications";
		if (IsA(name, "VehicleSimulation") || IsA(name, "VehicleWheeledSimulation") || IsA(name, "VehicleHelicopterSimulation") || IsA(name, "VehicleBoatSimulation")) return "vehicle_systems";
		if (IsA(name, "RigidBody")) return "physics";
		if (IsA(name, "BaseCompartmentManagerComponent") || IsA(name, "BaseCompartmentSlot") || IsA(name, "TurretComponent")) return "vehicle_systems";
		if (IsA(name, "SCR_FuelManagerComponent") || IsA(name, "SCR_FuelConsumptionComponent") || IsA(name, "SCR_VehicleBuoyancyComponent") || IsA(name, "SCR_PowerComponent")) return "vehicle_systems";
		if (IsA(name, "SCR_VehicleSoundComponent") || IsA(name, "SCR_BaseHUDComponent") || IsA(name, "SCR_CarControllerComponent")) return "vehicle_systems";
		if (IsA(name, "SignalsManagerComponent")) return "utility";
		if (IsA(name, "SCR_GadgetComponent") || IsA(name, "BaseTriggerComponent") || IsA(name, "SCR_ResourceComponent") || IsA(name, "BaseExplosiveComponent")) return "utility";
		if (IsA(name, "UIInfo") || IsA(name, "MeshObject") || IsA(name, "PreviewRenderAttributes")) return "visuals";
		return string.Empty;
	}

	static string Field(string property)
	{
		if (property == "Trigger Offset") return "trigger_offset_vector3";
		if (property == "AmmoTemplate") return "default_projectile";
		string output;
		string previous;
		for (int i = 0; i < property.Length(); i++)
		{
			string character = property.Substring(i, 1);
			string lower = character;
			lower.ToLower();
			bool letter = "abcdefghijklmnopqrstuvwxyz0123456789".Contains(lower);
			if (!letter)
			{
				if (!output.IsEmpty() && !output.EndsWith("_")) output += "_";
				previous = character;
				continue;
			}
			bool upper = character != lower;
			string previousLower = previous;
			previousLower.ToLower();
			string next;
			if (i + 1 < property.Length()) next = property.Substring(i + 1, 1);
			string nextLower = next;
			nextLower.ToLower();
			if (upper && !output.IsEmpty() && !output.EndsWith("_") && (previous == previousLower || next == nextLower && !next.IsEmpty())) output += "_";
			output += lower;
			previous = character;
		}
		while (output.EndsWith("_")) output = output.Substring(0, output.Length() - 1);
		if (output.IsEmpty()) output = "property";
		return output;
	}
}

// Relationships are queried for every native type mentioned by a source container or property constraint.
class TBD_SourceTypeHierarchy
{
	ref array<string> m_aTypes = {};
	ref array<string> m_aErrors = {};

	void Add(string name)
	{
		if (!name.IsEmpty() && m_aTypes.Find(name) < 0) m_aTypes.Insert(name);
	}

	string Json()
	{
		TBD_SourceDeclaredAncestors declarations = new TBD_SourceDeclaredAncestors();
		if (!m_aTypes.IsEmpty()) declarations.Expand(this);
		foreach (string error : declarations.m_aErrors) m_aErrors.Insert(error);
		m_aTypes.Sort();
		array<string> entries = {};
		foreach (string name : m_aTypes)
		{
			typename actual = name.ToType();
			array<string> ancestors = {};
			string status = "present";
			string reason = "null";
			if (!actual)
			{
				status = "unavailable";
				reason = TBD_SourceExportJson.Quote("Native container class is not exposed as a script TypeName");
			}
			else
			{
				foreach (string baseName : m_aTypes)
				{
					if (baseName == name) continue;
					typename baseType = baseName.ToType();
					if (baseType && actual.IsInherited(baseType)) ancestors.Insert(baseName);
				}
			}
			string entry = TBD_SourceExportJson.Quote(name) + ":{\"status\":" + TBD_SourceExportJson.Quote(status);
			entry += ",\"ancestor_types\":" + TBD_SourceExportJson.Strings(ancestors) + ",\"reason\":" + reason + "}";
			entries.Insert(entry);
		}
		return "{\"method\":\"TypeName.IsInherited\",\"scope\":\"referenced_types_and_native_ancestors\",\"types\":{" + TBD_SourceExportJson.Join(entries) + "}}";
	}
}

// Script declarations supply ancestor candidates; TypeName verifies every relationship.
class TBD_SourceDeclaredAncestors
{
	ref array<string> m_aErrors = {};
	protected ref map<string, ref array<string>> m_mParents = new map<string, ref array<string>>();
	protected ref array<string> m_aFiles = {};
	protected bool m_bBlockComment;
	protected int m_iState;
	protected string m_sClass;

	void Expand(TBD_SourceTypeHierarchy hierarchy)
	{
		array<string> addons = {};
		GameProject.GetLoadedAddons(addons);
		foreach (string guid : addons)
			FileIO.FindFiles(m_aFiles.Insert, "$" + GameProject.GetAddonID(guid) + ":Scripts/", ".c");
		if (m_aFiles.IsEmpty()) m_aErrors.Insert("Native script declaration enumeration returned no files");
		m_aFiles.Sort();
		foreach (string path : m_aFiles) ReadDeclarations(path);
		for (int index = 0; index < hierarchy.m_aTypes.Count(); index++)
		{
			string name = hierarchy.m_aTypes[index];
			typename actual = name.ToType();
			array<string> candidates = m_mParents.Get(name);
			if (!actual || !candidates) continue;
			foreach (string candidate : candidates)
			{
				typename baseType = candidate.ToType();
				if (baseType && actual.IsInherited(baseType)) hierarchy.Add(candidate);
			}
		}
	}

	protected void ReadDeclarations(string path)
	{
		FileHandle file = FileIO.OpenFile(path, FileMode.READ);
		if (!file) { m_aErrors.Insert("Cannot read a listed native script declaration: " + path); return; }
		m_bBlockComment = false;
		m_iState = 0;
		string line;
		while (file.ReadLine(line) >= 0) ReadLine(line);
		file.Close();
	}

	protected void ReadLine(string line)
	{
		string token;
		bool quoted;
		bool escaped;
		for (int i = 0; i < line.Length(); i++)
		{
			string c = line.Substring(i, 1);
			string next;
			if (i + 1 < line.Length()) next = line.Substring(i + 1, 1);
			if (m_bBlockComment)
			{
				if (c == "*" && next == "/") { m_bBlockComment = false; i++; }
				continue;
			}
			if (quoted)
			{
				if (escaped) escaped = false;
				else if (c == "\\") escaped = true;
				else if (c == "\"") quoted = false;
				continue;
			}
			if (c == "/" && next == "/") break;
			if (c == "/" && next == "*") { Consume(token); token = ""; m_bBlockComment = true; i++; continue; }
			if (c == "\"") { Consume(token); token = ""; quoted = true; continue; }
			string lower = c;
			lower.ToLower();
			if ("abcdefghijklmnopqrstuvwxyz_0123456789".Contains(lower)) token += c;
			else
			{
				Consume(token); token = "";
				if (c == ":") { if (m_iState == 2) m_iState = 3; }
				else if (c == "{" || c == ";") m_iState = 0;
			}
		}
		Consume(token);
	}

	protected void Consume(string token)
	{
		if (token.IsEmpty()) return;
		if (token == "class") { m_iState = 1; return; }
		if (m_iState == 1) { m_sClass = token; m_iState = 2; return; }
		if (m_iState != 3) return;
		array<string> parents = m_mParents.Get(m_sClass);
		if (!parents) { parents = {}; m_mParents.Insert(m_sClass, parents); }
		if (parents.Find(token) < 0) parents.Insert(token);
		m_iState = 0;
	}
}
