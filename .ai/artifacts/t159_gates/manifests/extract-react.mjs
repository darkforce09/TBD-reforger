#!/usr/bin/env node
// T-159.0.5 — S-gate structural manifest extractor (React oracle side).
//
// Emits the five structural manifests that the S gate diffs against the Leptos source once
// each subsystem is ported:
//   routes.csv  hooks.csv  components.csv  css_tokens.txt  deps.csv
//
// Zero npm deps (Node built-ins only), matching the repo's verify-* discipline so it runs in
// the CI backend job without `npm ci` (see memory t125-verify-scripts-portable). Deterministic
// output (sorted rows) so a `diff` against the committed manifest is the pass/fail signal.
//
// Usage:
//   node .ai/artifacts/t159_gates/manifests/extract-react.mjs           # write manifests here
//   node .ai/artifacts/t159_gates/manifests/extract-react.mjs --check   # re-extract + diff; exit 1 on drift
//
// The Leptos-side extractor (extract-leptos.mjs) lands per slice and emits the same schemas;
// gate_s.mjs set/column-diffs the two. This file is the oracle: it defines the target shape.

import { readFileSync, writeFileSync, readdirSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join, resolve, relative } from 'node:path'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(HERE, '../../../../') // manifests → t159_gates → artifacts → .ai → repo root
const FRONTEND = join(ROOT, 'apps/website/frontend')
const SRC = join(FRONTEND, 'src')

const CHECK = process.argv.includes('--check')

/* ─────────────────────────── helpers ─────────────────────────── */

const read = (p) => readFileSync(p, 'utf8')
const csvCell = (v) => {
  const s = String(v)
  return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s
}
const toCsv = (header, rows) =>
  [header.join(','), ...rows.map((r) => r.map(csvCell).join(','))].join('\n') + '\n'

/** Match the `[...]` array starting at `openIdx` (must point at `[`); returns end index of `]`. */
function matchBracket(src, openIdx) {
  let depth = 0
  for (let i = openIdx; i < src.length; i++) {
    if (src[i] === '[') depth++
    else if (src[i] === ']') {
      depth--
      if (depth === 0) return i
    }
  }
  return -1
}

/* ─────────────────────────── routes.csv ─────────────────────────── */
// Oracle: src/router.tsx. Columns: path, component, fullBleed, chromeless, router_auth.
// DEV-only /_spike/* routes are excluded from the strict manifest until ported.

function extractRoutes() {
  const src = read(join(SRC, 'router.tsx'))

  // Auth spans: each `minRole="X"` guards the paths inside the `children: [ … ]` that follows it.
  const authSpans = []
  const minRoleRe = /minRole="(\w+)"/g
  let m
  while ((m = minRoleRe.exec(src))) {
    const childrenIdx = src.indexOf('children: [', m.index)
    if (childrenIdx === -1) continue
    const open = src.indexOf('[', childrenIdx)
    const end = matchBracket(src, open)
    if (end !== -1) authSpans.push({ role: m[1], start: open, end })
  }
  const authAt = (offset) => {
    for (const s of authSpans) if (offset > s.start && offset < s.end) return s.role
    return 'none'
  }

  // Route anchors: every `path: '…'` plus the index route.
  const anchors = []
  const pathRe = /\bpath:\s*'([^']*)'/g
  while ((m = pathRe.exec(src))) anchors.push({ offset: m.index, path: m[1] })
  const idxRe = /\bindex:\s*true/g
  while ((m = idxRe.exec(src))) anchors.push({ offset: m.index, path: '/' })
  anchors.sort((a, b) => a.offset - b.offset)

  const rows = []
  for (let i = 0; i < anchors.length; i++) {
    const a = anchors[i]
    const winEnd = i + 1 < anchors.length ? anchors[i + 1].offset : src.length
    const window = src.slice(a.offset, winEnd)
    const comp = window.match(/<([A-Z][A-Za-z0-9]*Page)\b/)
    const component = comp ? comp[1] : ''
    // Drop layout-only routes: the `path: '/'` AppLayout container renders <AppLayout/> (no
    // *Page) and only wraps children — it is not a leaf the manifest tracks. Every real leaf
    // renders a <…Page/>, so an empty component reliably identifies the container.
    if (!component) continue
    const fullBleed = /fullBleed:\s*true/.test(window)
    const chromeless = /chromeless:\s*true/.test(window)
    // Normalize to absolute paths; AppLayout children are declared relative.
    let path = a.path
    if (path !== '*' && !path.startsWith('/')) path = '/' + path
    const isDev = path.startsWith('/_spike')
    if (isDev) continue // excluded from strict manifest
    rows.push([path, component, fullBleed, chromeless, authAt(a.offset)])
  }
  rows.sort((x, y) => x[0].localeCompare(y[0]))
  return toCsv(['path', 'component', 'fullBleed', 'chromeless', 'router_auth'], rows)
}

/* ─────────────────────────── hooks.csv ─────────────────────────── */
// Oracle: src/hooks/queries.ts (kind=query) + src/hooks/mutations.ts (kind=mutation).
// One row per (@route tag, enclosing export function). useSaveFaction has two @route tags → two rows.

function extractHooksFile(file, kind) {
  const src = read(join(SRC, 'hooks', file))
  const lines = src.split('\n')
  const rows = []
  let pending = []
  for (const line of lines) {
    const r = line.match(/@route\s+(\w+)\s+(\S+)/)
    if (r) {
      pending.push({ method: r[1], url: r[2] })
      continue
    }
    const fn = line.match(/export function (use\w+)\s*\(/)
    if (fn) {
      for (const p of pending) rows.push([fn[1], kind, p.method, p.url])
      pending = []
    }
  }
  return rows
}

function extractHooks() {
  const rows = [
    ...extractHooksFile('queries.ts', 'query'),
    ...extractHooksFile('mutations.ts', 'mutation'),
  ]
  rows.sort((a, b) => a[0].localeCompare(b[0]) || a[2].localeCompare(b[2]) || a[3].localeCompare(b[3]))
  return toCsv(['name', 'kind', 'method', 'url'], rows)
}

/* ─────────────────────────── components.csv ─────────────────────────── */
// Exported PascalCase component identifiers across the component dirs.
// kind = ui | layout | shell (shell = the loose components/*.tsx helpers).

function extractComponentsDir(subdir, kind, rows) {
  const dir = join(SRC, subdir)
  if (!existsSync(dir)) return
  for (const f of readdirSync(dir)) {
    if (!f.endsWith('.tsx')) continue
    const rel = relative(SRC, join(dir, f))
    const body = read(join(dir, f))
    const names = new Set()
    let m
    const fnRe = /export function ([A-Z]\w+)\s*\(/g
    while ((m = fnRe.exec(body))) names.add(m[1])
    const constRe = /export const ([A-Z]\w+)\s*[:=]/g
    while ((m = constRe.exec(body))) names.add(m[1])
    // Named-export blocks — `export { Button, buttonVariants }` / `export { Foo as Bar }` — the
    // shadcn idiom the fn/const regexes miss. Skip `export type { … }`; keep PascalCase only
    // (variant objects like `buttonVariants` are lowercase → dropped); resolve `as` aliases.
    const blockRe = /export\s*(?!type\b)\{([^}]*)\}/g
    while ((m = blockRe.exec(body))) {
      for (let part of m[1].split(',')) {
        part = part.trim()
        if (!part) continue
        const asM = part.match(/\bas\s+([A-Za-z0-9_]+)/)
        const name = asM ? asM[1] : part
        if (/^[A-Z]\w*$/.test(name)) names.add(name)
      }
    }
    for (const n of names) rows.push([n, kind, rel])
  }
}

function extractComponents() {
  const rows = []
  extractComponentsDir('components/ui', 'ui', rows)
  extractComponentsDir('components/layout', 'layout', rows)
  extractComponentsDir('components', 'shell', rows) // top-level helpers (AuthGate, QueryState, …)
  rows.sort((a, b) => a[2].localeCompare(b[2]) || a[0].localeCompare(b[0]))
  return toCsv(['name', 'kind', 'path'], rows)
}

/* ─────────────────────────── css_tokens.txt ─────────────────────────── */
// Every CSS custom-property NAME declared in index.css (the @theme block, @theme inline
// aliases, and the :root/.dark shadcn vars). Sorted, unique — set-equality is the gate.

function extractCssTokens() {
  const src = read(join(SRC, 'index.css'))
  const names = new Set()
  const re = /^\s*(--[a-z0-9-]+)\s*:/gm
  let m
  while ((m = re.exec(src))) names.add(m[1])
  return [...names].sort().join('\n') + '\n'
}

/* ─────────────────────────── deps.csv ─────────────────────────── */
// Disposition ledger: every runtime dependency in package.json MUST have a disposition here,
// so a newly-added npm dep fails the extractor (forces a migration decision). The Leptos side
// asserts each `reimplement` row maps to a Cargo dep or a documented drop.

const DISPOSITION = {
  react: ['reimplement', 'leptos'],
  'react-dom': ['reimplement', 'leptos'],
  'react-router-dom': ['reimplement', 'leptos_router'],
  '@tanstack/react-query': ['reimplement', 'leptos Resource/Action + invalidation registry'],
  '@tanstack/react-virtual': ['reimplement', 'hand-rolled virtual window'],
  zustand: ['reimplement', 'signals + provide_context'],
  '@base-ui/react': ['reimplement', 'hand-rolled Aegis primitives'],
  sonner: ['reimplement', 'hand-rolled toast'],
  'lucide-react': ['reimplement', 'leptos-icons'],
  comlink: ['reimplement', 'gloo-worker'],
  axios: ['reimplement', 'gloo-net'],
  'class-variance-authority': ['reimplement', 'cn helper'],
  clsx: ['reimplement', 'cn helper'],
  'tailwind-merge': ['reimplement', 'cn helper'],
  'date-fns': ['reimplement', 'chrono'],
  idb: ['reimplement', 'web_sys IDB / rexie'],
  '@fontsource/inter': ['keep', 'copy woff2 into assets'],
  '@fontsource/jetbrains-mono': ['keep', 'copy woff2 into assets'],
  shadcn: ['keep-build', 'Tailwind CSS input, folded into aegis.css'],
  'tw-animate-css': ['keep-build', 'Tailwind CSS input, folded into aegis.css'],
  'react-hook-form': ['drop', '0 imports'],
  '@hookform/resolvers': ['drop', '0 imports'],
  zod: ['drop', 'unused src/schemas'],
  buffer: ['drop', 'DEM decode in wasm'],
  pngjs: ['drop', 'DEM decode in wasm'],
  rbush: ['drop', 'wasm spatial index'],
}

function extractDeps() {
  const pkg = JSON.parse(read(join(FRONTEND, 'package.json')))
  const deps = Object.keys(pkg.dependencies ?? {}).sort()
  const missing = deps.filter((d) => !DISPOSITION[d])
  if (missing.length) {
    console.error(`deps.csv: runtime deps with no disposition (add to DISPOSITION): ${missing.join(', ')}`)
    process.exit(2)
  }
  const rows = deps.map((d) => [d, DISPOSITION[d][0], DISPOSITION[d][1]])
  return toCsv(['npm_pkg', 'disposition', 'replacement'], rows)
}

/* ─────────────────────────── run ─────────────────────────── */

const manifests = {
  'routes.csv': extractRoutes(),
  'hooks.csv': extractHooks(),
  'components.csv': extractComponents(),
  'css_tokens.txt': extractCssTokens(),
  'deps.csv': extractDeps(),
}

let drift = false
for (const [name, content] of Object.entries(manifests)) {
  const path = join(HERE, name)
  if (CHECK) {
    const prev = existsSync(path) ? read(path) : ''
    if (prev !== content) {
      drift = true
      console.error(`DRIFT: ${name} differs from committed manifest`)
    }
  } else {
    writeFileSync(path, content)
  }
}

// Summary counts (stdout).
const count = (csv) => csv.trim().split('\n').length - 1
const lines = (txt) => txt.trim().split('\n').length
console.log(
  JSON.stringify(
    {
      mode: CHECK ? 'check' : 'write',
      routes: count(manifests['routes.csv']),
      hooks: count(manifests['hooks.csv']),
      components: count(manifests['components.csv']),
      css_tokens: lines(manifests['css_tokens.txt']),
      deps: count(manifests['deps.csv']),
    },
    null,
    2,
  ),
)

if (CHECK && drift) process.exit(1)
