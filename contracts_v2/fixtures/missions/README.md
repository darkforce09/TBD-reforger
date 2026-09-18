# Mission Fixtures (`contracts_v2/fixtures/missions/`)

The mission contract's acceptance and rejection corpus.

```text
missions/
├── README.md
├── valid/                              <-- must always parse, validate and compile
└── invalid/                            <-- must always be rejected, one reason each
```

Every file is read by three independent implementations: the API's schema validator, the map engine's scenario compiler, and the Enfusion mission loader. A mission that all three accept is the operational definition of a valid mission.
