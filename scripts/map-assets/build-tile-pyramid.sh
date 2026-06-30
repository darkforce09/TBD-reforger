#!/usr/bin/env bash
# T-090.1 — Build an XYZ WebP tile pyramid from a full-extent terrain ortho.
#
# Input is the engine map-rasterization export (TBD_SatelliteExportPlugin.c ->
# MapDataExporter.ExportRasterization), a single full-world ortho. magick decodes the .tga
# respecting its origin bit, so the raw export is already NORTH-UP for Deck's BitmapLayer
# (top scanline -> maxY/north): do NOT pass --flip-v for this source. (The BI wiki's
# "upside-down" note is about importing back into Enfusion's texture space, a different
# consumer.) --flip-v stays available for sources actually stored bottom-up. T-090.1.1:
# building WITH --flip-v rendered the basemap upside-down (north at bottom).
#
# Output: <out>/{z}/{x}/{y}.webp in XYZ order (y=0 = NORTHERNMOST row, = the image's top
# rows). The frontend (useTerrainBasemapLayer.ts) applies the single TMS Y-flip at fetch
# (tmsY = 2**z-1-y); disk stays XYZ so that flip is the only inversion point.
#
# Usage (engine rasterization → north-up, no flip):
#   scripts/map-assets/build-tile-pyramid.sh \
#     --input  packages/map-assets/everon/staging/spike/TBD_SatExport_everon.tga \
#     --out    packages/map-assets/everon/tiles/satellite \
#     --minzoom 0 --maxzoom 5 --tilesize 256 --quality 80
#
# Requires: magick (ImageMagick 7) + cwebp. Deterministic; safe to re-run (clears --out).
set -euo pipefail

INPUT="" OUT="" MINZOOM=0 MAXZOOM=5 TILE=256 QUALITY=80 FLIP_V=0 LOSSLESS=0 QUALITY_SET=0
while [ $# -gt 0 ]; do
  case "$1" in
    --input) INPUT="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --minzoom) MINZOOM="$2"; shift 2;;
    --maxzoom) MAXZOOM="$2"; shift 2;;
    --tilesize) TILE="$2"; shift 2;;
    --quality) QUALITY="$2"; QUALITY_SET=1; shift 2;;
    --lossless) LOSSLESS=1; shift;;
    --flip-v) FLIP_V=1; shift;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done

[ -n "$INPUT" ] && [ -n "$OUT" ] || { echo "Usage: --input <ortho> --out <tilesdir> [--minzoom N --maxzoom N --tilesize 256 --quality 80 | --lossless] [--flip-v]" >&2; exit 2; }
[ -f "$INPUT" ] || { echo "input not found: $INPUT" >&2; exit 1; }
command -v magick >/dev/null || { echo "magick (ImageMagick 7) required" >&2; exit 1; }
command -v cwebp  >/dev/null || { echo "cwebp required" >&2; exit 1; }

# Encoding mode: --lossless (cwebp -lossless, VP8L) is mutually exclusive with --quality (lossy VP8).
# T-090.1.2.1: the SAP satellite pyramid ships lossless so max-zoom ground texture is pixel-sharp.
[ "$LOSSLESS" = "1" ] && [ "$QUALITY_SET" = "1" ] && { echo "error: --lossless and --quality are mutually exclusive" >&2; exit 2; }
if [ "$LOSSLESS" = "1" ]; then
  CWEBP_ENC=(-lossless)
  ENC_DESC="lossless"
else
  CWEBP_ENC=(-q "$QUALITY")
  ENC_DESC="q=$QUALITY"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Normalize: optional V-flip (upside-down rasterization -> north-up), force RGB, strip metadata.
NORM="$WORK/ortho.png"
FLIP_OP=()
[ "$FLIP_V" = "1" ] && FLIP_OP=(-flip)
echo "[pyramid] normalizing input (flipV=$FLIP_V) -> $NORM"
magick "$INPUT" "${FLIP_OP[@]}" -alpha off -colorspace sRGB "$NORM"

read -r SRCW SRCH < <(magick identify -format "%w %h\n" "$NORM")
echo "[pyramid] source ${SRCW}x${SRCH}; tile=$TILE enc=$ENC_DESC zoom ${MINZOOM}..${MAXZOOM}"

rm -rf "$OUT"
mkdir -p "$OUT"

total=0
NPROC="$(nproc 2>/dev/null || echo 4)"
for ((z=MINZOOM; z<=MAXZOOM; z++)); do
  n=$((1 << z))                 # tiles per axis at this level
  side=$((n * TILE))            # full level pixel side
  LV="$WORK/z$z.png"
  # Resize the (square) ortho to the level resolution. Force exact NxN so crop is clean.
  magick "$NORM" -resize "${side}x${side}!" "$LV"
  echo "[pyramid] z=$z  ${n}x${n} tiles (${side}px)"
  # Single-pass crop: decode the level image ONCE and slice every tile in one magick run. The
  # old code re-decoded the whole level PNG per tile — at z6 (16384^2) that was ~4 s/tile, hours
  # total. magick crops row-major, so scene index i maps to column x=i%n, row y=i/n (y=0 = TOP =
  # NORTH row -> XYZ on disk), verified byte-identical to the per-tile crop (T-090.1.2.1). The
  # cwebp encode (now the only per-tile cost, esp. lossless) is fanned out across all cores.
  rm -f "$WORK"/tile_*.png
  magick "$LV" -crop "${TILE}x${TILE}" +repage +adjoin "$WORK/tile_%d.png"
  for ((x=0; x<n; x++)); do mkdir -p "$OUT/$z/$x"; done
  export _ENC_LOSSLESS="$LOSSLESS" _ENC_Q="$QUALITY" _WORK="$WORK" _OUT="$OUT"
  seq 0 $((n * n - 1)) | xargs -P"$NPROC" -I{} bash -c '
    i="{}"; n='"$n"'; z='"$z"'
    x=$((i % n)); y=$((i / n))
    if [ "$_ENC_LOSSLESS" = 1 ]; then enc=(-lossless); else enc=(-q "$_ENC_Q"); fi
    cwebp -quiet "${enc[@]}" "$_WORK/tile_$i.png" -o "$_OUT/$z/$x/$y.webp"
  '
  total=$((total + n * n))
done

# Full-extent single ortho for the single-bitmap render mode (the H1/H2/H2b judge surface).
# Capped to <=4096 px edge to bound LFS size; this is the preferred frontend render path.
FULL_EDGE=4096
fe=$SRCW; [ "$fe" -gt "$FULL_EDGE" ] && fe=$FULL_EDGE
magick "$NORM" -resize "${fe}x${fe}" "$WORK/full.png"
cwebp -quiet "${CWEBP_ENC[@]}" "$WORK/full.png" -o "$OUT/full.webp"
echo "[pyramid] wrote full.webp (${fe}px)"

echo "[pyramid] wrote $total tiles to $OUT"
test -f "$OUT/0/0/0.webp" || { echo "[pyramid] FAIL: missing $OUT/0/0/0.webp" >&2; exit 1; }
echo "[pyramid] OK  0/0/0.webp + full.webp present"
