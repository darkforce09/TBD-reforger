/**
 * @file TBD_SessionBottomBar.c
 * @brief The action bar every pre-game screen wears: left actions, then quiet and primary buttons.
 *
 * Role: handler of `Session/Shared/TBD_SessionBottomBar.layout`; creates one
 * `Common/TBD_Button.layout` per action a screen adds and reports clicks by action id.
 * Position: mounted by `TBD_DockScreen` into `BottomDock`; the mission selector, lobby and briefing
 * screens add their actions and listen to `GetOnAction()(bar, actionId)`.
 * State: the widget references, the parallel id and button arrays and the primary id, on the client.
 * Invariants: widget contract `BarBorder`, `BarBG`, `LeftActions`, `RightActions`; the layout
 * carries no buttons; at most one action is primary, and a second primary demotes the first and
 * logs a WARNING.
 */

//! Bottom action bar handler.
class TBD_SessionBottomBar : ScriptedWidgetComponent
{
	protected Widget m_wRoot; //!< the layout root this handler sits on
	protected Widget m_wBorder; //!< `BarBorder` frame dock
	protected Widget m_wBackground; //!< `BarBG` frame dock
	protected Widget m_wLeftActions; //!< `LeftActions` dock
	protected Widget m_wRightActions; //!< `RightActions` dock

	protected ref array<string> m_aIds; //!< Parallel arrays: an action's id and its button. Small lists; no map needed.
	protected ref array<TBD_UIButton> m_aButtons; //!< the buttons, parallel to m_aIds; weak, each owned by its widget
	protected string m_sPrimaryId; //!< the primary action's id; empty = none

	protected ref ScriptInvoker m_OnAction; //!< (TBD_SessionBottomBar bar, string actionId)

	//! Find the widgets, round and paint the bar over the backdrop.
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_aIds = {};
		m_aButtons = {};

		m_wBorder = w.FindAnyWidget("BarBorder");
		m_wBackground = w.FindAnyWidget("BarBG");
		m_wLeftActions = w.FindAnyWidget("LeftActions");
		m_wRightActions = w.FindAnyWidget("RightActions");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_PANEL - 1);

		// The bar sits on the backdrop, not on a panel.
		TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.STRIP_BORDER, TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.INPUT_FILL, TBD_UITheme.Ground());
	}

	//! Remove every action and drop the root when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		RemoveAll();
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//! Add a button. `left` puts it in the left dock (Back-style); default is the right cluster.
	//! Returns the button so a screen may keep a handle, or null when the layout is missing.
	TBD_UIButton AddAction(string id, string label, bool primary = false, bool left = false)
	{
		Widget dock = m_wRightActions;
		if (left)
			dock = m_wLeftActions;

		if (!dock)
			return null;

		TBD_UIButton button = TBD_UIButton.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.BUTTON, dock, TBD_UIButton));
		if (!button)
			return null;

		button.SetLabel(label);
		button.SetGround(TBD_UITheme.Over(TBD_UITheme.INPUT_FILL, TBD_UITheme.Ground()));
		button.GetOnActivate().Insert(OnButtonActivated);

		m_aIds.Insert(id);
		m_aButtons.Insert(button);

		if (primary)
			SetActionPrimary(id, true);
		else
			button.SetPrimary(false);

		return button;
	}

	//! Set the label of action `id`; an unknown id is ignored.
	void SetActionLabel(string id, string label)
	{
		TBD_UIButton button = Find(id);
		if (button)
			button.SetLabel(label);
	}

	//! Enable or disable action `id`; an unknown id is ignored.
	void SetActionEnabled(string id, bool enabled)
	{
		TBD_UIButton button = Find(id);
		if (button)
			button.SetInteractive(enabled);
	}

	//! Show or hide action `id`; an unknown id is ignored.
	void SetActionVisible(string id, bool visible)
	{
		TBD_UIButton button = Find(id);
		if (button)
			TBD_UITheme.Show(button.GetRootWidget(), visible);
	}

	//! Promote one action to THE primary. Any previous primary is demoted -- one loud button.
	void SetActionPrimary(string id, bool primary)
	{
		TBD_UIButton button = Find(id);
		if (!button)
			return;

		if (!primary)
		{
			button.SetPrimary(false);
			if (m_sPrimaryId == id)
				m_sPrimaryId = string.Empty;
			return;
		}

		if (!m_sPrimaryId.IsEmpty() && m_sPrimaryId != id)
		{
			TBD_UIButton previous = Find(m_sPrimaryId);
			if (previous)
				previous.SetPrimary(false);

			Print(string.Format("[TBD][ui] bottom bar: '%1' takes primary from '%2' -- one primary action per screen.", id, m_sPrimaryId), LogLevel.WARNING);
		}

		m_sPrimaryId = id;
		button.SetPrimary(true);
	}

	//! @return the button of action `id`, or null when unknown
	TBD_UIButton GetAction(string id)
	{
		return Find(id);
	}

	//! Unsubscribe and remove every action button and clear the primary.
	void RemoveAll()
	{
		if (!m_aButtons)
			return;

		foreach (TBD_UIButton button : m_aButtons)
		{
			if (!button)
				continue;

			button.GetOnActivate().Remove(OnButtonActivated);
			Widget root = button.GetRootWidget();
			if (root)
				root.RemoveFromHierarchy();
		}

		m_aButtons.Clear();
		m_aIds.Clear();
		m_sPrimaryId = string.Empty;
	}

	//! Focus the primary action (or the first one). Returns false when the bar is empty.
	bool FocusPrimary()
	{
		TBD_UIButton target = Find(m_sPrimaryId);
		if (!target && m_aButtons.Count() > 0)
			target = m_aButtons[0];

		if (!target || !target.IsInteractive())
			return false;

		target.Focus();
		return true;
	}

	//! (TBD_SessionBottomBar bar, string actionId)
	ScriptInvoker GetOnAction()
	{
		if (!m_OnAction)
			m_OnAction = new ScriptInvoker();

		return m_OnAction;
	}

	//! @return the layout root this handler sits on
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! @return the button of action `id`, or null when `id` is empty or unknown
	protected TBD_UIButton Find(string id)
	{
		if (id.IsEmpty() || !m_aIds)
			return null;

		int index = m_aIds.Find(id);
		if (index < 0)
			return null;

		return m_aButtons[index];
	}

	//! Fire OnAction with the clicked button's action id; an unknown button is ignored.
	protected void OnButtonActivated(TBD_UIButton button)
	{
		int index = m_aButtons.Find(button);
		if (index < 0)
			return;

		if (m_OnAction)
			m_OnAction.Invoke(this, m_aIds[index]);
	}
}
