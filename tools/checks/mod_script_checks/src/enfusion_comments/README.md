# Enfusion comment card check

The body of `cargo xtask verify enfusion-comments`: the in-code documentation card for
[EnfScript](/documentation/glossary/a_to_f.md#enfscript) sources, checked rule by rule over the
`.c` files of the [mod](/documentation/glossary/g_to_m.md#mod).

## Contents

```text
tools/checks/mod_script_checks/src/enfusion_comments/
├── ascii_rule.rs                    ECM-1: ASCII only, string literals included
├── attribute_description_rule.rs    ECM-7: `desc:` on `[Attribute]`, `description:` on `[ComponentEditorProps]`
├── boundary_tag_rule.rs             ECM-6: `@route` on REST call sites, `@contract` and the `*Struct`/`*Wire` names
├── checked_script.rs                one lexed, outlined script and its "directly above" banner lookups
├── context_free_prose_rule.rs       ECM-8: the comment lexicon, separators, commented-out code, file:line references
├── declaration_banner_rule.rs       ECM-3: a `//!` banner above every class, enum and method
├── file_header_rule.rs              ECM-2: the `/** @file @brief Role Position State Invariants */` header
├── findings.rs                      the rule ids ECM-1 to ECM-9 and the finding type
├── mod.rs                           the entry: pinned roots, `--path`, the walk, the report and the exit code
├── network_authority_rule.rs        ECM-5: `@authority`, `@rpc` and `@replicated`
├── primary_type_rule.rs             ECM-9: the file is named after its primary type
├── script_outline.rs                the declaration outline: types, methods, fields, enum members, attributes
└── trailing_member_doc_rule.rs      ECM-4: a trailing `//!<` on every field and enum member
```

## How it works

`verify_enfusion_comments` takes the `--path` values, or `PINNED_ROOTS` when there are none, and
refuses any root outside `mod`. It walks the roots with `verification_core::scan::walk_files`,
keeping `.c` files, and runs `check_script` on each. `check_script` builds one `CheckedScript`:
`enfusion_script_lexer::split_script_lines` splits every line into code and comments (a `//` inside
a string literal stays code), and `script_outline::outline_script` reads the declarations from the
code alone. Each rule module then returns its findings.

The outline is deterministic: a method is a class-body or top-level statement with `(` before any
`=` that ends at `{` or `;`; a field is any other class-body statement ending at `;`; a `[ ... ]`
opening a statement is an attribute of what follows; preprocessor lines are skipped. "Directly
above" walks up past attribute-only lines and nothing else.

The report prints one `<rule> <path>:<line> <message>` line per finding, sorted by path, line and
rule, then a summary line and one count per rule. Exit codes: 0 clean; 1 findings; 2 did not run
(no roots, a root outside `mod`, a missing root, an unreadable file, or a walk that found no
`.c` file).

## Boundaries

- Depends on: `super::enfusion_script_lexer`; `verification_core::scan` and
  `verification_core::NotRun`; `regex`.
- Used by: `tools/xtask/src/commands/verify/dispatch.rs` (`verify enfusion-comments`).
- Rules:
  - One rule module per card item, each at or under 500 lines.
  - `PINNED_ROOTS` entries sit under `mod/` (`pinned_roots_sit_under_the_mod_tree`).
  - A missing root or an empty walk is exit 2, never a pass (`a_missing_root_does_not_run`,
    `an_empty_walk_does_not_run`).
  - Every rule has a passing and a failing fixture in
    `tools/checks/mod_script_checks/src/tests/enfusion_comments_tests.rs`.

## Related documentation

- [Enfusion script header](/documentation/standards/templates/enfusion_script_header.md) — the
  card, its header skeleton and a worked sample.
