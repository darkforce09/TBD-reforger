# Contract Fixtures (`contracts_v2/fixtures/`)

Ground truth for every boundary: samples that must always be accepted, and samples that must always be rejected.

---

## 1. Topology

```text
fixtures/
├── README.md
├── missions/
│   ├── valid/                          <-- 9 missions that must parse, validate and compile
│   └── invalid/                        <-- 6 malformed missions, one rejection gate each
├── map/                                <-- 20 spatial fixtures, JSON and binary
├── registry/                           <-- 8 arsenal, loadout, and faction samples
├── enfusion_samples/                   <-- 10 raw payloads as the game mod emits them
└── bridge_samples/                     <-- 6 voice-bridge IPC messages
```

---

## 2. What a Fixture Is For

A fixture pins a behaviour that no unit test can state on its own, because the behaviour is agreement *between* components. The mission goldens are read by the API validator, the map engine's compiler, and the game mod's loader; when all three agree on the same file, the contract holds.

The negative fixtures matter more than the positive ones. Each invalid mission isolates exactly one defect, so a gate that stops rejecting it names the rule that broke. A validator that grows permissive fails loudly here instead of silently accepting malformed missions until one reaches a live operation.

---

## 3. Invariants

1. **Both halves move together.** Adding a property to a closed schema means updating every fixture that schema validates, in the same commit.
2. **Binary and JSON twins agree.** In `map/`, a decode of the `.bin` must equal a decode of the `.json`. That pair is what keeps the zero-copy reader honest about alignment and endianness.
3. **A fixture is never repaired to make a test pass.** If a golden stops validating, either the schema changed or the validator regressed; both are findings.
