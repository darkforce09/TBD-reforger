**Status:** live

# S1 agents R2 to R4: area fix-ups template

The orchestrator fills `{ID}` and `{BODY}` from R1's report. Areas: R2 tools (xtask, verification core, developer tools, ticket engine tests and gates), R3 apps (API, frontend, map engine tests that read moved paths), R4 documentation (readme-coverage, link-check, markdown-placement, archived-file status lines, the agent instruction files, the documentation mirror rule and the workspace layout document).

```text
You are agent {ID} of stage S1 of the workspace restructure program. R1 has just dropped the
`_v2` suffix of the four top-level folders, now `assets`, `contracts`, `documentation` and `tools`,
and given the tool crates snake_case folders and package names in place of their hyphenated
spellings (`tools/verification_core`, `tools/developer_tools`, `tools/ticket_engine`) using the
relocation tool; the tree compiles. Your job is to make your
area's tests and gates green again without hand-rewriting anything the tool can rewrite, and
without weakening any test.

Read [the shared brief](/documentation/restructure/agent_briefs/shared_brief.md) first, where
`<scratch>` = `<scratch>`.
Note: the program documents now live in `documentation/restructure/`. R1's report is in
`<scratch>/report_R1.md`; read its step-6 list first: items left for your area are yours.
Agents R2, R3 and R4 run in parallel with disjoint ownership (below).

{BODY}

Budget: M (250k tokens). Stop there and report done and not done.
```
