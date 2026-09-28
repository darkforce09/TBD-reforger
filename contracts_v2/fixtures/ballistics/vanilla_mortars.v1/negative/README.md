# Refused calibration bundles

Copies of `../calibration.json` that each carry exactly one defect, so the calibration evaluator
and the catalog upload must refuse every one of them for its own reason. The trim writes them with the
good bundle; they are never edited by hand.

## Contents

```text
contracts_v2/fixtures/ballistics/vanilla_mortars.v1/negative/
├── skewed_native_row.calibration.json  shell m821 coefficient 2.977 row 20 (maximum range) moved from 2959.697 m to 2964.697 m
├── wrong_game_build.calibration.json   game_build 0.0.0.0 instead of 1.8.0.13
├── missing_shell.calibration.json      no native table, wind table or oracle sample for shell s832s
└── stale_catalog_sha.calibration.json  catalog_sha256 of the catalog serialized on one line, not of the committed bytes
```

## Hashes

- skewed_native_row.calibration.json: `8bda3a204f27fd7a45116f0b66bd988d35883e528646414e0a850da7e123f7c3`
- wrong_game_build.calibration.json: `f9aba83813697d40b2bdef3750496da014d35a6ef082b9bc99edf3158bd47191`
- missing_shell.calibration.json: `c96692bfa18c76297bd187ca53fc1bbfac3291cfd053ec9fa7f9c93d736d9ddd`
- stale_catalog_sha.calibration.json: `f61e1b4dff60719e021e080c1a8a36debd93a67a055daaecf07749275b43541a`
