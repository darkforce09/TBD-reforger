#!/bin/bash
# crop.sh IMAGE X Y W H [SCALE] [OUTNAME]
# Crops a region out of an Arma 3 screenshot so it can be Read at full detail.
#
# The Read tool downscales any image over ~190,000 pixels. Keep W*H*SCALE^2
# under ~190000 and the crop arrives pixel-perfect.
#
#   1920x1077 source. Useful native regions (verified):
#     menu bar          0    0    1920  22
#     toolbar row       0    22   1920  18
#     left panel        0    36   250   1000
#     right panel       1520 36   400   1000
#     bottom status     0    1037 1920  40
#     viewport/centre   250  40   1270  1000
#
# Examples:
#   ./crop.sh shot.png 0 0 960 40 3 menubar_left     # 2880x120, readable
#   ./crop.sh shot.png 1520 60 400 340 1 rightpanel  # native, readable
#   ./crop.sh shot.png 600 400 400 300 2 ctxmenu     # 800x600, crisp
set -euo pipefail
IMG="$1"; X="$2"; Y="$3"; W="$4"; H="$5"; S="${6:-1}"; NAME="${7:-crop}"
OUT="${CROPDIR:-/tmp/claude-1000/-home-Samuel/429c40b7-ef6b-4d3b-8b75-4dac381575e0/scratchpad/crops}"
mkdir -p "$OUT"
case "$IMG" in /*) ;; *) IMG="/home/Samuel/Documents/Arma_3_Screenshots/$IMG" ;; esac
VF="crop=${W}:${H}:${X}:${Y}"
[ "$S" != "1" ] && VF="${VF},scale=iw*${S}:ih*${S}:flags=neighbor"
ffmpeg -loglevel error -y -i "$IMG" -vf "$VF" "$OUT/${NAME}.png"
PX=$(python3 -c "print(int($W*$S)*int($H*$S))")
echo "$OUT/${NAME}.png  ($((W*S))x$((H*S)) = ${PX}px)"
[ "$PX" -gt 190000 ] && echo "WARNING: over ~190000px, Read will downscale this. Use a smaller region or scale." || true
