/**
 * @file TBD_SpectatorInputActions.c
 * @brief Spectator keyboard accelerators: roster, next, previous, view and free.
 *
 * Role: registers and removes the five spectator input action listeners and routes each to the
 * roster or to TBD_SpectatorTargeting.
 * Position: TBD_SpectatorController.Start and Shutdown call Register and Unregister; the actions
 * come from the TBD_SpectatorContext input context, which TBD_SpectatorCamera.EOnPostFrame arms
 * each frame.
 * State: the registered flag (static, client).
 * Invariants: registration is idempotent; every accelerator is also a click in the roster, so the
 * feature is complete with a mouse alone; the targeting actions do nothing while not spectating.
 */

//! Spectator input accelerators. Static; client only.
class TBD_SpectatorInputActions
{
	protected static bool s_bListenersRegistered; //!< the input action listeners are registered

	//! Register the spectator input accelerators (roster, next, previous, view, free). Every one is
	//! also a click in the roster, so the feature is complete with a mouse alone; the action configs
	//! share the menu preset's resource database dependency, free flight (ManualCameraContext) does not.
	//! @authority client
	static void Register()
	{
		if (s_bListenersRegistered)
			return;

		InputManager input = GetGame().GetInputManager();
		if (!input)
			return;

		input.AddActionListener("TBD_SpecRoster", EActionTrigger.DOWN, OnActionRoster);
		input.AddActionListener("TBD_SpecNext",   EActionTrigger.DOWN, OnActionNext);
		input.AddActionListener("TBD_SpecPrev",   EActionTrigger.DOWN, OnActionPrev);
		input.AddActionListener("TBD_SpecView",   EActionTrigger.DOWN, OnActionView);
		input.AddActionListener("TBD_SpecFree",   EActionTrigger.DOWN, OnActionFree);

		s_bListenersRegistered = true;
	}

	//! Remove the listeners Register added; no-op when none are registered.
	//! @authority client
	static void Unregister()
	{
		if (!s_bListenersRegistered)
			return;

		InputManager input = GetGame().GetInputManager();
		if (input)
		{
			input.RemoveActionListener("TBD_SpecRoster", EActionTrigger.DOWN, OnActionRoster);
			input.RemoveActionListener("TBD_SpecNext",   EActionTrigger.DOWN, OnActionNext);
			input.RemoveActionListener("TBD_SpecPrev",   EActionTrigger.DOWN, OnActionPrev);
			input.RemoveActionListener("TBD_SpecView",   EActionTrigger.DOWN, OnActionView);
			input.RemoveActionListener("TBD_SpecFree",   EActionTrigger.DOWN, OnActionFree);
		}

		s_bListenersRegistered = false;
	}

	//! TBD_SpecRoster: toggle the roster. The input contexts are armed per frame by
	//! TBD_SpectatorCamera.EOnPostFrame, so every accelerator is live exactly while a camera exists.
	protected static void OnActionRoster(float value, EActionTrigger trigger) { TBD_SpectatorController.ToggleRoster(); }
	//! TBD_SpecNext: follow the next target.
	protected static void OnActionNext(float value, EActionTrigger trigger)   { if (TBD_SpectatorController.IsActive()) TBD_SpectatorTargeting.CycleTarget(1); }
	//! TBD_SpecPrev: follow the previous target.
	protected static void OnActionPrev(float value, EActionTrigger trigger)   { if (TBD_SpectatorController.IsActive()) TBD_SpectatorTargeting.CycleTarget(-1); }
	//! TBD_SpecView: toggle first and third person on the target.
	protected static void OnActionView(float value, EActionTrigger trigger)   { if (TBD_SpectatorController.IsActive()) TBD_SpectatorTargeting.ToggleFirstPerson(); }
	//! TBD_SpecFree: back to free flight.
	protected static void OnActionFree(float value, EActionTrigger trigger)   { if (TBD_SpectatorController.IsActive()) TBD_SpectatorTargeting.SetFree(); }
}
