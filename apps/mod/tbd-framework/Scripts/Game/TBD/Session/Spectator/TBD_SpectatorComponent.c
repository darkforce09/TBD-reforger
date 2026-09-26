/**
 * @file TBD_SpectatorComponent.c
 * @brief Game mode component that starts and stops the spectator's client and server halves.
 *
 * Role: on init, arms TBD_SpectatorHost on the authority (when the streaming host is enabled) and,
 * on a machine with a workspace, starts TBD_SpectatorController after START_DELAY_MS; on delete,
 * shuts both down. Holds the operator attributes of the streaming host.
 * Position: attached to the game mode prefab beside TBD_FrameworkManager and TBD_SpawnManager;
 * feeds TBD_SpectatorHost.Start and TBD_SpectatorController.Start.
 * State: the three attributes; the managers own their own static state.
 * Invariants: a lifecycle on the game mode, not on SCR_PlayerController (whose modded blocks are
 * RPC transports) and not a GameSystem; on a listen host both halves start; shutdown is
 * unconditional, so flipping m_bStreamingHost mid-session cannot leak a host; the range leash
 * handed to the host is always finite.
 */

//! Editor descriptor of TBD_SpectatorComponent.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD spectator -- free camera, follow, the unit list a dead player lives in, and the server-side streaming host that keeps the world around their camera loaded.")]
class TBD_SpectatorComponentClass : SCR_BaseGameModeComponentClass {}

//! Starts and stops the spectator controller (client) and streaming host (server) with the world.
class TBD_SpectatorComponent : SCR_BaseGameModeComponent
{
	static const int START_DELAY_MS = 2000; //!< ms after init before the client controller starts; the controller polls, so nothing is missed
	static const float DEFAULT_HOST_MAX_RANGE_M = 2000; //!< metres; the leash used when m_fHostMaxRangeM is 0 or less

	//! The kill switch: possession is the most invasive thing the mod does to a player controller,
	//! so this one attribute stands the whole streaming host down (the camera still works, and
	//! streaming stays anchored to the corpse).
	[Attribute("1", desc: "Give a dead player an inert entity to possess so the server keeps streaming the world around their spectator camera. Off = the camera still works, but flying far from your corpse shows an empty world.")]
	protected bool m_bStreamingHost; //!< default true; false stands the streaming host down

	//! Normally empty: the built-in host is spawned by type name with no prefab and no
	//! resourceDatabase.rdb dependency. A prefab whose root class is TBD_SpectatorHostEntity is the
	//! route to a replicated host (TBD_SpectatorHostFactory.SpawnHostEntity); a character prefab is
	//! refused at runtime, since it would be a second door into the world.
	[Attribute("", desc: "Optional prefab for the spectator streaming host. EMPTY = the built-in prefab-free host (no resourceDatabase.rdb dependency). A character prefab is refused at runtime.", params: "et")]
	protected ResourceName m_sHostPrefab; //!< default empty = the built-in prefab-free host

	//! How far a spectator may steer their streaming origin from where they died. 0 or less is the
	//! 2000 m default, never unlimited, so a modified client cannot pull the whole map by
	//! requesting a host far from its corpse; a positive value sets the leash.
	[Attribute("2000", desc: "Max metres a spectator may steer their streaming host from their own death position. Default 2000. 0 uses the default; never unlimited.")]
	protected float m_fHostMaxRangeM; //!< metres; default 2000; 0 or less = DEFAULT_HOST_MAX_RANGE_M

	//! Start the two halves under two separate guards, since a listen host runs both: the server
	//! half arms TBD_SpectatorHost when m_bStreamingHost is set; the client half, only where a
	//! workspace exists, starts TBD_SpectatorController after START_DELAY_MS.
	//! @param owner the game mode entity
	//! @authority server
	//! @authority client
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// SERVER half. Authority is the only place that may spawn or possess anything, and a
		// dedicated server reaches this line while a client never does.
		if (m_bStreamingHost && TBD_Authority.IsServer())
			TBD_SpectatorHost.Start(m_sHostPrefab, ClampHostMaxRangeM(m_fHostMaxRangeM));

		// CLIENT half. A dedicated server has no workspace at all (measured -- see TBD_UILayouts).
		// That is the cleanest available "am I a machine with a screen" test, and it is the one the
		// rest of the UI framework already trusts.
		if (!GetGame().GetWorkspace())
			return;

		GetGame().GetCallqueue().CallLater(TBD_SpectatorController.Start, START_DELAY_MS, false);
	}

	//! Shut both halves down. Statics outlive a world inside one process, so without this the next
	//! round would hold a camera and a possessed host from a deleted world.
	//! @param owner the game mode entity
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

	//! Resolve the configured leash to a finite value: 0 or less becomes DEFAULT_HOST_MAX_RANGE_M.
	//! TBD_SpectatorHost treats a leash of 0 or less as unlimited, so it must never receive one.
	//! @param requestedM the configured m_fHostMaxRangeM
	//! @return the leash in metres, always positive
	protected static float ClampHostMaxRangeM(float requestedM)
	{
		float ceiling = requestedM;
		if (ceiling <= 0)
			ceiling = DEFAULT_HOST_MAX_RANGE_M;

		float value = requestedM;
		if (value <= 0)
			value = ceiling;

		return Math.Clamp(value, 0, ceiling);
	}
}
