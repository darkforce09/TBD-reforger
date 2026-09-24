**Status:** live

# README template: documentation_v2 folder

**When to use:** any folder under `documentation_v2/`, the root included: a feature's folder in
the code mirror, `documentation_v2/runbooks/`, `documentation_v2/known_bugs/`, a folder of
tickets or an archive topic. The pending-merge area is exempt and needs no README. The
[README standard](/documentation_v2/standards/readme_standard.md) defines every rule this template
follows; the documentation_v2 folder kind adds Code.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. The status line
`**Status:** live` comes first, as the standard sets for every README under `documentation_v2/`,
the index of a frozen or archived folder included.

A visual reference set, one folder under a feature's `visual_references/`, fills the skeleton this
way. The purpose opens "Live design target for …" or "Design-phase reference for …", naming the
page or screen. Contents lists `<set>.html`, `<set>.png` and `design_tokens.md`, as the set has
them. How it works, kept although the set holds at most three files, says what the set shows and
how the built UI differs from it. The Code section links the code folder of the page or screen.

````markdown
**Status:** live

# <What the documents cover, in plain words: no path, no backticks>

<One to three sentences: what the folder documents and who reads it.>

## Contents

```text
<repository path of the folder>/
├── <child folder>/   <what it covers: a lowercase phrase, no closing period>
├── <document>.md     <what it covers>
└── <document glob>   <the collection, when the documents are alike>
```

## How it works

<How the documents are organised: which template each follows, how they are named and numbered,
which one to read first, and how a new one is added. A folder with no child folders besides exempt
ones and at most three files, whatever its kind, may leave the section out (README.md not
counted).>

## Code

- [<code folder name>](/<repository path of a code folder>/) — <what the documents say about it>

## Boundaries

- Depends on: <the templates and standards the documents follow, and the sources they draw on>
- Used by: <the documents, READMEs, rules and code comments that link here, found with git grep>
- Rules: <the invariants a change must keep: naming, numbering, what stays and what moves to the
  archive>

## Related documentation

- [<document title>](/documentation_v2/<path to the document>) — <what it covers>
````

## Worked sample

Written from `documentation_v2/known_bugs/`, a folder of two numbered entries listed by one glob.
The sample sits in a fenced block, so no gate reads it as a README; the folder's own README.md is
written from the same files and may differ.

````markdown
**Status:** live

# Known bugs

The registry of recorded, triaged defects: one file per bug, saying what it is, why it happens,
how to live with it and what fixes it, so no one derives the analysis again.

## Contents

```text
documentation_v2/known_bugs/
└── kb_*.md  one known bug per file, named kb_<NNN>_<subject>.md and headed KB-NNN
```

## How it works

Each entry follows the [known bug template](/documentation_v2/standards/templates/known_bug.md):
its number and title, then Status, Symptom, Cause, Workaround, Fix and Related tickets. A new entry
takes the next free number and gets a row below. A resolved entry stays in the folder with its
status set to resolved.

| Entry | Area | Status |
|---|---|---|
| [KB-001](/documentation_v2/known_bugs/kb_001_mission_creator_selection_at_scale.md): selection and copy-paste break at extreme slot counts | Mission Creator | deferred |
| [KB-002](/documentation_v2/known_bugs/kb_002_editor_gate_boot_wedge.md): the editor gate wedges at boot on a font-fallback crash | browser gate harness | resolved |

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — KB-001, selection and paste at
  extreme scale.
- [Browser gate harness](/tools_v2/developer-tools/src/browser_testing/) — KB-002, the Chromium
  build and font cache it launches with.

## Boundaries

- Depends on: the known bug template, which fixes each entry's sections, and the ticket registry
  in `.ai/tickets/` for Related tickets.
- Used by: the editor gate and editor capture runbooks, which cite KB-002; the Cursor rule
  `.cursor/rules/acceptance-gates-reproducible.mdc`, which sends a recorded gate defect here; and
  comments in the gate harness that name KB-002.
- Rules: one bug per file; a number is never reused; a resolved entry stays with its status set to
  resolved.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the gate's wedge modes and debug
  recipe.
````
