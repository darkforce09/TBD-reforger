/**
 * @file TBD_SearchBoxComponent.c
 * @brief The search field: leading glyph, edit box and a clear button shown while there is text.
 *
 * Role: handler of `TBD_SearchBox.layout`; reports every keystroke and clear through
 * `GetOnChanged()(box, query)` and offers the shared `Matches` filter test.
 * Position: nested in the pre-game screens' list panels; owners read `GetQuery()` and filter.
 * State: the widget references, the focus flag and the ground colour, on the client.
 * Invariants: widget contract `SearchBorder`, `SearchBG`, `SearchIcon`, `SearchInput`
 * (EditBoxWidget), `SearchClear` (ButtonWidget); the handler sits on the layout root, not on the
 * edit box, so `OnChange` arrives with `w == m_wInput` and the clear click with `w == m_wClear`;
 * filtering belongs to the owner.
 */

//! Search box handler: query text, clear button, focus border.
class TBD_SearchBoxComponent : ScriptedWidgetComponent
{
	[Attribute("Search...", UIWidgets.EditBox, desc: "Placeholder shown while empty")]
	protected string m_sPlaceholder; //!< edit box placeholder; default "Search..."

	protected Widget m_wRoot; //!< the layout root this handler sits on
	protected Widget m_wBorder; //!< `SearchBorder` frame dock, rounded at attach
	protected Widget m_wBackground; //!< `SearchBG` frame dock, rounded at attach
	protected ImageWidget m_wIcon; //!< `SearchIcon` glyph
	protected EditBoxWidget m_wInput; //!< `SearchInput` edit box
	protected Widget m_wClear; //!< `SearchClear` button, shown while there is text
	protected TextWidget m_wClearGlyph; //!< `SearchClearGlyph` text

	protected bool m_bFocused; //!< true while the edit box has focus
	protected int m_iGround; //!< Opaque colour under the box; 0 = glass panel.

	//! (TBD_SearchBoxComponent box, string query) -- fires on every keystroke and on clear.
	protected ref ScriptInvoker m_OnChanged; //!< created on first GetOnChanged

	//! Find the widgets, round the frame docks, set the placeholder and the search glyph, paint.
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

	//! Drop the widget references when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		m_wInput = null;
		super.HandlerDeattached(w);
	}

	//! An edit box change shows or hides the clear button and fires OnChanged.
	//! @return false for the edit box; otherwise the base handler's answer
	override bool OnChange(Widget w, bool finished)
	{
		if (w != m_wInput)
			return super.OnChange(w, finished);

		UpdateClear();
		Notify();
		return false;
	}

	//! A click on the clear button or its glyph empties the query.
	//! @return true when the click cleared; otherwise the base handler's answer
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (m_wClear && (w == m_wClear || w.GetName() == "SearchClearGlyph"))
		{
			Clear();
			return true;
		}

		return super.OnClick(w, x, y, button);
	}

	//! The edit box gaining focus lights the border.
	override bool OnFocus(Widget w, int x, int y)
	{
		if (w == m_wInput)
		{
			m_bFocused = true;
			Repaint();
		}

		return super.OnFocus(w, x, y);
	}

	//! The edit box losing focus dims the border.
	override bool OnFocusLost(Widget w, int x, int y)
	{
		if (w == m_wInput)
		{
			m_bFocused = false;
			Repaint();
		}

		return super.OnFocusLost(w, x, y);
	}

	//! Current text, trimmed. Empty means "no filter".
	string GetQuery()
	{
		if (!m_wInput)
			return string.Empty;

		string text = m_wInput.GetText();
		return text.Trim();
	}

	//! Empty the edit box, hide the clear button and fire OnChanged.
	void Clear()
	{
		if (m_wInput)
			m_wInput.SetText(string.Empty);

		UpdateClear();
		Notify();
	}

	//! Set the placeholder shown while the box is empty.
	void SetPlaceholder(string placeholder)
	{
		m_sPlaceholder = placeholder;
		if (m_wInput)
			m_wInput.SetPlaceholderText(placeholder);
	}

	//! (TBD_SearchBoxComponent box, string query)
	ScriptInvoker GetOnChanged()
	{
		if (!m_OnChanged)
			m_OnChanged = new ScriptInvoker();

		return m_OnChanged;
	}

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

	//! @return the layout root this handler sits on
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! Fire OnChanged with the current query.
	protected void Notify()
	{
		if (m_OnChanged)
			m_OnChanged.Invoke(this, GetQuery());
	}

	//! Show the clear button while the query is not empty.
	protected void UpdateClear()
	{
		TBD_UITheme.Show(m_wClear, !GetQuery().IsEmpty());
	}

	//! Opaque colour under the box (the owning panel's GetGround()).
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
	}

	//! Paint the fill, the clear glyph and the text over the ground; the border lights while focused.
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
