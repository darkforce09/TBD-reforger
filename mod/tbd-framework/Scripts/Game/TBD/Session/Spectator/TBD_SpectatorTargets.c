/**
 * @file TBD_SpectatorTargets.c
 * @brief Who a spectator may watch, grouped by faction and group, and what counts as alive.
 *
 * Role: builds the list of living, followable players this client can see, filtered by the
 * faction restriction and sorted by faction, group and name; counts connected players whose
 * character is not on this machine; answers "is this entity alive" for the spectator.
 * Position: pure data, no widgets, camera or lifecycle; TBD_SpectatorScreen renders the list,
 * TBD_SpectatorTargeting cycles through it, TBD_SpectatorController asks IsAlive of the local
 * entity, TBD_MissionWorldApplier and TBD_SpectatorController.SyncSpectatorPolicy set the
 * restriction.
 * State: the restriction flag (default true, own side only) and the latched viewer faction key
 * (static, client).
 * Invariants: under one life a dead player spectates for the rest of the event while still in
 * their squad's voice channel, so the default shows the own side only; it is a discipline measure,
 * not a security boundary (a modified client can ignore it; the engine's replication range is the
 * hard limit); it fails closed to an empty list when the viewer's faction cannot be resolved; the
 * viewer faction is latched the first time it resolves; a streaming host is never alive.
 */

//! The spectator's target policy and roster builder. Static; client only.
class TBD_SpectatorTargets
{
	protected static bool s_bFactionRestricted = true; //!< own side only; default true
	protected static string s_sViewerFactionKey; //!< the local player's faction, latched once resolved, so a deleted corpse cannot widen the view

	//! Restrict spectators to their own faction or show every side, and log the mode.
	//! @param restricted true for own side only
	static void SetFactionRestricted(bool restricted)
	{
		s_bFactionRestricted = restricted;

		// Enfusion has no ternary operator.
		string mode = "OFF (all sides)";
		if (restricted)
			mode = "ON (own side only)";

		Print(string.Format("[TBD][spectator] faction restriction %1", mode));
	}

	//! @return true when spectators see their own side only
	static bool IsFactionRestricted()
	{
		return s_bFactionRestricted;
	}

	//! Drop the latched faction. Called on mission teardown so a new round starts clean.
	static void Reset()
	{
		s_sViewerFactionKey = string.Empty;
	}

	//! The local player's faction key, latched. Empty only if it has never once resolved.
	static string GetViewerFactionKey()
	{
		if (!s_sViewerFactionKey.IsEmpty())
			return s_sViewerFactionKey;

		int localId = SCR_PlayerController.GetLocalPlayerId();
		if (localId <= 0)
			return string.Empty;

		// Own body first -- it is authoritative and survives death, which is exactly when we need it.
		string key = FactionKeyOf(SCR_PlayerController.GetLocalControlledEntity());

		if (key.IsEmpty())
		{
			// Fallback: the faction manager still remembers an assignment after the body is gone.
			SCR_FactionManager factionManager = SCR_FactionManager.Cast(GetGame().GetFactionManager());
			if (factionManager)
			{
				Faction faction = factionManager.GetPlayerFaction(localId);
				if (faction)
					key = faction.GetFactionKey();
			}
		}

		if (!key.IsEmpty())
			s_sViewerFactionKey = key;

		return key;
	}

	//! Every player this spectator may watch, sorted faction -> group -> name.
	//!
	//! `notInView` reports how many living players were skipped because their entity is not
	//! streamed to this client. That count is not noise -- see the streaming note on
	//! `TBD_SpectatorController`. Showing it is the difference between "nobody else is alive" and
	//! "nobody else is alive *near you*", and the spectator must not be lied to about which.
	static void Collect(notnull array<ref TBD_SpectatorTarget> targets, out int notInView)
	{
		targets.Clear();
		notInView = 0;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		string viewerFaction = GetViewerFactionKey();

		// Fail closed: restriction on and we do not know our own side -> show nothing.
		if (s_bFactionRestricted && viewerFaction.IsEmpty())
			return;

		int localId = SCR_PlayerController.GetLocalPlayerId();

		array<int> ids = {};
		players.GetPlayers(ids);

		SCR_GroupsManagerComponent groups = SCR_GroupsManagerComponent.GetInstance();

		foreach (int playerId : ids)
		{
			if (playerId == localId)
				continue;

			IEntity entity = players.GetPlayerControlledEntity(playerId);
			if (!entity)
			{
				// Connected, but their character is not on this machine: unknown life, side and
				// position, so counted rather than offered. A dead player possessing the
				// server-only streaming host also resolves to null and is counted here; the count
				// errs high ("there may be more out there"), never low, and correcting it would
				// need the server's dead list on every client.
				notInView++;
				continue;
			}

			// A streaming host is not a person; IsAlive refuses it, which keeps a dead player's
			// host out of every roster.
			if (!IsAlive(entity))
				continue;

			string factionKey = FactionKeyOf(entity);
			if (s_bFactionRestricted && factionKey != viewerFaction)
				continue;

			TBD_SpectatorTarget target = new TBD_SpectatorTarget();
			target.m_iPlayerId = playerId;
			target.m_sName = players.GetPlayerName(playerId);
			target.m_sFactionKey = factionKey;
			target.m_sFactionName = FactionNameOf(entity, factionKey);
			target.m_sGroupName = GroupNameOf(groups, playerId);
			target.m_Entity = entity;

			if (target.m_sName.IsEmpty())
				target.m_sName = string.Format("Player %1", playerId);

			targets.Insert(target);
		}

		Sort(targets);
	}

	//! The entity a target resolves to right now, or null if it died / left our range since the
	//! list was built. Every follow goes through this so a stale row can never point the camera at
	//! a corpse.
	static IEntity ResolveLivingEntity(int playerId)
	{
		if (playerId <= 0)
			return null;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return null;

		IEntity entity = players.GetPlayerControlledEntity(playerId);
		if (!entity || !IsAlive(entity))
			return null;

		return entity;
	}

	//! Is this entity alive? A streaming host is never alive, tested first so it holds whatever
	//! components a host prefab carries: otherwise TBD_SpectatorController.Tick would read a
	//! spectator's own host as a living body and leave spectator for good, and other spectators
	//! could follow an invisible host. Then the character controller decides; a damage state not
	//! DESTROYED is the fallback for anything that is not a character (a manned turret); anything
	//! else is alive.
	//! @param entity the entity to test; null is not alive
	//! @return true when alive
	static bool IsAlive(IEntity entity)
	{
		if (!entity)
			return false;

		if (TBD_SpectatorHostEntity.IsHost(entity))
			return false;

		SCR_CharacterControllerComponent controller = SCR_CharacterControllerComponent.Cast(entity.FindComponent(SCR_CharacterControllerComponent));
		if (controller)
			return !controller.IsDead();

		DamageManagerComponent damage = DamageManagerComponent.Cast(entity.FindComponent(DamageManagerComponent));
		if (damage)
			return damage.GetState() != EDamageState.DESTROYED;

		return true;
	}


	//! @param entity a character, or any entity
	//! @return the entity's faction key, or empty when it has none
	protected static string FactionKeyOf(IEntity entity)
	{
		if (!entity)
			return string.Empty;

		SCR_ChimeraCharacter character = SCR_ChimeraCharacter.Cast(entity);
		if (character)
		{
			Faction faction = character.GetFaction();
			if (faction)
				return faction.GetFactionKey();
		}

		FactionAffiliationComponent affiliation = FactionAffiliationComponent.Cast(entity.FindComponent(FactionAffiliationComponent));
		if (affiliation)
		{
			Faction faction = affiliation.GetAffiliatedFaction();
			if (faction)
				return faction.GetFactionKey();
		}

		return string.Empty;
	}

	//! Display name for a faction, falling back to the key so a section heading is never blank.
	protected static string FactionNameOf(IEntity entity, string factionKey)
	{
		SCR_ChimeraCharacter character = SCR_ChimeraCharacter.Cast(entity);
		if (character)
		{
			Faction faction = character.GetFaction();
			if (faction)
			{
				string name = faction.GetFactionName();
				if (!name.IsEmpty())
					return name;
			}
		}

		if (factionKey.IsEmpty())
			return "UNASSIGNED";

		return factionKey;
	}

	//! Group label. `GetCustomName()` is what a squad leader typed; the numeric id is the fallback
	//! so every player still lands under a heading rather than in a flat wall of names.
	protected static string GroupNameOf(SCR_GroupsManagerComponent groups, int playerId)
	{
		if (!groups)
			return "UNGROUPED";

		SCR_AIGroup group = groups.GetPlayerGroup(playerId);
		if (!group)
			return "UNGROUPED";

		string custom = group.GetCustomName();
		if (!custom.IsEmpty())
			return custom;

		return string.Format("GROUP %1", group.GetGroupID());
	}

	//! Insertion sort on faction -> group -> name. A spectator list is tens of rows, not thousands,
	//! and an insertion sort is stable and allocation-free -- which matters because this runs on a
	//! refresh timer for the whole rest of the event.
	protected static void Sort(notnull array<ref TBD_SpectatorTarget> targets)
	{
		for (int i = 1; i < targets.Count(); i++)
		{
			TBD_SpectatorTarget moving = targets[i];
			int j = i - 1;

			while (j >= 0 && Compare(targets[j], moving) > 0)
			{
				targets[j + 1] = targets[j];
				j--;
			}

			targets[j + 1] = moving;
		}
	}

	//! <0 when `a` sorts first.
	protected static int Compare(TBD_SpectatorTarget a, TBD_SpectatorTarget b)
	{
		int byFaction = StringCompare(a.m_sFactionName, b.m_sFactionName);
		if (byFaction != 0)
			return byFaction;

		int byGroup = StringCompare(a.m_sGroupName, b.m_sGroupName);
		if (byGroup != 0)
			return byGroup;

		return StringCompare(a.m_sName, b.m_sName);
	}

	//! Enforce has no string comparison operator that yields an ordering, only equality -- so it is
	//! done by hand, character by character, once, here.
	protected static int StringCompare(string a, string b)
	{
		int lengthA = a.Length();
		int lengthB = b.Length();
		int shared = Math.Min(lengthA, lengthB);

		for (int i = 0; i < shared; i++)
		{
			int codeA = a.Get(i).ToAscii();
			int codeB = b.Get(i).ToAscii();

			if (codeA != codeB)
			{
				if (codeA < codeB)
					return -1;

				return 1;
			}
		}

		if (lengthA == lengthB)
			return 0;

		if (lengthA < lengthB)
			return -1;

		return 1;
	}
}

//! One spectate-able player. A view record, rebuilt on every refresh -- it holds no authority and
//! must never be cached across a refresh, because `m_Entity` can be destroyed under it.
class TBD_SpectatorTarget
{
	int m_iPlayerId; //!< the player id
	string m_sName; //!< the player name
	string m_sFactionKey; //!< faction key, empty when unknown
	string m_sFactionName; //!< faction display name
	string m_sGroupName; //!< group display name
	IEntity m_Entity; //!< weak; re-resolve through TBD_SpectatorTargets.ResolveLivingEntity before a camera points at it
}
