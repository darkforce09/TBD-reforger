# Invalid Mission Fixtures (`contracts_v2/fixtures/missions/invalid/`)

Six missions that must always be rejected. Each isolates one defect, so a gate that stops rejecting one names the rule that regressed.

| Fixture | Defect | Rule it pins |
|:---|:---|:---|
| `cargo-container-typo.json` | Misspelled cargo container key | Closed objects reject unknown keys |
| `kit-alias-not-in-registry.json` | Kit alias absent from `rules/kit-aliases.json` | Aliases resolve against the table, not free text |
| `net-range-any.json` | Unbounded radio net range | Net identifiers match their pattern |
| `slot-callsign-separator.json` | Illegal separator in a slot callsign | Callsign format constraint |
| `zone-label-newline.json` | Newline inside a zone label | Labels are single-line |
| `zone-rules-key-typo.json` | Misspelled key in a zone rules block | Closed objects reject unknown keys |

None of these is a "bad JSON" test: every file is syntactically valid and fails only on a contract rule. A parser error would prove nothing about the validator.
