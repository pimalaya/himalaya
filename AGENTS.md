# AGENTS.md

## Cairn

This repository follows **Cairn**, a language- and tool-agnostic convention for keeping a living spec, reviewable change proposals, and an honest history next to the code. The full format and by-hand guide live at <https://github.com/pimalaya/cairn> (`CAIRN.md` and `GUIDE.md`). No tooling is required: you create and check the structure by reading and following the rules.

If you are an agent working in this repository, do the following **by default, without being asked**.

### 1. Locate the Cairn root

The root is this repository (marked by the `cairn/` directory). All Cairn artifacts live under `cairn/`: `spec/` is current truth (one file per capability), `changes/` holds in-flight proposals, `log/` is the dated history.

### 2. Before non-trivial work, propose

For anything beyond a trivial fix, create `cairn/changes/<change-id>/` (kebab-case) with:

- `proposal.md`: *why* and *what* (frontmatter: `cairn: change`, `id`, `status: active`, `created`).
- `tasks.md`: the checklist.
- `delta.md`: what this changes in the spec (`## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`).

Let the human review intent **before** you write code. Trivial fixes may skip this and go straight to landing.

### 3. After work lands, fold and log (never skip)

- Fold the change's delta into `cairn/spec/<capability>.md` so the spec always reflects current truth (append ADDED, replace MODIFIED, delete REMOVED). A spec file holds current truth only, no history and no rationale.
- Append a dated entry `cairn/log/YYYY-MM-DD-<change-id>.md` describing what landed and which capabilities moved. Log entries are immutable.
- Set the change `status: landed`.

> **The forcing rule:** a change that affects behaviour is not *done* until the spec is updated and the log entry is written.

### 4. Stay conformant

Check the structure yourself against the strict rules (CAIRN.md §8): a discoverable root, `spec/ changes/ log/` present, every Cairn file carrying a valid `cairn:` type, each change having `proposal.md` and `tasks.md`, kebab-case ids, literal delta headings, and a log entry for every landed change. Everything else (prose, naming, ordering, extra files) is free.

## Everything else

For how this binary is built, where changes belong, and the Pimalaya standards it follows, read `CONTRIBUTING.md` and the `src/main.rs` header (the crate's architecture document). Manual provider test reports live under `cairn/spec/testing/`.
