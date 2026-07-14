# T-152.2 verify log — Reforger map icon art + atlas rebuild

**Slice:** T-152.2  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`  
**Branch:** `ticket/T-152`  
**Date:** 2026-07-13

---

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | Discovery artifact | **PASS** | `.ai/artifacts/t152_2_icon_discovery.json` — 21 keys, each `source:redraw` + `redrawRationale`; Workbench MCP FAIL documented |
| **G2** | Coverage | **PASS** | `manifest.json` defines all 21 `LANDMARK_SET` keys |
| **G3** | SVG on disk | **PASS** | `test -f packages/map-assets/glyphs/svg/${k}.svg` for all 21 keys |
| **G4** | Placeholder evicted | **PASS** | All 21 pre/post SHA256 differ (see appendix) |
| **G5** | Atlas build | **PASS** | `make map-glyphs-build` exit 0 — **28** icons → 1024×512 atlas (31.8 KB) |
| **G6** | Verify script | **PASS** | `make map-glyphs-verify` exit 0 |
| **G7** | Golden prefabs | **PASS** | `verify-map-glyphs: OK (28 glyphs, golden + everon iconKeys covered, atlas rects verified)` |

**Atlas key count:** 28 (`world-glyphs.json` `icons` object)

---

## Automated commands

```bash
make map-glyphs-build     # exit 0
make map-glyphs-verify    # exit 0
```

---

## Manual checklist (advisory)

| ID | Check | Pass |
|----|-------|------|
| M1 | Lighthouse + castle + military @ zoom ≥ +1.5 recognizable vs Reforger map | ☐ operator |
| M2 | Atlas preview — no magenta/empty cells for LANDMARK_SET | ☐ operator |

---

## Discovery summary

- **Workbench MCP:** `api_search "map icon atlas UI"` — **TIMEOUT** (daemon not warm). Redraw path per spec.
- **Artifact:** `.ai/artifacts/t152_2_icon_discovery.json`
- **Art path:** All 21 `LANDMARK_SET` SVGs redrawn with distinct Reforger/A3-familiar silhouettes (24×24 viewBox, simple fills).

---

## Appendix — SHA256 pre/post (LANDMARK_SET)

| iconKey | sha256_pre | sha256_post | G4 |
|---------|------------|-------------|-----|
| building-agricultural | ad33f714906c1e6483d7927e8209a3d7910fa8311337d5d9bf095a02f80c01a1 | 675135487f68cbede022a9a72b85b30a0005d44e1ca1300c2ccb8afe68728c70 | PASS |
| building-badge-bunker | 83f5293e160ee79e3e251a66041845a2a9de2c6cb53ef2ca8110a2594cfd633d | 1d4ac9cbe5e16e4cbfe447b220a5b7c4fd2128d427348b269af248f1c0c43782 | PASS |
| building-badge-military | cd3026ac4ee750a4bf336f1986d51de5cbaf3f289573569a98d0d7317da60e32 | cc25cf7c23e7c61df3ae84322c0eb96ad1b8dc132a140c679c0ac0872eed0592 | PASS |
| building-badge-tower | d9290c0cd01a14fba5796bac0500817d74a412bc8f8664e1a9311d56c2f2bcc8 | 6052564f109d0155fa0b910b57aaf6312282e21df3f17c12ba73850007483b32 | PASS |
| building-bridge | dd50fded8c23084e1831cc76ade255c419d028657ac312a997750fee37617d3c | c9e271b2e930e3466c5179188e1ac8f13124765ff7533f8de1e1d2926c1658d4 | PASS |
| building-bunker | 57fc669114697d7ee902512131cd6a2bd73b71533d88b0eff6fdd4efef6e7f71 | feef457d77aba033add517a721260a89b7105ab721005085a0429f1ea8526f9f | PASS |
| building-castle | 183eac9702dabe23eb339e1f644ad690b0eb10af2301b8684273446eeacdfc70 | bc3f53dafbcd89bdd26896e7c0f074b0b75925d87d2a3662dea3c9e4e339de32 | PASS |
| building-civic | 7405ad7bbd2759965a540fb18ba5fe980f2a2d3fcd80be184affa098776db758 | 06db0c9e9fc6ca1f3a8e4471a905badbc3b6c1b5047a72e78fbdaefab6c9f2ee | PASS |
| building-commercial | 051c584f47cab00873a6366ffaf21c962b53b25b0493c8a269e4df1aed00a7a5 | ba0a0840c36f539252f359124d37792d5c3249267fe5b5f82bf70cf0e1e1d3ee | PASS |
| building-container | 2057341900462f9c85914e6fac2d54cb127ab8145f8ab762f59d556c96f3770f | 7b5ed4e685defa5afc1ab49db7692429b3ee564ebbe1079094d7591e578d7f96 | PASS |
| building-garage | d77601b15746adc048e431312138729c0bc13daef2d4efe7643f0f840e69484b | 74f6416702055573b59154551d7f7bc4f856f0bff4c22589130d8de9a52e8b00 | PASS |
| building-generic | 1fc494e71050edd9cfd530a1346eb29b381a882bd9bda752171515482c77356a | 4e6893c1c6ad87d852a93fecbf53ac4deae92445b5b1035597221cf4537f1f36 | PASS |
| building-hangar | 269faeb4221cba9e031451bb9ffb5d2dd5ac38e927123d12b2792197b7d3cb5a | 6edb303dffccff14cf13b4e6fce0658004b30cd10f405de42a36968deb4c832e | PASS |
| building-industrial | 2c73452c3e11b35cdd38bcbf99744d9d026c6423e93550f8c671cae195dfdf88 | ca381543e90a8852fdd9a571acdeee62b0ccaa4aabd5560a268e4032cf1d9a78 | PASS |
| building-lighthouse | 1307ca5d8ac8406777f08ae329c1307f635961a5bacf18d4cae7259ed5bb3ace | f368c3e958fec831591843a052ad2fb51c57ff900afb468ea2fc7ed40cc810da | PASS |
| building-military | 0f92c71dd393b0c2a05252feabbc550e0c9b90d50d9bbbbb0e7b57bf7ba00ff6 | fb3c1d424c8d27296cd30d2ae00675396c9bee8eba53e20e9c5b1e9272f9c6b5 | PASS |
| building-residential | 94361bdc0fce93e65a8242ae2f066205528971f8ada44218999147886253a639 | 4388c9370160b065abf9bad6354c78717d715b69981ce22f8024b0311584a3ae | PASS |
| building-ruin | 17d25babdc70b5ce03da469413138f8b3810b36a31e759ed654c824272034a0e | df0734812ccc1a6224f02c712bbb7f1e80db5c7e90321d25cac3c5c4e50d14ef | PASS |
| building-shed | 671ed8765eee4ac67c7f2e8644d2a384550b5d2f7725f38f2cbf73e9cb75267b | f6b03d91a714ded67a754b3c28f791c41aba3810535761a90a98541e73c6b6d1 | PASS |
| building-tent | 20dcc9ef52bcf76ba6290df1042cd30da3c43696659bd8d0fc1ab1d23a2dc070 | 4e1dcea35e306bf84c610a6e8399eddf28108c2751765bbbcaa8edc66ac709dc | PASS |
| building-tower | d5dbfbf5fed46a7eb2cc619b8bc738702c3c625423085407934d83901fbc4546 | f75c0b75146fcda093dabf2c435ee1293f38f2b0a0cbcd276aaeb7419137ef89 | PASS |

---

## Files changed

- `packages/map-assets/glyphs/svg/building-*.svg` (21 LANDMARK_SET)
- `packages/map-assets/glyphs/atlas/world-glyphs.webp`
- `packages/map-assets/glyphs/atlas/world-glyphs.json`
- `.ai/artifacts/t152_2_icon_discovery.json`
- `.ai/artifacts/t152_2_verify_log.md`

**Ready for T-152.3** (residency / GPU wire).
