# Browser testing crates

The headless browser gates of the single-page app: the Chrome DevTools Protocol client they drive
Chromium with, and the gate suites that check the Mission Creator and the DOM oracle routes
against their committed fixtures. The offline mortar and ballistics agreement gates live in
`tools/developer_tools/src/browser_testing/`, beside the `gate` command line that runs them all.

## Contents

```text
tools/browser_testing/
├── browser_gate_suites/       `browser_gate_suites`: the static server, the DOM oracle, the route drift check, the Mission Creator smokes, the data viewer gate, the capture rig and the doctor, with the DOM oracle fixtures and the gate pin
└── chrome_devtools_protocol/  `chrome_devtools_protocol`: Chromium discovery and launch, pages over WebSockets, the gate font cache
```
