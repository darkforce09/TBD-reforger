[WorkbenchPluginAttribute(name: "Export Equipment and Vehicles", description: "Capture equipment, vehicles and gameplay dependencies in a source-only generation", category: "TBD", wbModules: {"ResourceManager", "WorldEditor"})]
class TBD_EquipmentVehicleExportPlugin : WorkbenchPlugin
{
	override void Run()
	{
		if (TBD_SourceExportGeneration.s_Active && !TBD_SourceExportGeneration.s_Active.m_bFinished)
		{
			Print("[TBD Source Export] An export is already running", LogLevel.ERROR);
			return;
		}
		TBD_SourceExportGeneration.s_Active = new TBD_SourceExportGeneration();
		TBD_SourceExportGeneration generation = TBD_SourceExportGeneration.s_Active;
		if (generation.Start()) while (generation.Step()) {}
		Print("[TBD Source Export] Generation: " + generation.m_sDirectory);
	}
}

// Specialist actions select diagnostic seeds and use the same extraction and dependency pipeline.
class TBD_SourceDiagnosticExport
{
	static void Run(string domain, array<string> nativeTypes, string folder = "")
	{
		if (TBD_SourceExportGeneration.s_Active && !TBD_SourceExportGeneration.s_Active.m_bFinished)
		{ Print("[TBD Source Export] An export is already running", LogLevel.ERROR); return; }
		TBD_SourceResourceDiscovery discovery = new TBD_SourceResourceDiscovery();
		discovery.Scan();
		array<string> candidates = discovery.m_aEquipment;
		if (domain == "vehicle") candidates = discovery.m_aVehicles;
		array<string> selected = {};
		foreach (string name : candidates)
		{
			if (!folder.IsEmpty() && !name.Contains(folder)) continue;
			if (!nativeTypes || nativeTypes.IsEmpty()) { selected.Insert(name); continue; }
			Resource resource = Resource.Load(name);
			if (!resource || !resource.GetResource()) { discovery.m_aErrors.Insert("Cannot load diagnostic candidate: " + name); continue; }
			TBD_SourceContainerReader reader = new TBD_SourceContainerReader();
			reader.m_sResource = name;
			reader.Capture(resource.GetResource().ToBaseContainer());
			foreach (string error : reader.m_aErrors) discovery.m_aErrors.Insert(name + ": " + error);
			if (Matches(reader, nativeTypes)) selected.Insert(name);
		}
		TBD_SourceExportGeneration.s_Active = new TBD_SourceExportGeneration();
		TBD_SourceExportGeneration generation = TBD_SourceExportGeneration.s_Active;
		foreach (string failure : discovery.m_aErrors) generation.m_aErrors.Insert(failure);
		if (generation.Start("diagnostic", selected)) while (generation.Step()) {}
		Print("[TBD Source Export] Diagnostic generation: " + generation.m_sDirectory);
	}

	protected static bool Matches(TBD_SourceContainerReader reader, array<string> types)
	{
		foreach (TBD_SourceExportNode node : reader.m_aNodes)
		{
			if (node.m_sView != "effective") continue;
			foreach (string baseType : types)
				if (TBD_SourceCapabilityRules.IsA(node.m_sClass, baseType)) return true;
		}
		return false;
	}
}
