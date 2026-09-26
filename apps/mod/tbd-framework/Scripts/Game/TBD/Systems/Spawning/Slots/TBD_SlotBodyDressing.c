/**
 * @file TBD_SlotBodyDressing.c
 * @brief Applies a slot's authored identity (stance and rank) to its freshly spawned body.
 *
 * Role: binds the slot identity keys that have an engine home.  Position: called by
 * TBD_SlotBodyMaterializer.SpawnSlotBody on every spawn of a slot body, initial and respawn.
 * State: none.  Invariants: `stance` goes through CharacterControllerComponent.SetStanceChange and
 * `rank` through the replicated SCR_CharacterRankComponent.SetCharacterRank; `callsign`,
 * `unitName` and `tag` have no replicated free-text home on a character, so they stay on
 * TBD_MissionSlotStruct and are only logged here; a missing component or unknown token is reported at
 * WARNING and never costs the player their body.
 */

//! Slot identity application for slot bodies.
class TBD_SlotBodyDressing
{
	//! Apply the stance and rank of `slot` to `body` and log the authored identity once per spawn.
	//! No-op without an identity.
	static void ApplySlotIdentity(IEntity body, TBD_MissionSlotStruct slot)
	{
		if (!body || !slot || !slot.HasIdentity())
			return;

		ApplySpawnStance(body, slot);
		ApplySlotRank(body, slot);

		Print(string.Format("[TBD][Identity] slot=%1 callsign='%2' rank='%3' stance='%4' unitName='%5' tag='%6' leader=%7",
			slot.Key(), slot.callsign, slot.rank, slot.stance, slot.unitName, slot.tag,
			TBD_MissionLoader.IsSquadLeader(slot)));
	}

	//! Request the authored initial stance. SetStanceChange plays the transition, so it runs after
	//! the body exists.
	protected static void ApplySpawnStance(IEntity body, TBD_MissionSlotStruct slot)
	{
		if (slot.stance.IsEmpty())
			return;

		ECharacterStanceChange change = StanceChangeFor(slot.stance);
		if (change == ECharacterStanceChange.STANCECHANGE_NONE)
		{
			Print(string.Format("[TBD][Identity] slot=%1 stance='%2' is not one of stand|crouch|prone -- pose not applied",
				slot.Key(), slot.stance), LogLevel.WARNING);
			return;
		}

		CharacterControllerComponent controller = CharacterControllerComponent.Cast(body.FindComponent(CharacterControllerComponent));
		if (!controller)
		{
			Print(string.Format("[TBD][Identity] slot=%1 authored stance='%2' but its kit body has no CharacterControllerComponent -- pose NOT applied",
				slot.Key(), slot.stance), LogLevel.WARNING);
			return;
		}

		controller.SetStanceChange(change);
	}

	//! The engine stance request for a `$defs/slot.stance` token; STANCECHANGE_NONE for an unknown
	//! token.
	protected static ECharacterStanceChange StanceChangeFor(string stance)
	{
		if (stance == "stand")
			return ECharacterStanceChange.STANCECHANGE_TOERECTED;
		if (stance == "crouch")
			return ECharacterStanceChange.STANCECHANGE_TOCROUCH;
		if (stance == "prone")
			return ECharacterStanceChange.STANCECHANGE_TOPRONE;

		return ECharacterStanceChange.STANCECHANGE_NONE;
	}

	//! Set the authored rank silently: an authored rank fires no promotion notification.
	protected static void ApplySlotRank(IEntity body, TBD_MissionSlotStruct slot)
	{
		if (slot.rank.IsEmpty())
			return;

		bool known;
		SCR_ECharacterRank rank = CharacterRankFor(slot.rank, known);
		if (!known)
		{
			Print(string.Format("[TBD][Identity] slot=%1 rank='%2' is not on the ladder -- rank not applied",
				slot.Key(), slot.rank), LogLevel.WARNING);
			return;
		}

		SCR_CharacterRankComponent rankComponent = SCR_CharacterRankComponent.GetCharacterRankComponent(body);
		if (!rankComponent)
		{
			Print(string.Format("[TBD][Identity] slot=%1 authored rank='%2' but its kit body has no SCR_CharacterRankComponent -- rank NOT applied",
				slot.Key(), slot.rank), LogLevel.WARNING);
			return;
		}

		rankComponent.SetCharacterRank(rank, true);
	}

	//! The SCR_ECharacterRank for a `$defs/slot.rank` token (private to colonel, the engine's own
	//! spelling lowercased).
	//! @param known set false for an unknown token, which returns PRIVATE
	protected static SCR_ECharacterRank CharacterRankFor(string rank, out bool known)
	{
		known = true;

		if (rank == "private")
			return SCR_ECharacterRank.PRIVATE;
		if (rank == "corporal")
			return SCR_ECharacterRank.CORPORAL;
		if (rank == "sergeant")
			return SCR_ECharacterRank.SERGEANT;
		if (rank == "lieutenant")
			return SCR_ECharacterRank.LIEUTENANT;
		if (rank == "captain")
			return SCR_ECharacterRank.CAPTAIN;
		if (rank == "major")
			return SCR_ECharacterRank.MAJOR;
		if (rank == "colonel")
			return SCR_ECharacterRank.COLONEL;

		known = false;
		return SCR_ECharacterRank.PRIVATE;
	}
}
