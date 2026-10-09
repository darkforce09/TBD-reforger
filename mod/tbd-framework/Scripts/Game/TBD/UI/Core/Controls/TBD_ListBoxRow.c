/**
 * @file TBD_ListBoxRow.c
 * @brief One pooled row of a `TBD_ListBox`: an interactive item or a quiet section heading.
 *
 * Role: handler of `TBD_ListRow.layout`; re-binds title, detail, tag and state and paints them
 * from `TBD_UIStateColours`; reports clicks and highlights to its list.
 * Position: created and pooled by `TBD_ListBox`; derives from `TBD_UIInteractive`.
 * State: the widget references, the owning list, the pool index, tag, state and the section and
 * selected flags, on the client.
 * Invariants: a row is a view, never a record: it owns no data and is re-bound, never rebuilt;
 * a section is neither focusable nor clickable; widget contract `Background`, `Accent` (the 2 px
 * active rail), `Title`, `Detail`, every write null-safe.
 */

//! One pooled list row.
//!
//! A row is a *view*, never a record: it owns no data, it is re-bound. That is what makes the
//! list cheap -- see the cost note in TBD_ListBox.
//!
//! Two kinds, one widget:
//!   * **item**    -- interactive. Click activates it (direct manipulation: no select-then-confirm).
//!   * **section** -- a quiet heading. Not focusable, not clickable. Sections are how a 128-slot
//!                   mission is shown as "side -> group -> slot" instead of a wall of rows.
//!
//! The layout must provide, by name: `Background` (image), `Accent` (image, the 2px active rail),
//! `Title` (text), `Detail` (text). Missing widgets are tolerated -- every write is null-safe -- so
//! a screen can ship a stripped-down row layout without touching this class.
class TBD_ListBoxRow : TBD_UIInteractive
{
	protected Widget m_wBackground; //!< `Background`
	protected Widget m_wAccent; //!< `Accent`, the 2 px active rail
	protected TextWidget m_wTitle; //!< `Title`
	protected TextWidget m_wDetail; //!< `Detail`, hidden while empty

	protected TBD_ListBox m_Owner; //!< Weak -- the list owns the pool, the pool does not own the list.

	protected int m_iIndex = -1;          //!< stable index into the owner's pool
	protected int m_iTag = -1;            //!< caller's id: slot index, group id, mission number...
	protected TBD_EUIState m_eState = TBD_EUIState.NORMAL; //!< the row's semantic state; default NORMAL
	protected bool m_bSection; //!< true = a section heading
	protected bool m_bSelected; //!< true while the list marks this row selected

	//! Find the row widgets.
	override protected void OnBind(Widget w)
	{
		m_wBackground = w.FindAnyWidget("Background");
		m_wAccent = w.FindAnyWidget("Accent");
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wDetail = TextWidget.Cast(w.FindAnyWidget("Detail"));
	}

	//! Called once, when the list creates this row.
	void Attach(TBD_ListBox owner, int index)
	{
		m_Owner = owner;
		m_iIndex = index;
	}

	//! Re-point this row at different content. No widget is created or destroyed here -- that is
	//! the whole point of the pool.
	void Bind(string title, string detail, int tag, TBD_EUIState state, bool enabled, bool section)
	{
		m_iTag = tag;
		m_eState = state;
		m_bSection = section;
		m_bSelected = false; // the owning list re-applies selection once the build closes

		TBD_UITheme.Write(m_wTitle, title);
		TBD_UITheme.Write(m_wDetail, detail);
		TBD_UITheme.Show(m_wDetail, !detail.IsEmpty());

		// A section heading is text, not a control: it must never take focus or eat a click.
		SetInteractive(!section && enabled && state != TBD_EUIState.LOCKED);

		Repaint();
	}

	//! Mark the row selected or not and repaint; does nothing when unchanged.
	void SetSelected(bool selected)
	{
		if (m_bSelected == selected)
			return;

		m_bSelected = selected;
		Repaint();
	}

	//! Pooling: surplus rows are hidden, never destroyed, so the next refresh costs nothing.
	void SetRowVisible(bool visible)
	{
		if (!m_wRoot)
			return;

		m_wRoot.SetVisible(visible);
	}

	//! Paint a heading without chrome, or an item from its state, highlight and selection.
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		if (m_bSection)
		{
			// Headings carry no chrome at all -- generous whitespace does the grouping.
			TBD_UITheme.Paint(m_wBackground, TBD_UITheme.TRANSPARENT);
			TBD_UITheme.Paint(m_wAccent, TBD_UITheme.TRANSPARENT);
			TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE_VARIANT);
			TBD_UITheme.Paint(m_wDetail, TBD_UITheme.ON_SURFACE_VARIANT);
			return;
		}

		bool highlighted = IsHighlighted() && m_bInteractive;

		TBD_UITheme.Paint(m_wBackground, TBD_UIStateColours.StateBackground(m_eState, highlighted, m_bSelected));
		TBD_UITheme.Paint(m_wAccent, TBD_UIStateColours.StateAccent(m_eState, m_bSelected));
		TBD_UITheme.Paint(m_wTitle, TBD_UIStateColours.StateTitle(m_eState));
		TBD_UITheme.Paint(m_wDetail, TBD_UIStateColours.StateDetail(m_eState));
	}

	//! A click reports this row's pool index to the list.
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnRowActivated(m_iIndex);
	}

	//! A hover or focus reports this row's pool index to the list.
	override protected void OnHighlighted()
	{
		if (m_Owner)
			m_Owner.OnRowHighlighted(m_iIndex);
	}

	//! @return the caller's id bound to this row; -1 before Bind
	int GetTag()
	{
		return m_iTag;
	}

	//! @return the row's stable pool index; -1 before Attach
	int GetIndex()
	{
		return m_iIndex;
	}

	//! @return true for a section heading
	bool IsSection()
	{
		return m_bSection;
	}

	//! Can this row be picked? Sections and locked/disabled rows cannot.
	bool IsSelectable()
	{
		return !m_bSection && m_bInteractive;
	}
}
