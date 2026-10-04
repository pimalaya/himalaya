---
cairn: log
change: json-error-code
landed: 2026-10-04
---

# A JSON error names a stable code

`main` prints failures through `error::eval`, which flattens pimalaya-cli's `ErrorReport` and adds `code` when the error chain carries a `CodedError`. The first code is `body-pending`, raised by the pimdir backend for a message whose body is not local yet, so `message read` and `attachment` commands under `--json` print `{"code":"body-pending","error":…,"sources":…,"backtrace":…}`. A failure without a code prints as before; the text output is unchanged.

Capabilities moved: **commands** (a JSON error carries a stable code), **backends** (a pimdir body not local fails with `body-pending`).
