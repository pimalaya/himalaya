---
cairn: log
change: machine-output-contract
landed: 2026-10-27
---

# Adopted the shared machine output contract

The coded-error mechanism moved to pimalaya-cli (`55b6693`, pinned until 0.3.0); src/error.rs keeps only the `ErrorCode` vocabulary, and main.rs evaluates through `ErrorReport::<ErrorCode>::eval_coded`. The JSON error object prints exactly as before.

`json-schema` gained `himalaya-error`, and each schema now carries `$id` = `https://pimalaya.org/schemas/himalaya/v2/<key>.json`. A test asserts every payload root is an object; all already were, so no output moved. Piped `--json` is now one value per line, from the pimalaya-cli printer.

Verified: 296 binary tests pass with all features, clippy clean on all features, `--no-default-features` builds.

Spec updated: commands (ADDED: The JSON contract is described and versioned).
