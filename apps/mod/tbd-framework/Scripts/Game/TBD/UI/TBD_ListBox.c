//! T-181.7 — the reusable TBD list. The slot list, the unit list and the mission list are all
//! this widget.
//!
//! ── Why not vanilla SCR_ListBoxComponent ────────────────────────────────────────────────────
//! Read from real source (`apps/mod/vanilla_reference/Source/SCR_ListBoxComponent.c`): its
//! `AddItem` calls `CreateWidgets` per item and `Clear()` destroys every child. A TBD lobby list
//! is rebuilt on **every replicated slot claim/release** — with a 128-slot mission that is 128
//! widget creations and 128 destructions per broadcast, for content that barely changed. It also
//! drags vanilla's element layout, and therefore vanilla's look, into our screens.
//!
//! ── The cost model ──────────────────────────────────────────────────────────────────────────
//! Rows are **pooled and re-bound**. `CreateWidgets` runs once per row index, ever; a refresh
//! writes text and colour onto widgets that already exist, and surplus rows are hidden rather
//! than destroyed. So a 128-slot mission costs 128 creations across the whole session no matter
//! how many times players claim and release, and the steady-state refresh is O(visible rows) of
//! pure property writes.
//!
//! Progressive disclosure (design law) keeps that number far below 128 in practice: the lobby
//! shows side -> group -> slot, so a typical list is a dozen rows.
//!
//! ── The build cycle ─────────────────────────────────────────────────────────────────────────
//! ```
//! list.BeginUpdate();
//! list.AddSection("BRAVO");
//! list.AddItem("Squad Leader", "Cpl. Hicks", slotId, TBD_EUIState.TAKEN, false);
//! list.AddItem("Rifleman",     "",           slotId2, TBD_EUIState.NORMAL, true);
//! list.EndUpdate();
//! ```
//! No allocation on that path. `SetRows()` exists for callers that already hold a list of
//! TBD_ListRowData.
//!
//! Attach to a widget containing a `VerticalLayoutWidget` named `Content` (configurable) — put
//! that inside a `ScrollLayoutWidget` in the layout and long lists scroll for free.
class TBD_ListBox : ScriptedWidgetComponent
{
	[Attribute("{7BD1A70000000702}UI/layouts/TBD_ListRow.layout", UIWidgets.ResourceNamePicker, "Layout instantiated for every pooled row", "layout")]
	protected ResourceName m_sRowLayout;

	[Attribute("Content", UIWidgets.EditBox, "Name of the vertical layout rows are parented to")]
	protected string m_sContentName;

	[Attribute("EmptyState", UIWidgets.EditBox, "Name of the widget shown while the list is empty")]
	protected string m_sEmptyStateName;

	protected Widget m_wRoot;
	protected Widget m_wContent;
	protected Widget m_wEmptyState;

	//! Pool, index-stable. Weak elements: each row handler is owned by its widget.
	protected ref array<TBD_ListBoxRow> m_aPool;

	protected int m_iLiveRows;        //!< rows bound and visible after the last EndUpdate
	protected int m_iBuildCursor = -1;//!< >= 0 while a BeginUpdate/EndUpdate cycle is open
	protected int m_iSelectedTag = -1;

	//! (TBD_ListBox list, int tag) — the user picked a row. One click, no confirm step.
	protected ref ScriptInvoker m_OnActivate;

	//! (TBD_ListBox list, int tag) — the user is hovering/focusing a row. Preview only; this is
	//! the hook that drives progressive disclosure (hover a group, reveal its slots).
	protected ref ScriptInvoker m_OnHighlight;

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	// ── Build cycle ─────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Start a rebuild. Existing rows stay on screen until EndUpdate, so a refresh never flashes.
	void BeginUpdate()
	{
		m_iBuildCursor = 0;
	}

	//------------------------------------------------------------------------------------------------
	//! Append an interactive row. Returns the row index, or -1 if the row could not be created.
	int AddItem(string title, string detail = string.Empty, int tag = -1, TBD_EUIState state = TBD_EUIState.NORMAL, bool enabled = true)
	{
		return Emit(title, detail, tag, state, enabled, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Append a non-interactive heading — the "group" level of side -> group -> slot.
	int AddSection(string title, string detail = string.Empty)
	{
		return Emit(title, detail, -1, TBD_EUIState.NORMAL, false, true);
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Empty the list without discarding the pool.
	void Clear()
	{
		BeginUpdate();
		EndUpdate();
	}

	// ── Selection ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Tag of the selected row, or -1. Selection is a *visual echo* of the last activation, not a
	//! step the user has to take — clicking already did the thing.
	int GetSelectedTag()
	{
		return m_iSelectedTag;
	}

	//------------------------------------------------------------------------------------------------
	//! Set the visual selection without firing OnActivate — for restoring state after a refresh
	//! or reflecting an authoritative server answer.
	void SetSelectedTag(int tag)
	{
		m_iSelectedTag = tag;
		ApplySelection();
	}

	//------------------------------------------------------------------------------------------------
	int GetRowCount()
	{
		return m_iLiveRows;
	}

	//------------------------------------------------------------------------------------------------
	//! Tag of a live row by position, or -1.
	int GetTagAt(int index)
	{
		if (index < 0 || index >= m_iLiveRows)
			return -1;

		return m_aPool[index].GetTag();
	}

	//------------------------------------------------------------------------------------------------
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

	// ── Invokers ────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! (TBD_ListBox list, int tag)
	ScriptInvoker GetOnActivate()
	{
		if (!m_OnActivate)
			m_OnActivate = new ScriptInvoker();

		return m_OnActivate;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_ListBox list, int tag)
	ScriptInvoker GetOnHighlight()
	{
		if (!m_OnHighlight)
			m_OnHighlight = new ScriptInvoker();

		return m_OnHighlight;
	}

	// ── Called by TBD_ListBoxRow ────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	void OnRowHighlighted(int index)
	{
		if (index < 0 || index >= m_iLiveRows)
			return;

		if (!m_OnHighlight)
			return;

		m_OnHighlight.Invoke(this, m_aPool[index].GetTag());
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Bind the next pooled row, growing the pool only when the list has never been this long.
	protected int Emit(string title, string detail, int tag, TBD_EUIState state, bool enabled, bool section)
	{
		if (m_iBuildCursor < 0)
		{
			Print("[TBD][ui] TBD_ListBox: AddItem/AddSection outside BeginUpdate/EndUpdate — ignored.", LogLevel.WARNING);
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected void ApplySelection()
	{
		for (int i = 0; i < m_iLiveRows; i++)
		{
			TBD_ListBoxRow row = m_aPool[i];
			row.SetSelected(m_iSelectedTag >= 0 && row.GetTag() == m_iSelectedTag && row.IsSelectable());
		}
	}

	//------------------------------------------------------------------------------------------------
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
	string m_sTitle;
	string m_sDetail;
	int m_iTag;
	TBD_EUIState m_eState;
	bool m_bEnabled;
	bool m_bSection;

	//------------------------------------------------------------------------------------------------
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
