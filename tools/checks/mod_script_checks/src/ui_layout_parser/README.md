# Layout parser record helpers

The line-level helpers of the [Enfusion](/documentation/glossary/a_to_f.md#enfusion) `.layout` parser in
`tools/checks/mod_script_checks/src/ui_layout_parser.rs`: record splitting, keyword and
value extraction, the quoted-string strip that keeps the brace counter in step, and the awk-style
number and string conversions the parser's findings print with.

## Contents

```text
tools/checks/mod_script_checks/src/ui_layout_parser/
└── awk_records.rs  record splitting, widget and slot recognisers, keyword values, quote strip, awk conversions
```

## Boundaries

- Depends on: the standard library only.
- Used by: `Analyzer` in the parent file, which is the only caller; `pub(super)` keeps every
  helper inside the parser.
- Rules: records split on `\n` alone, so a trailing `\r` stays part of the line
  (`records_keep_a_trailing_carriage_return`); quoted strings, [Workbench](/documentation/glossary/n_to_z.md#workbench) GUIDs among them, are
  stripped before braces are counted (`guid_braces_do_not_desync_the_counter`); numbers convert
  the way awk's `v+0` does (`awk_number_conversion_matches_v_plus_zero`). All three tests are in
  `tools/checks/mod_script_checks/src/tests/ui_layout_parser/tests.rs`.
