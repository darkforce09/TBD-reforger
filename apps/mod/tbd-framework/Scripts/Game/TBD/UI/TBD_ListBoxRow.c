//! T-181.7 — one pooled row of a TBD_ListBox.
//!
//! A row is a *view*, never a record: it owns no data, it is re-bound. That is what makes the
//! list cheap — see the cost note in TBD_ListBox.
//!
//! Two kinds, one widget:
//!   * **item**    — interactive. Click activates it (direct manipulation: no select-then-confirm).
//!   * **section** — a quiet heading. Not focusable, not clickable. Sections are how a 128-slot
//!                   mission is shown as "side -> group -> slot" instead of a wall of rows.
//!
//! The layout must provide, by name: `Background` (image), `Accent` (image, the 2px active rail),
//! `Title` (text), `Detail` (text). Missing widgets are tolerated — every write is null-safe — so
//! a screen can ship a stripped-down row layout without touching this class.
class TBD_ListBoxRow : TBD_UIInteractive
{
	protected Widget m_wBackground;
	protected Widget m_wAccent;
	protected TextWidget m_wTitle;
	protected TextWidget m_wDetail;

	//! Weak — the list owns the pool, the pool does not own the list.
	protected TBD_ListBox m_Owner;

	protected int m_iIndex = -1;          //!< stable index into the owner's pool
	protected int m_iTag = -1;            //!< caller's id: slot index, group id, mission number…
	protected TBD_EUIState m_eState = TBD_EUIState.NORMAL;
	protected bool m_bSection;
	protected bool m_bSelected;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wBackground = w.FindAnyWidget("Background");
		m_wAccent = w.FindAnyWidget("Accent");
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wDetail = TextWidget.Cast(w.FindAnyWidget("Detail"));
	}

	//------------------------------------------------------------------------------------------------
	//! Called once, when the list creates this row.
	void Attach(TBD_ListBox owner, int index)
	{
		m_Owner = owner;
		m_iIndex = index;
	}

	//------------------------------------------------------------------------------------------------
	//! Re-point this row at different content. No widget is created or destroyed here — that is
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

	//------------------------------------------------------------------------------------------------
	void SetSelected(bool selected)
	{
		if (m_bSelected == selected)
			return;

		m_bSelected = selected;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	//! Pooling: surplus rows are hidden, never destroyed, so the next refresh costs nothing.
	void SetRowVisible(bool visible)
	{
		if (!m_wRoot)
			return;

		m_wRoot.SetVisible(visible);
	}

	//------------------------------------------------------------------------------------------------
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		if (m_bSection)
		{
			// Headings carry no chrome at all — generous whitespace does the grouping.
			TBD_UITheme.Paint(m_wBackground, TBD_UITheme.TRANSPARENT);
			TBD_UITheme.Paint(m_wAccent, TBD_UITheme.TRANSPARENT);
			TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE_VARIANT);
			TBD_UITheme.Paint(m_wDetail, TBD_UITheme.ON_SURFACE_VARIANT);
			return;
		}

		bool highlighted = IsHighlighted() && m_bInteractive;

		TBD_UITheme.Paint(m_wBackground, TBD_UITheme.StateBackground(m_eState, highlighted, m_bSelected));
		TBD_UITheme.Paint(m_wAccent, TBD_UITheme.StateAccent(m_eState, m_bSelected));
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.StateTitle(m_eState));
		TBD_UITheme.Paint(m_wDetail, TBD_UITheme.StateDetail(m_eState));
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnRowActivated(m_iIndex);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnHighlighted()
	{
		if (m_Owner)
			m_Owner.OnRowHighlighted(m_iIndex);
	}

	//------------------------------------------------------------------------------------------------
	int GetTag()
	{
		return m_iTag;
	}

	//------------------------------------------------------------------------------------------------
	int GetIndex()
	{
		return m_iIndex;
	}

	//------------------------------------------------------------------------------------------------
	bool IsSection()
	{
		return m_bSection;
	}

	//------------------------------------------------------------------------------------------------
	//! Can this row be picked? Sections and locked/disabled rows cannot.
	bool IsSelectable()
	{
		return !m_bSection && m_bInteractive;
	}
}
