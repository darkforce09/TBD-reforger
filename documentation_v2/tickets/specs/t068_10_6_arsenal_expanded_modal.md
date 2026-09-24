# T-068.10.6 — Arsenal expanded modal (ACE panes)

**Ticket:** T-068 · **Slice:** T-068.10.6 · **Status:** shipped ·
**Executor:** claude-code (Mode C session) ·
**Verify:** [`.ai/artifacts/t068_10_6_verify_log.md`](../../../.ai/artifacts/t068_10_6_verify_log.md)

## In one sentence

The Attributes modal grows (same menu, `max-w-6xl` when the Arsenal tab is active) and the
Arsenal adopts the ACE interaction model: pick an item → its detail shows in a LEFT pane
(silhouette placeholder, kind/addon chips, kg/cm³, variant relations); containers open a
RIGHT capacity panel with the reserved cargo area; the 14 picker rows stay center.

## Shipped shape

- `itemDetail.ts` — pure selector over the loaded catalog: identity, phys attrs (null =
  engine class default, never guessed), container capacity, `variant_of` back-link +
  reverse configurations list. **Container semantics exclude weapon kinds** (rifles carry a
  `SCR_WeaponAttachmentsStorageComponent` whose capacity is attachment storage, not cargo).
- `ItemDetailPane.tsx` — detail pane + `ContainerPanel` (capacity summary + cargo-slice
  placeholder). Variant links hop the inspection between base ⇄ configurations.
- `ArsenalTab` — inspection state (last pick, seeded from the first non-empty pick);
  3-zone grid `[230px | 1fr | 210px]`, third column only for containers.
- `AttributesModal` — conditional `className="max-w-6xl"` via the shared DialogContent
  (tailwind-merge lets it override the `max-w-lg` default; other tabs unchanged — V3).

## Gates

vitest **346/346** (new `itemDetail` suite on the committed envelope with real GUIDs:
weapons null-attrs + non-container; RGD5 0.31 kg/100 cm³ flow; backpack container detect;
AK74N 1P29 ⇄ AK74N variant links, locale-sorted configurations; unknown → null; abstract
flag) · build + `tsc --noEmit` clean · lint = pre-existing `router.tsx` only ·
operator visual at the Mode C close-out pause.
