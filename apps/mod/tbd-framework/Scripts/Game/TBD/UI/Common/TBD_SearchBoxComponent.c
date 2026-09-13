//! Pre-game rebuild (2026-09-12) — the search field: leading glyph, EditBox, clear button.
//!
//! Widget contract on `TBD_SearchBox.layout`: `SearchBorder`, `SearchBG`, `SearchIcon`,
//! `SearchInput` (EditBoxWidget), `SearchClear` (ButtonWidget, shown while there is text).
//!
//! The handler sits on the layout ROOT, not on the EditBox — the same shape vanilla uses — so
//! `OnChange` arrives with `w == m_wInput` and the clear button's `OnClick` arrives with
//! `w == m_wClear`. Owners subscribe to `GetOnChanged()` and read `GetQuery()`; filtering is
//! theirs, not ours.
class TBD_SearchBoxComponent : ScriptedWidgetComponent
{
	[Attribute("Search...", UIWidgets.EditBox, "Placeholder shown while empty")]
	protected string m_sPlaceholder;

	protected Widget m_wRoot;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected ImageWidget m_wIcon;
	protected EditBoxWidget m_wInput;
	protected Widget m_wClear;
	protected TextWidget m_wClearGlyph;

	protected bool m_bFocused;
	//! Opaque colour under the box; 0 = glass panel.
	protected int m_iGround;

	//! (TBD_SearchBoxComponent box, string query) — fires on every keystroke and on clear.
	protected ref ScriptInvoker m_OnChanged;

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBorder = w.FindAnyWidget("SearchBorder");
		m_wBackground = w.FindAnyWidget("SearchBG");
		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("SearchIcon"));
		m_wInput = EditBoxWidget.Cast(w.FindAnyWidget("SearchInput"));
		m_wClear = w.FindAnyWidget("SearchClear");
		m_wClearGlyph = TextWidget.Cast(w.FindAnyWidget("SearchClearGlyph"));

		if (m_wInput)
			m_wInput.SetPlaceholderText(m_sPlaceholder);

		if (m_wIcon)
		{
			TBD_UIIcons.Load(m_wIcon, "search");
			TBD_UITheme.Paint(m_wIcon, TBD_UITheme.MUTED_INK);
		}

		UpdateClear();
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		m_wInput = null;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	override bool OnChange(Widget w, bool finished)
	{
		if (w != m_wInput)
			return super.OnChange(w, finished);

		UpdateClear();
		Notify();
		return false;
	}

	//------------------------------------------------------------------------------------------------
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (m_wClear && (w == m_wClear || w.GetName() == "SearchClearGlyph"))
		{
			Clear();
			return true;
		}

		return super.OnClick(w, x, y, button);
	}

	//------------------------------------------------------------------------------------------------
	override bool OnFocus(Widget w, int x, int y)
	{
		if (w == m_wInput)
		{
			m_bFocused = true;
			Repaint();
		}

		return super.OnFocus(w, x, y);
	}

	//------------------------------------------------------------------------------------------------
	override bool OnFocusLost(Widget w, int x, int y)
	{
		if (w == m_wInput)
		{
			m_bFocused = false;
			Repaint();
		}

		return super.OnFocusLost(w, x, y);
	}

	// ── Public surface ──────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Current text, trimmed. Empty means "no filter".
	string GetQuery()
	{
		if (!m_wInput)
			return string.Empty;

		string text = m_wInput.GetText();
		return text.Trim();
	}

	//------------------------------------------------------------------------------------------------
	void Clear()
	{
		if (m_wInput)
			m_wInput.SetText(string.Empty);

		UpdateClear();
		Notify();
	}

	//------------------------------------------------------------------------------------------------
	void SetPlaceholder(string placeholder)
	{
		m_sPlaceholder = placeholder;
		if (m_wInput)
			m_wInput.SetPlaceholderText(placeholder);
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_SearchBoxComponent box, string query)
	ScriptInvoker GetOnChanged()
	{
		if (!m_OnChanged)
			m_OnChanged = new ScriptInvoker();

		return m_OnChanged;
	}

	//------------------------------------------------------------------------------------------------
	//! Case-insensitive "does `haystack` contain the query". Shared so every list filters alike.
	static bool Matches(string query, string haystack)
	{
		if (query.IsEmpty())
			return true;

		string q = query;
		q.ToLower();
		string h = haystack;
		h.ToLower();
		return h.Contains(q);
	}

	//------------------------------------------------------------------------------------------------
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void Notify()
	{
		if (m_OnChanged)
			m_OnChanged.Invoke(this, GetQuery());
	}

	//------------------------------------------------------------------------------------------------
	protected void UpdateClear()
	{
		TBD_UITheme.Show(m_wClear, !GetQuery().IsEmpty());
	}

	//------------------------------------------------------------------------------------------------
	//! Opaque colour under the box (the owning panel's GetGround()).
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	protected void Repaint()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.PanelGround();

		TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.INPUT_FILL, ground);
		TBD_UITheme.Paint(m_wClearGlyph, TBD_UITheme.MUTED_INK);
		TBD_UITheme.Paint(m_wInput, TBD_UITheme.ON_SURFACE);

		if (m_bFocused)
			TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.INPUT_FOCUS_BORDER, ground);
		else
			TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.INPUT_BORDER, ground);
	}
}
