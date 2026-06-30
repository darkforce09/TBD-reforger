#!/usr/bin/env bash
# T-090.1.2 — build bcdec.wasm from the vendored public-domain bcdec.h + wrapper.
# Reproducible: freestanding wasm32 via clang (no emscripten/wasi-sdk needed).
# Re-run after changing bcdec_bc7.c or bumping bcdec.h.
set -euo pipefail
cd "$(dirname "$0")"

command -v clang >/dev/null || { echo "FAIL: clang not found"; exit 1; }
clang -print-targets | grep -q wasm32 || { echo "FAIL: clang lacks wasm32 target"; exit 1; }

# Locate a wasm linker. Prefer a PATH wasm-ld; else fall back to the rust-lld
# wasm shim shipped with a rustup toolchain (no separate lld install needed).
WASM_LD="$(command -v wasm-ld 2>/dev/null || true)"
if [ -z "$WASM_LD" ]; then
  WASM_LD="$(ls "$HOME"/.rustup/toolchains/*/lib/rustlib/*/bin/gcc-ld/wasm-ld 2>/dev/null | head -1 || true)"
fi
[ -n "$WASM_LD" ] && [ -x "$WASM_LD" ] || { echo "FAIL: no wasm-ld (install lld, or a rustup toolchain)"; exit 1; }
echo "using wasm-ld: $WASM_LD"
# clang's wasm driver invokes `wasm-ld` by name — put its dir on PATH.
PATH="$(dirname "$WASM_LD"):$PATH"
export PATH

clang \
  --target=wasm32 \
  -nostdlib \
  -O3 -ffreestanding -fno-builtin \
  -Wl,--no-entry \
  -Wl,--export-dynamic \
  -Wl,--export=reset \
  -Wl,--export=alloc \
  -Wl,--export=decode_bc7_image \
  -Wl,--export=__heap_base \
  -Wl,--export-memory \
  -Wl,--initial-memory=33554432 \
  -Wl,--allow-undefined \
  -o bcdec.wasm \
  bcdec_bc7.c

echo "built bcdec.wasm ($(wc -c < bcdec.wasm) bytes)"
