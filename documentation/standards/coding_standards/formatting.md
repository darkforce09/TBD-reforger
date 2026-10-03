**Status:** live

# Formatting

Rules FMT-1 to FMT-3: how source files are formatted. Code comments and help strings that cite
"§7" or "FMT-2" point here.

## Rules

- **FMT-1 (Readability) — Source is formatter clean.** Rust form: `rustfmt` with its defaults,
  the 100-column width included; the repository holds no rustfmt configuration file, so
  `cargo fmt` takes each crate's edition from its manifest (2024 through the root `Cargo.toml`
  `[workspace.package]`, 2021 for the frontend, which declares its own).
  `cargo xtask mk rust-fmt` runs `cargo fmt --check` in `apps/api`, then
  `cargo fmt --all --check` over every workspace member, the `tools` crates included. Gate:
  CI-BLOCK, the "FMT-1 analog" step of the `api` job; `cargo xtask mk wasm-ci` and
  `cargo xtask mk ci-local-leptos` also check the offline service worker and the app. The rule was first written
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
