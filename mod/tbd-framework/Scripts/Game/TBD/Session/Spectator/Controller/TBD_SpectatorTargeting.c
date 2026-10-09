/**
 * @file TBD_SpectatorTargeting.c
 * @brief What the spectator camera follows: free flight, follow, first person, cycling and the status line.
 *
 * Role: switches the spectator camera between free flight and following a living player in third
 * or first person, steps through the valid targets, drops a follow whose target died, and describes
 * the camera's mode in one line.
 * Position: the TBD_SpectatorScreen roster and TBD_SpectatorInputActions call the switches;
 * TBD_SpectatorController.Tick calls ValidateFollow, and Enter, Leave and Shutdown call ClearFollow
 * and Reset; targets come from TBD_SpectatorTargets; the camera from TBD_SpectatorController.
 * State: the followed player id, the first-person flag and the cycling target cache (static, client).
 * Invariants: a follow is kept as a player id, so it survives the target's entity being re-created;
 * every follow re-resolves a living entity first and falls back to free flight when there is none;
 * every switch is a no-op without a camera.
 */

//! Spectator camera targeting. Static; client only.
class TBD_SpectatorTargeting
{
	protected static int s_iFollowPlayerId = -1; //!< followed player id; -1 in free flight
	protected static bool s_bFirstPerson; //!< the follow is first person; default false
	protected static ref array<ref TBD_SpectatorTarget> s_aCycleTargets; //!< cycling cache, rebuilt on each cycle; the screen keeps its own

	//! Forget the followed player and the first-person choice. Enter and Leave call it.
	static void ClearFollow()
	{
		s_iFollowPlayerId = -1;
		s_bFirstPerson = false;
	}

	//! ClearFollow and empty the cycling cache. TBD_SpectatorController.Shutdown calls it.
	static void Reset()
	{
		ClearFollow();
		if (s_aCycleTargets)
			s_aCycleTargets.Clear();
	}

	//! @return the followed player id, or -1 when not spectating or in free flight
	static int GetFollowedPlayerId()
	{
		TBD_SpectatorCamera camera = TBD_SpectatorController.GetCamera();
		if (!TBD_SpectatorController.IsActive() || !camera || camera.GetMode() == TBD_ESpectatorCameraMode.FREE)
			return -1;

		return s_iFollowPlayerId;
	}

	//! @return true when spectating in first person
	static bool IsFirstPerson()
	{
		TBD_SpectatorCamera camera = TBD_SpectatorController.GetCamera();
		return TBD_SpectatorController.IsActive() && camera && camera.GetMode() == TBD_ESpectatorCameraMode.FIRST_PERSON;
	}

	//! One line describing what the camera is doing, for the roster's status area, so the
	//! spectator never has to guess which mode they are in.
	//! @return "Free camera -- speed xN", "First person -- name", "Following name -- ...", or empty
	//! when not spectating
	static string GetStatusLine()
	{
		TBD_SpectatorCamera camera = TBD_SpectatorController.GetCamera();
		if (!TBD_SpectatorController.IsActive() || !camera)
			return string.Empty;

		if (camera.GetMode() == TBD_ESpectatorCameraMode.FREE)
		{
			// A float divisor: an integer one would round x1.5 down to x1.
			float speed = Math.Round(camera.GetSpeedScale() * 10) / 10.0;
			return string.Format("Free camera -- speed x%1", speed);
		}

		string name = PlayerName(s_iFollowPlayerId);
		if (camera.GetMode() == TBD_ESpectatorCameraMode.FIRST_PERSON)
			return string.Format("First person -- %1", name);

		return string.Format("Following %1 -- click again for first person", name);
	}

	//! Switch to free flight, the one obvious way out of any follow.
	static void SetFree()
	{
		TBD_SpectatorCamera camera = TBD_SpectatorController.GetCamera();
		if (!camera)
			return;

		camera.SetModeFree();
		s_iFollowPlayerId = -1;
		s_bFirstPerson = false;
	}

	//! Follow a player. Re-resolves a living entity first, so a row that went stale between the
	//! refresh and the click cannot point the camera at a corpse; without one, free flight.
	//! @param playerId the player to follow
	//! @param firstPerson first person when true, third-person orbit otherwise
	//! @return false when there is no camera or the target is gone
	static bool FollowPlayer(int playerId, bool firstPerson)
	{
		TBD_SpectatorCamera camera = TBD_SpectatorController.GetCamera();
		if (!camera)
			return false;

		IEntity entity = TBD_SpectatorTargets.ResolveLivingEntity(playerId);
		if (!entity)
		{
			SetFree();
			return false;
		}

		s_iFollowPlayerId = playerId;
		s_bFirstPerson = firstPerson;
		camera.SetModeFollow(entity, firstPerson);
		return true;
	}

	//! Toggle third and first person on the current target. No-op in free flight.
	static void ToggleFirstPerson()
	{
		TBD_SpectatorCamera camera = TBD_SpectatorController.GetCamera();
		if (!camera || s_iFollowPlayerId <= 0)
			return;

		FollowPlayer(s_iFollowPlayerId, !s_bFirstPerson);
	}

	//! Step through the valid targets, wrapping at both ends. From free flight this picks the first
	//! target, so one key starts watching somebody; with no targets, free flight.
	//! @param delta +1 for the next target, -1 for the previous
	static void CycleTarget(int delta)
	{
		TBD_SpectatorCamera camera = TBD_SpectatorController.GetCamera();
		if (!camera)
			return;

		if (!s_aCycleTargets)
			s_aCycleTargets = {};

		int notInView;
		TBD_SpectatorTargets.Collect(s_aCycleTargets, notInView);

		int count = s_aCycleTargets.Count();
		if (count == 0)
		{
			SetFree();
			return;
		}

		int current = -1;
		for (int i = 0; i < count; i++)
		{
			if (s_aCycleTargets[i].m_iPlayerId == s_iFollowPlayerId)
			{
				current = i;
				break;
			}
		}

		int next;
		if (current < 0)
		{
			next = 0;
		}
		else
		{
			next = current + delta;
			// Wrap by hand: Enforce's % on a negative left operand is not worth trusting here.
			if (next >= count)
				next = 0;
			else if (next < 0)
				next = count - 1;
		}

		FollowPlayer(s_aCycleTargets[next].m_iPlayerId, s_bFirstPerson);
	}

	//! Drop a follow whose target died. A ragdoll is still an entity, so the camera's own "target
	//! vanished" guard never fires; "is that player alive" is a roster question, answered here.
	static void ValidateFollow()
	{
		TBD_SpectatorCamera camera = TBD_SpectatorController.GetCamera();
		if (!camera || camera.GetMode() == TBD_ESpectatorCameraMode.FREE)
			return;

		if (TBD_SpectatorTargets.ResolveLivingEntity(s_iFollowPlayerId))
			return;

		SetFree();
	}

	//! @param playerId the player to name
	//! @return the player's name, "Player N" when unknown, or "nobody" for an id of 0 or less
	protected static string PlayerName(int playerId)
	{
		if (playerId <= 0)
			return "nobody";

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return string.Format("Player %1", playerId);

		string name = players.GetPlayerName(playerId);
		if (name.IsEmpty())
			return string.Format("Player %1", playerId);

		return name;
	}
}
