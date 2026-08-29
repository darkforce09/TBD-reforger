//! T-181.7 - the Aegis style layer for every TBD screen.
//!
//! ONE place holds colour. Screens call TBD_UITheme.Paint(...) with a named token; they never
//! write a literal colour. Tokens are ported 1:1 from the website's design system,
//! `apps/website/frontend/style/aegis.css` (@theme block) - same names, same hex, so the mod and
//! the site cannot drift.
//!
//! -- Colour encoding ------------------------------------------------------------------------
//! Every token is packed **0xAARRGGBB**, which is what `Widget.SetColorInt(int)` consumes.
//! Grounded, not guessed: vanilla exposes the global `ARGB(a, r, g, b)` builder and its results
//! are fed straight to `SetColorInt` - so the high byte is alpha. `Color.FromInt(int)` uses the
//! same packing and is what `Colour()` below returns for the APIs that want a `Color` object.
//!
//! -- Typography / spacing -------------------------------------------------------------------
//! Enfusion layouts carry absolute font sizes, authored against a 1920x1080 reference surface,
//! so the CSS px scale maps across unchanged. The constants exist so a `.layout` and a runtime
//! `SetText` agree on the same ladder; the engine has no stylesheet to read them from, so they
//! are duplicated by hand into the `.layout` files and MUST be kept in step with this file.
//!
//! Design law this file encodes (docs/mod/TBD_MOD_DESIGN.md S2):
//!   * ONE accent colour. ACTION is the single high-priority trigger blue; PRIMARY is the
//!     everyday "active/selected" blue. Nothing else is allowed to shout.
//!   * Generous whitespace - the spacing ladder starts at 8 and the screen gutter is 24.
class TBD_UITheme
{
	// -- Surfaces - "Midnight Navy" foundation ------------------------------------------------
	static const int SURFACE                  = 0xFF0D1322; //!< --color-surface / --color-background
	static const int SURFACE_CONTAINER_LOWEST = 0xFF080E1D; //!< --color-surface-container-lowest
	static const int SURFACE_CONTAINER_LOW    = 0xFF151B2B; //!< --color-surface-container-low
	static const int SURFACE_CONTAINER        = 0xFF191F2F; //!< --color-surface-container
	static const int SURFACE_CONTAINER_HIGH   = 0xFF242A3A; //!< --color-surface-container-high
	static const int SURFACE_CONTAINER_HIGHEST= 0xFF2F3445; //!< --color-surface-container-highest
	static const int SURFACE_VARIANT          = 0xFF2F3445; //!< --color-surface-variant
	static const int SURFACE_BRIGHT           = 0xFF333949; //!< --color-surface-bright

	// -- Content ------------------------------------------------------------------------------
	static const int ON_SURFACE               = 0xFFDDE2F7; //!< --color-on-surface (body text)
	static const int ON_SURFACE_VARIANT       = 0xFFC4C6D0; //!< --color-on-surface-variant (secondary)

	// -- Accents. PRIMARY = active/selected. ACTION = the one primary trigger. ----------------
	static const int PRIMARY                  = 0xFFADC6FF; //!< --color-primary
	static const int ON_PRIMARY               = 0xFF122F5F; //!< --color-on-primary
	static const int PRIMARY_FIXED            = 0xFFD8E2FF; //!< --color-primary-fixed
	static const int ON_PRIMARY_CONTAINER     = 0xFF385283; //!< --color-on-primary-container
	static const int ACTION                   = 0xFF3B82F6; //!< --color-action  (Deploy / Save)
	static const int ON_ACTION                = 0xFFFFFFFF; //!< --color-on-action
	static const int TERTIARY                 = 0xFFC3E7FF; //!< --color-tertiary

	// -- Lines --------------------------------------------------------------------------------
	static const int OUTLINE                  = 0xFF8E909A; //!< --color-outline
	static const int OUTLINE_VARIANT          = 0xFF44474F; //!< --color-outline-variant
	static const int BORDER_SUBTLE            = 0xFF374151; //!< --color-border-subtle

	// -- Semantic -----------------------------------------------------------------------------
	static const int SUCCESS                  = 0xFF22C55E; //!< --color-success
	static const int WARNING                  = 0xFFEAB308; //!< --color-warning
	static const int ERROR                    = 0xFFEF4444; //!< --color-error
	static const int ERROR_ALERT              = 0xFFF87171; //!< --color-error-alert
	static const int TACTICAL_YELLOW          = 0xFFFACC15; //!< --color-tactical-yellow

	// -- Composites the CSS expresses with rgba() ---------------------------------------------
	static const int SURFACE_GLASS            = 0xB31F2937; //!< --color-surface-glass  rgba(31,41,55,.70)
	static const int SCRIM                    = 0xB8080E1D; //!< full-bleed backdrop     rgba(8,14,29,.72)
	static const int TRANSPARENT              = 0x00000000;

	// -- Derived interaction tints. Nothing outside this file may invent one. -----------------
	static const int ROW_IDLE                 = 0x00000000; //!< rows sit on the panel, not on a chip
	static const int ROW_HOVER                = 0xFF242A3A; //!< surface-container-high
	static const int ROW_SELECTED             = 0xFF2F3445; //!< surface-container-highest
	static const int ROW_DISABLED_TEXT        = 0xFF8E909A; //!< outline, used as "unavailable" ink

	// -- Type scale (px @ 1920x1080). Mirrors aegis.css --text-*. -----------------------------
	static const int TEXT_HEADLINE_LG = 30;
	static const int TEXT_HEADLINE_MD = 24;
	static const int TEXT_HEADLINE_SM = 20;
	static const int TEXT_BODY_LG     = 18;
	static const int TEXT_BODY_MD     = 16;
	static const int TEXT_LABEL_MD    = 14;
	static const int TEXT_LABEL_SM    = 12;

	// -- Spacing ladder. GUTTER mirrors --spacing-gutter (1.5rem). ----------------------------
	static const int SPACE_XS = 4;
	static const int SPACE_SM = 8;
	static const int SPACE_MD = 16;
	static const int GUTTER   = 24;
	static const int SPACE_LG = 32;
	static const int SPACE_XL = 48;

	//! Lazily built Color objects, one per token that ever needs the object form. Widgets take
	//! ints via SetColorInt on the hot path; this exists for the handful of engine APIs (and
	//! `.layout` attributes) that insist on a Color.
	protected static ref map<int, ref Color> m_mColours;

	//------------------------------------------------------------------------------------------------
	//! ARGB int -> Color, memoised. Never allocate a Color per frame.
	static Color Colour(int argb)
	{
		if (!m_mColours)
			m_mColours = new map<int, ref Color>();

		Color cached = m_mColours.Get(argb);
		if (cached)
			return cached;

		// Insert the new instance straight into the owning map - never park a freshly `new`ed
		// managed object in a non-ref local first.
		m_mColours.Insert(argb, Color.FromInt(argb));
		return m_mColours.Get(argb);
	}

	//------------------------------------------------------------------------------------------------
	//! Null-safe tint. Every screen paints through here so a missing widget is a no-op, not a
	//! crash on a layout that changed under us.
	static void Paint(Widget w, int argb)
	{
		if (w)
			w.SetColorInt(argb);
	}

	//------------------------------------------------------------------------------------------------
	//! Null-safe text write.
	static void Write(TextWidget w, string text)
	{
		if (w)
			w.SetText(text);
	}

	//------------------------------------------------------------------------------------------------
	//! Null-safe visibility.
	static void Show(Widget w, bool visible)
	{
		if (w)
			w.SetVisible(visible);
	}

	//------------------------------------------------------------------------------------------------
	//! Background tint for an interactive surface (list row, button) given its semantic state.
	//! Hover beats selection beats idle - immediate feedback is design law, so the pointer
	//! always wins the readout.
	static int StateBackground(TBD_EUIState state, bool hovered, bool selected)
	{
		if (state == TBD_EUIState.LOCKED)
			return ROW_IDLE;

		if (hovered)
			return ROW_HOVER;

		if (selected || state == TBD_EUIState.ACTIVE)
			return ROW_SELECTED;

		return ROW_IDLE;
	}

	//------------------------------------------------------------------------------------------------
	//! Ink for the primary line of an interactive surface.
	static int StateTitle(TBD_EUIState state)
	{
		switch (state)
		{
			case TBD_EUIState.ACTIVE: return PRIMARY;
			case TBD_EUIState.TAKEN:  return ON_SURFACE_VARIANT;
			case TBD_EUIState.LOCKED: return ROW_DISABLED_TEXT;
			case TBD_EUIState.DANGER: return ERROR_ALERT;
		}

		return ON_SURFACE;
	}

	//------------------------------------------------------------------------------------------------
	//! Ink for the secondary/right-hand line.
	static int StateDetail(TBD_EUIState state)
	{
		switch (state)
		{
			case TBD_EUIState.ACTIVE: return PRIMARY;
			case TBD_EUIState.LOCKED: return ROW_DISABLED_TEXT;
			case TBD_EUIState.DANGER: return ERROR_ALERT;
		}

		return ON_SURFACE_VARIANT;
	}

	//------------------------------------------------------------------------------------------------
	//! The 2px leading rail that marks the active row. TRANSPARENT means "draw nothing" -
	//! progressive disclosure, not a permanent cage of borders.
	static int StateAccent(TBD_EUIState state, bool selected)
	{
		if (state == TBD_EUIState.ACTIVE || selected)
			return PRIMARY;

		if (state == TBD_EUIState.DANGER)
			return ERROR;

		return TRANSPARENT;
	}
}

//! Semantic state shared by every TBD interactive surface. The lobby maps its own vocabulary
//! onto these (free slot -> NORMAL, your slot -> ACTIVE, someone else's -> TAKEN, wrong side ->
//! LOCKED) so colour decisions stay in TBD_UITheme and never in a screen.
enum TBD_EUIState
{
	NORMAL,
	ACTIVE,
	TAKEN,
	LOCKED,
	DANGER
}
