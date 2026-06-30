// T-090.1.2 — JS glue for the vendored bcdec.wasm BC7 decoder.
// decodeBc7(bc7Bytes, w, h) -> Buffer of w*h*4 RGBA8.
// w,h must be multiples of 4 (BC7 is block-based). No native deps.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const wasmPath = join(dirname(fileURLToPath(import.meta.url)), "bcdec.wasm");
let inst = null;

function instance() {
  if (!inst) {
    const mod = new WebAssembly.Module(readFileSync(wasmPath));
    inst = new WebAssembly.Instance(mod, {});
  }
  return inst;
}

/** Decode a full BC7 surface. `bc7` = w*h bytes (16B per 4x4 block). */
export function decodeBc7(bc7, w, h) {
  if (w % 4 || h % 4) throw new Error(`BC7 dims must be /4, got ${w}x${h}`);
  const srcLen = w * h; // BC7 = 1 byte/px
  const dstLen = w * h * 4;
  if (bc7.length < srcLen) throw new Error(`BC7 src too short: ${bc7.length} < ${srcLen}`);
  const ex = instance().exports;
  ex.reset();
  const srcPtr = ex.alloc(srcLen);
  const dstPtr = ex.alloc(dstLen);
  new Uint8Array(ex.memory.buffer, srcPtr, srcLen).set(bc7.subarray(0, srcLen));
  ex.decode_bc7_image(srcPtr, w, h, dstPtr);
  // copy out (linear memory is reused on the next call)
  return Buffer.from(new Uint8Array(ex.memory.buffer, dstPtr, dstLen));
}
