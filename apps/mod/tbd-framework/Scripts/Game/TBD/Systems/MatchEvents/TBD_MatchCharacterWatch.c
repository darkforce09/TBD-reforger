/**
 * @file TBD_MatchCharacterWatch.c
 * @brief The life-state and seat subscriptions of one player character.
 *
 * Role: subscribes to one player character's life-state changes and seat changes and forwards
 * them, with the player's id, to `TBD_MatchEventCapture`.  Position: created by
 * `TBD_MatchTelemetryComponent` when a player spawns and detached when the character is deleted
 * or the component goes; the engine's `SCR_CharacterControllerComponent.m_OnLifeStateChanged` and
 * `SCR_CompartmentAccessComponent.GetOnCompartmentEntered` / `GetOnCompartmentLeft` call it.
 * State: the character, its player id and the two components it subscribed to.
 * Invariants: every subscription made in `Attach` is removed by `Detach`, which is safe to call
 * twice; a seat move inside a vehicle is neither an entry nor an exit.
 */

//! Subscriptions of one player character.
//! @authority server
class TBD_MatchCharacterWatch : Managed
{
	protected IEntity m_Character; //!< the watched character
	protected int m_iPlayerId; //!< the player who spawned into it
	protected SCR_CharacterControllerComponent m_Controller; //!< life-state source; null once detached
	protected SCR_CompartmentAccessComponent m_Access; //!< seat source; null once detached

	//! Remember the character and its player.
	void TBD_MatchCharacterWatch(IEntity character, int playerId)
	{
		m_Character = character;
		m_iPlayerId = playerId;
	}

	//! Subscribe to the character's life state and seats.
	//! @return false, subscribing nothing, when the entity is not a character
	//! @authority server
	bool Attach()
	{
		ChimeraCharacter character = ChimeraCharacter.Cast(m_Character);
		if (!character)
			return false;

		m_Controller = SCR_CharacterControllerComponent.Cast(character.GetCharacterController());
		if (m_Controller)
			m_Controller.m_OnLifeStateChanged.Insert(OnLifeStateChanged);

		m_Access = SCR_CompartmentAccessComponent.Cast(character.GetCompartmentAccessComponent());
		if (m_Access)
		{
			m_Access.GetOnCompartmentEntered().Insert(OnCompartmentEntered);
			m_Access.GetOnCompartmentLeft().Insert(OnCompartmentLeft);
		}

		return true;
	}

	//! Remove every subscription `Attach` made.
	//! @authority server
	void Detach()
	{
		if (m_Controller)
			m_Controller.m_OnLifeStateChanged.Remove(OnLifeStateChanged);

		if (m_Access)
		{
			m_Access.GetOnCompartmentEntered().Remove(OnCompartmentEntered);
			m_Access.GetOnCompartmentLeft().Remove(OnCompartmentLeft);
		}

		m_Controller = null;
		m_Access = null;
	}

	//! Forward a life-state change.
	//! @authority server
	protected void OnLifeStateChanged(ECharacterLifeState previousLifeState, ECharacterLifeState newLifeState)
	{
		TBD_MatchEventCapture.OnLifeStateChanged(m_iPlayerId, previousLifeState, newLifeState);
	}

	//! Forward a seat taken from outside the vehicle.
	//! @param targetEntity the entity owning the compartment manager
	//! @param move true for a move between seats of one vehicle
	//! @authority server
	protected void OnCompartmentEntered(IEntity targetEntity, BaseCompartmentManagerComponent manager, int mgrID, int slotID, bool move)
	{
		if (move)
			return;

		TBD_MatchEventCapture.OnSeat(m_iPlayerId, targetEntity, FindSlot(manager, mgrID, slotID), true);
	}

	//! Forward a seat left for outside the vehicle.
	//! @param targetEntity the entity owning the compartment manager
	//! @param move true for a move between seats of one vehicle
	//! @authority server
	protected void OnCompartmentLeft(IEntity targetEntity, BaseCompartmentManagerComponent manager, int mgrID, int slotID, bool move)
	{
		if (move)
			return;

		TBD_MatchEventCapture.OnSeat(m_iPlayerId, targetEntity, FindSlot(manager, mgrID, slotID), false);
	}

	//! The seat the engine named.
	//! @return the slot, or null without a manager
	protected static BaseCompartmentSlot FindSlot(BaseCompartmentManagerComponent manager, int mgrID, int slotID)
	{
		if (!manager)
			return null;

		return manager.FindCompartment(slotID, mgrID);
	}
}
