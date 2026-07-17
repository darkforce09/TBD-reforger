#!/usr/bin/env node
// T-152.10 — program-wide cartographic fidelity aggregator (G2).
// Checks prior-slice verify logs (G1 subset) and re-runs committed-data sub-verifiers.
import { readFileSync, existsSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const artifacts = join(repoRoot, ".ai", "artifacts");

const SLICE_LOGS = Array.from({ length: 10 }, (_, i) =>
  join(artifacts, `t152_${i}_verify_log.md`),
);

let failures = 0;
const pass = (msg) => console.log(`  PASS  ${msg}`);
const fail = (msg) => {
  failures++;
  console.log(`  FAIL  ${msg}`);
};

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...opts,
  });
  return { ok: r.status === 0, status: r.status ?? 1, stdout: r.stdout ?? "", stderr: r.stderr ?? "" };
}

function checkVerifyLogs() {
  console.log("verify-t152-cartographic: slice logs (G1 subset)");
  for (const [i, path] of SLICE_LOGS.entries()) {
    const label = `T-152.${i}`;
    if (!existsSync(path)) {
      fail(`${label} missing ${path}`);
      continue;
    }
    const text = readFileSync(path, "utf8");
    const manualIdx = text.search(/^## Manual/m);
    const autoSection = manualIdx >= 0 ? text.slice(0, manualIdx) : text;
    if (/\*\*FAIL\*\*/.test(autoSection)) {
      fail(`${label} verify log contains **FAIL** in automated section`);
      continue;
    }
    const gateRows = [...autoSection.matchAll(/\| \*\*G\d+[^|]*\|[^|]*\| \*\*PASS\*\*/g)];
    const gateFailRows = [...autoSection.matchAll(/\| \*\*G\d+[^|]*\|[^|]*\| \*\*FAIL\*\*/g)];
    const verdictOk =
      gateFailRows.length === 0 &&
      (gateRows.length > 0 ||
        /ALL (automated )?Gn PASS/i.test(text) ||
        /Automated Gn all \*\*PASS\*\*/i.test(text) ||
        /tag \*\*T-152\.\d+\*\* allowed/i.test(text) ||
        (i === 0 && /\*\*ALL Gn PASS\*\*/.test(text)) ||
        (i === 2 && /\*\*G7\*\*.*\*\*PASS\*\*/.test(text)));
    if (!verdictOk) {
      fail(`${label} verify log missing PASS verdict / ship marker`);
      continue;
    }
    pass(`${label} log OK (${path.replace(repoRoot + "/", "")})`);
  }
}

function runStep(name, fn) {
  console.log(`\nverify-t152-cartographic: ${name}`);
  fn();
}

function runCargo(args) {
  const label = `cargo xtask ${args.join(" ")}`;
  const r = run("cargo", ["run", "-q", "-p", "xtask", "--", ...args]);
  if (r.ok) {
    pass(`${label} exit 0`);
    return true;
  }
  fail(`${label} exit ${r.status}`);
  const err = (r.stderr + r.stdout).trim();
  if (err) console.log(err.split("\n").slice(-8).join("\n"));
  return false;
}

function runNode(rel, args = [], env = {}) {
  const r = run("node", [join(repoRoot, rel), ...args], { env: { ...process.env, ...env } });
  if (r.ok) {
    pass(`${rel} exit 0`);
    return true;
  }
  fail(`${rel} exit ${r.status}`);
  if (r.stderr.trim()) console.log(r.stderr.trim().split("\n").slice(-8).join("\n"));
  return false;
}

function runMake(target, env = {}) {
  const r = run("make", [target], { env: { ...process.env, ...env } });
  if (r.ok) {
    pass(`make ${target} exit 0`);
    return true;
  }
  fail(`make ${target} exit ${r.status}`);
  if (r.stderr.trim()) console.log(r.stderr.trim().split("\n").slice(-8).join("\n"));
  return false;
}

checkVerifyLogs();

runStep("glyph atlas (.2)", () => {
  runMake("map-glyphs-verify");
});

runStep("export artifacts (G6 subset)", () => {
  runMake("map-export-validate");
});

runStep("P5_props phase census (.4)", () => {
  runMake("map-verify-phase", { TERRAIN: "everon", PHASE: "P5_props" });
});

// T-165.4: the four label gates are Rust (`cargo xtask schema …`) — the .mjs versions are deleted.
runStep("locations (.6)", () => {
  runCargo(["schema", "locations", "--terrain", "everon"]);
});

runStep("height labels (.7)", () => {
  runCargo(["schema", "height-labels", "--terrain", "everon"]);
});

runStep("town labels (.8)", () => {
  runCargo(["schema", "town-labels", "--terrain", "everon", "--zoom=-2"]);
});

runStep("road names (.9)", () => {
  runCargo(["schema", "road-names", "--terrain", "everon", "--zoom", "0"]);
});

runStep("wasm telemetry (L5)", () => {
  // T-164: the wasm-bindgen pkg died with the React app (T-159.29.3) and `make wasm` with it —
  // the size guard is obsolete (map-engine crates are gated by `make wasm-ci` cargo tests). If a
  // pkg is hand-built to the old path the guard still runs; otherwise skip, never fail.
  const wasmPath = join(
    repoRoot,
    "apps/website/frontend/src/wasm/pkg/map_engine_wasm_bg.wasm",
  );
  if (!existsSync(wasmPath)) {
    console.log("  SKIP  wasm size guard — retired with the React wasm pkg (make wasm-ci owns the crates)");
    return;
  }
  const bytes = statSync(wasmPath).size;
  const t152_3_tip = 4_193_922;
  if (bytes < t152_3_tip) {
    fail(`wasm ${bytes} B < T-152.3 tip ${t152_3_tip} B`);
  } else {
    pass(`wasm size ${bytes} B (≥ T-152.3 tip ${t152_3_tip} B)`);
  }
});

console.log("");
if (failures) {
  console.error(`verify-t152-cartographic: FAIL (${failures})`);
  process.exit(1);
}
console.log("verify-t152-cartographic: OK");
