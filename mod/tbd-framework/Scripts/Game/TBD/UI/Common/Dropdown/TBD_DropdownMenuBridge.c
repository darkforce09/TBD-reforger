/**
 * @file TBD_DropdownMenuBridge.c
 * @brief Forwards the open dropdown menu's scrim click to its dropdown.
 *
 * Role: closes the dropdown when the full-bleed `Scrim` of `TBD_DropdownMenu.layout` is clicked.
 * Position: attached by `TBD_DropdownMenu` to the menu root; calls `TBD_DropdownComponent.Close`.
 * State: a weak reference to the owning dropdown, for the life of the open menu.
 * Invariants: a widget carries one handler of a class, so the menu root gets this stub instead of
 * the dropdown itself; clicks on any other widget pass through to the base handler.
 */

//! Forwards the menu's own widget events (the scrim click) to the dropdown. A handler may only
//! be attached to one widget, so the menu root gets this stub instead of the dropdown itself.
class TBD_DropdownMenuBridge : ScriptedWidgetComponent
{
	protected TBD_DropdownComponent m_Owner; //!< the dropdown to close; weak, the dropdown owns this bridge

	//! Bind the bridge to `owner`.
	void TBD_DropdownMenuBridge(TBD_DropdownComponent owner)
	{
		m_Owner = owner;
	}

	//! Close the owner when the click lands on `Scrim`.
	//! @return true when the click closed the menu; otherwise the base handler's answer
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (m_Owner && w && w.GetName() == "Scrim")
		{
			m_Owner.Close();
			return true;
		}

		return super.OnClick(w, x, y, button);
	}
}
