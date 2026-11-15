---
cairn: delta
change: machine-output-contract
---

## ADDED Requirements

### Requirement: The JSON contract is described and versioned
`json-schema` SHALL describe the failure object under the key `himalaya-error`, its `code` typed by the code vocabulary. Every generated schema SHALL carry an `$id` naming the contract version, the major version of Himalaya. Every registered payload SHALL be a JSON object, checked by a test. Piped `--json` output SHALL print one value per line.
