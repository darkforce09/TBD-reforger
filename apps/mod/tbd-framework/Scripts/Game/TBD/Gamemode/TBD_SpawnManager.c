//! Result of a deploy attempt (spawn-authority contract, determinism slice A1).
//! Only NOT_MINE may reach the vanilla spawn path — everything else means the
//! framework owns this player and vanilla must stand down.
enum TBD_EDeployResult
{
	DEPLOYED,  //!< Bound to the slot body this call.
	ALREADY,   //!< This player is already bound.
	RETRY,     //!< Transient precondition (bodies/roster/controller) — retry shortly.
	FAILED,    //!< Permanent failure (kit resolve / body spawn) — logged ERROR, no vanilla body.
	NOT_MINE,  //!< Client side or no framework mission — vanilla may handle it.
	//! T-181.21 — refused by POLICY, not by a fault: this player has spent their one life.
	//! Deliberately distinct from FAILED so a one-life refusal can never be misread in the log
	//! as a kit/prefab error, and so nothing retries it. Callers treat it exactly like FAILED
	//! (never fall through to vanilla); only an admin route can turn it into a deploy.
	DENIED,
}

[ComponentEditorProps(category: "TBD/Framework", description: "Server-only: slot-body materialization + claim/bind deploy from mission JSON.")]
class TBD_SpawnManagerClass : SCR_BaseGameModeComponentClass {}

//! Slot-body materialization (operator-approved synthesis of CRF + PlayableSelector):
//! at mission load, one numbered slot BODY per compiled slots[] entry is spawned at
//! the exact JSON transform (kit prefab, AI disabled, Arsenal loadout applied) and
//! stands in the world through the lobby. Deploy = claim + hand the player onto the
//! pre-materialized body through vanilla's POSSESS spawn request: it takes over an
//! entity that already exists, so it never creates the second body that the
//! body-creating spawn requests did (the measured double-spawn class), while still
//! running the vanilla finalize the client needs to leave the loading screen.
//! @authority server — the whole manager runs server-side.
class TBD_SpawnManager : SCR_BaseGameModeComponent
{

	//! Vertical offset (m) added to the resolved ground/JSON height so the character
	//! capsule sits feet-on-ground. Measured on a human character spawn in wb_play
	//! (T-092.1) — NOT guessed; measurement log in .ai/artifacts/t092_1_verify_log.md.
	protected const float CAPSULE_GROUND_OFFSET_M = 0.0;

	//! Warn threshold (m) between an explicit JSON y and the live terrain surface —
	//! larger deltas usually mean a stale DEM or a mis-authored slot. Start 2.0 (T-092.1).
	protected const float MAX_Y_DELTA_M = 2.0;

	protected static TBD_SpawnManager s_Instance;

	//! A1 — the LOBBY auto-deploy wave (PIE/dev convenience: deploy everyone on stage
	//! entry without the deploy menu). The T-068.13 slot picker will default this off;
	//! the pull path (SCR_MenuSpawnLogic → DeployPlayerEx) is the production entry.
	//!
	//! T-181.21 — DEFAULT DELIBERATELY LEFT ON, and here is the reasoning, because the old
	//! comment on m_bOneLife told you the opposite. Two facts decide it:
	//!   1. On a framework world this wave is currently the ONLY working way into the world.
	//!      Vanilla registration/audit are swallowed (TBD_SCR_RespawnSystemComponent), so
	//!      SCR_SpawnLogic.DoInitialSpawn_S -> DoSpawn_S never fires and the "pull path"
	//!      is dead until the T-068.13 picker calls ClaimSlot + DeployPlayer itself.
	//!      Shipping this off today would ship a mod nobody can deploy into.
	//!   2. The reason it used to be unsafe next to ONE LIFE — a LOBBY re-entry re-running
	//!      the wave and mass-resurrecting the dead — is now structurally impossible: the
	//!      wave goes through DeployPlayerEx, and DeployPlayerEx refuses a spent life.
	//! The safety comes from the guard, not from this flag. Turn it off when the picker lands.
	[Attribute("1", desc: "Auto-deploy all connected players on LOBBY (PIE/dev wave; slot picker turns this off). Safe next to one life: the wave goes through DeployPlayerEx, which refuses a spent life.")]
	protected bool m_bAutoDeploy;

	//! Pause between death and the automatic redeploy. Vanilla's deploy menu used to be
	//! what put a killed player back in the world; with it stood down (see
	//! TBD_SCR_RespawnSystemComponent) the framework owns that too, and the delay is the
	//! respawn beat — long enough for the kill to read as a death, not a teleport.
	[Attribute("5000", desc: "Delay (ms) between death and automatic redeploy (auto-deploy worlds only).")]
	protected int m_iRedeployDelayMs;

	//! T-181.11 — ONE LIFE. Operator-locked for TBD events: death is terminal, the slot stays
	//! claimed (nobody else takes your seat, and a reconnect still finds you), and the only way
	//! back into the world is an admin acting on a glitch death. Deliberate divergence from CRF,
	//! which is wave/ticket respawn. See docs/mod/TBD_MOD_DESIGN.md §2 — this is a
	//! non-negotiable, which is why the default is 1 and stays 1.
	//!
	//! T-181.21 — the old comment here said "turn OFF for PIE/dev worlds so the auto-deploy wave
	//! keeps working". That advice is retired: the wave and one life no longer fight, because the
	//! wave deploys through DeployPlayerEx and DeployPlayerEx is the one-life boundary. Leave
	//! this ON. Turning it off does not "make dev easier", it silently ships a different game.
	[Attribute("1", desc: "ONE LIFE: death is terminal; only an admin (AdminRespawn) can put a player back in.")]
	protected bool m_bOneLife;

	//! Who has used their life, keyed on the DURABLE player key (PlayerBindKey), not on the
	//! numeric playerId.
	//!
	//! T-181.21 — this used to be map<int,bool> keyed on playerId, and that made ONE LIFE
	//! self-service: numeric playerIds are reused/reassigned on dedicated servers, so die →
	//! quit → rejoin handed the player a brand-new id, an empty dead-set lookup, and a fresh
	//! life. The rest of this file already knew that (m_mIdentityReclaim and m_mBodyBoundTo are
	//! identity-keyed for exactly this reason); dead-tracking was the one that was not.
	//!
	//! A map, not a set: Enfusion `set`/`array` Remove is BY INDEX (measured landmine) whereas
	//! map.Remove is by key, which is what this needs.
	protected ref map<string, bool> m_mDeadPlayers;

	//! T-181.21 — playerId → the durable key we resolved for that player, learned as early as
	//! the engine will tell us (OnPlayerAuditSuccess) and refreshed on every successful lookup.
	//!
	//! Needed because SCR_PlayerIdentityUtils.GetPlayerIdentityId stops answering once a player
	//! is being torn down, and the disconnect handler still has to know who it is dealing with.
	//! The entry is dropped when the player leaves, so a recycled numeric id can never inherit
	//! the previous occupant's identity.
	protected ref map<int, string> m_mBindKeyCache;

	//! T-181.21 — seats belonging to players who SPENT THEIR LIFE AND THEN LEFT, keyed on the
	//! durable player key.
	//!
	//! Two things force this to exist, and one forces it to be keyed on identity rather than
	//! parked in m_mPlayerSlot:
	//!   * OnPlayerDisconnected used to drop the slot outright — the exact thing ReleaseSlot
	//!     refuses to do for a dead player. TBD_FrameworkManager.TickWinConditions skips any
	//!     faction with 0 CLAIMED slots ("never fielded ≠ eliminated"), so the last man of a
	//!     side dying and then quitting erased his side and the round could never end.
	//!   * Leaving the row in m_mPlayerSlot under his numeric playerId would be worse than the
	//!     bug: dedicated servers reuse playerIds, so the next joiner to be handed that number
	//!     would silently inherit a dead man's seat (and his materialized body). Keyed on
	//!     identity there is nothing to inherit.
	protected ref map<string, ref TBD_MissionSlotStruct> m_mDepartedSlots;

	//! T-181.21 — players for whom THIS manager has an authorized spawn request in flight,
	//! playerId → ticket epoch. The vanilla spawn authority
	//! (TBD_SCR_RespawnSystemComponent.CanRequestSpawn_S) refuses every request that is not in
	//! here, which is what finally stands the vanilla DEATH door down. Opened only by
	//! DeployPlayerEx — i.e. only after the one-life guard has passed.
	//!
	//! The epoch exists so the timeout arm can only close the ticket it was issued for: a
	//! second deploy inside the window would otherwise be cut short by the first deploy's timer.
	protected ref map<int, int> m_mSpawnAuthorized;
	protected int m_iSpawnAuthEpoch;

	//! T-181.21 — deny-log latch so a client that re-asks in a loop cannot flood the log.
	protected ref map<int, bool> m_mDenyLogged;

	//! T-181.21 — an admin respawn that came back RETRY: the retry must carry the admin
	//! override, or the retry would be refused by the very guard the admin is overriding.
	protected ref map<int, bool> m_mAdminRespawnPending;

	//! T-181.21 — one-time warning latch for "this server issues no player identities".
	protected bool m_bIdentityDegradedLogged;

	//! T-181.21 — how long an authorized spawn ticket stays open. It only has to cover the
	//! request RPC hop: SCR_SpawnHandlerComponent consults CanRequestSpawn_S from
	//! CanHandleRequest_S at the START of HandleRequest_S, before preload and long before
	//! finalize (vanilla SCR_SpawnHandlerComponent.c). Seconds is already generous; the ticket
	//! is normally closed earlier, by OnPlayerSpawnedHook.
	protected const int SPAWN_AUTH_WINDOW_MS = 5000;

	protected ref map<int, ref TBD_MissionSlotStruct> m_mPlayerSlot;
	//! Slot key (uid-else-id) → the materialized slot body standing in the world.
	protected ref map<string, IEntity> m_mSlotBodies;
	//! T-181.10 — slot key → the IDENTITY a slot body has already been handed to.
	//! Keyed on identity, not playerId, because a mid-life reconnect must get its OWN body
	//! back (numeric ids are reused/reassigned on dedicated servers) while a genuinely
	//! different person taking a vacated slot must NOT inherit the previous occupant's
	//! state. See the re-equip block in DeployPlayerEx.
	protected ref map<string, string> m_mBodyBoundTo;
	protected int m_iRoundRobin;
	protected bool m_bSlotBodiesMaterialized;
	protected ref map<int, bool> m_mDeployRequested;
	//! A1 — pull-path retry bookkeeping (transient RETRY results; cap = 20 × 500 ms).
	protected ref map<int, int> m_mRetryCount;
	//! A1 — watchdog: players whose requested spawn has been observed to materialize.
	protected ref map<int, bool> m_mSpawnSeen;
	//! A6 — identityId → slot key, so a reconnect reclaims the same slot (dedicated
	//! servers reuse numeric playerIds; identity is the durable key).
	protected ref map<string, string> m_mIdentityReclaim;
	//! A7 — settle-census debounce + counter.
	protected bool m_bCensusScheduled;
	protected int m_iCensusCount;
	//! T-068.12 — strong refs to in-flight loadout applications (CallLater holds none);
	//! pruned of completed apps whenever a new one starts.
	protected ref array<ref TBD_LoadoutApplication> m_aLoadoutApps = {};

	//------------------------------------------------------------------------------------------------
	void TBD_SpawnManager(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		s_Instance = this;
		m_mPlayerSlot = new map<int, ref TBD_MissionSlotStruct>();
		m_mSlotBodies = new map<string, IEntity>();
		m_mBodyBoundTo = new map<string, string>();
		m_mDeployRequested = new map<int, bool>();
		m_mRetryCount = new map<int, int>();
		m_mSpawnSeen = new map<int, bool>();
		m_mDeadPlayers = new map<string, bool>();
		m_mIdentityReclaim = new map<string, string>();
		m_mBindKeyCache = new map<int, string>();
		m_mDepartedSlots = new map<string, ref TBD_MissionSlotStruct>();
		m_mSpawnAuthorized = new map<int, int>();
		m_mDenyLogged = new map<int, bool>();
		m_mAdminRespawnPending = new map<int, bool>();
	}

	//------------------------------------------------------------------------------------------------
	static TBD_SpawnManager GetInstance()
	{
		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	bool AreSlotBodiesMaterialized()
	{
		return m_bSlotBodiesMaterialized;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — is this slot already somebody else's seat?
	//!
	//! Three kinds of holder block a slot; only the first existed before:
	//!   * a CONNECTED player — the first-come guard T-181.9 gave ClaimSlot;
	//!   * a connected holder who SPENT THEIR LIFE — ONE LIFE says the seat stays theirs and is
	//!     not recycled (it is why ReleaseSlot refuses them);
	//!   * a DEPARTED holder who spent their life before leaving (m_mDepartedSlots) — same rule,
	//!     applied consistently instead of only while the corpse is still connected.
	//! @authority server
	protected bool IsSlotHeldByAnother(string slotKey, int playerId)
	{
		foreach (int otherId, TBD_MissionSlotStruct assigned : m_mPlayerSlot)
		{
			if (!assigned || otherId == playerId || assigned.Key() != slotKey)
				continue;

			if (GetGame().GetPlayerManager().GetPlayerController(otherId))
				return true;

			if (m_bOneLife && IsPlayerDead(otherId))
				return true;
		}

		string bindKey = PlayerBindKey(playerId);
		foreach (string otherKey, TBD_MissionSlotStruct departed : m_mDepartedSlots)
		{
			if (!departed || otherKey == bindKey || departed.Key() != slotKey)
				continue;
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Assign mission slot to player (roster or round-robin). Idempotent per player.
	//!
	//! T-181.21 — this path had NO exclusivity check at all. ClaimSlot got first-come
	//! exclusivity in T-181.9, but AssignSlotForPlayer — which runs on every join and is what
	//! actually seats people today — only asked "does this player already have one", so a roster
	//! entry pointing two people at one slot, or a round-robin wrapping past the slot count, sat
	//! two players on the same seat and the same materialized body. Every candidate is now
	//! checked, and the round-robin fallback SCANS for a free slot instead of trusting modulo.
	void AssignSlotForPlayer(int playerId)
	{
		if (m_mPlayerSlot.Contains(playerId))
			return;

		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots || slots.IsEmpty())
		{
			Print("[TBD] SpawnManager: no mission slots — cannot assign player " + playerId, LogLevel.ERROR);
			return;
		}

		string bindKey = PlayerBindKey(playerId);

		// T-181.21 — a spent life coming back takes its own seat back, ahead of everything
		// else. The row moves out of m_mDepartedSlots and into m_mPlayerSlot under whatever
		// numeric id the server handed them this time; they are still dead (m_mDeadPlayers is
		// identity-keyed), so DeployPlayerEx will still refuse them — they get their seat and
		// their place in the win-condition count back, not their life.
		TBD_MissionSlotStruct slot;
		if (m_mDepartedSlots.Find(bindKey, slot))
		{
			m_mDepartedSlots.Remove(bindKey);
			Print(string.Format("[TBD][Spawn] player=%1 rejoined on a spent life — slot %2 handed back (still dead)", playerId, slot.Key()));
		}

		// A6 — reconnect reclaim beats roster/round-robin: same identity → same slot.
		if (!slot && IsDurableKey(bindKey))
		{
			string reclaimId;
			if (m_mIdentityReclaim.Find(bindKey, reclaimId))
				slot = TBD_MissionLoader.GetSlotById(reclaimId);
		}
		if (slot && IsSlotHeldByAnother(slot.Key(), playerId))
		{
			Print(string.Format("[TBD][Spawn] player=%1 reclaim of slot %2 refused — held by someone else", playerId, slot.Key()), LogLevel.WARNING);
			slot = null;
		}

		if (!slot)
		{
			string slotId = ResolveSlotIdForPlayer(playerId);
			slot = TBD_MissionLoader.GetSlotById(slotId);
			if (slot && IsSlotHeldByAnother(slot.Key(), playerId))
			{
				Print(string.Format("[TBD][Spawn] player=%1 roster slot %2 already held — falling back to a free slot", playerId, slot.Key()), LogLevel.WARNING);
				slot = null;
			}
		}

		if (!slot)
		{
			// Round-robin fallback: walk the whole list once from the cursor so a taken slot is
			// skipped rather than double-booked.
			int count = slots.Count();
			for (int i = 0; i < count; i++)
			{
				TBD_MissionSlotStruct candidate = slots[(m_iRoundRobin + i) % count];
				if (!candidate || IsSlotHeldByAnother(candidate.Key(), playerId))
					continue;
				slot = candidate;
				m_iRoundRobin = (m_iRoundRobin + i + 1) % count;
				break;
			}
		}

		if (!slot)
		{
			// Every seat is taken (or held by a spent life). Assigning anyway would put two
			// players on one body, so refuse — DeployPlayerEx reports RETRY and the retry cap
			// turns it into a visible ERROR rather than a silent double-book.
			Print(string.Format("[TBD][Spawn] player=%1 could not be seated — every mission slot is held (%2 slots)", playerId, slots.Count()), LogLevel.ERROR);
			return;
		}

		m_mPlayerSlot.Insert(playerId, slot);
		Print(string.Format("[TBD] SpawnManager: assigned slot %1 to player %2 at (%3)", slot.id, playerId, slot.x.ToString() + "," + slot.z.ToString()));
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — routed through PlayerBindKey so there is exactly one definition of "who is
	//! this player" in the file. The roster is keyed by real backend identities, so the
	//! `player:<id>` fallback simply matches nothing and drops to round-robin, which is the
	//! honest outcome; the old code formatted a NULL uuid into a non-empty constant and looked
	//! *that* up.
	protected string ResolveSlotIdForPlayer(int playerId)
	{
		if (!TBD_RosterLoader.IsLoaded())
			return string.Empty;

		string bindKey = PlayerBindKey(playerId);
		if (!IsDurableKey(bindKey))
			return string.Empty;

		return TBD_RosterLoader.GetSlotForIdentity(bindKey);
	}

	//------------------------------------------------------------------------------------------------
	TBD_MissionSlotStruct GetAssignedSlot(int playerId)
	{
		return m_mPlayerSlot.Get(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! The materialized body standing on a slot (null when never materialized).
	IEntity GetSlotBody(string slotKey)
	{
		return m_mSlotBodies.Get(slotKey);
	}

	//------------------------------------------------------------------------------------------------
	//! PS-shaped server claim guard (backend for the T-068.13 picker): a slot can be
	//! claimed when unclaimed, already ours, or its previous claimant disconnected.
	//! Rejected when a DIFFERENT live player holds it. Round-robin/roster auto-claim
	//! goes through AssignSlotForPlayer as before.
	bool ClaimSlot(int playerId, string slotKey)
	{
		TBD_MissionSlotStruct slot = TBD_MissionLoader.GetSlotById(slotKey);
		if (!slot)
			return false;

		// T-181.9 — ONE LIFE integrity. Note this is a CONVENIENCE guard, not the enforcement
		// boundary: claiming a slot does not put anybody in the world, so on its own it never
		// protected the invariant (T-181.21 moved the real guard to DeployPlayerEx, the only
		// door in). It stays because rejecting the claim at the picker is a better experience
		// than accepting it and refusing the deploy. Their own slot stays assigned to them (see
		// OnPlayerKilled), so this costs a live player nothing.
		if (m_bOneLife && IsPlayerDead(playerId))
		{
			Print(string.Format("[TBD][Spawn] claim rejected player=%1 slot=%2 (one life spent)", playerId, slot.Key()), LogLevel.WARNING);
			return false;
		}

		// T-181.21 — one shared exclusivity rule with AssignSlotForPlayer (which had none).
		if (IsSlotHeldByAnother(slot.Key(), playerId))
		{
			Print(string.Format("[TBD][Spawn] claim rejected player=%1 slot=%2 (held by another player)", playerId, slot.Key()));
			return false;
		}

		m_mPlayerSlot.Set(playerId, slot);
		Print(string.Format("[TBD][Spawn] claim player=%1 slot=%2", playerId, slot.Key()));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.9 — give a slot back while still in the lobby, so a player can change their mind
	//! before deploying.
	//!
	//! Refused once the life is spent: under ONE LIFE the slot is deliberately retained for a
	//! dead player (it is their seat, and releasing it would both recycle it to someone else and
	//! let the dead player re-claim elsewhere). Also refused after deploy — you are in the world,
	//! the seat is yours.
	//! @authority server
	bool ReleaseSlot(int playerId)
	{
		if (RplSession.Mode() == RplMode.Client)
			return false;

		if (!m_mPlayerSlot.Contains(playerId))
			return false;

		if (m_bOneLife && IsPlayerDead(playerId))
		{
			Print(string.Format("[TBD][Spawn] release rejected player=%1 (one life spent — slot retained)", playerId), LogLevel.WARNING);
			return false;
		}

		if (m_mDeployRequested.Contains(playerId))
		{
			Print(string.Format("[TBD][Spawn] release rejected player=%1 (already deployed)", playerId), LogLevel.WARNING);
			return false;
		}

		TBD_MissionSlotStruct slot = m_mPlayerSlot.Get(playerId);
		m_mPlayerSlot.Remove(playerId);
		string key;
		if (slot)
			key = slot.Key();
		Print(string.Format("[TBD][Spawn] release player=%1 slot=%2", playerId, key));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.9 — the lobby's view of the roster: one line per mission slot with who holds it and
	//! whether it can still be taken. This is the data the slot picker (T-181.9.1) binds to, kept
	//! deliberately as plain text so the authority side is testable from the server log long
	//! before any widget exists.
	//!
	//! Format: `<slotKey>\t<faction>\t<group>\t<role>\t<state>\t<holderPlayerId>`
	//! state: OPEN | HELD | DEAD  (DEAD = holder spent their life; the seat is not recyclable)
	//!
	//! T-181.21 — a seat whose holder died and then quit reports DEAD with holder -1, not OPEN.
	//! The state column is what a picker must believe; reporting the seat OPEN would invite a
	//! claim the authority then refuses (IsSlotHeldByAnother).
	array<string> BuildSlotRoster()
	{
		array<string> roster = {};
		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots)
			return roster;

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;

			int holder = -1;
			foreach (int playerId, TBD_MissionSlotStruct assigned : m_mPlayerSlot)
			{
				if (assigned && assigned.Key() == slot.Key())
				{
					holder = playerId;
					break;
				}
			}

			string state = "OPEN";
			if (holder > 0)
			{
				if (IsPlayerDead(holder))
					state = "DEAD";
				else
					state = "HELD";
			}
			else
			{
				foreach (string bindKey, TBD_MissionSlotStruct departed : m_mDepartedSlots)
				{
					if (departed && departed.Key() == slot.Key())
					{
						state = "DEAD";
						break;
					}
				}
			}

			roster.Insert(string.Format("%1\t%2\t%3\t%4\t%5\t%6",
				slot.Key(), slot.faction, slot.groupCallsign, slot.role, state, holder));
		}
		return roster;
	}

	//------------------------------------------------------------------------------------------------
	//! Engine faction key a materialized body was built with (kit prefab affiliation) —
	//! the fallback when a mission faction key has no mapping above.
	protected string BodyFactionKey(IEntity body)
	{
		if (!body)
			return string.Empty;

		FactionAffiliationComponent affiliation = FactionAffiliationComponent.Cast(
			body.FindComponent(FactionAffiliationComponent));
		if (!affiliation)
			return string.Empty;

		Faction faction = affiliation.GetDefaultAffiliatedFaction();
		if (!faction)
			return string.Empty;

		return faction.GetFactionKey();
	}

	//------------------------------------------------------------------------------------------------
	//! Engine faction key for mission faction key.
	string EngineFactionKey(string missionFactionKey)
	{
		switch (missionFactionKey)
		{
			case "blufor": return "US";
			case "opfor": return "USSR";
			case "indfor": return "FIA";
			case "civ": return "CIV";
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Authority-only: materialize one slot BODY per mission slots[] entry at the exact
	//! JSON transform — kit prefab, AI disabled (CRF pattern), Arsenal loadout applied.
	//! The numbered lineup stands in the world through the lobby; deploy binds onto it.
	void MaterializeSlotBodies()
	{
		if (m_bSlotBodiesMaterialized)
			return;

		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots || slots.IsEmpty())
		{
			Print("[TBD] SpawnManager: no mission slots — cannot materialize bodies.", LogLevel.ERROR);
			return;
		}

		int built = 0;
		int loadouts = 0;
		int kitOnly = 0;
		int failed = 0;
		int number = 0;
		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;
			number++;

			IEntity body = SpawnSlotBody(slot, number);
			if (!body)
			{
				failed++;
				continue;
			}

			m_mSlotBodies.Set(slot.Key(), body);
			built++;
			if (slot.loadout)
				loadouts++;
			else
				kitOnly++;
		}

		if (built > 0)
			m_bSlotBodiesMaterialized = true;

		// T-181.10 — kit-only slots are legal (the kit prefab dresses them), but a mission
		// that meant to author loadouts and shipped none has to be visible at a glance, and
		// a slot whose body never spawned is an outright error, not a quiet shortfall.
		Print(string.Format("[TBD][Slots] materialized %1/%2 bodies — %3 with a JSON loadout, %4 kit-only, %5 failed",
			built, number, loadouts, kitOnly, failed));
		if (failed > 0)
			Print(string.Format("[TBD][Slots] %1 of %2 slot bodies FAILED to materialize — see the kit resolve / prefab errors above",
				failed, number), LogLevel.ERROR);
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn one slot body at the slot's JSON transform: kit prefab → AI off →
	//! Arsenal loadout (when authored). Also the respawn path (fresh body per life —
	//! operator-locked). Returns null on kit/prefab failure (logged ERROR).
	protected IEntity SpawnSlotBody(TBD_MissionSlotStruct slot, int number)
	{
		bool kitOk;
		ResourceName prefab = TBD_Registry.Resolve(slot.kit, kitOk);
		if (!kitOk || prefab.IsEmpty())
		{
			Print("[TBD] SpawnManager: kit resolve failed: " + slot.kit, LogLevel.ERROR);
			return null;
		}

		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			Print("[TBD] SpawnManager: kit prefab failed to load: " + prefab, LogLevel.ERROR);
			return null;
		}

		float x = slot.x;
		float z = slot.z;

		// Spawn height policy (T-092.1): explicit JSON y wins, else live terrain
		// surface; both get the measured capsule offset on top.
		float surfaceY = GetGame().GetWorld().GetSurfaceY(x, z);
		float spawnY = surfaceY;
		float delta = 0;
		string jsonYLabel = "-";
		if (slot.HasJsonY())
		{
			spawnY = slot.y;
			delta = Math.AbsFloat(slot.y - surfaceY);
			jsonYLabel = slot.y.ToString();
			if (delta > MAX_Y_DELTA_M)
				Print(string.Format("[TBD][Spawn] slot=%1 jsonY=%2 deviates %3 m from surfaceY=%4 (> %5 m) — stale DEM or mis-authored slot?",
					slot.id, slot.y, delta, surfaceY, MAX_Y_DELTA_M), LogLevel.WARNING);
		}
		spawnY += CAPSULE_GROUND_OFFSET_M;

		vector pos = Vector(x, spawnY, z);

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = pos;

		// Apply heading from JSON (yaw around Y)
		float yawRad = slot.headingDeg * Math.DEG2RAD;
		params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
		params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

		IEntity body = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (!body)
		{
			Print("[TBD] SpawnManager: failed to spawn slot body for " + slot.id, LogLevel.ERROR);
			return null;
		}

		// CRF pattern: deactivate once + next-frame re-check. No repeating hammer —
		// created-at-load bodies don't fight the PS parked-AI reactivation bug.
		DisableBodyAI(body);

		Print(string.Format("[TBD][Slots] Slot-%1 %2 (%3) kit %4 at %5",
			number, slot.Key(), slot.id, slot.kit, pos.ToString()));
		Print(string.Format("[TBD][Spawn] slot=%1 Y=%2 jsonY=%3 surfaceY=%4 delta=%5 heading=%6",
			slot.id, spawnY, jsonYLabel, surfaceY, delta, slot.headingDeg));

		if (slot.loadout)
		{
			PruneDoneLoadoutApps();
			TBD_LoadoutApplication app = new TBD_LoadoutApplication(body, slot.loadout, "[TBD][Loadout][Slot]", slot.id);
			m_aLoadoutApps.Insert(app);
			app.Run();
		}

		return body;
	}

	//------------------------------------------------------------------------------------------------
	//! CRF_PlayerCharacter.DisableAI port: deactivate the agent + one next-frame re-check.
	protected void DisableBodyAI(IEntity body)
	{
		AIControlComponent aiComponent = AIControlComponent.Cast(body.FindComponent(AIControlComponent));
		if (!aiComponent)
			return;

		AIAgent agent = aiComponent.GetAIAgent();
		if (agent)
			agent.DeactivateAI();

		GetGame().GetCallqueue().Call(DisableBodyAIRecheck, aiComponent);
	}

	//------------------------------------------------------------------------------------------------
	protected void DisableBodyAIRecheck(AIControlComponent aiComponent)
	{
		if (!aiComponent)
			return;
		AIAgent agent = aiComponent.GetAIAgent();
		if (agent)
			agent.DeactivateAI();
	}

	//------------------------------------------------------------------------------------------------
	protected void PruneDoneLoadoutApps()
	{
		for (int i = m_aLoadoutApps.Count() - 1; i >= 0; i--)
		{
			if (m_aLoadoutApps[i].IsDone())
				m_aLoadoutApps.Remove(i);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.10 — stop any loadout pass still dressing a body we are about to abandon.
	//! Without this a superseded body keeps spawning and verifying items for seconds after
	//! its replacement exists, and its ERROR lines carry the same slot id as the live body's.
	protected void CancelLoadoutAppsFor(IEntity body)
	{
		if (!body)
			return;

		foreach (TBD_LoadoutApplication app : m_aLoadoutApps)
		{
			if (!app.IsDone() && app.GetCharacter() == body)
				app.Cancel("slot body superseded by a fresh spawn");
		}
		PruneDoneLoadoutApps();
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.10 — the durable "who is this" key for body ownership. Identity first (numeric
	//! playerIds are reused/reassigned on dedicated servers, so a mid-life reconnect must
	//! still resolve to its own body); the numeric id only as a PIE/local fallback.
	//!
	//! T-181.21 — this is now ALSO the key ONE LIFE is enforced on (m_mDeadPlayers), so its
	//! two weak spots had to be fixed:
	//!
	//!   * The old emptiness test was `string.Format("%1", uuid).IsEmpty()`. GetPlayerIdentityId
	//!     returns a UUID (vanilla SCR_SpawnLogic.c does `const UUID identity = ...; ...
	//!     playerCharacterId.IsNull()`), and a NULL uuid does not format to "" — it formats to
	//!     the same constant string for everybody. So on a server that issues no identities,
	//!     every player collapsed onto ONE shared key. Under identity-keyed dead-tracking that
	//!     would mean the first death kills the whole server. UUID.IsNull() is the correct test
	//!     and is what this uses.
	//!   * A player being torn down (disconnect) no longer answers the identity lookup, but we
	//!     still have to know who held that seat. Hence the cache.
	//!
	//! WHAT THE `player:<id>` FALLBACK MEANS, said plainly: on a host that hands out no
	//! identities at all (pure local PIE), one life degrades to exactly the old numeric-id
	//! behaviour and a rejoin CAN buy a fresh life. That is not fixable from script — there is
	//! no other durable handle — so it is logged loudly once (NoteIdentityDegraded) instead of
	//! being hidden.
	protected string PlayerBindKey(int playerId)
	{
		UUID identity = SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId);
		if (!identity.IsNull())
		{
			string identityId = string.Format("%1", identity);
			if (!identityId.IsEmpty())
			{
				m_mBindKeyCache.Set(playerId, identityId);
				return identityId;
			}
		}

		// Live lookup unavailable (teardown, or an identity-less host). A key we resolved
		// earlier for this numeric id is strictly better than inventing a new one.
		string cached;
		if (m_mBindKeyCache.Find(playerId, cached))
			return cached;

		NoteIdentityDegraded(playerId);
		return string.Format("player:%1", playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — true when a key is a real player identity rather than the numeric fallback.
	//! Anything keyed on the fallback is NOT durable across a reconnect and must not be written
	//! into the reclaim map, where a different person inheriting the numeric id would inherit
	//! the seat with it.
	protected static bool IsDurableKey(string key)
	{
		return !key.IsEmpty() && !key.StartsWith("player:");
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — say it once, loudly: on this host ONE LIFE is only as durable as the numeric
	//! playerId, so die → quit → rejoin can buy a fresh life. Operators need to know that before
	//! they run an event on it, not after.
	protected void NoteIdentityDegraded(int playerId)
	{
		if (m_bIdentityDegradedLogged)
			return;
		m_bIdentityDegradedLogged = true;
		Print(string.Format("[TBD][Spawn] player=%1 has NO durable identity — ONE LIFE falls back to the numeric playerId on this host, so a reconnect can buy a fresh life. Expected in local PIE; NOT acceptable for an event server.", playerId), LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — learn who a numeric playerId belongs to as early as the engine will say.
	//! OnPlayerAuditSuccess is the join hook (design §5: NOT OnPlayerConnected), and doing the
	//! resolve here means the bind-key cache is refreshed for a joining player BEFORE anything
	//! reads it — which is what stops a recycled numeric id from inheriting the previous
	//! occupant's cached identity (and therefore their death).
	//! @authority server
	override void OnPlayerAuditSuccess(int playerId)
	{
		super.OnPlayerAuditSuccess(playerId);

		if (RplSession.Mode() == RplMode.Client)
			return;

		m_mBindKeyCache.Remove(playerId);
		string bindKey = PlayerBindKey(playerId);
		Print(string.Format("[TBD][Spawn] player=%1 audit ok — bindKey=%2 durable=%3 lifeSpent=%4",
			playerId, bindKey, IsDurableKey(bindKey), IsBindKeyDead(bindKey)));
	}

	//------------------------------------------------------------------------------------------------
	void OnStageChanged(TBD_EGameStage stage)
	{
		if (stage == TBD_EGameStage.LOBBY && m_bAutoDeploy)
			ScheduleDeployAllConnectedPlayers();
	}

	//------------------------------------------------------------------------------------------------
	protected void ScheduleDeployAllConnectedPlayers()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!m_bSlotBodiesMaterialized)
			return;

		GetGame().GetCallqueue().CallLater(DeployAllConnectedPlayers, 250, false);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — deploys every connected player from the server.
	protected void DeployAllConnectedPlayers()
	{
		// Authority only — spawning happens on the server.
		if (RplSession.Mode() == RplMode.Client)
			return;

		array<int> players = {};
		int count = GetGame().GetPlayerManager().GetPlayers(players);
		for (int i = 0; i < count; i++)
		{
			// T-181.21 — a re-entry into LOBBY re-runs this wave, and under ONE LIFE that used
			// to be a mass resurrection. DeployPlayerEx would refuse each of them anyway; the
			// skip is here so a wave over a mostly-dead server does not bury the log in
			// refusals, and so the intent is legible at the call site.
			if (m_bOneLife && IsPlayerDead(players[i]))
			{
				Print(string.Format("[TBD][Spawn] path=push player=%1 skipped — one life spent", players[i]));
				continue;
			}

			TBD_EDeployResult r = DeployPlayerEx(players[i]);
			Print(string.Format("[TBD][Spawn] path=push player=%1 result=%2", players[i], typename.EnumToString(TBD_EDeployResult, r)));
			if (r == TBD_EDeployResult.RETRY)
				ScheduleDeployRetry(players[i]);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — back-compat bool wrapper over DeployPlayerEx; true only when
	//! a bind happened in THIS call.
	bool DeployPlayer(int playerId)
	{
		return DeployPlayerEx(playerId) == TBD_EDeployResult.DEPLOYED;
	}

	//------------------------------------------------------------------------------------------------
	//! Authority: claim the player's slot and BIND them onto its pre-materialized body
	//! via SCR_PlayerController.SetInitialMainEntity — the CRF/PlayableSelector-proven
	//! takeover; the vanilla RequestSpawn pipeline (measured double-spawn source) is
	//! never used. Spawn-authority contract (A1): NOT_MINE is the only result that may
	//! reach vanilla spawn; ALREADY/FAILED/DENIED all mean "vanilla stands down".
	//!
	//! T-181.10 — `forceFreshBody` makes the re-equip guarantee unconditional for the ONE
	//! LIFE return path (AdminRespawn): whatever is standing on the slot is abandoned and a
	//! newly dressed body is materialized, so an admin respawn can never hand back a body
	//! carrying the life that was just spent. Defaulted, so every existing caller is
	//! unchanged and keeps the ordinary "reuse if it is alive and yours" behaviour.
	//!
	//! T-181.21 — THIS FUNCTION IS THE ONE-LIFE ENFORCEMENT BOUNDARY. Read this before
	//! "simplifying" the guard below away.
	//!
	//! It used to live on ClaimSlot() and ReleaseSlot(), neither of which can put anybody into
	//! the world — so the invariant the whole design calls non-negotiable was not actually
	//! enforced anywhere. `DeployPlayerEx` is the only door: every path in the framework ends
	//! here (the LOBBY wave, the pull path in TBD_SCR_MenuSpawnLogic.DoSpawn_S, RedeployAfterDeath,
	//! RetryDeploy, AdminRespawn), and the vanilla doors are refused by
	//! TBD_SCR_RespawnSystemComponent.CanRequestSpawn_S unless THIS function authorized them.
	//! So: one guard, on the choke point, plus a backstop that only opens for this function.
	//!
	//! `adminOverride` is the one documented bypass — the glitch-death escape hatch the design
	//! doc §2 requires ("an admin can respawn a player who died to a glitch — that path must
	//! always exist"). It is opt-in, never defaulted on, and only AdminRespawn (and the retry it
	//! owns) passes it.
	//! @authority server
	TBD_EDeployResult DeployPlayerEx(int playerId, bool forceFreshBody = false, bool adminOverride = false)
	{
		// Authority only — slot assignment + binding run on the server.
		if (RplSession.Mode() == RplMode.Client)
			return TBD_EDeployResult.NOT_MINE;

		// No valid framework mission → vanilla owns spawning entirely.
		if (!TBD_MissionLoader.IsLoaded() || !TBD_MissionLoader.IsValid())
			return TBD_EDeployResult.NOT_MINE;

		// ── ONE LIFE (T-181.21) ────────────────────────────────────────────────────────────
		// Deliberately the FIRST thing checked once we know the mission is ours, ahead of the
		// materialized/roster RETRY below: a spent life must never start a retry loop, never
		// remake a body, never touch faction affiliation. Refusal is DENIED, not FAILED, so the
		// log cannot confuse a policy decision with a kit error, and no caller retries it.
		if (m_bOneLife && !adminOverride && IsPlayerDead(playerId))
		{
			Print(string.Format("[TBD][Spawn] deploy DENIED player=%1 key=%2 — one life spent (admin respawn is the only way back)",
				playerId, PlayerBindKey(playerId)), LogLevel.WARNING);
			return TBD_EDeployResult.DENIED;
		}

		if (!m_bSlotBodiesMaterialized || !TBD_RosterLoader.IsLoaded())
			return TBD_EDeployResult.RETRY;

		if (m_mDeployRequested.Contains(playerId))
			return TBD_EDeployResult.ALREADY;

		AssignSlotForPlayer(playerId);

		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (!slot)
			return TBD_EDeployResult.RETRY;

		// T-181.10 — RE-EQUIP ON EVERY SPAWN (operator-locked). A standing slot body is only
		// reused when it is alive AND belongs to the identity asking for it. Everything else
		// is a new life or a new occupant, and gets a brand-new body materialized at the slot
		// transform — SpawnSlotBody re-applies the JSON loadout, so nobody ever inherits the
		// previous life's fired mags, dropped kit or damage. The old body stays where it is
		// (the corpse-stays rule); an abandoned LIVE body is logged so the settle census's
		// characters/bodies delta stays explainable.
		string bindKey = PlayerBindKey(playerId);
		IEntity body = m_mSlotBodies.Get(slot.Key());
		string remakeReason;
		if (!body)
		{
			remakeReason = "no standing body";
		}
		else if (forceFreshBody)
		{
			remakeReason = "fresh body enforced by the caller (admin respawn)";
		}
		else if (IsBodyDead(body))
		{
			remakeReason = "previous life spent";
		}
		else
		{
			string boundTo;
			if (m_mBodyBoundTo.Find(slot.Key(), boundTo) && boundTo != bindKey)
				remakeReason = "body already used by another occupant";
		}

		if (!remakeReason.IsEmpty())
		{
			if (body && !IsBodyDead(body))
				Print(string.Format("[TBD][Slots] slot=%1 abandoning a LIVE body (%2) — it stays in the world", slot.Key(), remakeReason), LogLevel.WARNING);

			// Stop anything still dressing the outgoing body first: its pending items are
			// cleaned up, and its loadout log lines cannot be confused with the new body's.
			CancelLoadoutAppsFor(body);

			body = SpawnSlotBody(slot, 0);
			if (!body)
				return TBD_EDeployResult.FAILED;
			m_mSlotBodies.Set(slot.Key(), body);
			Print(string.Format("[TBD][Slots] rematerialized body for slot %1 (%2) — freshly dressed from mission JSON", slot.Key(), remakeReason));
		}

		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (!pc)
		{
			Print("[TBD] SpawnManager: no player controller for player " + playerId, LogLevel.ERROR);
			return TBD_EDeployResult.RETRY;
		}

		SCR_PlayerFactionAffiliationComponent factionComp = SCR_PlayerFactionAffiliationComponent.Cast(
			pc.FindComponent(SCR_PlayerFactionAffiliationComponent));
		if (factionComp)
		{
			// Mission key first; if it maps to nothing (modded kit faction, unmapped side)
			// fall back to whatever faction the body itself was built as, so the player is
			// never registered under an empty key.
			string engineKey = EngineFactionKey(slot.faction);
			if (engineKey.IsEmpty())
				engineKey = BodyFactionKey(body);

			if (!engineKey.IsEmpty())
			{
				factionComp.SetAffiliatedFactionByKey(engineKey);
				// Vanilla only learns about the affiliation through the manager (the
				// PlayableSelector finalize); without it the player is faction-correct
				// locally but invisible to faction-keyed vanilla systems.
				SCR_FactionManager fm = SCR_FactionManager.Cast(GetGame().GetFactionManager());
				if (fm)
					fm.UpdatePlayerFaction_S(factionComp);
			}
			else
			{
				Print(string.Format("[TBD][Spawn] slot=%1 faction=%2 has no engine mapping — affiliation left untouched",
					slot.id, slot.faction), LogLevel.WARNING);
			}
		}

		// The takeover. Preferred route is vanilla's POSSESS spawn request: it is the
		// engine's own "this player takes over an entity that already exists" path, so it
		// creates no second body (the double-spawn class stays fixed) while running the
		// full spawn finalize — including the client-side notification the loading screen
		// waits on. A raw SetInitialMainEntity possesses the body and gives it a camera,
		// but the client is never told a spawn happened and sits on the loading screen
		// forever (measured 2026-07-25). SetInitialMainEntity stays as the fallback for
		// when the request component is missing or refuses.
		//
		// T-181.21 — open the spawn ticket FIRST. The possess route is a vanilla spawn request,
		// and the vanilla authority now refuses every request this manager did not authorize
		// (TBD_SCR_RespawnSystemComponent.CanRequestSpawn_S). Without this our own deploy would
		// be the first casualty of the backstop.
		AuthorizeSpawn(playerId);
		bool possessed = PossessSlotBody(pc, body, playerId);
		if (!possessed)
			pc.SetInitialMainEntity(body);

		m_mDeployRequested.Set(playerId, true);
		// T-181.10 — this body is now spoken for. A later deploy by anyone else on this slot
		// sees the mismatch and materializes a fresh dressed body instead of inheriting it.
		m_mBodyBoundTo.Set(slot.Key(), bindKey);
		m_mRetryCount.Remove(playerId);
		m_mSpawnSeen.Remove(playerId);
		Print(string.Format("[TBD] SpawnManager: bound player %1 to slot %2 body (kit %3)", playerId, slot.Key(), slot.kit));

		// Announce the spawn ourselves ONLY on the fallback route. The possess pipeline
		// fires the game mode's spawn invoker itself, and our hook is subscribed to it —
		// self-announcing there notified every listener twice (measured: two
		// "deployed player=" diagnostics per bind).
		if (!possessed)
			NotifySpawnedManually(playerId, body);

		// A1 watchdog: if control never materializes, re-arm so the next pull
		// attempt can deploy instead of wedging on ALREADY forever.
		GetGame().GetCallqueue().CallLater(CheckSpawnArrived, 10000, false, playerId);
		return TBD_EDeployResult.DEPLOYED;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — open a spawn ticket for a player DeployPlayerEx has just cleared.
	//!
	//! The window only has to cover the request's trip into ProcessRequest_S: vanilla
	//! SCR_SpawnHandlerComponent.HandleRequest_S asks CanHandleRequest_S (→ CanRequestSpawn_S)
	//! before it spawns anything, i.e. before preload and long before finalize. The ticket is
	//! normally closed sooner than the timeout, by OnPlayerSpawnedHook.
	//! @authority server
	protected void AuthorizeSpawn(int playerId)
	{
		m_iSpawnAuthEpoch++;
		m_mSpawnAuthorized.Set(playerId, m_iSpawnAuthEpoch);
		m_mDenyLogged.Remove(playerId);
		GetGame().GetCallqueue().CallLater(ExpireSpawnAuthorization, SPAWN_AUTH_WINDOW_MS, false, playerId, m_iSpawnAuthEpoch);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — the timeout arm. Closes ONLY the ticket it was issued for, so a redeploy
	//! inside the window is not cut short by the previous deploy's timer.
	protected void ExpireSpawnAuthorization(int playerId, int epoch)
	{
		int held;
		if (m_mSpawnAuthorized.Find(playerId, held) && held == epoch)
			m_mSpawnAuthorized.Remove(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — close the ticket unconditionally. Idempotent: called by the spawn hook when
	//! the spawn lands, and by OnPlayerKilled/OnPlayerDisconnected so a ticket cannot outlive
	//! the state it was issued against.
	protected void RevokeSpawnAuthorization(int playerId)
	{
		m_mSpawnAuthorized.Remove(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — the question TBD_SCR_RespawnSystemComponent.CanRequestSpawn_S asks of every
	//! spawn request on a framework world.
	//!
	//! This is the backstop for B2: vanilla routes death → SCR_RespawnSystemComponent.OnPlayerKilled_S
	//! → SCR_SpawnLogic.OnPlayerKilled_S → OnPlayerEntityLost_S → (SCR_MenuSpawnLogic)
	//! NotifyReadyForSpawn_S, which invites the client to ask for a spawn — a door the JOIN-side
	//! suppression (OnPlayerRegistered_S / OnPlayerAuditSuccess_S) never touched. Rather than
	//! chase each such chain, every request has to prove that THIS manager issued it, and this
	//! manager only issues one after the one-life guard in DeployPlayerEx has passed.
	//!
	//! `denyLogOnce` keeps a client that re-asks in a loop from flooding the log while still
	//! making the first refusal visible.
	bool IsSpawnAuthorized(int playerId, out bool denyLogOnce)
	{
		if (m_mSpawnAuthorized.Contains(playerId))
			return true;

		denyLogOnce = !m_mDenyLogged.Contains(playerId);
		if (denyLogOnce)
			m_mDenyLogged.Set(playerId, true);
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Hand the player to its slot body through vanilla's possess spawn request.
	//! Returns false when the route is unavailable, so the caller can fall back.
	protected bool PossessSlotBody(SCR_PlayerController pc, IEntity body, int playerId)
	{
		SCR_PossessSpawnRequestComponent request = SCR_PossessSpawnRequestComponent.Cast(
			pc.FindComponent(SCR_PossessSpawnRequestComponent));
		if (!request)
		{
			Print(string.Format("[TBD][Spawn] player=%1 has no possess request component — falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		SCR_PossessSpawnData data = SCR_PossessSpawnData.FromEntity(body);
		if (!data)
		{
			Print(string.Format("[TBD][Spawn] player=%1 possess data build failed — falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		if (!request.RequestRespawn(data))
		{
			Print(string.Format("[TBD][Spawn] player=%1 possess request refused — falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		Print(string.Format("[TBD][Spawn] player=%1 possess request accepted", playerId));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! FALLBACK ROUTE ONLY (the possess pipeline announces its own spawns).
	//! SetInitialMainEntity bypasses the vanilla spawn pipeline, so nothing fires the
	//! usual spawn notifications (the CRF finding). Fire the game mode's own invoker
	//! rather than calling our hook directly: our hook is subscribed to it (OnPostInit),
	//! so our bookkeeping still runs exactly once, and the vanilla listeners that assume
	//! a spawn always announces itself finally hear it too (the PlayableSelector finalize).
	//! Server-side only — a dedicated server also needs the client-side invoke, which is
	//! the named follow-up in the verify log.
	//! (CRF also notifies its own MODDED data collector here — vanilla
	//! SCR_DataCollectorComponent has no such entry point; stats integration is a
	//! future slice if the platform ever consumes vanilla session stats.)
	protected void NotifySpawnedManually(int playerId, IEntity body)
	{
		// Fire only once the player ACTUALLY controls the body: SetInitialMainEntity hands
		// over asynchronously, and listeners that react to a spawn (the client-side ones
		// that take the player off the loading screen among them) check the controlled
		// entity and bail when it is still null. PlayableSelector fires from
		// OnControlledEntityChanged for the same reason.
		FinalizeSpawnWhenControlled(playerId, 0);
	}

	//------------------------------------------------------------------------------------------------
	//! Poll until possession lands (200 ms × 25 = 5 s ceiling), then announce the spawn.
	protected void FinalizeSpawnWhenControlled(int playerId, int attempt)
	{
		IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (!controlled)
		{
			if (attempt < 25)
				GetGame().GetCallqueue().CallLater(FinalizeSpawnWhenControlled, 200, false, playerId, attempt + 1);
			else
				Print(string.Format("[TBD][Spawn] player=%1 never took control of its body — spawn not announced", playerId), LogLevel.WARNING);
			return;
		}

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(GetOwner());
		if (gm)
			gm.GetOnPlayerSpawned().Invoke(playerId, controlled);
		else
			OnPlayerSpawnedHook(playerId, controlled);
	}

	//------------------------------------------------------------------------------------------------
	//! True when a materialized body is destroyed/dead (corpse — respawn replaces it).
	//! T-181.10: a character with NO controller counts as dead too. It used to count as
	//! alive, so a half-torn-down body could be handed to a player with no re-equip — the
	//! exact "inherits the previous life" case the operator locked out. Rematerializing is
	//! always the safe answer here: the worst case is one extra fresh dressed body.
	protected static bool IsBodyDead(IEntity body)
	{
		ChimeraCharacter character = ChimeraCharacter.Cast(body);
		if (!character)
			return true;
		CharacterControllerComponent ccc = character.GetCharacterController();
		if (!ccc)
			return true;
		return ccc.IsDead();
	}

	//------------------------------------------------------------------------------------------------
	//! A2 — subscribe the spawn invoker (SCR_BaseGameModeComponent has no
	//! OnPlayerSpawned virtual in 1.7 — measured compile error; the vanilla
	//! SCR_BaseGameMode ScriptInvoker is the supported seam).
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(owner);
		if (gm)
			gm.GetOnPlayerSpawned().Insert(OnPlayerSpawnedHook);
	}

	//------------------------------------------------------------------------------------------------
	override void OnDelete(IEntity owner)
	{
		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(owner);
		if (gm)
			gm.GetOnPlayerSpawned().Remove(OnPlayerSpawnedHook);

		super.OnDelete(owner);
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn-notify sink: fired by NotifySpawnedManually on every bind (and by the
	//! vanilla invoker for any non-framework spawn). Bookkeeping only — dressing is
	//! owned by materialization (SpawnSlotBody dresses both initial and respawn
	//! bodies), so no equip runs here; the reaper died with the vanilla RequestSpawn
	//! pipeline (nothing can double-spawn any more).
	//! @authority server
	protected void OnPlayerSpawnedHook(int playerId, IEntity controlledEntity)
	{
		if (RplSession.Mode() == RplMode.Client || !controlledEntity)
			return;

		m_mSpawnSeen.Set(playerId, true);
		// T-181.21 — the spawn landed, so the ticket has done its job. Closing it here rather
		// than waiting for the timeout keeps the authorized window as small as the pipeline
		// allows.
		RevokeSpawnAuthorization(playerId);
		GetGame().GetCallqueue().CallLater(LogDeployedTransform, 500, false, playerId);
		ScheduleCensus();
	}

	//------------------------------------------------------------------------------------------------
	//! A6 — death re-arms the deploy guard; the slot assignment survives, so the next
	//! deploy finds the slot body dead and REMATERIALIZES a fresh dressed one at the
	//! slot transform (operator-locked re-equip-every-spawn; corpse stays). (1.7
	//! component virtual takes SCR_InstigatorContextData — the CRF Rally precedent.)
	//! @authority server
	override void OnPlayerKilled(notnull SCR_InstigatorContextData instigatorContextData)
	{
		super.OnPlayerKilled(instigatorContextData);

		if (RplSession.Mode() == RplMode.Client)
			return;

		int playerId = instigatorContextData.GetVictimPlayerID();
		if (playerId <= 0)
			return;

		m_mDeployRequested.Remove(playerId);
		m_mRetryCount.Remove(playerId);
		m_mSpawnSeen.Remove(playerId);
		// T-181.21 — close any spawn ticket this player still holds. A request authorized a
		// moment before the kill must not be honoured by the vanilla authority after it.
		RevokeSpawnAuthorization(playerId);

		// T-181.11 — ONE LIFE: death is terminal. The slot deliberately STAYS claimed so the
		// seat is not recycled and a reconnecting player still resolves to it. Only
		// AdminRespawn() can clear this.
		//
		// T-181.21 — the mark goes in FIRST, before anything else can run. Every door into the
		// world funnels through DeployPlayerEx, which reads it; a retry already sitting in the
		// callqueue from before the kill (ScheduleDeployRetry fires ~500 ms later and used to
		// carry no death check at all) therefore finds a spent life when it lands and is
		// refused with DENIED, which also ends the retry loop.
		if (m_bOneLife)
		{
			MarkLifeSpent(playerId);
			m_mAdminRespawnPending.Remove(playerId);
			Print(string.Format("[TBD][Spawn] player=%1 KILLED — one life spent (key=%2), slot retained, awaiting admin",
				playerId, PlayerBindKey(playerId)));
			return;
		}

		Print(string.Format("[TBD][Spawn] player=%1 killed — re-armed for respawn (slot retained)", playerId));

		// Re-arming alone used to be enough because the vanilla deploy menu asked again;
		// it is stood down now, so the framework drives the next life itself.
		if (m_bAutoDeploy)
			GetGame().GetCallqueue().CallLater(RedeployAfterDeath, m_iRedeployDelayMs, false, playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.11 — has this player spent their one life?
	//! T-181.21 — resolved through the DURABLE key, so quitting and rejoining under a fresh
	//! numeric playerId does not clear the record.
	bool IsPlayerDead(int playerId)
	{
		return IsBindKeyDead(PlayerBindKey(playerId));
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — the raw lookup, for callers that already hold the key (disconnect teardown,
	//! where resolving it again would be a second chance to get it wrong).
	protected bool IsBindKeyDead(string bindKey)
	{
		return m_mDeadPlayers && m_mDeadPlayers.Contains(bindKey);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — spend the life. Recorded against the durable key; the log line names the key
	//! so a "why is this player still dead / not dead" question can be answered from the log.
	protected void MarkLifeSpent(int playerId)
	{
		string bindKey = PlayerBindKey(playerId);
		m_mDeadPlayers.Set(bindKey, true);
		if (!IsDurableKey(bindKey))
			NoteIdentityDegraded(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — give the life back. The ONLY caller is the admin route, and only after a body
	//! is genuinely in the player's hands (see AdminRespawn) — never speculatively, because a
	//! deploy that then fails would leave a player neither dead nor deployed.
	protected void ClearLifeSpent(int playerId)
	{
		m_mDeadPlayers.Remove(PlayerBindKey(playerId));
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.11 — how many of a faction are still alive. The primitive a side-eliminated win
	//! condition (T-181.13) reads; counts CLAIMED slots, so a player who never deployed still
	//! counts as alive and cannot silently end the round.
	//!
	//! T-181.21 — m_mDepartedSlots is deliberately NOT walked: every seat in it belongs to a
	//! spent life by construction, so it contributes zero alive. It does count as CLAIMED
	//! below, which is the whole point.
	int CountAliveForFaction(string factionKey)
	{
		int alive;
		foreach (int playerId, TBD_MissionSlotStruct slot : m_mPlayerSlot)
		{
			if (!slot || slot.faction != factionKey)
				continue;
			if (!IsPlayerDead(playerId))
				alive++;
		}
		return alive;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.11 — how many slots of a faction are claimed at all (alive or dead). The win
	//! evaluator needs this to tell "eliminated" apart from "nobody ever played this side" —
	//! without it, an unfielded faction would end the round the instant it started.
	//!
	//! T-181.21 — now includes the seats of players who died and then quit. Without them the
	//! last man of a side dying and leaving dropped his faction to 0 claimed, TickWinConditions
	//! read that as "never fielded", and the round could never end.
	int CountClaimedForFaction(string factionKey)
	{
		int claimed;
		foreach (int playerId, TBD_MissionSlotStruct slot : m_mPlayerSlot)
		{
			if (slot && slot.faction == factionKey)
				claimed++;
		}
		foreach (string bindKey, TBD_MissionSlotStruct departed : m_mDepartedSlots)
		{
			if (departed && departed.faction == factionKey)
				claimed++;
		}
		return claimed;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.11.1 — ADMIN RESPAWN. The escape hatch for a glitch death: clears the one-life
	//! mark and rematerializes a fresh dressed body on the player's own slot (re-equip every
	//! spawn — operator-locked; the old corpse stays where it fell).
	//!
	//! Deliberately NOT a normal respawn path: it is server-authority only, refuses players who
	//! are not actually dead, and writes an audit line so every use is visible in the log. The
	//! caller is responsible for permission (see TBD_AdminCommands).
	//!
	//! T-181.21 — the one-life mark is now cleared only AFTER a body is genuinely in the
	//! player's hands. It used to be cleared up front, before DeployPlayerEx ran, and only
	//! re-applied on FAILED — so a RETRY (or the retry loop later giving up) left the player
	//! neither dead nor deployed: invisible to `#tbd dead`, uncounted by CountAliveForFaction
	//! (which reported them ALIVE and could hang the round open), and un-respawnable because
	//! AdminRespawn refuses anyone "not dead". Dead until proven deployed is the correct order.
	//! @authority server
	TBD_EDeployResult AdminRespawn(int playerId, string byAdmin = "unknown")
	{
		if (RplSession.Mode() == RplMode.Client)
			return TBD_EDeployResult.NOT_MINE;

		if (!IsPlayerDead(playerId))
		{
			Print(string.Format("[TBD][Admin] respawn REFUSED player=%1 by=%2 — not dead", playerId, byAdmin), LogLevel.WARNING);
			return TBD_EDeployResult.ALREADY;
		}

		if (!GetGame().GetPlayerManager().GetPlayerController(playerId))
		{
			Print(string.Format("[TBD][Admin] respawn REFUSED player=%1 by=%2 — disconnected", playerId, byAdmin), LogLevel.WARNING);
			return TBD_EDeployResult.FAILED;
		}

		// Re-arm the deploy bookkeeping, but NOT the life. The life is the last thing to move.
		m_mDeployRequested.Remove(playerId);
		m_mRetryCount.Remove(playerId);
		m_mSpawnSeen.Remove(playerId);

		// T-181.10 — forceFreshBody: the admin route always gets a NEWLY materialized,
		// newly dressed body from the mission JSON. Never the one the spent life left behind.
		// T-181.21 — adminOverride: the documented, opt-in bypass of the one-life boundary in
		// DeployPlayerEx. This call site and the retry it owns are the only two that pass it.
		TBD_EDeployResult r = DeployPlayerEx(playerId, true, true);
		Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 result=%3",
			playerId, byAdmin, typename.EnumToString(TBD_EDeployResult, r)));

		FinishAdminRespawn(playerId, r, byAdmin);
		return r;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — settle an admin respawn attempt. DEPLOYED is the ONLY outcome that gives the
	//! life back; RETRY keeps the player dead and hands the admin override to the retry so the
	//! retry is not refused by the guard the admin just overrode; anything else leaves them dead
	//! (which they already were — nothing was speculatively cleared) with a loud line saying so.
	protected void FinishAdminRespawn(int playerId, TBD_EDeployResult r, string byAdmin)
	{
		if (r == TBD_EDeployResult.DEPLOYED)
		{
			ClearLifeSpent(playerId);
			m_mAdminRespawnPending.Remove(playerId);
			Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 — back in the world, life restored", playerId, byAdmin));
			return;
		}

		if (r == TBD_EDeployResult.RETRY)
		{
			m_mAdminRespawnPending.Set(playerId, true);
			ScheduleDeployRetry(playerId);
			Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 — RETRY queued, player stays DEAD until a body lands", playerId, byAdmin));
			return;
		}

		m_mAdminRespawnPending.Remove(playerId);
		Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 did NOT deploy (%3) — player REMAINS dead, run '#tbd respawn %1' again",
			playerId, byAdmin, typename.EnumToString(TBD_EDeployResult, r)), LogLevel.ERROR);
	}

	//------------------------------------------------------------------------------------------------
	//! Puts a killed player back on his slot: DeployPlayerEx finds the slot body dead and
	//! rematerializes a fresh dressed one (re-equip every spawn — operator-locked; the
	//! corpse stays where it fell).
	//! @authority server
	protected void RedeployAfterDeath(int playerId)
	{
		if (!GetGame().GetPlayerManager().GetPlayerController(playerId))
			return;  // Disconnected during the respawn beat.

		if (m_mDeployRequested.Contains(playerId))
			return;  // Already back in the world by another path.

		TBD_EDeployResult r = DeployPlayerEx(playerId);
		Print(string.Format("[TBD][Spawn] path=redeploy player=%1 result=%2", playerId, typename.EnumToString(TBD_EDeployResult, r)));
		if (r == TBD_EDeployResult.RETRY)
			ScheduleDeployRetry(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! A6 — disconnect clears the per-player deploy state; the identity → slot pairing is
	//! remembered so a reconnecting player (dedicated servers reuse numeric playerIds)
	//! reclaims the same slot ahead of roster/round-robin.
	//!
	//! T-181.21 — the slot is now RETAINED when the holder died first, moved into
	//! m_mDepartedSlots under their DURABLE key. It used to be removed unconditionally, which
	//! is precisely what ReleaseSlot refuses to do for a dead player, and it had a nasty
	//! second-order effect: TBD_FrameworkManager.TickWinConditions skips any faction with 0
	//! CLAIMED slots ("never fielded ≠ eliminated"), so the last man of a side dying and then
	//! quitting erased his side from the count and the round could never end. Retaining the
	//! seat keeps the side fielded and keeps the man counted as dead, so the elimination fires
	//! exactly when it should, and keeps the seat off the market — the ONE LIFE rule ("nobody
	//! else takes your seat") applied consistently instead of only while the corpse is still
	//! connected. Identity-keyed, so the next player handed his recycled numeric playerId
	//! inherits nothing.
	//! @authority server
	override void OnPlayerDisconnected(int playerId, KickCauseCode cause, int timeout)
	{
		super.OnPlayerDisconnected(playerId, cause, timeout);

		if (RplSession.Mode() == RplMode.Client)
			return;

		// Resolve the durable key BEFORE the engine finishes tearing the player down — after
		// that the identity lookup stops answering and only the cache can.
		string bindKey = PlayerBindKey(playerId);
		bool lifeSpent = m_bOneLife && IsBindKeyDead(bindKey);

		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		string slotKey = "-";
		if (slot)
		{
			slotKey = slot.Key();
			if (IsDurableKey(bindKey))
				m_mIdentityReclaim.Set(bindKey, slot.id);
		}

		m_mDeployRequested.Remove(playerId);
		m_mRetryCount.Remove(playerId);
		m_mSpawnSeen.Remove(playerId);
		m_mAdminRespawnPending.Remove(playerId);
		RevokeSpawnAuthorization(playerId);
		m_mDenyLogged.Remove(playerId);

		if (lifeSpent && slot)
			m_mDepartedSlots.Set(bindKey, slot);

		// The numeric id is released back to the server here, so everything keyed on it goes
		// with it — including the bind-key cache, or the next holder of that number would
		// resolve to this player's identity.
		m_mPlayerSlot.Remove(playerId);
		m_mBindKeyCache.Remove(playerId);

		if (lifeSpent)
			Print(string.Format("[TBD][Spawn] player=%1 disconnected after spending their life — slot %2 RETAINED under key %3 (seat not recyclable, side stays fielded)",
				playerId, slotKey, bindKey));
		else
			Print(string.Format("[TBD][Spawn] player=%1 disconnected — state cleared, slot reclaim recorded", playerId));
	}

	//------------------------------------------------------------------------------------------------
	//! A7 — settle census (~5 s after the first spawn of a wave): the orphan-body
	//! oracle. characters != players means a duplicate/abandoned body exists.
	protected void ScheduleCensus()
	{
		if (m_bCensusScheduled)
			return;
		m_bCensusScheduled = true;
		GetGame().GetCallqueue().CallLater(RunCensus, 5000, false);
	}

	//------------------------------------------------------------------------------------------------
	protected void RunCensus()
	{
		m_iCensusCount = 0;
		BaseWorld world = GetGame().GetWorld();
		if (world)
			world.QueryEntitiesByAABB(Vector(-1000, -2000, -1000), Vector(20000, 4000, 20000), CensusAddEntity);

		array<int> players = {};
		int playerCount = GetGame().GetPlayerManager().GetPlayers(players);
		Print(string.Format("[TBD][Audit] characters=%1 bodies=%2 players=%3", m_iCensusCount, m_mSlotBodies.Count(), playerCount));
		m_bCensusScheduled = false;
	}

	//------------------------------------------------------------------------------------------------
	protected bool CensusAddEntity(IEntity ent)
	{
		if (ChimeraCharacter.Cast(ent))
			m_iCensusCount++;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! A1 — pull-path retry for transient RETRY results (500 ms cadence, cap 20 = 10 s;
	//! cap-hit logs ERROR and stops — the vanilla wait screen keeps the player parked).
	void ScheduleDeployRetry(int playerId)
	{
		GetGame().GetCallqueue().CallLater(RetryDeploy, 500, false, playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.21 — two things changed here.
	//!
	//! (1) THE DEATH RACE. A retry queued before a kill still fires ~500 ms after it, and used
	//! to redeploy the player with no death check anywhere on the path. It no longer can:
	//! DeployPlayerEx refuses a spent life with DENIED, and DENIED (being != RETRY) also ends
	//! the loop. No extra check is needed here, and adding one would put the invariant back in
	//! two places — which is how it got lost the first time.
	//!
	//! (2) THE ADMIN RESPAWN. When the retry belongs to an admin respawn it must carry the
	//! override, otherwise the one attempt that is allowed past the guard would be refused by
	//! it, and the give-up at the cap must be attributed — it used to just stop, leaving an
	//! admin who typed `#tbd respawn` with no idea nothing happened.
	protected void RetryDeploy(int playerId)
	{
		bool adminRespawn = m_mAdminRespawnPending.Contains(playerId);

		int n;
		m_mRetryCount.Find(playerId, n);
		if (n >= 20)
		{
			Print(string.Format("[TBD][Spawn] path=retry player=%1 gave up after %2 attempts", playerId, n), LogLevel.ERROR);
			m_mRetryCount.Remove(playerId);
			if (adminRespawn)
			{
				m_mAdminRespawnPending.Remove(playerId);
				Print(string.Format("[TBD][Admin] respawn player=%1 gave up after %2 attempts — player REMAINS dead, run '#tbd respawn %1' again",
					playerId, n), LogLevel.ERROR);
			}
			return;
		}
		m_mRetryCount.Set(playerId, n + 1);

		TBD_EDeployResult r = DeployPlayerEx(playerId, adminRespawn, adminRespawn);
		Print(string.Format("[TBD][Spawn] path=retry player=%1 attempt=%2 admin=%3 result=%4",
			playerId, n + 1, adminRespawn, typename.EnumToString(TBD_EDeployResult, r)));

		if (r == TBD_EDeployResult.RETRY)
		{
			ScheduleDeployRetry(playerId);
			return;
		}

		m_mRetryCount.Remove(playerId);
		if (adminRespawn)
			FinishAdminRespawn(playerId, r, "retry");
	}

	//------------------------------------------------------------------------------------------------
	//! A1 watchdog — a DEPLOYED request whose spawn never arrived re-arms the player.
	//! Spawn-seen is marked by the transform log today (A2 moves it to OnPlayerSpawned).
	protected void CheckSpawnArrived(int playerId)
	{
		if (m_mSpawnSeen.Contains(playerId))
			return;
		if (GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId))
			return;

		Print(string.Format("[TBD][Spawn] watchdog player=%1 — spawn request never materialized, re-arming", playerId), LogLevel.WARNING);
		m_mDeployRequested.Remove(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! Post-deploy diagnostic (T-092.1): logs the spawned character's actual feet height
	//! against the live terrain — groundDelta is the measured capsule/ground offset on a
	//! human character spawn, the calibration source for CAPSULE_GROUND_OFFSET_M.
	protected void LogDeployedTransform(int playerId)
	{
		IEntity ent = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (!ent)
		{
			Print(string.Format("[TBD][Spawn] deployed player=%1 — no controlled entity yet (spawn pending?)", playerId), LogLevel.WARNING);
			return;
		}
		m_mSpawnSeen.Set(playerId, true);

		vector org = ent.GetOrigin();
		float surfaceY = GetGame().GetWorld().GetSurfaceY(org[0], org[2]);
		float groundDelta = org[1] - surfaceY;
		float yaw = ent.GetYawPitchRoll()[0];

		string slotId = "-";
		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (slot)
			slotId = slot.id;

		Print(string.Format("[TBD][Spawn] deployed player=%1 slot=%2 pos=%3 feetY=%4 surfaceY=%5 groundDelta=%6 yaw=%7",
			playerId, slotId, org.ToString(), org[1], surfaceY, groundDelta, yaw));
	}
}
