//! T-181.12 — who a spectator is allowed to watch, and how that list is grouped.
//!
//! Deliberately pure: no widgets, no camera, no lifecycle. The roster screen renders what this
//! returns and the controller cycles through it, so the policy question ("may I see the enemy?")
//! is answered in exactly one place and can be read without opening a UI file.
//!
//! ── FACTION DISCIPLINE — the decision and why ────────────────────────────────────────────────
//! **Default: own side only.** `s_bFactionRestricted` starts true.
//!
//! TBD events are ONE LIFE, which is precisely what makes this non-negotiable rather than
//! stylistic. In a wave/ticket mode a dead player is out for thirty seconds; here they are out for
//! the rest of the event, so by the back half of an op a large fraction of the server is
//! spectating — and every one of them is still sitting in their squad's voice channel. An
//! unrestricted spectator is therefore not a viewer, it is a live intel feed: one dead man
//! watching the enemy assault form up can hand his side the whole enemy plan for free. That is
//! why milsim groups restrict spectator, and it is why TBD does.
//!
//! **It is configurable and cheap to flip** — `SetFactionRestricted(false)` — because the same
//! framework runs training nights and AARs where watching the other side is the entire point.
//!
//! **Honest about what it is.** This is a *discipline* measure, not a security boundary. It runs
//! on the client, and a client that has been modified can ignore it. The real limit is the
//! engine's own replication range: an entity that was never streamed to you does not exist on your
//! machine and cannot be rendered by any camera, honest or otherwise. This filter is the policy
//! layer on top of that, and it is the layer that stops an *unmodified* client from becoming an
//! intel leak — which is every client in an organised event.
//!
//! **It fails CLOSED.** If the viewer's own faction cannot be resolved while the restriction is
//! on, the list comes back empty with a status line saying so, rather than quietly showing
//! everyone. The cached-faction path below makes that safe in practice: the key is latched the
//! first time it resolves, so a deleted corpse or a mid-round reconnect cannot lose it.
class TBD_SpectatorTargets
{
	//! Own side only. See the class header for the reasoning; flip it for a training night.
	protected static bool s_bFactionRestricted = true;

	//! Latched the first time the local player's faction resolves. A spectator's corpse can be
	//! deleted and their faction lookup can then fail — that must not silently widen what they are
	//! allowed to see, so the answer is remembered rather than re-derived.
	protected static string s_sViewerFactionKey;

	//------------------------------------------------------------------------------------------------
	//! Restrict spectators to their own faction? Default true.
	static void SetFactionRestricted(bool restricted)
	{
		s_bFactionRestricted = restricted;

		// MEASURED: Enfusion has NO ternary operator. `cond ? a : b` fails with
		// "Broken expression (missing ';'?)" — which points at the whole statement and says
		// nothing about `?`, so it is worth knowing rather than rediscovering.
		string mode = "OFF (all sides)";
		if (restricted)
			mode = "ON (own side only)";

		Print(string.Format("[TBD][spectator] faction restriction %1", mode));
	}

	//------------------------------------------------------------------------------------------------
	static bool IsFactionRestricted()
	{
		return s_bFactionRestricted;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop the latched faction. Called on mission teardown so a new round starts clean.
	static void Reset()
	{
		s_sViewerFactionKey = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! The local player's faction key, latched. Empty only if it has never once resolved.
	static string GetViewerFactionKey()
	{
		if (!s_sViewerFactionKey.IsEmpty())
			return s_sViewerFactionKey;

		int localId = SCR_PlayerController.GetLocalPlayerId();
		if (localId <= 0)
			return string.Empty;

		// Own body first — it is authoritative and survives death, which is exactly when we need it.
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

	//------------------------------------------------------------------------------------------------
	//! Every player this spectator may watch, sorted faction -> group -> name.
	//!
	//! `notInView` reports how many living players were skipped because their entity is not
	//! streamed to this client. That count is not noise — see the streaming note on
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
				// Connected, but their character is not on this machine. We cannot know whether
				// they are alive, whose side they are on, or where they are — so we cannot offer
				// them as a target, only count them.
				notInView++;
				continue;
			}

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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Alive, from the character controller. A destroyed damage state is the fallback for anything
	//! that is not a character (a spectator could be watching a manned turret).
	static bool IsAlive(IEntity entity)
	{
		if (!entity)
			return false;

		SCR_CharacterControllerComponent controller = SCR_CharacterControllerComponent.Cast(entity.FindComponent(SCR_CharacterControllerComponent));
		if (controller)
			return !controller.IsDead();

		DamageManagerComponent damage = DamageManagerComponent.Cast(entity.FindComponent(DamageManagerComponent));
		if (damage)
			return damage.GetState() != EDamageState.DESTROYED;

		return true;
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Insertion sort on faction -> group -> name. A spectator list is tens of rows, not thousands,
	//! and an insertion sort is stable and allocation-free — which matters because this runs on a
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Enforce has no string comparison operator that yields an ordering, only equality — so it is
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

//! One spectate-able player. A view record, rebuilt on every refresh — it holds no authority and
//! must never be cached across a refresh, because `m_Entity` can be destroyed under it.
class TBD_SpectatorTarget
{
	int m_iPlayerId;
	string m_sName;
	string m_sFactionKey;
	string m_sFactionName;
	string m_sGroupName;

	//! Weak. Re-resolve through TBD_SpectatorTargets.ResolveLivingEntity before pointing a camera
	//! at it — the row can outlive the character by a frame.
	IEntity m_Entity;
}
