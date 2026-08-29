//! T-181.12 - where the spectator lifecycle is hosted.
//!
//! Spectator has two halves and this component owns both:
//!   * CLIENT - a camera, a roster screen, and a poll of the local player's own body
//!     (`TBD_SpectatorController`).
//!   * SERVER - the streaming host, the inert dummy a dead player possesses so the engine keeps
//!     sending them a world to look at (`TBD_SpectatorHost`, T-181.24).
//!
//! Both need a place that starts with the world and dies with it, and in this codebase that place
//! is a component on the game mode prefab - the same seat `TBD_FrameworkManager`,
//! `TBD_SpawnManager` and `TBD_LoadoutEquipComponent` already occupy.
//!
//! Deliberately NOT a `modded class SCR_PlayerController` lifecycle. T-181.30 - there are **six**
//! such blocks in the addon today (`TBD_MissionBrowser.c`, `TBD_BriefingController.c`,
//! `TBD_LobbyController.c`, `TBD_SpectatorHost.c`, `TBD_MarkerController.c`,
//! `TBD_RadioController.c`); this header used to say three, and the number keeps moving, so treat
//! the Landmines section of `docs/mod/t181_event_mod_program.md` as the count of record rather than
//! any file header. What matters here is unchanged and is not about the count: every one of those
//! blocks is a narrow RPC transport (`TBD_SpectatorHost.c` included), because the player controller
//! is the only entity a client OWNS and can therefore send a `RplRcver.Server` message on. A
//! LIFECYCLE on top of that is a different and much wider thing, and it belongs on the game mode.
//! Deliberately NOT a bare `GameSystem` either -
//! auto-registration of a scripted system is not something the headless compile lane can prove, and
//! an unprovable lifecycle is exactly what this program refuses to ship.
//!
//! All this class does is start and stop the two managers. Every decision lives in them; this is
//! the socket, not the logic.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD spectator - free camera, follow, the unit list a dead player lives in, and the server-side streaming host that keeps the world around their camera loaded.")]
class TBD_SpectatorComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_SpectatorComponent : SCR_BaseGameModeComponent
{
	//! The game mode component graph is up well before the local player has a controller, so the
	//! start is nudged past init rather than racing it. Nothing is lost by being late: the
	//! controller polls, so it cannot miss a death that happened while it was waiting.
	static const int START_DELAY_MS = 2000;

	//! T-181.24 - the kill switch. Possession is the most invasive thing this mod does to a player
	//! controller, so there is exactly one attribute that makes the whole streaming host stand down
	//! and leaves the spectator behaving as it did before T-181.24 (camera works, streaming stays
	//! anchored to the corpse).
	[Attribute("1", desc: "Give a dead player an inert entity to possess so the server keeps streaming the world around their spectator camera. Off = the camera still works, but flying far from your corpse shows an empty world.")]
	protected bool m_bStreamingHost;

	//! T-181.24 - leave EMPTY. The built-in host is spawned BY TYPENAME with no prefab, which is
	//! what lets it work before the Workbench pass that `resourceDatabase.rdb` is waiting on.
	//! Set this only if a live test proves a REPLICATED host is required (see the
	//! `SpawnHostEntity` header in `TBD_SpectatorHost.c`), and only to a prefab whose root class is
	//! `TBD_SpectatorHostEntity`. A character prefab is REFUSED at runtime, loudly - that would be a
	//! second door into the world and ONE LIFE says there is only one.
	[Attribute("", desc: "Optional prefab for the spectator streaming host. EMPTY = the built-in prefab-free host (no resourceDatabase.rdb dependency). A character prefab is refused at runtime.", params: "et")]
	protected ResourceName m_sHostPrefab;

	//! T-181.24 - how far a spectator may steer their own streaming origin from where they died.
	//!
	//! 0 (the default) is unlimited, because watching the AO is the entire point of a spectator
	//! camera. The cost of unlimited is stated plainly in the `TBD_SpectatorHost` header: it is the
	//! engine's replication range that stops a MODIFIED client from seeing the enemy, and this
	//! feature moves that range on request. An operator who cares more about that than about
	//! spectator reach sets a number here.
	[Attribute("0", desc: "Max metres a spectator may steer their streaming host from their own death position. 0 = unlimited.")]
	protected float m_fHostMaxRangeM;

	//------------------------------------------------------------------------------------------------
	//! Two halves, two guards, and on a LISTEN HOST both of them fire - which is the point of
	//! testing them separately rather than with one `if/else`.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// SERVER half. Authority is the only place that may spawn or possess anything, and a
		// dedicated server reaches this line while a client never does.
		if (m_bStreamingHost && RplSession.Mode() != RplMode.Client)
			TBD_SpectatorHost.Start(m_sHostPrefab, m_fHostMaxRangeM);

		// CLIENT half. A dedicated server has no workspace at all (measured - see TBD_UILayouts).
		// That is the cleanest available "am I a machine with a screen" test, and it is the one the
		// rest of the UI framework already trusts.
		if (!GetGame().GetWorkspace())
			return;

		GetGame().GetCallqueue().CallLater(TBD_SpectatorController.Start, START_DELAY_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (measured landmine in this codebase), so both
	//! managers MUST be torn down here or the next round starts holding a camera - and a possessed
	//! dummy - that belong to a world that no longer exists.
	override void OnDelete(IEntity owner)
	{
		// Unconditional, unlike the arming above: `m_bStreamingHost` is a live attribute and a
		// shutdown that only ran when it was set would leak every host if it were ever flipped off
		// mid-session. `Shutdown` is a no-op when nothing was started.
		TBD_SpectatorHost.Shutdown();

		if (GetGame().GetWorkspace())
		{
			GetGame().GetCallqueue().Remove(TBD_SpectatorController.Start);
			TBD_SpectatorController.Shutdown();
		}

		super.OnDelete(owner);
	}
}
