**Status:** live

# Formatting

Rules FMT-1 to FMT-3: how source files are formatted. Code comments and help strings that cite
"§7" or "FMT-2" point here.

## Rules

- **FMT-1 (Readability) — Source is formatter clean.** Rust form: `rustfmt`, with
  `apps/website/api_v2/rustfmt.toml` setting edition 2024 and a 100-column width for the API
  crate. `cargo xtask mk rust-fmt` runs `cargo fmt --check` in `apps/website/api_v2`, then
  `cargo fmt --all --check` over every workspace member, the `tools_v2` crates included. Gate:
  CI-BLOCK, the "FMT-1 analog" step of the `website-api` job; `cargo xtask mk wasm-ci` and
  `cargo xtask mk ci-local-leptos` also check the engines and the app. The rule was first written
  for `gofmt`; no Go remains, and `cargo fmt` is its form now.
- **FMT-2 (Readability) — The root `.editorconfig` governs whitespace in every file.** Every file
  is UTF-8, ends lines with LF, ends with a final newline and has no trailing whitespace; JSON and
  YAML indent with two spaces. Markdown keeps trailing whitespace, since two trailing spaces are a
  hard line break, and its indentation is not pinned. The paths the checker skips (generated
  output, fixtures, design exports, binary formats and `apps/mod/`) are listed in
  `.editorconfig-checker.json`. Gate: CI-BLOCK, `cargo xtask ci verify-editorconfig`, the first
  step of `cargo xtask ci ci-local` and the `editorconfig` job. The task runs
  `editorconfig-checker` from the repository root and installs the pinned version with
  `go install` when the checker is not on `PATH`; a missing checker with no Go toolchain is a check
  that did not run, never a pass.
- **FMT-3 (Readability) — One formatter of record per language.** It was written for Prettier on
  TypeScript, TSX and CSS; no TypeScript remains, and `rustfmt` is the formatter of record for
  every Rust file. Status: retired; FMT-1 covers it. The app's CSS has no formatter and no gate
  beyond FMT-2.

## Notes

`.editorconfig` still carries sections for Go, TypeScript, JavaScript and Make files, and its
comments cite FMT-1 as `gofmt` and FMT-3 as Prettier; none of those file types is tracked, so the
sections match nothing. Rust indentation is `rustfmt`'s, not `.editorconfig`'s.
