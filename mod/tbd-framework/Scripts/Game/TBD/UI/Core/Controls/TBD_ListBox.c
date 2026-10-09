/**
 * @file TBD_ListBox.c
 * @brief The reusable TBD list: pooled rows that are re-bound, never rebuilt.
 *
 * Role: handler of a list layout; builds its rows from `BeginUpdate`, `AddSection`, `AddItem`,
 * `EndUpdate` (or `SetRows`), echoes the selection and fires activation and highlight events.
 * Position: attached by class in the list layouts (the dropdown menu, the shell screen, the lobby,
 * mission and roster lists); its rows are `TBD_ListBoxRow`s from `TBD_ListRow.layout`.
 * State: the widget references, the row pool, the live row count, the build cursor and the
 * selected tag, on the client.
 * Invariants: a row widget is created once per index, ever; a refresh writes text and colour onto
 * existing widgets and hides surplus rows, so the steady-state refresh is O(visible rows) of
 * property writes; rows stay on screen until `EndUpdate`, so a refresh never flashes.
 */

//! Pooled list handler.
//!
//! Vanilla `SCR_ListBoxComponent.AddItem` creates widgets per item and its `Clear()` destroys every
//! child, so a lobby list rebuilt on every replicated slot claim would create and destroy a widget
//! per slot per broadcast and carry vanilla's look into the screens. Pooling makes a 128-slot
//! mission cost 128 creations for the whole session. Progressive disclosure (side, group, slot)
//! keeps a typical list to a dozen rows. A build is `BeginUpdate()`, then `AddSection("BRAVO")` and
//! `AddItem("Squad Leader", "Cpl. Hicks", slotId, TBD_EUIState.TAKEN, false)` per row, then
//! `EndUpdate()`, with no allocation on that path; `SetRows()` serves callers that already hold
//! `TBD_ListRowData`. Rows go into the vertical layout named `Content` (configurable); put it
//! inside a `ScrollLayoutWidget` and long lists scroll.
class TBD_ListBox : ScriptedWidgetComponent
{
	[Attribute("{7BD1A70000000702}UI/layouts/Common/TBD_ListRow.layout", UIWidgets.ResourceNamePicker, desc: "Layout instantiated for every pooled row", params: "layout")]
	protected ResourceName m_sRowLayout; //!< the pooled row layout; default TBD_ListRow.layout

	[Attribute("Content", UIWidgets.EditBox, desc: "Name of the vertical layout rows are parented to")]
	protected string m_sContentName; //!< row container widget name; default "Content"

	[Attribute("EmptyState", UIWidgets.EditBox, desc: "Name of the widget shown while the list is empty")]
	protected string m_sEmptyStateName; //!< empty-state widget name; default "EmptyState"

	protected Widget m_wRoot; //!< the widget this handler sits on
	protected Widget m_wContent; //!< the row container; the root when the layout has none
	protected Widget m_wEmptyState; //!< shown while no row is live; optional

	//! Pool, index-stable. Weak elements: each row handler is owned by its widget.
	protected ref array<TBD_ListBoxRow> m_aPool; //!< index-stable; weak elements, each row handler is owned by its widget

	protected int m_iLiveRows;        //!< rows bound and visible after the last EndUpdate
	protected int m_iBuildCursor = -1;//!< >= 0 while a BeginUpdate/EndUpdate cycle is open
	protected int m_iSelectedTag = -1; //!< the echoed selection; -1 = none

	//! (TBD_ListBox list, int tag) -- the user picked a row. One click, no confirm step.
	protected ref ScriptInvoker m_OnActivate; //!< created on first GetOnActivate

	//! (TBD_ListBox list, int tag) -- the user is hovering/focusing a row. Preview only; this is
	//! the hook that drives progressive disclosure (hover a group, reveal its slots).
	protected ref ScriptInvoker m_OnHighlight; //!< created on first GetOnHighlight

	//! Find the row container (the root with a WARNING when missing) and the empty-state widget.
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);

		m_wRoot = w;
		m_aPool = {};

		m_wContent = w.FindAnyWidget(m_sContentName);
		if (!m_wContent)
		{
			// Tolerate a layout that puts rows straight on the root.
			m_wContent = w;
			Print(string.Format("[TBD][ui] TBD_ListBox: no '%1' widget, parenting rows to the root.", m_sContentName), LogLevel.WARNING);
		}

		m_wEmptyState = w.FindAnyWidget(m_sEmptyStateName);
		UpdateEmptyState();
	}

	//! Drop the pool and every widget reference when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		if (m_aPool)
			m_aPool.Clear();

		m_wContent = null;
		m_wEmptyState = null;
		m_wRoot = null;
		m_iLiveRows = 0;
		m_iBuildCursor = -1;

		super.HandlerDeattached(w);
	}

	//! Start a rebuild. Existing rows stay on screen until EndUpdate, so a refresh never flashes.
	void BeginUpdate()
	{
		m_iBuildCursor = 0;
	}

	//! Append an interactive row. Returns the row index, or -1 if the row could not be created.
	int AddItem(string title, string detail = string.Empty, int tag = -1, TBD_EUIState state = TBD_EUIState.NORMAL, bool enabled = true)
	{
		return Emit(title, detail, tag, state, enabled, false);
	}

	//! Append a non-interactive heading -- the "group" level of side -> group -> slot.
	int AddSection(string title, string detail = string.Empty)
	{
		return Emit(title, detail, -1, TBD_EUIState.NORMAL, false, true);
	}

	//! Finish a rebuild: hide the surplus, restore the visual selection, update the empty state.
	void EndUpdate()
	{
		if (m_iBuildCursor < 0)
			return;

		m_iLiveRows = m_iBuildCursor;
		m_iBuildCursor = -1;

		for (int i = m_iLiveRows; i < m_aPool.Count(); i++)
		{
			m_aPool[i].SetRowVisible(false);
		}

		ApplySelection();
		UpdateEmptyState();
	}

	//! Convenience wrapper for callers that already hold row data.
	void SetRows(notnull array<ref TBD_ListRowData> rows)
	{
		BeginUpdate();

		foreach (TBD_ListRowData row : rows)
		{
			if (!row)
				continue;

			Emit(row.m_sTitle, row.m_sDetail, row.m_iTag, row.m_eState, row.m_bEnabled, row.m_bSection);
		}

		EndUpdate();
	}

	//! Empty the list without discarding the pool.
	void Clear()
	{
		BeginUpdate();
		EndUpdate();
	}

	//! Tag of the selected row, or -1. Selection is a *visual echo* of the last activation, not a
	//! step the user has to take -- clicking already did the thing.
	int GetSelectedTag()
	{
		return m_iSelectedTag;
	}

	//! Set the visual selection without firing OnActivate -- for restoring state after a refresh
	//! or reflecting an authoritative server answer.
	void SetSelectedTag(int tag)
	{
		m_iSelectedTag = tag;
		ApplySelection();
	}

	//! @return how many rows are live after the last EndUpdate
	int GetRowCount()
	{
		return m_iLiveRows;
	}

	//! Tag of a live row by position, or -1.
	int GetTagAt(int index)
	{
		if (index < 0 || index >= m_iLiveRows)
			return -1;

		return m_aPool[index].GetTag();
	}

	//! Put input focus on the first row that can actually be picked (skipping section headings), so
	//! a gamepad or keyboard user lands somewhere useful. Returns false when there is nothing to
	//! focus.
	bool FocusFirst()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return false;

		for (int i = 0; i < m_iLiveRows; i++)
		{
			TBD_ListBoxRow row = m_aPool[i];
			if (!row.IsSelectable())
				continue;

			Widget rowWidget = row.GetRootWidget();
			if (!rowWidget)
				continue;

			workspace.SetFocusedWidget(rowWidget);
			return true;
		}

		return false;
	}

	//! (TBD_ListBox list, int tag)
	ScriptInvoker GetOnActivate()
	{
		if (!m_OnActivate)
			m_OnActivate = new ScriptInvoker();

		return m_OnActivate;
	}

	//! (TBD_ListBox list, int tag)
	ScriptInvoker GetOnHighlight()
	{
		if (!m_OnHighlight)
			m_OnHighlight = new ScriptInvoker();

		return m_OnHighlight;
	}

	//! Called by a row on click: select it and fire OnActivate with its tag; a heading, a disabled row
	//! or an index outside the live rows is ignored.
	void OnRowActivated(int index)
	{
		if (index < 0 || index >= m_iLiveRows)
			return;

		TBD_ListBoxRow row = m_aPool[index];
		if (!row.IsSelectable())
			return;

		m_iSelectedTag = row.GetTag();
		ApplySelection();

		if (m_OnActivate)
			m_OnActivate.Invoke(this, m_iSelectedTag);
	}

	//! Called by a row on hover or focus: fire OnHighlight with its tag; an index outside the live
	//! rows is ignored.
	void OnRowHighlighted(int index)
	{
		if (index < 0 || index >= m_iLiveRows)
			return;

		if (!m_OnHighlight)
			return;

		m_OnHighlight.Invoke(this, m_aPool[index].GetTag());
	}

	//! Bind the next pooled row, growing the pool only when the list has never been this long.
	protected int Emit(string title, string detail, int tag, TBD_EUIState state, bool enabled, bool section)
	{
		if (m_iBuildCursor < 0)
		{
			Print("[TBD][ui] TBD_ListBox: AddItem/AddSection outside BeginUpdate/EndUpdate -- ignored.", LogLevel.WARNING);
			return -1;
		}

		TBD_ListBoxRow row = AcquireRow(m_iBuildCursor);
		if (!row)
			return -1;

		row.Bind(title, detail, tag, state, enabled, section);
		row.SetRowVisible(true);

		int index = m_iBuildCursor;
		m_iBuildCursor++;
		return index;
	}

	//! Pool accessor. Creates a widget only for an index the list has never reached.
	protected TBD_ListBoxRow AcquireRow(int index)
	{
		if (index < m_aPool.Count())
			return m_aPool[index];

		Widget rowWidget = TBD_UILayouts.Create(m_sRowLayout, m_wContent);
		if (!rowWidget)
		{
			Print(string.Format("[TBD][ui] TBD_ListBox: could not create row layout %1", m_sRowLayout), LogLevel.ERROR);
			return null;
		}

		// Pin the row to the full width of the content column. TBD_ListRow.layout declares Stretch on
		// its root slot, but a root slot only exists once the widget has a parent, and this one gets its
		// parent at runtime; without the stretch the row falls back to its desired width and the list
		// renders as a column of clipped text. One call per row created (rows are pooled).
		AlignableSlot.SetHorizontalAlign(rowWidget, LayoutHorizontalAlign.Stretch);

		TBD_ListBoxRow row = TBD_ListBoxRow.Cast(rowWidget.FindHandler(TBD_ListBoxRow));
		if (!row)
		{
			Print(string.Format("[TBD][ui] TBD_ListBox: row layout %1 has no TBD_ListBoxRow handler", m_sRowLayout), LogLevel.ERROR);
			rowWidget.RemoveFromHierarchy();
			return null;
		}

		int poolIndex = m_aPool.Insert(row);
		row.Attach(this, poolIndex);

		// Explicit up/down navigation, exactly as vanilla SCR_ListBoxComponent does: without it,
		// a widget sitting above or below the list steals focus at the ends of the list.
		rowWidget.SetName(string.Format("TBD_ListRow_%1", poolIndex));
		if (poolIndex > 0)
		{
			Widget previous = m_aPool[poolIndex - 1].GetRootWidget();
			if (previous)
			{
				previous.SetNavigation(WidgetNavigationDirection.DOWN, WidgetNavigationRuleType.EXPLICIT, rowWidget.GetName());
				rowWidget.SetNavigation(WidgetNavigationDirection.UP, WidgetNavigationRuleType.EXPLICIT, previous.GetName());
			}
		}

		return row;
	}

	//! Mark the live row whose tag equals the selected tag, when it is selectable; clear the others.
	protected void ApplySelection()
	{
		for (int i = 0; i < m_iLiveRows; i++)
		{
			TBD_ListBoxRow row = m_aPool[i];
			row.SetSelected(m_iSelectedTag >= 0 && row.GetTag() == m_iSelectedTag && row.IsSelectable());
		}
	}

	//! Nothing blocking, ever: an empty list says so instead of showing a void.
	protected void UpdateEmptyState()
	{
		TBD_UITheme.Show(m_wEmptyState, m_iLiveRows == 0);
	}
}

//! Row description for callers that build a list up-front (mission list, roster) rather than
//! streaming it. The streaming `AddItem`/`AddSection` path allocates nothing and should be
//! preferred for lists that refresh on replication.
class TBD_ListRowData
{
	string m_sTitle; //!< the row title
	string m_sDetail; //!< the row detail; empty hides it
	int m_iTag; //!< the caller's id; -1 for headings
	TBD_EUIState m_eState; //!< the row state
	bool m_bEnabled; //!< false disables the row
	bool m_bSection; //!< true = a section heading

	//! Describe one row; the parameters mirror `TBD_ListBox.AddItem` plus `section`.
	void TBD_ListRowData(string title, string detail = string.Empty, int tag = -1, TBD_EUIState state = TBD_EUIState.NORMAL, bool enabled = true, bool section = false)
	{
		m_sTitle = title;
		m_sDetail = detail;
		m_iTag = tag;
		m_eState = state;
		m_bEnabled = enabled;
		m_bSection = section;
	}
}
