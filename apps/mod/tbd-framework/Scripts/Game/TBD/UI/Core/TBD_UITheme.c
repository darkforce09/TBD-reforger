//! T-181.7 — the Aegis style layer for every TBD screen.
//!
//! ONE place holds colour. Screens call TBD_UITheme.Paint(...) with a named token; they never
//! write a literal colour. Tokens are ported 1:1 from the website's design system,
//! `apps/website/frontend/style/aegis.css` (@theme block) — same names, same hex, so the mod and
//! the site cannot drift.
//!
//! ── Colour law (three parts, all measured 2026-09-12 on the rebuilt selector) ──────────────
//! 1. **Tokens are CSS sRGB**, packed 0xAARRGGBB — the hex you read off aegis.css / the Stitch
//!    mockups. The engine works in LINEAR space: `Widget.SetColorInt(int)` and `Color.FromInt`
//!    read their bytes as linear, so a raw token renders ~2.2x too bright (`#0D1322` came out as
//!    `#405273`). `Colour()` converts through `Color.FromSRGBA(r, g, b, a)` and every paint uses
//!    `SetColor(Color)`. Never call `SetColorInt` with a token.
//! 2. **Alpha is composited HERE, in sRGB, never by the engine.** A browser blends `rgba()` in
//!    sRGB; Enfusion blends in linear. White/3 % over `#0c1220` is `#141927` in the mockup and a
//!    `#343434` grey band in the engine; 25 % blue-950 is a whisper in CSS and a saturated navy
//!    column in the engine; white/12 hairlines came out `#616161`. So `Paint()` hands the engine
//!    an OPAQUE colour: a translucent token is flattened with `Over(token, ground)` first, where
//!    `ground` is the opaque colour the widget actually sits on (`PanelGround()` by default —
//!    components that know better pass theirs through `PaintOver`). Only `PaintAlpha()` sends
//!    real alpha, and only for surfaces over the 3D world (SCRIM, SURFACE_GLASS).
//! 3. **Fonts are layout-side.** `TextWidget` has no font setter, so the `.layout` files carry
//!    `Font "{GUID}…fnt"`; the FONT_* constants below are the single list of which GUID is which.
//!
//! ── Typography / spacing ───────────────────────────────────────────────────────────────────
//! Enfusion layouts carry absolute font sizes, authored against a 1920x1080 reference surface,
//! so the CSS px scale maps across unchanged. The constants exist so a `.layout` and a runtime
//! `SetText` agree on the same ladder; the engine has no stylesheet to read them from, so they
//! are duplicated by hand into the `.layout` files and MUST be kept in step with this file.
//!
//! Design law this file encodes (docs/mod/TBD_MOD_DESIGN.md §2):
//!   * ONE accent colour. ACTION is the single high-priority trigger blue; PRIMARY is the
//!     everyday "active/selected" blue. Nothing else is allowed to shout.
//!   * Generous whitespace — the spacing ladder starts at 8 and the screen gutter is 24.
class TBD_UITheme
{
	// ── Surfaces — "Midnight Navy" foundation ────────────────────────────────────────────────
	static const int SURFACE                  = 0xFF0D1322; //!< --color-surface / --color-background
	static const int SURFACE_CONTAINER_LOWEST = 0xFF080E1D; //!< --color-surface-container-lowest
	static const int SURFACE_CONTAINER_LOW    = 0xFF151B2B; //!< --color-surface-container-low
	static const int SURFACE_CONTAINER        = 0xFF191F2F; //!< --color-surface-container
	static const int SURFACE_CONTAINER_HIGH   = 0xFF242A3A; //!< --color-surface-container-high
	static const int SURFACE_CONTAINER_HIGHEST= 0xFF2F3445; //!< --color-surface-container-highest
	static const int SURFACE_VARIANT          = 0xFF2F3445; //!< --color-surface-variant
	static const int SURFACE_BRIGHT           = 0xFF333949; //!< --color-surface-bright

	// ── Content ──────────────────────────────────────────────────────────────────────────────
	static const int ON_SURFACE               = 0xFFDDE2F7; //!< --color-on-surface (body text)
	static const int ON_SURFACE_VARIANT       = 0xFFC4C6D0; //!< --color-on-surface-variant (secondary)

	// ── Accents. PRIMARY = active/selected. ACTION = the one primary trigger. ────────────────
	static const int PRIMARY                  = 0xFFADC6FF; //!< --color-primary
	static const int ON_PRIMARY               = 0xFF122F5F; //!< --color-on-primary
	static const int PRIMARY_FIXED            = 0xFFD8E2FF; //!< --color-primary-fixed
	static const int PRIMARY_CONTAINER        = 0xFF4D8EFF; //!< --color-primary-container
	static const int ON_PRIMARY_CONTAINER     = 0xFF385283; //!< --color-on-primary-container
	static const int ACTION                   = 0xFF3B82F6; //!< --color-action  (Deploy / Save)
	static const int ON_ACTION                = 0xFFFFFFFF; //!< --color-on-action
	static const int ACTION_BORDER            = 0x6660A5FA; //!< border-blue-400/40 on the primary button
	static const int TERTIARY                 = 0xFFC3E7FF; //!< --color-tertiary
	static const int TERTIARY_WARM            = 0xFFFFB786; //!< --color-tertiary-warm (#ffb786 in stitch mockup)
	static const int TERTIARY_CONTAINER       = 0xFFDF7412; //!< --color-tertiary-container (#df7412 in stitch mockup)
	static const int CARD_BORDER              = 0xFF38BDF8; //!< --color-card-border (#38bdf8 in stitch mockup)

	// ── Lines ────────────────────────────────────────────────────────────────────────────────
	static const int OUTLINE                  = 0xFF8E909A; //!< --color-outline
	static const int OUTLINE_VARIANT          = 0xFF44474F; //!< --color-outline-variant
	static const int BORDER_SUBTLE            = 0xFF374151; //!< --color-border-subtle

	// ── Semantic ─────────────────────────────────────────────────────────────────────────────
	static const int SUCCESS                  = 0xFF22C55E; //!< --color-success
	static const int WARNING                  = 0xFFEAB308; //!< --color-warning
	static const int ERROR                    = 0xFFEF4444; //!< --color-error
	static const int ERROR_ALERT              = 0xFFF87171; //!< --color-error-alert
	static const int TACTICAL_YELLOW          = 0xFFFACC15; //!< --color-tactical-yellow

	// ── Composites the CSS expresses with rgba() ─────────────────────────────────────────────
	static const int SURFACE_GLASS            = 0xB31F2937; //!< --color-surface-glass  rgba(31,41,55,.70)
	static const int SCRIM                    = 0xB8080E1D; //!< full-bleed backdrop     rgba(8,14,29,.72)
	static const int TRANSPARENT              = 0x00000000;

	// ── Derived interaction tints. Nothing outside this file may invent one. ─────────────────
	static const int ROW_IDLE                 = 0x00000000; //!< rows sit on the panel, not on a chip
	static const int ROW_HOVER                = 0xFF242A3A; //!< surface-container-high
	static const int ROW_SELECTED             = 0xFF2F3445; //!< surface-container-highest
	static const int ROW_DISABLED_TEXT        = 0xFF8E909A; //!< outline, used as "unavailable" ink

	// ── Type scale (px @ 1920x1080). Mirrors aegis.css --text-*. ─────────────────────────────
	static const int TEXT_HEADLINE_LG = 30;
	static const int TEXT_HEADLINE_MD = 24;
	static const int TEXT_HEADLINE_SM = 20;
	static const int TEXT_BODY_LG     = 18;
	static const int TEXT_BODY_MD     = 16;
	static const int TEXT_LABEL_MD    = 14;
	static const int TEXT_LABEL_SM    = 12;
	// Pre-game ladder (mockup px + 1: MSDF glyphs at 1080p read a step smaller than browser px).
	static const int TEXT_HEADER      = 14; //!< uppercase panel headers, card titles (mockup 13)
	static const int TEXT_BODY        = 13; //!< row titles, body copy (mockup 12)
	static const int TEXT_MONO_SM     = 12; //!< slot counts, versions, values (mockup 11)
	static const int TEXT_TAG         = 11; //!< chips, tags, badges (mockup 10)

	// ── Fonts. Layout-side only (`Font "{GUID}…"`); TextWidget has no setter. ────────────────
	//! Uppercase headers, card titles, nav + button labels (mockup: Inter 600).
	static const ResourceName FONT_HEAD      = "{CD2634D279AB011A}UI/Fonts/Roboto/Roboto_Bold.fnt";
	//! Body copy, row titles, key text (mockup: Inter 400/500).
	static const ResourceName FONT_BODY      = "{3E7733BAC8C831F6}UI/Fonts/RobotoCondensed/RobotoCondensed_Regular.fnt";
	//! Emphasised body (selected row title, faction role).
	static const ResourceName FONT_BODY_BOLD = "{EABA4FE9D014CCEF}UI/Fonts/RobotoCondensed/RobotoCondensed_Bold.fnt";
	//! Chips, counts, versions, values, search input (mockup: JetBrains Mono).
	static const ResourceName FONT_MONO      = "{0E041C5B1F27DCEA}ui/fonts/robotomono_msdf_28.fnt";

	// ── Corner radii (reference px). Applied by TBD_UILayouts.MountRounded. ──────────────────
	static const int RADIUS_PANEL = 12; //!< rounded-xl: panels, inspector, dropdown menu
	static const int RADIUS_ROW   = 8;  //!< rounded-lg: rows, cards, inputs, buttons, nav items
	static const int RADIUS_TAG   = 6;  //!< rounded-md: chips / tags
	static const int RADIUS_PILL  = 10; //!< rounded-full on a 20 px pill

	// ── Spacing ladder. GUTTER mirrors --spacing-gutter (1.5rem). ────────────────────────────
	static const int SPACE_XS = 4;
	static const int SPACE_SM = 8;
	static const int SPACE_MD = 16;
	static const int GUTTER   = 24;
	static const int SPACE_LG = 32;
	static const int SPACE_XL = 48;

	// ── Glass chrome (Stitch pre-game mockups, 2026-09-12). Every value is an alpha over SURFACE. ─
	static const int GLASS_BORDER             = 0x1AFFFFFF; //!< border-white/10 (browser + inspector mockups)
	static const int GLASS_BORDER_STRONG      = 0x26FFFFFF; //!< border-white/15
	static const int PANEL_FILL               = 0xE60F172A; //!< bg-slate-900/90 (terrain card); lighter than the backdrop so a panel edge reads
	static const int PILL_FILL                = 0x263B82F6; //!< bg-blue-500/15  (rounded-full count pills)
	static const int PILL_BORDER              = 0x4D60A5FA; //!< border-blue-400/30
	static const int PANEL_HEADER_FILL        = 0x08FFFFFF; //!< bg-white/[0.03]
	static const int INSET_FILL               = 0x59141D2E; //!< bg-slate-800/35 inset boxes
	static const int INPUT_FILL               = 0xCC0F172A; //!< bg-slate-900/80
	static const int INPUT_BORDER             = 0x99334155; //!< border-slate-700/60
	static const int INPUT_FOCUS_BORDER       = 0xCC3B82F6; //!< focus:border-blue-500/80
	static const int MUTED_INK                = 0xFF94A3B8; //!< text-slate-400
	static const int DIM_INK                  = 0xFF64748B; //!< text-slate-500
	static const int BRIGHT_INK               = 0xFFF8FAFC; //!< text-white / slate-50

	// ── Nav item / tab strip ────────────────────────────────────────────────────────────────
	static const int STRIP_FILL               = 0xB3020617; //!< bg-slate-950/70
	static const int STRIP_BORDER             = 0xCC1E293B; //!< border-slate-800/80
	static const int NAV_ACTIVE_FILL          = 0xCC334155; //!< bg-slate-700/80
	static const int NAV_ACTIVE_BORDER        = 0x99475569; //!< border-slate-600/60
	static const int NAV_HOVER_FILL           = 0x661E293B; //!< hover:bg-slate-800/40

	// ── Cards (scenario browser) ────────────────────────────────────────────────────────────
	static const int CARD_IDLE_FILL           = 0xB3141D2E; //!< rgba(20,29,46,.7)
	static const int CARD_IDLE_BORDER         = 0x14FFFFFF; //!< rgba(255,255,255,.08)
	static const int CARD_HOVER_FILL          = 0xE61A253A; //!< rgba(26,37,58,.9)
	static const int CARD_HOVER_BORDER        = 0x6660A5FA; //!< rgba(96,165,250,.4)
	static const int CARD_SELECTED_FILL       = 0x472563EB; //!< rgba(37,99,235,.28)
	static const int CARD_SELECTED_BORDER     = 0xCC60A5FA; //!< rgba(96,165,250,.8)
	static const int CARD_SELECTED_INK        = 0xFF7DD3FC; //!< sky-300
	static const int ROW_ACTIVE_GLOW          = 0xFF60A5FA; //!< terrain row top-left light bar
	static const int ROW_SELECTED_BORDER      = 0xB33B82F6; //!< terrain row border-blue-500/70
	static const int SEARCH_STRIP_FILL        = 0x4D0B1120; //!< terrain search strip bg-tactical-surface/30
	// Inspector hero (a photo under a fade; the only chrome painted with real alpha besides SCRIM).
	static const int HERO_FADE                = 0xFF0D1525; //!< from-[#0d1525]; the fade texture carries the ramp
	static const int HERO_DIM                 = 0x730B1120; //!< uniform dim over the satellite so white type reads
	static const int HERO_ART_INK             = 0x4038BDF8; //!< topo-grid art at 25 % (terrains without imagery)
	// Custom 4 px scrollbar (the engine one is clipped away).
	static const int SCROLL_TRACK             = 0x800F172A; //!< rgba(15,23,42,.5)
	static const int SCROLL_THUMB             = 0x99334155; //!< rgba(51,65,85,.6)
	static const int SCROLL_THUMB_HOVER       = 0xCC64748B; //!< rgba(100,116,139,.8)

	// ── Lobby (pass 5): faction rows, ORBAT squads / slots, kit inspector, bottom-bar toggles ──
	static const int HOLDER_INK               = 0xFFFBBF24; //!< text-amber-400 — a seat's holder, and kit counts (`x4`)
	static const int SLOT_OPEN_FILL           = 0x660F172A; //!< bg-slate-900/40 — an unslotted row
	static const int SLOT_HOVER_FILL          = 0x331E293B; //!< hover:bg-slate-800/20
	static const int SLOT_RULE                = 0x801E293B; //!< divide-slate-800/50 between slot rows
	static const int SQUAD_CARD_FILL          = 0x80080E1D; //!< bg-surface-dim/50
	static const int SQUAD_HEADER_FILL        = 0x661E293B; //!< bg-slate-800/40 — the callsign strip
	static const int SQUAD_BODY_FILL          = 0xCC080D19; //!< bg-[#080d19]/80 — the slot list under it
	static const int KIT_HEADER_FILL          = 0xFF0D1527; //!< kit inspector title band
	static const int KIT_CARD_FILL            = 0xFF0E172A; //!< kit section card
	static const int KIT_CARD_BORDER          = 0xFF1F293D;
	static const int KIT_CELL_FILL            = 0xFF111C30; //!< gear / gadget / tool cell
	static const int KIT_CELL_BORDER          = 0xFF1C2A42;
	static const int KIT_WEAPON_FILL          = 0xFF090F1D; //!< weapon slot card + the preview box
	static const int KIT_WEAPON_BORDER        = 0xFF1A2337;
	static const int BTN_SUCCESS_FILL         = 0xFF059669; //!< bg-emerald-600 (Ready, armed)
	static const int BTN_SUCCESS_BORDER       = 0x6634D399; //!< border-emerald-400/40
	static const int BTN_WARNING_FILL         = 0x99451A03; //!< bg-amber-950/60 (Unlock Lobby)
	static const int BTN_WARNING_BORDER       = 0x99B45309; //!< border-amber-700/60
	static const int BTN_WARNING_INK          = 0xFFFCD34D; //!< text-amber-300

	//! Memoised linear Color per sRGB token — one allocation per distinct colour, ever. Every
	//! paint goes through here (`SetColor(Color)`); nothing in the framework uses SetColorInt.
	protected static ref map<int, ref Color> m_mColours;

	//------------------------------------------------------------------------------------------------
	//! sRGB ARGB token -> LINEAR Color, memoised. Never allocate a Color per frame.
	static Color Colour(int argb)
	{
		if (!m_mColours)
		{
			m_mColours = new map<int, ref Color>();
			SelfCheck();
		}

		Color cached = m_mColours.Get(argb);
		if (cached)
			return cached;

		int a = (argb >> 24) & 0xFF;
		int r = (argb >> 16) & 0xFF;
		int g = (argb >> 8) & 0xFF;
		int b = argb & 0xFF;

		// Insert the new instance straight into the owning map — never park a freshly `new`ed
		// managed object in a non-ref local first.
		m_mColours.Insert(argb, Color.FromSRGBA(r, g, b, a));
		return m_mColours.Get(argb);
	}

	//------------------------------------------------------------------------------------------------
	//! Runs once, on the first colour ever built. The two values are the ones from the colour law
	//! above (white/3 % and blue-600/28 over the panel fill); a mismatch means Over() drifted and
	//! every tint on screen is wrong, so it is a WARNING, not a debug line.
	protected static void SelfCheck()
	{
		int header = Over(0x08FFFFFF, 0xFF0C1220);
		int selected = Over(0x472563EB, 0xFF0C1220);
		if (header != 0xFF141927 || selected != 0xFF132959)
			Print(string.Format("[TBD][ui] WARNING sRGB compositing self-check failed: header %1 selected %2", header, selected), LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	//! CSS source-over in sRGB: `top` (any alpha) flattened onto an opaque `ground`. Integer math,
	//! rounds to nearest. This is the only place a translucent token may be resolved.
	static int Over(int top, int ground)
	{
		int a = (top >> 24) & 0xFF;
		if (a >= 0xFF)
			return top;
		if (a <= 0)
			return ground;

		int ia = 255 - a;
		int r = (((top >> 16) & 0xFF) * a + ((ground >> 16) & 0xFF) * ia + 127) / 255;
		int g = (((top >> 8) & 0xFF) * a + ((ground >> 8) & 0xFF) * ia + 127) / 255;
		int b = ((top & 0xFF) * a + (ground & 0xFF) * ia + 127) / 255;
		return 0xFF000000 | (r << 16) | (g << 8) | b;
	}

	// ── Grounds: the opaque colour a widget actually sits on. Derived, never typed. ──────────
	//! The screen backdrop.
	static int Ground()
	{
		return SURFACE_CONTAINER_LOWEST;
	}

	//! A glass panel on the backdrop — what almost everything sits on.
	static int PanelGround()
	{
		return Over(PANEL_FILL, SURFACE_CONTAINER_LOWEST);
	}

	//! An idle scenario card on a panel.
	static int CardGround()
	{
		return Over(CARD_IDLE_FILL, PanelGround());
	}

	//! A selected scenario card on a panel (its tag chip sits on this).
	static int SelectedCardGround()
	{
		return Over(CARD_SELECTED_FILL, PanelGround());
	}

	//! A tinted panel (faction column) on a glass panel.
	static int TintGround(TBD_EUITint tint)
	{
		return Over(PanelFill(tint), PanelGround());
	}

	//------------------------------------------------------------------------------------------------
	//! Null-safe tint over the default ground (a glass panel). A translucent token is flattened
	//! in sRGB first; the engine only ever receives opaque or fully transparent colours.
	static void Paint(Widget w, int argb)
	{
		PaintOver(w, argb, PanelGround());
	}

	//------------------------------------------------------------------------------------------------
	//! Same, over a known ground. Components that know what they sit on use this.
	static void PaintOver(Widget w, int argb, int ground)
	{
		if (!w)
			return;

		int a = (argb >> 24) & 0xFF;
		if (a <= 0)
		{
			// Through Tint(), never a bare SetColor: the rounded shape under the dock must be
			// HIDDEN (MEASURED run 3, 2026-09-12: the early return here left idle rows and
			// inactive nav pills solid white - seven untinted images).
			Tint(w, Colour(TRANSPARENT));
			return;
		}

		if (a < 0xFF)
			argb = Over(argb, ground);

		Tint(w, Colour(argb));
	}

	//------------------------------------------------------------------------------------------------
	//! Real engine alpha (linear blend). ONLY for surfaces over the 3D world — SCRIM, SURFACE_GLASS.
	//! Chrome never comes through here; it would render brighter than the mockup.
	static void PaintAlpha(Widget w, int argb)
	{
		if (w)
			Tint(w, Colour(argb));
	}

	//------------------------------------------------------------------------------------------------
	//! SetColor on `w`, and on every image of a rounded shape mounted inside it. MEASURED
	//! 2026-09-12: a colour set on a FrameWidget dock does not reach its children even with
	//! `"Inherit Color"` — the shapes rendered white — so the seven images are painted by hand.
	//! Only subtrees rooted at a `Rounded*` widget are walked; a dock's other children (text,
	//! icons) keep their own ink.
	protected static void Tint(Widget w, Color colour)
	{
		w.SetColor(colour);

		// A transparent paint HIDES the shape rather than trusting alpha 0 to reach seven
		// children (MEASURED 2026-09-12: it did not — idle rows drew white outlines).
		bool visible = colour.A() > 0;

		Widget child = w.GetChildren();
		while (child)
		{
			if (child.GetName().IndexOf("Rounded") == 0)
			{
				child.SetVisible(visible);
				if (visible)
					TintTree(child, colour);
			}

			child = child.GetSibling();
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void TintTree(Widget w, Color colour)
	{
		w.SetColor(colour);

		Widget child = w.GetChildren();
		while (child)
		{
			TintTree(child, colour);
			child = child.GetSibling();
		}
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
	//! Hover beats selection beats idle — immediate feedback is design law, so the pointer
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
	//! The 2px leading rail that marks the active row. TRANSPARENT means "draw nothing" —
	//! progressive disclosure, not a permanent cage of borders.
	static int StateAccent(TBD_EUIState state, bool selected)
	{
		if (state == TBD_EUIState.ACTIVE || selected)
			return PRIMARY;

		if (state == TBD_EUIState.DANGER)
			return ERROR;

		return TRANSPARENT;
	}
	// ── Tinted surfaces: chips, badges and faction-coloured panels ──────────────────────────
	// One tint enum drives fill / border / ink so a chip and the panel around it always agree.

	//------------------------------------------------------------------------------------------------
	static int ChipFill(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.PRIMARY:  return 0x333B82F6; // blue-500/20
			case TBD_EUITint.SUCCESS:  return 0x3310B981; // emerald-500/20
			case TBD_EUITint.WARNING:  return 0x33F59E0B; // amber-500/20
			case TBD_EUITint.DANGER:   return 0x33EF4444; // red-500/20
			case TBD_EUITint.TERTIARY: return 0x33DF7412; // tertiary-container/20
			case TBD_EUITint.BLUFOR:   return 0x333B82F6;
			case TBD_EUITint.OPFOR:    return 0x33EF4444;
			case TBD_EUITint.SOLID:    return 0xFF2563EB; // bg-blue-600 (selected card tag)
		}

		return 0xCC1E293B; // slate-800/80
	}

	//------------------------------------------------------------------------------------------------
	static int ChipBorder(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.PRIMARY:  return 0x4D60A5FA;
			case TBD_EUITint.SUCCESS:  return 0x4D34D399;
			case TBD_EUITint.WARNING:  return 0x66F59E0B;
			case TBD_EUITint.DANGER:   return 0x4DF87171;
			case TBD_EUITint.TERTIARY: return 0x66DF7412;
			case TBD_EUITint.BLUFOR:   return 0x4D60A5FA;
			case TBD_EUITint.OPFOR:    return 0x4DF87171;
			case TBD_EUITint.SOLID:    return 0x33FFFFFF;
		}

		return 0x99334155; // slate-700/60
	}

	//------------------------------------------------------------------------------------------------
	static int ChipInk(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.PRIMARY:  return 0xFF93C5FD; // blue-300
			case TBD_EUITint.SUCCESS:  return 0xFF6EE7B7; // emerald-300
			case TBD_EUITint.WARNING:  return 0xFFFBBF24; // amber-400
			case TBD_EUITint.DANGER:   return 0xFFFCA5A5; // red-300
			case TBD_EUITint.TERTIARY: return 0xFFFDBA74; // orange-300
			case TBD_EUITint.BLUFOR:   return 0xFF93C5FD;
			case TBD_EUITint.OPFOR:    return 0xFFFCA5A5;
			case TBD_EUITint.SOLID:    return 0xFFFFFFFF;
		}

		return 0xFFCBD5E1; // slate-300
	}

	//------------------------------------------------------------------------------------------------
	//! Faction columns and inset cards. NEUTRAL is the plain glass card.
	static int PanelFill(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.BLUFOR:  return 0x401E3A8A; // from-blue-950/25
			case TBD_EUITint.OPFOR:   return 0x40450A0A; // from-red-950/25
			case TBD_EUITint.PRIMARY: return 0x261E3A8A;
			case TBD_EUITint.DANGER:  return 0x26450A0A;
		}

		return PANEL_FILL;
	}

	//------------------------------------------------------------------------------------------------
	//! Lobby faction rows (lobby_sidebar mockup): BLUFOR `bg-blue-950/80 border-blue-500/40
	//! text-blue-300`, OPFOR `bg-red-500/20 border-red-500/30 text-rose-400`, spectators neutral.
	static int FactionRowFill(TBD_EUITint tint, bool hovered = false)
	{
		switch (tint)
		{
			case TBD_EUITint.BLUFOR:
			case TBD_EUITint.PRIMARY:
				if (hovered) return 0x402563EB; // hover:bg-blue-600/25
				return 0xCC172554;               // bg-blue-950/80
			case TBD_EUITint.OPFOR:
			case TBD_EUITint.DANGER:
				if (hovered) return 0x4DEF4444; // hover:bg-red-500/30
				return 0x33EF4444;               // bg-red-500/20
		}

		if (hovered) return 0x801E293B;     // hover:bg-slate-800/50
		return 0x660F172A;                   // bg-slate-900/40
	}

	//------------------------------------------------------------------------------------------------
	static int FactionRowBorder(TBD_EUITint tint, bool selected = false)
	{
		switch (tint)
		{
			case TBD_EUITint.BLUFOR:
			case TBD_EUITint.PRIMARY:
				if (selected) return 0xFF3B82F6; // blue-500 solid (operator: selection must read)
				return 0x663B82F6;                // border-blue-500/40
			case TBD_EUITint.OPFOR:
			case TBD_EUITint.DANGER:
				if (selected) return 0xFFEF4444; // red-500 solid
				return 0x4DEF4444;                // border-red-500/30
		}

		if (selected) return 0xFF64748B;     // slate-500 solid
		return 0x991E293B;                   // border-slate-800/60
	}

	//------------------------------------------------------------------------------------------------
	static int FactionRowInk(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.BLUFOR:
			case TBD_EUITint.PRIMARY: return 0xFF93C5FD; // text-blue-300
			case TBD_EUITint.OPFOR:
			case TBD_EUITint.DANGER:  return 0xFFFB7185; // text-rose-400
		}

		return MUTED_INK;
	}

	//------------------------------------------------------------------------------------------------
	static int PanelBorder(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.BLUFOR:  return 0x403B82F6; // border-blue-500/25
			case TBD_EUITint.OPFOR:   return 0x40EF4444; // border-red-500/25
			case TBD_EUITint.PRIMARY: return 0x403B82F6;
			case TBD_EUITint.DANGER:  return 0x40EF4444;
		}

		return STRIP_BORDER; // border-slate-800/80: readable against the backdrop (white/10 was not)
	}
}

//! Tint vocabulary for chips, badges and tinted panels. Screens pick a tint; TBD_UITheme owns
//! what it looks like. SOLID is the one filled variant (the selected card's tag pill).
enum TBD_EUITint
{
	NEUTRAL,
	PRIMARY,
	SUCCESS,
	WARNING,
	DANGER,
	TERTIARY,
	BLUFOR,
	OPFOR,
	SOLID
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
