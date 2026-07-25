#!/usr/bin/env bash
# T-181.47 — static gate for the TBD .layout files.
#
# ── Why this exists ────────────────────────────────────────────────────────────────────────
# A .layout only loads when a menu opens, which needs a connected client. `compile.sh` never
# reads it and `world-boot.sh` boots with zero players, so a broken layout ships silently and
# the first symptom is a human staring at an unreadable screen. That is exactly how T-181.47
# happened: the list rendered as a ~10px column of clipped text for a whole session.
#
# This gate cannot prove a layout *looks* right — only a client can. It proves the three
# things that were actually wrong, all of which are decidable from the text:
#
#   C1  brace balance                — a desynced parser reports every later keyword as unknown
#   C2  slot classes are attested    — only slot names observed working in shipped layouts
#   C3  FrameWidgetSlot geometry     — Position/Size must agree with the Offsets they mirror
#   C4  layout-container children    — must declare a slot, or they collapse to desired size
#   C5  widget-name contract         — every name FindAnyWidget() asks for must exist
#
# ── Measured facts this encodes (2026-07-25) ───────────────────────────────────────────────
#   * A FrameWidgetSlot rect is  left = parentW*Anchor[0] + OffsetLeft,
#                                right = parentW*Anchor[2] - OffsetRight  (same for Y).
#     Workbench also writes PositionX/Y and SizeX/Y, which mirror the same rect as
#         PositionX = OffsetLeft            SizeX = -(OffsetLeft + OffsetRight)
#     Where the two disagree the Offsets win (proven by a shipped, visible reference widget
#     with PositionX 0 / OffsetLeft 3 that renders with a 3px inset). C3 removes the
#     ambiguity by requiring them to agree, so it cannot matter which one the engine reads.
#   * Alignment is LayoutHorizontalAlign { Left=0, Center=1, Right=2, Stretch=3 } —
#     apps/mod/vanilla_reference/Scripts/Core/generated/UI/LayoutHorizontalAlign.c.
#   * ButtonSlot / OverlaySlot / SizeLayoutSlot / ScrollLayoutSlot all derive from
#     AlignableSlot and accept only HorizontalAlign / VerticalAlign / Padding. Anchor,
#     PositionX and Offset* belong to FrameWidgetSlot ALONE — putting them on a
#     ButtonWidgetSlot is what produced `GUI (E): Unknown keyword/data`.
#
# Exit 0 = clean.  Exit 1 = a check failed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
UI_DIR="$ROOT/apps/mod/tbd-framework/UI/layouts"
SCRIPT_DIR="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/UI"

fail=0
note() { printf '    %s\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; fail=1; }

shopt -s nullglob
layouts=("$UI_DIR"/*.layout)
if [ ${#layouts[@]} -eq 0 ]; then
	echo "FAIL: no .layout files under $UI_DIR"
	exit 1
fi

echo "==> verifying ${#layouts[@]} TBD layout(s)"

# ── C1/C2/C3/C4 ────────────────────────────────────────────────────────────────────────────
for f in "${layouts[@]}"; do
	out=$(awk -v FNAME="$(basename "$f")" '
	function flush_frame(   want_sx, want_sy) {
		# C3: Position/Size must mirror the Offsets of the same FrameWidgetSlot.
		if (!have_frame) return
		if (seen["PositionX"] && seen["OffsetLeft"] && val["PositionX"] != val["OffsetLeft"])
			printf "C3 %s:%d PositionX %s != OffsetLeft %s\n", FNAME, frame_line, val["PositionX"], val["OffsetLeft"]
		if (seen["PositionY"] && seen["OffsetTop"] && val["PositionY"] != val["OffsetTop"])
			printf "C3 %s:%d PositionY %s != OffsetTop %s\n", FNAME, frame_line, val["PositionY"], val["OffsetTop"]
		if (seen["SizeX"] && seen["OffsetLeft"] && seen["OffsetRight"]) {
			want_sx = -(val["OffsetLeft"] + val["OffsetRight"])
			if (val["SizeX"] != want_sx)
				printf "C3 %s:%d SizeX %s but -(OffsetLeft %s + OffsetRight %s) = %s\n", \
					FNAME, frame_line, val["SizeX"], val["OffsetLeft"], val["OffsetRight"], want_sx
		}
		if (seen["SizeY"] && seen["OffsetTop"] && seen["OffsetBottom"]) {
			want_sy = -(val["OffsetTop"] + val["OffsetBottom"])
			if (val["SizeY"] != want_sy)
				printf "C3 %s:%d SizeY %s but -(OffsetTop %s + OffsetBottom %s) = %s\n", \
					FNAME, frame_line, val["SizeY"], val["OffsetTop"], val["OffsetBottom"], want_sy
		}
		have_frame = 0
		delete seen; delete val
	}
	BEGIN {
		# Slot classes observed working in shipped Enfusion layouts. Anything else is a guess.
		ok_slot["FrameWidgetSlot"]=1; ok_slot["OverlayWidgetSlot"]=1; ok_slot["ButtonWidgetSlot"]=1
		ok_slot["LayoutSlot"]=1;      ok_slot["AlignableSlot"]=1
		# Widgets that size children by layout rules, not by anchors: a child of one of these
		# MUST declare a slot, or it silently falls back to its desired size (0 for a Frame).
		container["OverlayWidgetClass"]=1; container["SizeLayoutWidgetClass"]=1
		container["ScrollLayoutWidgetClass"]=1; container["VerticalLayoutWidgetClass"]=1
		container["HorizontalLayoutWidgetClass"]=1; container["ButtonWidgetClass"]=1
	}
	{
		line = $0
		# Widget declaration: remember what class owns the block we are about to enter.
		if (match(line, /^[ \t]*[A-Za-z_]+WidgetClass[ \t{]/)) {
			wclass = line; sub(/^[ \t]*/, "", wclass); sub(/[ \t].*$/, "", wclass); sub(/\{.*$/, "", wclass)
			pending_widget = wclass
			pending_line = NR
			# C4: a container child with no slot block at all. `depth` is still the enclosing
			# block; scan down past anonymous `{ }` child-lists for the nearest owning widget.
			parent = ""
			for (j = depth; j >= 1; j--) if (owner[j] != "") { parent = owner[j]; break }
			if (parent != "" && container[parent])
				needs_slot[NR] = parent " > " wclass
		}
		if (match(line, /^[ \t]*Slot[ \t]+[A-Za-z_]+/)) {
			slot = line; sub(/^[ \t]*Slot[ \t]+/, "", slot); sub(/[ \t].*$/, "", slot); sub(/\{.*$/, "", slot)
			if (!(slot in ok_slot))
				printf "C2 %s:%d unattested slot class %s\n", FNAME, NR, slot
			# A Slot block sits inside its widget block, so the widget owning `depth` declared it.
			# For a container child, having a slot is not enough: C6 requires it to say how it
			# is aligned, because an empty `Slot ButtonWidgetSlot { }` leaves the child at its
			# desired size — a FrameWidget then reports 0 and the row collapses to a sliver.
			ol = owner_line[depth]
			if (ol != "" && (ol in needs_slot)) {
				needs_align[ol] = needs_slot[ol]; delete needs_slot[ol]; align_owner = ol
			}
			flush_frame()
			if (slot == "FrameWidgetSlot") { have_frame = 1; frame_line = NR }
			in_slot = 1
		}
		if (in_slot && align_owner != "" && match(line, /^[ \t]*HorizontalAlign[ \t]/)) {
			delete needs_align[align_owner]
		}
		if (in_slot && match(line, /^[ \t]*(Anchor|PositionX|PositionY|SizeX|SizeY|OffsetLeft|OffsetTop|OffsetRight|OffsetBottom)[ \t]/)) {
			k = line; sub(/^[ \t]*/, "", k); sub(/[ \t].*$/, "", k)
			v = line; sub(/^[ \t]*[A-Za-z]+[ \t]+/, "", v); sub(/[ \t]*$/, "", v)
			if (k != "Anchor") { seen[k] = 1; val[k] = v + 0 }
		}
		# Brace tracking. Quoted strings are dropped first: every GUID is written "{...}" and
		# would otherwise desync the counter — the bug that made the first cut of this gate
		# pass a layout it was written to reject.
		bl = line; gsub(/"[^"]*"/, "", bl)
		n = gsub(/\{/, "{", bl); m = gsub(/\}/, "}", bl)
		for (i = 0; i < n; i++) {
			depth++
			owner[depth] = ""; owner_line[depth] = ""
			if (pending_widget != "") {
				owner[depth] = pending_widget; owner_line[depth] = pending_line; pending_widget = ""
			}
		}
		for (i = 0; i < m; i++) {
			if (in_slot) { in_slot = 0; align_owner = ""; flush_frame() }
			owner[depth] = ""; owner_line[depth] = ""
			depth--
			if (depth < 0) { printf "C1 %s:%d closing brace with no opener\n", FNAME, NR; exit }
		}
	}
	END {
		flush_frame()
		if (depth != 0) printf "C1 %s: unbalanced braces (depth %d at EOF)\n", FNAME, depth
		for (l in needs_slot) printf "C4 %s:%d %s has no Slot block — it will collapse to its desired size\n", FNAME, l, needs_slot[l]
		for (l in needs_align) printf "C6 %s:%d %s has a Slot block with no HorizontalAlign — it will collapse to its desired size\n", FNAME, l, needs_align[l]
	}
	' "$f")
	if [ -n "$out" ]; then
		while IFS= read -r l; do bad "$l"; done <<<"$out"
	else
		note "OK  $(basename "$f")  (braces, slot classes, geometry, container slots)"
	fi
done

# ── C5: widget-name contract ───────────────────────────────────────────────────────────────
# Every literal a script hands to FindAnyWidget()/Find()/FindText()/FindHandlerOn(), plus the
# TBD_ListBox attribute defaults, must be a Name in some layout. A missing one is not an error
# at runtime — it is a null the code politely tolerates and a feature that never appears.
names=$(grep -rhoE '(FindAnyWidget|Find|FindText|FindHandlerOn)\("[A-Za-z_][A-Za-z0-9_]*"' "$SCRIPT_DIR" \
	| grep -oE '"[A-Za-z_][A-Za-z0-9_]*"' | tr -d '"' | sort -u)
# Defaults of the [Attribute] driven lookups in TBD_ListBox.
names=$(printf '%s\nContent\nEmptyState\n' "$names" | sort -u)
# FocusAnchor is documented as optional — TBD_MenuBase falls back to the root widget.
names=$(printf '%s\n' "$names" | grep -vx 'FocusAnchor' || true)

declared=$(grep -rhoE '^[ \t]*Name "[A-Za-z_][A-Za-z0-9_]*"' "$UI_DIR"/*.layout \
	| grep -oE '"[A-Za-z_][A-Za-z0-9_]*"' | tr -d '"' | sort -u)

missing=$(comm -23 <(printf '%s\n' "$names") <(printf '%s\n' "$declared"))
if [ -n "$missing" ]; then
	while IFS= read -r n; do
		[ -z "$n" ] && continue
		bad "C5 script binds widget \"$n\" but no layout declares it"
	done <<<"$missing"
else
	note "OK  widget-name contract ($(printf '%s\n' "$names" | grep -c .) names bound by script, all declared)"
fi

if [ "$fail" -ne 0 ]; then
	echo "==> UI layout gate FAILED"
	exit 1
fi
echo "==> UI layout gate PASSED"
