#!/usr/bin/env node
// S-routes gate (Leptos side). Extracts the route table from apps/website-leptos/src/router.rs and
// diffs it against the React oracle manifest routes.csv. Robust to rustfmt line-wrapping: it splits
// on `RouteDef { … }` blocks and pulls each field by name. Zero deps (Node built-ins).
//
//   node .ai/artifacts/t159_gates/manifests/extract-leptos-routes.mjs
//
// Exit 0 = the Leptos route table set/column-diffs equal to routes.csv; 1 = drift (printed).

import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join, resolve } from 'node:path'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(HERE, '../../../../')
const ROUTER = join(ROOT, 'apps/website-leptos/src/router.rs')
const ORACLE = join(HERE, 'routes.csv')

const src = readFileSync(ROUTER, 'utf8')
// Isolate the ROUTES array body so the `struct RouteDef { … }` definition isn't parsed as an entry.
const body = (src.match(/static ROUTES[^=]*=\s*&\[([\s\S]*)\];/) || [, ''])[1]
const rows = []
const re = /RouteDef\s*\{([\s\S]*?)\}/g // no nested braces in a RouteDef, so non-greedy is exact
let m
while ((m = re.exec(body))) {
  const c = m[1]
  const s = (k) => (c.match(new RegExp(`${k}:\\s*"([^"]*)"`)) || [, ''])[1]
  const b = (k) => new RegExp(`${k}:\\s*true`).test(c)
  rows.push([s('path'), s('component'), b('full_bleed'), b('chromeless'), s('auth')])
}
rows.sort((a, b) => a[0].localeCompare(b[0]))
const leptosCsv = ['path,component,fullBleed,chromeless,router_auth', ...rows.map((r) => r.join(','))].join('\n') + '\n'

const oracle = readFileSync(ORACLE, 'utf8')

if (leptosCsv === oracle) {
  console.log(JSON.stringify({ gate: 'S-routes', pass: true, routes: rows.length }, null, 2))
  process.exit(0)
}

// Report the row-level diff.
const toMap = (csv) => new Map(csv.trim().split('\n').slice(1).map((l) => [l.split(',')[0], l]))
const lm = toMap(leptosCsv)
const om = toMap(oracle)
const diffs = []
for (const [path, line] of om) {
  if (!lm.has(path)) diffs.push({ path, oracle: line, leptos: '(missing)' })
  else if (lm.get(path) !== line) diffs.push({ path, oracle: line, leptos: lm.get(path) })
}
for (const [path, line] of lm) if (!om.has(path)) diffs.push({ path, oracle: '(missing)', leptos: line })

console.log(JSON.stringify({ gate: 'S-routes', pass: false, oracle: om.size, leptos: lm.size, diffs: diffs.length }, null, 2))
for (const d of diffs.slice(0, 40)) console.log(`  ${d.path}\n    oracle: ${d.oracle}\n    leptos: ${d.leptos}`)
process.exit(1)
