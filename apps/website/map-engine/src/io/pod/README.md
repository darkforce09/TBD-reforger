# World object instance row

`ObjectInstancePod`, the 32-byte row that stores one placed world object in a terrain's chunk
binaries: the same bytes on disk, in memory and in the GPU upload, read and written without a copy.

## Contents

```text
apps/website/map-engine/src/io/pod/
├── instance.rs  `ObjectInstancePod`, `POD_BYTES`, `POD_NAME` and the zero-copy byte casts
├── mod.rs       the module tree
└── tests/       unit tests for the row's size, alignment, field offsets and byte casts
```

## How it works

A row is `#[repr(C)]` and little-endian: `x`, `y`, `z`, `yaw`, `pitch`, `roll` and `scale` as
`f32` (bytes 0–27), `prefab_id` as `u16` (an index into the terrain's prefab catalogue), then
`class_code` and `_pad` as one byte each; `class_code` is the render class, `NO_CLASS` (255) for an
unclassified prefab, and `_pad` is always 0. `POD_BYTES` is 32, and compile-time assertions hold
the size, the 4-byte alignment and a little-endian target.

`instances_from_bytes` casts a `TBDC` payload to rows in place. It fails with `LengthMismatch` when
the length is not a multiple of 32 and with `Misaligned` when the buffer is not 4-byte aligned;
an empty payload is zero rows. `instances_to_bytes` is the writer's cast and cannot fail.
`ObjectInstancePod::identity()` is the row a five-wide JSON row widens to, with `scale` 1, where
`Default` leaves `scale` at 0. `POD_NAME` is the string a terrain manifest records in its
`objects.binary.pod` field, beside `podBytes`.

## Boundaries

- Depends on: `crate::io::archives::codec::BinaryError` and `bytemuck`.
- Used by:
  - `crate::io::containers::tbdc`, whose `TbdcHeader::instances` casts the payload;
  - `crate::streaming::loaders` (`chunk_bin.rs` reads chunk rows into columns, `manifest.rs`
    refuses a manifest whose `pod` or `podBytes` differs from `POD_NAME` and `POD_BYTES`);
  - the developer tools: the world export writes rows in
    `tools_v2/developer-tools/src/world_export_pipeline/binary_emit.rs`, and the map
    verifications read them in `tools_v2/developer-tools/src/map_verification/`.
- Rules: the row stays 32 bytes, 4-aligned and little-endian, with every byte a named field, since
  every committed chunk under `assets_v2/terrains/` places row `i` at byte `32 + 32·i`
  (`pod_is_thirty_two_bytes_and_four_aligned`, `every_byte_is_a_named_field` and
  `field_offsets_are_the_wire_layout` in `tests/instance_tests.rs`); a bad length or alignment is
  an error, never a panic (`truncated_payload_is_err_not_panic`,
  `misaligned_slice_is_err_not_panic`, same file).

## Related documentation

- [Map object instance schema](/contracts_v2/definitions/map-object-instance.schema.json) — the
  JSON encodings of the same row and, under `objectInstancePodRow`, its byte layout.
