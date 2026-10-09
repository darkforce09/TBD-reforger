/**
 * @file TBD_PlayAreaComponent.c
 * @brief Game mode component that confines players to the mission's play area.
 *
 * Role: one 1 Hz server tick over the connected players while the stage is LIVE: finds the
 * boundary or base-protection zone each player violates, counts the grace down, warns on the
 * zone's cadence and applies its penalty once.  Position: attached by `TBD_GameMode.et`; builds and
 * clears `TBD_ZoneRegistry`; hands warnings and penalties to `TBD_PlayAreaPenalties`.
 * State: the per-player `TBD_PlayAreaViolation` map, owned by the server component.
 * Invariants: one tick re-reads the live player list, so a departed player's row never reaches a recycled id;
 * enforcement stands down without a spawn manager rather than police a spectator's host.
 */

//! Editor class of `TBD_PlayAreaComponent`.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD play area -- boundary / base-protection zones, out-of-bounds warning, grace and penalty.")]
class TBD_PlayAreaComponentClass : SCR_BaseGameModeComponentClass {}

//! Server-authoritative play-area enforcement: warning, grace countdown and JSON-driven penalty,
//! `warn` unless the zone authors otherwise.
class TBD_PlayAreaComponent : SCR_BaseGameModeComponent
{
	static const int TICK_MS = 1000; //!< enforcement cadence in milliseconds
	static const float TICK_SECONDS = 1.0; //!< `TICK_MS` in seconds, the grace accumulator step

	static const string ANNOUNCE_NO_BOUNDARY_KEY = "PlayArea.noBoundary"; //!< `TBD_AnnounceOnce` key of the unrestricted line
	static const string ANNOUNCE_ARMED_KEY = "PlayArea.armed"; //!< `TBD_AnnounceOnce` key of the armed line
	static const string ANNOUNCE_NO_SPAWN_MANAGER_KEY = "PlayArea.noSpawnManager"; //!< `TBD_AnnounceOnce` key of the stood-down line

	protected ref map<int, ref TBD_PlayAreaViolation> m_mViolations; //!< player id to open violation

	//! Create the violation map, rearm the once-per-world log lines and, off a client, arm the tick.
	//! @param owner the game mode entity
	//! @authority server
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		m_mViolations = new map<int, ref TBD_PlayAreaViolation>();
		TBD_AnnounceOnce.Rearm(ANNOUNCE_NO_BOUNDARY_KEY);
		TBD_AnnounceOnce.Rearm(ANNOUNCE_ARMED_KEY);
		TBD_AnnounceOnce.Rearm(ANNOUNCE_NO_SPAWN_MANAGER_KEY);

		if (TBD_Authority.IsClient())
			return;

		GetGame().GetCallqueue().CallLater(Tick, TICK_MS, true);
	}

	//! Cancel the tick and clear the zone registry and violations, since statics outlive a world
	//! when a mission restarts in-process.
	//! @param owner the game mode entity
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		TBD_ZoneRegistry.Clear();
		if (m_mViolations)
			m_mViolations.Clear();

		super.OnDelete(owner);
	}

	//! One enforcement pass: build the registry when needed, drop every countdown outside LIVE,
	//! prune departed players, then evaluate each connected player.
	//! @authority server
	protected void Tick()
	{
		// The mission loads after this component exists; retry until the registry builds.
		if (!TBD_ZoneRegistry.IsBuilt())
		{
			if (!TBD_ZoneRegistry.Build())
				return;

			AnnounceOnce();
		}

		// Only LIVE is enforced; safe start and the other stages are not.
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
		{
			// Drop countdowns so none resumes half-way when LIVE returns.
			if (m_mViolations.Count() > 0)
				m_mViolations.Clear();
			return;
		}

		// Nothing to enforce: no usable boundary and no usable base-protection zone in the mission.
		if (TBD_ZoneRegistry.GetBoundaryCount() == 0 && TBD_ZoneRegistry.GetBaseProtectionCount() == 0)
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		// Prune first, so a departed player's row is never inherited by a recycled id.
		PruneDeparted(connected);

		foreach (int playerId : connected)
		{
			EvaluatePlayer(players, playerId);
		}
	}

	//! Log once per world, when the registry builds, that no play-area restriction is in force or
	//! the `armed` summary.
	protected void AnnounceOnce()
	{
		if (TBD_ZoneRegistry.GetBoundaryCount() == 0)
		{
			TBD_AnnounceOnce.Event(TBD_ZoneRegistry.CH, ANNOUNCE_NO_BOUNDARY_KEY,
				"no usable boundary zone in this mission -- NO play-area restriction is in force");
			return;
		}

		TBD_AnnounceOnce.Kv(TBD_ZoneRegistry.CH, ANNOUNCE_ARMED_KEY, "armed", string.Format("boundary=%1 baseProtection=%2 cadence=%3ms",
			TBD_ZoneRegistry.GetBoundaryCount(), TBD_ZoneRegistry.GetBaseProtectionCount(), TICK_MS));
	}

	//! Drop the violation rows of players absent from `connected`, collected first, removed by key.
	//! @param connected the live player ids
	protected void PruneDeparted(notnull array<int> connected)
	{
		if (m_mViolations.Count() == 0)
			return;

		array<int> stale = new array<int>();
		foreach (int playerId, TBD_PlayAreaViolation v : m_mViolations)
		{
			if (connected.Find(playerId) == -1)
				stale.Insert(playerId);
		}

		foreach (int playerId : stale)
		{
			m_mViolations.Remove(playerId);
		}
	}

	//! Check one player: no body, no spawn manager (logged once), a spent life or a dead body
	//! clears their row; otherwise find their violation and clear or accumulate it.
	//! @param players the player manager
	//! @param playerId the player
	//! @authority server
	protected void EvaluatePlayer(notnull PlayerManager players, int playerId)
	{
		IEntity body = players.GetPlayerControlledEntity(playerId);

		// No body (lobby, spectating, mid-deploy): a fresh deploy always starts clean.
		if (!body)
		{
			m_mViolations.Remove(playerId);
			return;
		}

		// Without the spawn manager a spent life cannot be told from a living one, so a spectator's
		// streaming host would be policed as a body: stand down.
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			if (TBD_AnnounceOnce.Claim(ANNOUNCE_NO_SPAWN_MANAGER_KEY))
			{
				Print("[TBD][playarea] enforcement STOOD DOWN -- framework world with no TBD_SpawnManager (cannot tell a dead player from a live one, so a spectator's streaming host would be policed as a body)", LogLevel.ERROR);
			}

			m_mViolations.Remove(playerId);
			return;
		}

		TBD_AnnounceOnce.Rearm(ANNOUNCE_NO_SPAWN_MANAGER_KEY);

		if (spawn.IsPlayerDead(playerId))
		{
			m_mViolations.Remove(playerId);
			return;
		}

		if (TBD_CharacterUtil.IsDead(body))
		{
			m_mViolations.Remove(playerId);
			return;
		}

		string factionKey = TBD_PlayerFaction.Of(spawn, playerId);
		vector origin = body.GetOrigin();
		float px = origin[0];
		float pz = origin[2];

		TBD_Zone violated = FindViolation(factionKey, px, pz);
		if (!violated)
		{
			ClearViolation(playerId);
			return;
		}

		AccumulateViolation(playerId, body, violated);
	}

	//! The zone this player violates: outside the governing boundary first, then inside another
	//! side's base protection.
	//! @param factionKey the player's side; may be empty
	//! @param px world X in metres
	//! @param pz world Z in metres
	//! @return the violated zone, or null when the player is where they belong
	protected TBD_Zone FindViolation(string factionKey, float px, float pz)
	{
		if (TBD_ZoneRegistry.HasBoundaryFor(factionKey) && !TBD_ZoneRegistry.IsInsideBoundary(factionKey, px, pz))
			return TBD_ZoneRegistry.GoverningBoundary(factionKey);

		return TBD_ZoneRegistry.FindViolatedProtection(factionKey, px, pz);
	}

	//! Drop a player's violation row; a player who was warned is told they are back inside.
	//! @param playerId the player
	protected void ClearViolation(int playerId)
	{
		TBD_PlayAreaViolation state = m_mViolations.Get(playerId);
		if (!state)
			return;

		m_mViolations.Remove(playerId);

		if (state.m_bWarned)
			TBD_PlayerChat.Tell(playerId, "TBD: back inside the play area.");

		TBD_Log.Kv(TBD_ZoneRegistry.CH, "returned", string.Format("player=%1 zone=%2 outsideFor=%3s",
			playerId, state.m_sZoneKey, state.m_fSecondsOutside));
	}

	//! Advance a player's violation: a new, changed-zone or new-body violation restarts the
	//! countdown with a warning; otherwise count one tick, warn on the zone's cadence and apply the
	//! penalty once when the grace expires.
	//! @param playerId the player
	//! @param body the player's controlled entity
	//! @param zone the violated zone
	protected void AccumulateViolation(int playerId, notnull IEntity body, notnull TBD_Zone zone)
	{
		string zoneKey = zone.LogKey();
		EntityID bodyId = body.GetID();

		TBD_PlayAreaViolation state = m_mViolations.Get(playerId);

		if (!state || state.m_sZoneKey != zoneKey || state.m_LastBody != bodyId)
		{
			state = new TBD_PlayAreaViolation();
			state.m_sZoneKey = zoneKey;
			state.m_LastBody = bodyId;
			m_mViolations.Set(playerId, state);

			TBD_Log.Kv(TBD_ZoneRegistry.CH, "violation", string.Format("player=%1 zone=%2 grace=%3s penalty=%4",
				playerId, zoneKey, zone.m_fGraceSeconds, typename.EnumToString(TBD_EZonePenalty, zone.m_ePenalty)));

			TBD_PlayAreaPenalties.WarnPlayer(playerId, state, zone, zone.m_fGraceSeconds);
			state.m_fSecondsSinceWarned = 0;
			return;
		}

		// Counting starts the tick after detection: the full grace plus up to one tick.
		state.m_fSecondsOutside += TICK_SECONDS;

		// The penalty latches: never applied twice, and a warn zone stops nagging after expiry.
		if (state.m_bPenaltyApplied)
			return;

		float remaining = zone.m_fGraceSeconds - state.m_fSecondsOutside;
		if (remaining <= 0)
		{
			state.m_bPenaltyApplied = true;
			TBD_PlayAreaPenalties.ApplyPenalty(playerId, body, zone);
			return;
		}

		state.m_fSecondsSinceWarned += TICK_SECONDS;
		if (state.m_fSecondsSinceWarned >= zone.m_fWarnEverySeconds)
		{
			state.m_fSecondsSinceWarned = 0;
			TBD_PlayAreaPenalties.WarnPlayer(playerId, state, zone, remaining);
		}
	}

}
