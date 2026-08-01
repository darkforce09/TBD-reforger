#!/bin/bash
# run_shot_gpu.sh <out.png> <url> <waitMs> [url waitMs ...]
# Real-GPU variant. The swiftshader build can't create a WebGPU buffer, so the wgpu engine
# panics and the map never renders. This one hands chrome the host RTX 3070 via /dev/dri.
#   GPU_MODE=vulkan   (default) headless + ANGLE/Vulkan on the real device
#   GPU_MODE=egl      headless + ANGLE/GL
#   GPU_MODE=x11      real window on the live :0 session — last resort, pops a window
set -uo pipefail
SP=/tmp/claude-1000/-home-Samuel/429c40b7-ef6b-4d3b-8b75-4dac381575e0/scratchpad
CHROME=/home/Samuel/.cache/ms-playwright/chromium-1228/chrome-linux64/chrome
NODE=/home/Samuel/.config/nvm/versions/node/v26.4.0/bin/node
PROFILE=/home/Samuel/.cache/tbd-shot-profile-gpu
MODE="${GPU_MODE:-vulkan}"

pkill -9 -f "chrome-linux64/chrome" 2>/dev/null
sleep 1
rm -rf "$PROFILE"

# KB-002: ostree host has an unwritable fontconfig cache; without this the renderer aborts
# on first text layout with "Could not find any font: , sans".
export XDG_CACHE_HOME=/home/Samuel/.cache/tbd-gate-fontcache
mkdir -p "$XDG_CACHE_HOME"

BASE="--no-sandbox --disable-gpu-sandbox --remote-debugging-port=9222
      --user-data-dir=$PROFILE --enable-unsafe-webgpu --hide-scrollbars
      --force-device-scale-factor=1 --disable-dev-shm-usage --no-first-run
      --no-default-browser-check --window-size=1920,1080"

case "$MODE" in
  vulkan) ARGS="--headless=new $BASE --use-angle=vulkan --enable-features=Vulkan --use-vulkan --ignore-gpu-blocklist" ;;
  egl)    ARGS="--headless=new $BASE --use-angle=gl --use-gl=angle --ignore-gpu-blocklist" ;;
  x11)    ARGS="$BASE --ozone-platform=x11 --ignore-gpu-blocklist --window-position=2400,0" ;;
  *)      echo "unknown GPU_MODE=$MODE"; exit 2 ;;
esac

echo "== GPU_MODE=$MODE =="
if [ "$MODE" = "x11" ]; then export DISPLAY=:0; fi
"$CHROME" $ARGS about:blank > "$SP/chrome_gpu.log" 2>&1 &
CHROME_PID=$!

for i in $(seq 1 30); do
  curl -s --max-time 2 http://127.0.0.1:9222/json/version >/dev/null 2>&1 && break
  sleep 1
done
echo "chrome pid=$CHROME_PID cdp=up"

"$NODE" "$SP/cdp2.mjs" "$@"
RC=$?
echo "== gpu status =="
grep -iE "swiftshader|vulkan|GPU process|gl_ozone|fallback|Passthrough" "$SP/chrome_gpu.log" 2>/dev/null | head -8
kill -9 "$CHROME_PID" 2>/dev/null
pkill -9 -f "chrome-linux64/chrome" 2>/dev/null
exit $RC
