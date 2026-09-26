---
cairn: tasks
change: json-keys-are-camel-case
---

# Tasks

Held until the 3.0 cycle opens.

- [ ] Put `#[serde(rename_all = "camelCase")]` on every type registered in src/json_schema.rs, dropping the twenty-six kebab-case renames it replaces
- [ ] Leave the provider passthrough renames (`@odata.nextLink`, `nextPageToken`) and the transparent newtypes over io-gmail and io-msgraph resources untouched
- [ ] Leave src/config.rs kebab-case: TOML keys are not `--json` keys
- [ ] Rename the forty-two registered types that are not yet `*Output`, the reference being `GmailProfileOutput`
- [ ] Update the doc comments naming a key spelling, starting with `GmailProfileOutput`
- [ ] Regenerate the JSON Schemas and check no key kept a hyphen
- [ ] CHANGELOG under `Changed`, as breaking, in the 3.0 section
