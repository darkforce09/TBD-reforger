//! T-936.2 - HUD markers for assigned tasks.
//!
//! One marker per assigned task, drawn through the existing placed-custom marker path
//! (`TBD_MarkerIcons.Resolve` + `SCR_MapMarkerManagerComponent.InsertStaticMarker`). Hidden the
//! moment the task leaves `assigned` (succeeded or failed): the server snapshot simply omits it.
//!
//! Transport hangs off SCR_PlayerController for the same reason markers and radio do: Owner RPC
//! delivers to one client. Tasks are not side-scoped on the wire, so every seated player gets the
//! same assigned snapshot.
//! @contract mission.schema.json#/$defs/task

//------------------------------------------------------------------------------------------------
class TBD_TaskHud
{
	static const string CH = "TaskHud";

	protected static ref array<ref SCR_MapMarkerBase> s_aApplied;
	protected static string s_sLastSignature;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		ClearApplied();
		s_sLastSignature = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Server: push the current assigned snapshot to every connected player.
	static void PushToPlayers()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> ids = {};
		int count = players.GetPlayers(ids);
		if (count == 0)
			return;

		array<int> xs;
		array<int> zs;
		array<string> icons;
		array<string> labels;
		array<string> idsOut;
		array<string> states;
		BuildSnapshot(xs, zs, icons, labels, idsOut, states);

		for (int i = 0; i < count; i++)
		{
			SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(ids[i]));
			if (controller)
				controller.TBD_PushTaskHud(xs, zs, icons, labels, idsOut, states);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Client (and listen-host): ask the authority for the current snapshot.
	static void RequestLocal()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		pc.TBD_RequestTaskHud();
	}

	//------------------------------------------------------------------------------------------------
	//! Apply one snapshot. Assigned tasks with a position become markers; everything else is hidden
	//! by not being in the snapshot.
	static void Accept(array<int> xs, array<int> zs, array<string> icons, array<string> labels,
		array<string> ids, array<string> states)
	{
		string signature = Signature(ids, states, xs, zs);
		if (signature == s_sLastSignature)
			return;

		s_sLastSignature = signature;
		ClearApplied();

		SCR_MapMarkerManagerComponent mgr = TBD_MarkerClient.FindMarkerManager();
		if (!mgr)
		{
			TBD_Log.Warn(CH, "no SCR_MapMarkerManagerComponent - assigned task markers cannot be drawn");
			return;
		}

		if (!xs)
			return;

		EnsureApplied();

		int count = xs.Count();
		for (int i = 0; i < count; i++)
		{
			if (!zs.IsIndexValid(i) || !icons.IsIndexValid(i) || !labels.IsIndexValid(i))
				break;

			bool recognised;
			int iconEntry = TBD_MarkerIcons.Resolve(icons[i], recognised);
			if (!recognised)
				TBD_MarkerIcons.ReportUnknown(icons[i]);

			iconEntry = TBD_MarkerIcons.ClampToLoadedConfig(iconEntry);

			SCR_MapMarkerBase marker = new SCR_MapMarkerBase();
			marker.SetType(SCR_EMapMarkerType.PLACED_CUSTOM);
			marker.SetWorldPos(xs[i], zs[i]);
			marker.SetIconEntry(iconEntry);
			marker.SetColorEntry(TBD_MarkerIcons.MARKER_COLOR);
			marker.SetCustomText(labels[i]);
			marker.SetCanBeRemovedByOwner(false);
			mgr.InsertStaticMarker(marker, true);
			s_aApplied.Insert(marker);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Fill the parallel arrays the RPC carries. Only ASSIGNED tasks with a world position.
	static void BuildSnapshot(out array<int> xs, out array<int> zs, out array<string> icons,
		out array<string> labels, out array<string> ids, out array<string> states)
	{
		xs = new array<int>();
		zs = new array<int>();
		icons = new array<string>();
		labels = new array<string>();
		ids = new array<string>();
		states = new array<string>();

		array<ref TBD_Task> tasks = TBD_TaskStateMachine.GetAll();
		if (!tasks)
			return;

		foreach (TBD_Task task : tasks)
		{
			if (!task)
				continue;

			if (task.m_eState != TBD_ETaskState.ASSIGNED)
				continue;

			if (!task.m_bHasPosition)
				continue;

			xs.Insert(task.m_iWorldX);
			zs.Insert(task.m_iWorldZ);
			icons.Insert(TBD_TaskStateMachine.IconFor(task));
			labels.Insert(task.m_sTitle);
			ids.Insert(task.m_sId);
			states.Insert(TBD_TaskStateMachine.StateName(task.m_eState));
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static string Signature(array<string> ids, array<string> states, array<int> xs, array<int> zs)
	{
		string sig;
		if (!ids)
			return sig;

		int count = ids.Count();
		for (int i = 0; i < count; i++)
		{
			sig += ids[i];
			sig += ":";
			if (states && states.IsIndexValid(i))
				sig += states[i];
			sig += ":";
			if (xs && xs.IsIndexValid(i))
				sig += xs[i].ToString();
			sig += ",";
			if (zs && zs.IsIndexValid(i))
				sig += zs[i].ToString();
			sig += ";";
		}

		return sig;
	}

	//------------------------------------------------------------------------------------------------
	protected static void ClearApplied()
	{
		if (!s_aApplied || s_aApplied.IsEmpty())
		{
			EnsureApplied();
			return;
		}

		SCR_MapMarkerManagerComponent mgr = TBD_MarkerClient.FindMarkerManager();
		foreach (SCR_MapMarkerBase marker : s_aApplied)
		{
			if (!marker)
				continue;

			if (mgr)
				mgr.RemoveStaticMarker(marker);
		}

		s_aApplied.Clear();
	}

	//------------------------------------------------------------------------------------------------
	protected static void EnsureApplied()
	{
		if (!s_aApplied)
			s_aApplied = new array<ref SCR_MapMarkerBase>();
	}
}

//------------------------------------------------------------------------------------------------
modded class SCR_PlayerController
{
	//------------------------------------------------------------------------------------------------
	void TBD_RequestTaskHud()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_TaskHud);
			return;
		}

		array<int> xs;
		array<int> zs;
		array<string> icons;
		array<string> labels;
		array<string> ids;
		array<string> states;
		TBD_TaskHud.BuildSnapshot(xs, zs, icons, labels, ids, states);
		TBD_TaskHud.Accept(xs, zs, icons, labels, ids, states);
	}

	//------------------------------------------------------------------------------------------------
	void TBD_PushTaskHud(array<int> xs, array<int> zs, array<string> icons, array<string> labels,
		array<string> ids, array<string> states)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (GetGame().GetPlayerController() == this)
		{
			TBD_TaskHud.Accept(xs, zs, icons, labels, ids, states);
			return;
		}

		Rpc(TBD_RpcDo_TaskHud, xs, zs, icons, labels, ids, states);
	}

	//------------------------------------------------------------------------------------------------
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_TaskHud()
	{
		array<int> xs;
		array<int> zs;
		array<string> icons;
		array<string> labels;
		array<string> ids;
		array<string> states;
		TBD_TaskHud.BuildSnapshot(xs, zs, icons, labels, ids, states);
		Rpc(TBD_RpcDo_TaskHud, xs, zs, icons, labels, ids, states);
	}

	//------------------------------------------------------------------------------------------------
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_TaskHud(array<int> xs, array<int> zs, array<string> icons,
		array<string> labels, array<string> ids, array<string> states)
	{
		TBD_TaskHud.Accept(xs, zs, icons, labels, ids, states);
	}
}
