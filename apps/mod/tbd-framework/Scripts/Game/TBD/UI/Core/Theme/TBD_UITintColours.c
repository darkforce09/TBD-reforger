/**
 * @file TBD_UITintColours.c
 * @brief Fill, border and ink of chips, panels and faction rows for each `TBD_EUITint`.
 *
 * Role: the one place a `TBD_EUITint` becomes a colour, so a chip and the panel around it agree.
 * Position: called by `TBD_ChipComponent`, `TBD_UITheme.TintGround` and, through the
 * `TBD_UITheme` forwarders, the lobby, briefing and admin panels; returns sRGB ARGB tokens.
 * State: none; pure functions.
 * Invariants: every value is sRGB, and a translucent one is composited by `TBD_UITheme.Over`
 * before it reaches the engine; NEUTRAL falls through to the plain glass values.
 */

//! Tint-to-colour tables for chips, tinted panels and lobby faction rows.
class TBD_UITintColours
{
	//! @return the chip fill for `tint`, sRGB with alpha; NEUTRAL is slate-800/80
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

	//! @return the chip border for `tint`, sRGB with alpha; NEUTRAL is slate-700/60
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

	//! @return the chip text ink for `tint`, opaque; NEUTRAL is slate-300
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

	//! Faction columns and inset cards. NEUTRAL is the plain glass card.
	//! @return the panel fill for `tint`; NEUTRAL returns `TBD_UITheme.PANEL_FILL`
	static int PanelFill(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.BLUFOR:  return 0x401E3A8A; // from-blue-950/25
			case TBD_EUITint.OPFOR:   return 0x40450A0A; // from-red-950/25
			case TBD_EUITint.PRIMARY: return 0x261E3A8A;
			case TBD_EUITint.DANGER:  return 0x26450A0A;
		}

		return TBD_UITheme.PANEL_FILL;
	}

	//! Lobby faction rows (lobby_sidebar mockup): BLUFOR `bg-blue-950/80 border-blue-500/40
	//! text-blue-300`, OPFOR `bg-red-500/20 border-red-500/30 text-rose-400`, spectators neutral.
	//! @return the faction row fill for `tint`, lighter while `hovered`
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

	//! Lobby faction row border; a selected row gets the solid colour so the selection reads.
	//! @return the border for `tint`, solid while `selected`
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

	//! @return the faction row text ink for `tint`; NEUTRAL is `TBD_UITheme.MUTED_INK`
	static int FactionRowInk(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.BLUFOR:
			case TBD_EUITint.PRIMARY: return 0xFF93C5FD; // text-blue-300
			case TBD_EUITint.OPFOR:
			case TBD_EUITint.DANGER:  return 0xFFFB7185; // text-rose-400
		}

		return TBD_UITheme.MUTED_INK;
	}

	//! Tinted panel border; NEUTRAL uses the strip border, which reads against the backdrop.
	//! @return the panel border for `tint`
	static int PanelBorder(TBD_EUITint tint)
	{
		switch (tint)
		{
			case TBD_EUITint.BLUFOR:  return 0x403B82F6; // border-blue-500/25
			case TBD_EUITint.OPFOR:   return 0x40EF4444; // border-red-500/25
			case TBD_EUITint.PRIMARY: return 0x403B82F6;
			case TBD_EUITint.DANGER:  return 0x40EF4444;
		}

		return TBD_UITheme.STRIP_BORDER; // border-slate-800/80: readable against the backdrop (white/10 was not)
	}
}
