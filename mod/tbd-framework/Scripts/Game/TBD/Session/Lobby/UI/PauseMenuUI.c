/**
 * @file PauseMenuUI.c
 * @brief Turns the vanilla pause menu's leave-faction button into "Change slot" on a framework world.
 *
 * Role: during BRIEFING, SAFE_START and LIVE, relabels and enables the `LeaveFaction` button and
 * routes its click to TBD_LobbyScreen.OpenFromPause.  Position: vanilla opens PauseMenuUI; this
 * modded block reads TBD_FrameworkManager's replicated stage.
 * State: the hooked button, held from menu open to menu close; client UI only.
 * Invariants: nothing changes outside a framework world or outside the three stages; the click
 * handler is removed on close; the lobby opens on the next frame, after the pause menu has closed.
 */

//! Pause menu hook that offers "Change slot".
modded class PauseMenuUI
{
	protected SCR_ButtonTextComponent m_TbdChangeSlotButton; //!< the hooked `LeaveFaction` button; null when not hooked

	//! Open the vanilla menu, then hook "Change slot" when the stage allows it.
	override void OnMenuOpen()
	{
		super.OnMenuOpen();
		HookTbdChangeSlot();
	}

	//! Unhook the button, then close the vanilla menu.
	override void OnMenuClose()
	{
		if (m_TbdChangeSlotButton)
		{
			m_TbdChangeSlotButton.m_OnClicked.Remove(OnTbdChangeSlot);
			m_TbdChangeSlotButton = null;
		}
		super.OnMenuClose();
	}

	//! On a framework world in BRIEFING, SAFE_START or LIVE, show, relabel and enable `LeaveFaction` as "Change slot" and subscribe to its click; otherwise leave the menu untouched.
	//! @authority client
	protected void HookTbdChangeSlot()
	{
		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		TBD_EGameStage stage = fm.GetStage();
		if (stage != TBD_EGameStage.BRIEFING
			&& stage != TBD_EGameStage.SAFE_START
			&& stage != TBD_EGameStage.LIVE)
			return;

		Widget root = GetRootWidget();
		if (!root)
			return;

		SCR_ButtonTextComponent btn = SCR_ButtonTextComponent.GetButtonText("LeaveFaction", root);
		if (!btn)
			return;

		Widget row = btn.GetRootWidget();
		if (row)
			row.SetVisible(true);

		btn.SetText("Change slot");
		btn.SetEnabled(true);
		btn.m_OnClicked.Insert(OnTbdChangeSlot);
		m_TbdChangeSlotButton = btn;
	}

	//! Close the pause menu and open the lobby on the next frame.
	protected void OnTbdChangeSlot()
	{
		Close();
		GetGame().GetCallqueue().CallLater(TBD_LobbyScreen.OpenFromPause, 0, false);
	}
}
