# Browser testing crates

The headless browser gates of the single-page app: the Chrome DevTools Protocol client they drive
Chromium with, and the gate suites that check the Mission Creator, the offline mortar and
ballistics agreement gates, and the
`gate` and `capture` command lines that run them all.

## Contents

```text
tools/browser_testing/
├── browser_gate_suites/       `browser_gate_suites`: the static server, the Mission Creator smokes, the data viewer gate, the offline mortar and ballistics agreement gates, the capture rig, the doctor and the `gate` and `capture` command lines, with the gate pin
└── chrome_devtools_protocol/  `chrome_devtools_protocol`: Chromium discovery and launch, pages over WebSockets, the gate font cache
```
