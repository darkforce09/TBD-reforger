# Camera matrix math

The f64 matrix and scalar helpers the cameras build their matrices from. Each one ports a
gl-matrix or deck.gl JavaScript routine with its expression tree kept, so the orthographic camera
reproduces deck.gl's numbers to the bit once its scale matches deck.gl's.

## Contents

```text
apps/website/map-engine/src/camera/math/
├── dimensions.rs  deck.gl's scalar rules: size coercion, rounding, NaN-aware min and max, clamp
├── glmat4.rs      gl-matrix's f64 4×4 routines: products, ortho, perspective, look-at, inverse
├── mod.rs         the module tree
├── shaping.rs     `round`, JavaScript's `Math.round`: halves round toward +∞
└── tests/         unit tests for the rounding rule
```

## How it works

A matrix is a column-major `[f64; 16]`, and every product stays in f64; a caller casts to f32 only
when it writes the final render uniform. `glmat4.rs` follows gl-matrix's `mat4`: `multiply`,
`translate_in_place` and `scale_in_place` (the in-place branch deck.gl's `Matrix4` always takes),
`ortho_no` (the GL clip convention, depth in [-1, 1]), `perspective_no`, `look_at` with gl-matrix's
epsilon short-circuit and zero-length guards, and `invert`, which returns `None` on a zero or NaN
determinant. `transform_vector` divides by w as one reciprocal and then multiplies, as
`@math.gl/web-mercator` does, because a straight division differs in the last bit; `lerp2` is
gl-matrix's `vec2.lerp`.

`dimensions.rs` and `shaping.rs` hold the scalar rules deck.gl applies to viewport sizes and
extents: `or_one` turns 0 or NaN into 1, `round_dimension` and `round` compute `floor(x + 0.5)`,
and `js_min4` and `js_max4` return NaN when any input is NaN, as `Math.min` and `Math.max` do.

## Boundaries

- Depends on: nothing outside the folder.
- Used by: `crate::camera::ortho` and `crate::camera::orbit`; `crate::doll` (picking, instance
  transforms and the model tests); `crate::diagnostics::readback` (the doll check); and
  `crate::world::terrain`, whose elevation grid and hillshade round with `shaping::round`. Nothing
  outside the crate: `dimensions` and `shaping` are `pub(crate)`.
- Rules: a port keeps its JavaScript operand order and its reciprocal w-divide, because
  `apps/website/map-engine/tests/deckgl_ortho_parity.rs` holds the orthographic camera to zero ULP
  against the deck.gl goldens (`t3_scale_injected_pipeline_bit_exact_all_cases`); `round` rounds a
  half up, so `round(-2.5)` is `-2` (`matches_js_math_round` in `tests/shaping_tests.rs`).
