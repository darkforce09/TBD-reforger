# Enfusion MCP node package

The private npm package `enfusion-mcp-node-package`, whose only job is to pin the `enfusion-mcp`
server, the Model Context Protocol bridge to the [Enfusion](/documentation_v2/glossary.md#enfusion)
[Workbench](/documentation_v2/glossary.md#workbench), so the repository's MCP commands start one
known version from disk instead of resolving one over the network.

## Contents

```text
tools_v2/enfusion_mcp_node_package/
├── .nvmrc             the Node.js major version (26) for `nvm use` in this folder
├── package-lock.json  the lockfile that fixes `enfusion-mcp` 0.6.1 and every transitive package
└── package.json       the private package: one dependency, `enfusion-mcp` at exactly 0.6.1
```

## How it works

`npm ci` in this folder installs the locked tree into `node_modules/`, which git ignores; the
server's entry module is then `node_modules/enfusion-mcp/dist/index.js`, which
`ENFUSION_MCP_ENTRYPOINT` in `tools_v2/developer-tools/src/repository_layout.rs` names. The folder
sits outside every crate root, so no crate-scoped file walk ever reads that installed tree.

`enfusion_mcp_entrypoint::resolve` in `tools_v2/developer-tools/src/enfusion_tooling/` decides
what starts the server, in this order:

1. `ENFUSION_MCP_BIN`, when it names an existing file;
2. `node` on the module installed here, the normal answer;
3. a copy npm already downloaded under `~/.npm/_npx`;
4. `npx -y enfusion-mcp`, which downloads the latest release on demand.

Only the second tier is pinned; the later tiers run whatever version npm finds.

## Getting started

Run these from the repository root:

```bash
cargo xtask mod dev-bootstrap   # runs npm ci here when the entry module is missing, then sets up Workbench
cargo xtask mcp daemon start    # starts the mcpd broker, which starts the resolved server
cargo xtask mcp daemon status   # reports whether the broker is running
```

`cargo xtask mod dev-bootstrap` treats a failed `npm ci`, offline for example, as a warning and
carries on, so the resolver falls to the npm cache or a download. To install by hand, run
`npm ci` inside this folder.

## Configuration

- `package.json` pins the version: change the `enfusion-mcp` entry and regenerate
  `package-lock.json` with `npm install` in this folder, then commit both.
- `.nvmrc` names Node.js 26; the pinned `enfusion-mcp` itself requires Node.js 20 or newer.
- `ENFUSION_MCP_BIN` overrides the server the resolver picks, as above.

## Public surface

- The installed entry module `node_modules/enfusion-mcp/dist/index.js`, which the resolver
  starts with `node` for the `mcpd` broker (`cargo xtask mcp daemon`) and the one-shot
  `cargo xtask mcp call`.

## Boundaries

- Depends on: npm and Node.js, and the npm registry for `npm ci`.
- Used by:
  - `tools_v2/developer-tools/src/repository_layout.rs`, which names this folder and the entry
    module, and `enfusion_mcp_entrypoint` in `tools_v2/developer-tools/src/enfusion_tooling/`,
    which starts it for `mcpd`;
  - `cargo xtask mcp call` and `cargo xtask mcp daemon` (`tools_v2/xtask/src/commands/mcp/`),
    through that resolver, and `cargo xtask mod dev-bootstrap`, which runs `npm ci` here;
  - `apps/mod/.cursor/mcp.json`, which starts the installed module with `node` by an absolute
    path. `apps/mod/.mcp.json` starts `npx -y enfusion-mcp` instead, which this package does not
    pin.
- Rules: `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs` requires `package.json` to
  exist here; the entry module must stay under this folder's `node_modules/`
  (`tools_v2/developer-tools/src/tests/repository_layout.rs`); only the manifest, the lockfile and
  `.nvmrc` are tracked.

## Related documentation

- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the daemon, the
  call path and the Workbench bootstrap.
