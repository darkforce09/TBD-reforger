#!/usr/bin/env node
// T-145 gate G5 — differential equivalence harness.
//
// Replays a request corpus against the Go and Rust backends (same Postgres, same
// seed) and asserts every response is EQUIVALENT under the `≡` relation defined in
// the plan: identical status; identical values for the present header subset
// {content-type, etag, location, content-disposition, cache-control}; and bodies
// canonically-equal — numbers by value, strings decoded, objects order-insensitive
// but key-presence-sensitive (absent ≠ null → enforces omitempty parity), arrays
// order-sensitive, temporal/string leaves exact.
//
// Usage: GO_URL=http://localhost:8080 RUST_URL=http://localhost:8081 \
//        SERVICE_TOKEN=... node scripts/website/differential.mjs

const GO = process.env.GO_URL || "http://localhost:8080";
const RUST = process.env.RUST_URL || "http://localhost:8081";
const SVC = process.env.SERVICE_TOKEN || "diff-service-token";

const HEADER_SUBSET = [
  "content-type",
  "etag",
  "location",
  "content-disposition",
  "cache-control",
];

// --- canonicalization (the `≡` body relation) ---
function canonical(v) {
  if (Array.isArray(v)) return v.map(canonical); // arrays: order-sensitive
  if (v && typeof v === "object") {
    const out = {};
    for (const k of Object.keys(v).sort()) out[k] = canonical(v[k]); // objects: order-insensitive, key-presence-sensitive
    return out;
  }
  return v; // numbers by value (JSON parses 0 and 0.0 identically); strings already decoded
}
const canonJSON = (v) => JSON.stringify(canonical(v));

// First differing JSON path (for a readable failure), or null.
function firstDiff(a, b, path = "$") {
  if (canonJSON(a) === canonJSON(b)) return null;
  const ta = kind(a), tb = kind(b);
  if (ta !== tb) return `${path}: type ${ta} ≠ ${tb} (${short(a)} vs ${short(b)})`;
  if (ta === "array") {
    if (a.length !== b.length) return `${path}: array length ${a.length} ≠ ${b.length}`;
    for (let i = 0; i < a.length; i++) {
      const d = firstDiff(a[i], b[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (ta === "object") {
    const ka = Object.keys(a).sort(), kb = Object.keys(b).sort();
    if (ka.join(",") !== kb.join(",")) return `${path}: keys {${ka}} ≠ {${kb}}`;
    for (const k of ka) {
      const d = firstDiff(a[k], b[k], `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return `${path}: ${short(a)} ≠ ${short(b)}`;
}
const kind = (v) => (Array.isArray(v) ? "array" : v === null ? "null" : typeof v);
const short = (v) => JSON.stringify(v)?.slice(0, 80);

// --- request execution ---
async function login(base) {
  const r = await fetch(`${base}/api/v1/auth/dev-login?role=admin`, { redirect: "manual" });
  const loc = r.headers.get("location") || "";
  const m = loc.match(/access_token=([^&]+)/);
  if (!m) throw new Error(`dev-login failed on ${base}: ${r.status}`);
  return m[1];
}

async function hit(base, tok, c) {
  const headers = {};
  if (c.auth === "admin") headers.authorization = `Bearer ${tok}`;
  if (c.auth === "service") headers["x-service-token"] = SVC;
  if (c.body != null) headers["content-type"] = "application/json";
  const r = await fetch(`${base}${c.path}`, {
    method: c.method || "GET",
    headers,
    body: c.body != null ? JSON.stringify(c.body) : undefined,
    redirect: "manual",
  });
  const text = await r.text();
  let json;
  try {
    json = JSON.parse(text);
  } catch {
    json = { __raw: text };
  }
  return { status: r.status, headers: r.headers, json };
}

function compareHeaders(go, rust) {
  for (const h of HEADER_SUBSET) {
    const a = go.get(h);
    const b = rust.get(h);
    // content-type: compare the media type only (charset formatting is cosmetic).
    if (h === "content-type") {
      if ((a || "").split(";")[0].trim() !== (b || "").split(";")[0].trim())
        return `header content-type: ${a} ≠ ${b}`;
      continue;
    }
    if ((a || "") !== (b || "")) return `header ${h}: ${a} ≠ ${b}`;
  }
  return null;
}

// Delete a dot-path from an object (for masking by-design-volatile fields).
function maskPath(obj, dotted) {
  const parts = dotted.split(".");
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (cur == null || typeof cur !== "object") return;
    cur = cur[parts[i]];
  }
  if (cur && typeof cur === "object") delete cur[parts[parts.length - 1]];
}

// --- corpus: reads + error cases exercising the encoder contract ---
const BAD_UUID = "11111111-1111-1111-1111-111111111111";
const corpus = [
  // identity + numeric(attendance_rate). last_login_at/updated_at are set by each
  // server's own dev-login instant → masked (by-design volatile, not a wire contract).
  {
    name: "me",
    path: "/api/v1/me",
    auth: "admin",
    mask: ["user.last_login_at", "user.updated_at", "user.created_at"],
  },
  // aggregate: nested, nulls, timestamps
  { name: "dashboard", path: "/api/v1/dashboard", auth: "admin" },
  // list envelope + omitempty + tag enum + published_at timestamp
  { name: "announcements", path: "/api/v1/announcements", auth: "admin" },
  // numeric floats (kd_ratio) + int casts
  { name: "leaderboards", path: "/api/v1/leaderboards?category=kd", auth: "admin" },
  { name: "leaderboards.bad", path: "/api/v1/leaderboards?category=nope", auth: "admin" },
  // numeric(server_fps) + inet(ip) + nested modpack
  { name: "servers", path: "/api/v1/servers", auth: "admin" },
  { name: "modpacks", path: "/api/v1/modpacks", auth: "admin" },
  { name: "modpacks.current", path: "/api/v1/modpacks/current", auth: "admin" },
  { name: "wiki", path: "/api/v1/wiki", auth: "admin" },
  { name: "vehicles", path: "/api/v1/vehicle-database", auth: "admin" },
  // missions: list + detail (json_payload passthrough, omitempty), time_of_day
  { name: "missions", path: "/api/v1/missions", auth: "admin" },
  // events: list (fill aggregates) + hub (nested dossiers, timestamps)
  { name: "events", path: "/api/v1/events?scope=all", auth: "admin" },
  // LOA: date columns serialized as midnight-UTC timestamps
  { name: "admin.leave", path: "/api/v1/admin/leave-requests", auth: "admin" },
  { name: "audit", path: "/api/v1/admin/audit-logs", auth: "admin" },
  // error-body + status parity
  { name: "missing.mission", path: `/api/v1/missions/${BAD_UUID}`, auth: "admin" },
  { name: "bad.mission.id", path: "/api/v1/missions/not-a-uuid", auth: "admin" },
  { name: "unauth", path: "/api/v1/announcements", auth: "none" },
  { name: "registry.nocurrent", path: "/api/v1/registry", auth: "admin" },
];

const run = async () => {
  const [goTok, rustTok] = await Promise.all([login(GO), login(RUST)]);
  let pass = 0;
  const fails = [];
  for (const c of corpus) {
    const [g, r] = await Promise.all([hit(GO, goTok, c), hit(RUST, rustTok, c)]);
    for (const m of c.mask || []) {
      maskPath(g.json, m);
      maskPath(r.json, m);
    }
    const problems = [];
    if (g.status !== r.status) problems.push(`status ${g.status} ≠ ${r.status}`);
    const hd = compareHeaders(g.headers, r.headers);
    if (hd) problems.push(hd);
    const bd = firstDiff(g.json, r.json);
    if (bd) problems.push(`body ${bd}`);
    if (problems.length === 0) {
      pass++;
      console.log(`  ✓ ${c.name} (${g.status})`);
    } else {
      fails.push({ name: c.name, problems, go: g.json, rust: r.json });
      console.log(`  ✗ ${c.name}: ${problems.join("; ")}`);
    }
  }
  console.log(`\nG5 differential: ${pass}/${corpus.length} equivalent`);
  if (fails.length) {
    console.log("\n--- failures ---");
    for (const f of fails) {
      console.log(`\n[${f.name}] ${f.problems.join("; ")}`);
      console.log("  go  :", JSON.stringify(f.go)?.slice(0, 400));
      console.log("  rust:", JSON.stringify(f.rust)?.slice(0, 400));
    }
    process.exit(1);
  }
  console.log("G5 PASS — every response ≡ across Go and Rust.");
};

run().catch((e) => {
  console.error("harness error:", e.message);
  process.exit(2);
});
