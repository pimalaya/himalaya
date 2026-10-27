---
cairn: tasks
change: json-keys-are-camel-case
---

# Tasks

Landed 2026-10-27 in the working tree, ahead of the version bump.

- [x] Put `#[serde(rename_all = "camelCase")]` on every type registered in src/json_schema.rs, dropping the twenty-six kebab-case renames it replaces
- [x] Leave the provider passthrough renames (`@odata.nextLink`, `nextPageToken`) and the transparent newtypes over io-gmail and io-msgraph resources untouched
- [x] Leave src/config.rs kebab-case: TOML keys are not `--json` keys
- [x] Rename the forty-two registered types that are not yet `*Output`, the reference being `GmailProfileOutput` (fifty-eight by the time it landed, counting each module of a shared name)
- [x] Update the doc comments naming a key spelling, starting with `GmailProfileOutput`
- [x] Regenerate the JSON Schemas and check no key kept a hyphen
- [x] CHANGELOG under `Changed`, as breaking: under [Unreleased], since no 3.0 section exists yet
