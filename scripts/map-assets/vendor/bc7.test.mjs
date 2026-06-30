// T-090.1.2 — regression test for the vendored bcdec.wasm BC7 decoder + glue.
// Run: node --test scripts/map-assets/vendor/
//
// Fixture is a varied 4x4 BC7 block from Eden_1174 mip0 (a detailed field cell);
// EXPECT_RGB was produced by an INDEPENDENT decoder (Pillow) so this guards the
// wasm build + JS glue against a broken rebuild / wrong memory layout — not a
// circular self-check. (The full-cell cross-check vs liblz4+Pillow was RGB AE=0.)
import { test } from "node:test";
import assert from "node:assert/strict";
import { decodeBc7 } from "./bc7.mjs";

const BLOCK = Buffer.from("c05ae575293dfeff0d726b56cf79ef7b", "hex");

// 16 px (row-major) x RGB, from Pillow's BC7 decode of the same block.
const EXPECT_RGB = [
  81, 76, 57, 107, 95, 75, 98, 88, 69, 77, 73, 54,
  60, 60, 43, 81, 76, 57, 81, 76, 57, 86, 79, 61,
  43, 47, 31, 56, 57, 40, 69, 67, 49, 77, 73, 54,
  43, 47, 31, 47, 50, 34, 60, 60, 43, 77, 73, 54,
];

test("bcdec.wasm decodes a known BC7 block to the expected RGB", () => {
  const rgba = decodeBc7(BLOCK, 4, 4);
  assert.equal(rgba.length, 64, "4x4 RGBA = 64 bytes");
  const rgb = [];
  for (let i = 0; i < 16; i++) rgb.push(rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2]);
  assert.deepEqual(rgb, EXPECT_RGB);
});

test("decodeBc7 rejects non-/4 dimensions", () => {
  assert.throws(() => decodeBc7(Buffer.alloc(16), 3, 4), /must be \/4/);
});
