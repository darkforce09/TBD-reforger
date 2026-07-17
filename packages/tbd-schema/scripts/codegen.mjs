// Contract codegen (DOCUMENTATION_STANDARDS §9.1). Generates the Rust projections of the
// shared JSON schemas so cross-boundary types are GENERATED, not hand-copied. Rust via
// `quicktype` (serde structs). The Rust validator + kit-aliases reach the canonical schemas
// directly via include_str!, so no schema-copy step is needed. (Go projection removed at the
// T-145 cutover; the TypeScript projection removed at the T-159.29.3 React deletion — the
// Leptos SPA consumes the same Rust crates, so one generated projection serves both sides.)
//
// Run via `make schema-codegen`. Generated outputs are marked DO NOT EDIT.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(root, "..", "..");
const schemaDir = join(root, "schema");
// T-145: serde projections live alongside the crate source.
const rustRoot = join(repoRoot, "apps", "website", "src", "contract", "generated");

// Schemas with a cross-boundary Rust projection.
const targets = [
  { schema: "registry-items.schema.json", rust: "registry_items" },
  { schema: "registry-compat.schema.json", rust: "registry_compat" },
  { schema: "loadout-export.schema.json", rust: "loadout" },
  { schema: "mission-editor-payload.schema.json", rust: "mission_editor" },
  { schema: "faction-library.schema.json", rust: "faction_library" },
];

mkdirSync(rustRoot, { recursive: true });

for (const t of targets) {
  const schemaPath = join(schemaDir, t.schema);

  // Rust — quicktype serde structs. include_str! reaches the canonical schemas directly,
  // so no schema copy step is needed on the Rust side.
  const rustFile = join(rustRoot, `${t.rust}.rs`);
  execFileSync(
    "npx",
    ["quicktype", "-s", "schema", schemaPath, "-l", "rust", "--visibility", "public", "--derive-debug", "-o", rustFile],
    { stdio: "inherit", cwd: root },
  );
  const banner = `// Code generated from JSON Schema using quicktype. DO NOT EDIT.\n// Source: packages/tbd-schema/schema/${t.schema} — regenerate with: make schema-codegen\n\n`;
  writeFileSync(rustFile, banner + readFileSync(rustFile, "utf8"));
  // Format so the committed output is rustfmt-stable (keeps codegen-drift + `cargo fmt
  // --check` both green). rustfmt is on PATH via the rust toolchain.
  execFileSync("rustfmt", ["--edition", "2024", rustFile], { stdio: "inherit" });

  console.log(`  ${t.schema} -> src/contract/generated/${t.rust}.rs`);
}

console.log("schema-codegen complete");
