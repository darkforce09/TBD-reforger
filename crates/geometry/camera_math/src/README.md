# Camera math source

The source of `camera_math`: the 4×4 matrix routines and scalar rules the cameras build on, the
orthographic and orbit cameras, and the crate root. Each routine ports a gl-matrix or deck.gl
JavaScript routine with its expression tree kept, so the orthographic camera reproduces deck.gl's
numbers to the bit once its scale matches deck.gl's.

## Contents

```text
crates/geometry/camera_math/src/
├── dimensions.rs  deck.gl's scalar rules: size coercion, rounding, NaN-aware min and max, clamp
├── lib.rs         the crate root: module header, `mod` lines
├── matrix4.rs     gl-matrix's f64 4×4 routines: products, ortho, perspective, look-at, inverse
├── orbit/         the doll preview's perspective camera, turned by yaw alone
├── ortho/         the tactical map's orthographic camera: matrices, controls, unprojection
└── prelude.rs     the names most callers import
```

## How it works

A matrix is a column-major `[f64; 16]`, and every product stays in f64; a caller casts to f32 only
when it writes the final render uniform. `matrix4.rs` follows gl-matrix's `mat4`: `multiply`,
`translate_in_place` and `scale_in_place` (the in-place branch deck.gl's `Matrix4` always takes),
`ortho_no` (the GL clip convention, depth in [-1, 1]), `perspective_no`, `look_at` with gl-matrix's
epsilon short-circuit and zero-length guards, and `invert`, which returns `None` on a zero or NaN
determinant. `transform_vector` divides by w as one reciprocal and then multiplies, as
`@math.gl/web-mercator` does, because a straight division differs in the last bit; `lerp2` is
gl-matrix's `vec2.lerp`.

`dimensions.rs`, private to the crate, holds the scalar rules deck.gl applies to viewport sizes
and extents: `or_one` turns 0 or NaN into 1, `round_dimension` rounds with
`map_coordinates::rounding::round` (`floor(x + 0.5)`), and `js_min4` and `js_max4` return NaN when
any input is NaN, as `Math.min` and `Math.max` do.

## Boundaries

- Depends on: `map_coordinates` (`rounding::round`).
- Used by: `ortho/` and `orbit/`; outside the crate, the map engine's doll (picking, instance
  transforms and the model tests) and its doll readback check call `matrix4`.
- Rules: a port keeps its JavaScript operand order and its reciprocal w-divide, because
  `crates/geometry/camera_math/tests/deckgl_ortho_parity.rs` holds the orthographic camera to zero
  ULP against the deck.gl goldens (`t3_scale_injected_pipeline_bit_exact_all_cases`).
