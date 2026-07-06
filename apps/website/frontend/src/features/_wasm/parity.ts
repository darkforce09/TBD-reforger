// Differential parity harness for the map-engine-wasm port (T-145). The correctness contract
// (plan §4) compares the TS reference output against the wasm output per numeric class:
//   - Class R (rational): byte-identical → `f32BytesEqual` / `bytesEqual` (memcmp).
//   - Class T (transcendental): ≤ 1 ULP pre-quantization → `ulpDistanceF64`.
//   - Class S (structural): result-set equality → asserted per index test.

const scratch = new DataView(new ArrayBuffer(8))

/** Raw bytes view over a Float32Array (no copy). */
function f32Bytes(a: Float32Array): Uint8Array {
  return new Uint8Array(a.buffer, a.byteOffset, a.byteLength)
}

/** memcmp of two byte arrays. */
export function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false
  return true
}

/** Class R: two Float32Array outputs are byte-identical (bit patterns, NaN-safe). */
export function f32BytesEqual(a: Float32Array, b: Float32Array): boolean {
  return a.length === b.length && bytesEqual(f32Bytes(a), f32Bytes(b))
}

/** Bit-exact f32 compare (NaN patterns included) via a shared DataView. */
function f32BitsEqual(x: number, y: number): boolean {
  scratch.setFloat32(0, x)
  const bx = scratch.getUint32(0)
  scratch.setFloat32(0, y)
  const by = scratch.getUint32(0)
  return bx === by
}

/** Index of the first differing f32 lane, or -1 if equal (test diagnostics). */
export function firstF32Mismatch(a: Float32Array, b: Float32Array): number {
  const n = Math.min(a.length, b.length)
  for (let i = 0; i < n; i++) if (!f32BitsEqual(a[i], b[i])) return i
  return a.length === b.length ? -1 : n
}

/** Exact equality of two integer typed arrays (Class R / Class S counts + indices). */
export function intArrayEqual(a: ArrayLike<number>, b: ArrayLike<number>): boolean {
  if (a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false
  return true
}

/** Class T: max absolute per-element difference between two integer arrays (assert ≤ 1 for the
 *  hillshade gray gate). Returns Infinity on a length mismatch. */
export function maxAbsDiff(a: ArrayLike<number>, b: ArrayLike<number>): number {
  if (a.length !== b.length) return Number.POSITIVE_INFINITY
  let m = 0
  for (let i = 0; i < a.length; i++) {
    const d = Math.abs(a[i] - b[i])
    if (d > m) m = d
  }
  return m
}

/** Class T: ULP distance between two f64 values (assert ≤ 1). +0/-0 → 0; any NaN → Infinity. */
export function ulpDistanceF64(a: number, b: number): number {
  if (a === b) return 0
  if (Number.isNaN(a) || Number.isNaN(b)) return Number.POSITIVE_INFINITY
  scratch.setFloat64(0, a)
  const ai = scratch.getBigInt64(0)
  scratch.setFloat64(0, b)
  const bi = scratch.getBigInt64(0)
  // Total ordering across the sign boundary: negative bits map below +0.
  const flip = (x: bigint): bigint => (x < 0n ? -9223372036854775808n - x : x)
  const fa = flip(ai)
  const fb = flip(bi)
  const d = fa > fb ? fa - fb : fb - fa
  return Number(d)
}
