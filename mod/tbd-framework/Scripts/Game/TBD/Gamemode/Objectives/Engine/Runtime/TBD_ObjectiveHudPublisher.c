/**
 * @file TBD_ObjectiveHudPublisher.c
 * @brief Sends each player the objective board and capture bar they may see, only when it changed.
 *
 * Role: renders every player's HUD snapshot (row glyph, title, detail, capture bar) from their
 * own side's view and pushes it over the Reliable Owner RPC behind
 * `SCR_PlayerController.TBD_PushObjectiveHud`.  Position: owned by `TBD_ObjectivesComponent`,
 * which calls `Replicate` every LIVE tick, `HideAll` when LIVE ends, and `PushTo` for a client's
 * pull; reads `TBD_ObjectiveText`, `TBD_ObjectiveKindBehaviour` and `TBD_PlayerFaction`.
 * State: the signature of the snapshot each player was last sent, keyed by player id, owned by
 * the component on the server.  Invariants: the 1 Hz push is sent only when a player's rendered
 * snapshot differs from their last one; a player with no record is always sent a full board; a
 * record is written only after a push actually left; records never outlive a connection, so a
 * recycled player id never inherits a frame.
 */

//! Per-player objective HUD delivery with change gating.
class TBD_ObjectiveHudPublisher : Managed
{
	protected ref map<int, string> m_mHudSignatures; //!< per-player signature of the last snapshot actually sent; bounded by concurrent players

	//! Allocate the signature map.
	void TBD_ObjectiveHudPublisher()
	{
		m_mHudSignatures = new map<int, string>();
	}

	//! Drop one player's record, so the next holder of their id gets a full board.
	void Forget(int playerId)
	{
		m_mHudSignatures.Remove(playerId);
	}

	//! Drop every record.
	void ForgetAll()
	{
		m_mHudSignatures.Clear();
	}

	//! Push the HUD snapshot to every connected player whose rendered frame changed since their
	//! last push; prunes departed players first.
	//! @param board the prepared objectives
	//! @authority server
	void Replicate(notnull PlayerManager players, notnull array<int> connected, notnull array<ref TBD_Objective> board)
	{
		PruneHudSignatures(connected);

		foreach (int playerId : connected)
		{
			array<string> icons = new array<string>();
			array<string> titles = new array<string>();
			array<string> details = new array<string>();
			string barLabel;
			int barPercent;
			int barVisible;

			FillHudSnapshot(playerId, board, icons, titles, details, barLabel, barPercent, barVisible);

			string signature = HudSignature(icons, titles, details, barLabel, barPercent, barVisible);
			if (!HudChanged(playerId, signature))
				continue;

			// Recorded ONLY when the RPC actually went out. A connected player with no controller yet
			// is skipped by the push, and recording here would leave them holding a frame they were
			// never sent and would never be offered again.
			if (PushHudSnapshot(players, playerId, icons, titles, details, barLabel, barPercent, barVisible, 1))
				m_mHudSignatures.Set(playerId, signature);
		}
	}

	//! Whether this player's snapshot differs from the last one they were sent. A player with no
	//! record reads as changed, so a joiner, a recycled id or a hidden HUD gets the whole board.
	protected bool HudChanged(int playerId, string signature)
	{
		string previous;
		if (!m_mHudSignatures.Find(playerId, previous))
			return true;

		return previous != signature;
	}

	//! One string for exactly what the RPC carries: the bar triple and every row. Each field is
	//! length-prefixed, so two different snapshots can never produce the same signature; the row
	//! count is clamped to the shortest array.
	protected string HudSignature(notnull array<string> icons, notnull array<string> titles,
		notnull array<string> details, string barLabel, int barPercent, int barVisible)
	{
		string signature = string.Format("%1|%2|%3|%4", barVisible, barPercent, barLabel.Length(), barLabel);

		int rows = icons.Count();
		if (titles.Count() < rows)
			rows = titles.Count();
		if (details.Count() < rows)
			rows = details.Count();

		for (int i = 0; i < rows; i++)
		{
			string icon = icons.Get(i);
			string title = titles.Get(i);
			string detail = details.Get(i);
			signature += string.Format("|%1|%2|%3|%4|%5|%6",
				icon.Length(), icon, title.Length(), title, detail.Length(), detail);
		}

		return signature;
	}

	//! Drop the records of players who have disconnected. Keys are collected first and removed
	//! after, because a map is not mutated while it is iterated.
	protected void PruneHudSignatures(notnull array<int> connected)
	{
		if (m_mHudSignatures.Count() == 0)
			return;

		array<int> stale = new array<int>();
		foreach (int playerId, string signature : m_mHudSignatures)
		{
			if (connected.Find(playerId) == -1)
				stale.Insert(playerId);
		}

		foreach (int playerId : stale)
		{
			m_mHudSignatures.Remove(playerId);
		}
	}

	//! Close the HUD on every connected client (LIVE ended, or the component is tearing down).
	//! @authority server
	void HideAll()
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);
		array<ref TBD_Objective> board = new array<ref TBD_Objective>();
		foreach (int playerId : connected)
		{
			PushHudToPlayer(players, playerId, board, 0);
		}
	}

	//! Push the current board to one player, ungated: a client's pull must never be suppressed.
	//! @authority server
	void PushTo(int playerId)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<ref TBD_Objective> board = TBD_ObjectiveRegistry.GetAll();
		if (!board)
			board = new array<ref TBD_Objective>();

		PushHudToPlayer(players, playerId, board, 1);
	}

	//! The ungated push behind `HideAll` and `PushTo`. It resets what the player is known to hold:
	//! a hide drops the record, a show records the frame sent, so re-entering LIVE on an unchanged
	//! board still reopens the panel.
	//! @param show 1 opens the HUD, 0 closes it
	protected void PushHudToPlayer(notnull PlayerManager players, int playerId, notnull array<ref TBD_Objective> board, int show)
	{
		array<string> icons = new array<string>();
		array<string> titles = new array<string>();
		array<string> details = new array<string>();
		string barLabel;
		int barPercent;
		int barVisible;

		FillHudSnapshot(playerId, board, icons, titles, details, barLabel, barPercent, barVisible);

		if (!PushHudSnapshot(players, playerId, icons, titles, details, barLabel, barPercent, barVisible, show))
			return;

		if (show == 0)
			m_mHudSignatures.Remove(playerId);
		else
			m_mHudSignatures.Set(playerId, HudSignature(icons, titles, details, barLabel, barPercent, barVisible));
	}

	//! Send one snapshot over `SCR_PlayerController.TBD_PushObjectiveHud`.
	//! @return false when the player has no controller (lobby, spectating, mid-deploy), so the
	//! caller records nothing that never left
	protected bool PushHudSnapshot(notnull PlayerManager players, int playerId, notnull array<string> icons,
		notnull array<string> titles, notnull array<string> details, string barLabel, int barPercent,
		int barVisible, int show)
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(playerId));
		if (!controller)
			return false;

		controller.TBD_PushObjectiveHud(icons, titles, details, barLabel, barPercent, barVisible, show);
		return true;
	}

	//! Render one player's snapshot: per objective the glyph, the side's title, and the status
	//! followed by the side's task text; the capture bar belongs to the first objective the player
	//! stands in whose kind behaviour claims the bar, and a contested one takes it over.
	protected void FillHudSnapshot(int playerId, notnull array<ref TBD_Objective> board,
		notnull array<string> icons, notnull array<string> titles, notnull array<string> details,
		out string barLabel, out int barPercent, out int barVisible)
	{
		barLabel = string.Empty;
		barPercent = 0;
		barVisible = 0;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		string factionKey = TBD_PlayerFaction.Of(spawn, playerId);

		foreach (TBD_Objective objective : board)
		{
			if (!objective)
				continue;

			icons.Insert(HudIcon(objective, factionKey));
			string title = objective.TitleFor(factionKey);
			titles.Insert(title);

			// Status first, then the side's task text; an unframed row is exactly its status.
			string detail = TBD_ObjectiveText.StatusText(objective, factionKey);
			string taskText = objective.TaskTextFor(factionKey);
			if (!taskText.IsEmpty())
			{
				if (!detail.IsEmpty())
					detail += " | ";
				detail += taskText;
			}
			details.Insert(detail);

			TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.For(objective.m_eKind);
			if (!behaviour.ClaimsCaptureBar())
				continue;
			if (objective.m_aPresentPlayers.Find(playerId) == -1)
				continue;

			bool take = barVisible == 0 || objective.m_bContested;
			if (!take)
				continue;

			barVisible = 1;
			barLabel = title;
			barPercent = objective.ProgressPercent();
		}
	}

	//! One ASCII row glyph: `.` inert, `v` complete, `!` contested, else the kind behaviour's
	//! glyph (`#` destroy, `H` hold, `o` neutral, `+` ours, `-` theirs for capture).
	protected string HudIcon(notnull TBD_Objective objective, string factionKey)
	{
		if (!objective.m_bUsable)
			return ".";
		if (objective.m_bComplete)
			return "v";
		if (objective.m_bContested)
			return "!";

		TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.For(objective.m_eKind);
		return behaviour.HudIcon(objective, factionKey);
	}
}
