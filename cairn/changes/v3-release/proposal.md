---
cairn: change
id: v3-release
status: active
created: 2026-11-02
---

# Cut the v3.0.0 release

## Why

Master breaks the v2 contract already: every `--json` key is camelCase, `message read --json` is a designed view, and Gmail and Graph refuse what they used to pretend to do (`capability-missing`). The 2027 plan's first task, a REPL and an MCP server over the unified API, landed on 2026-11-02 together with the capability union the MCP server's tool list rests on. A 2.2.2 was considered and dropped: the breaking changes are on master, so the next release is a major.

## What

- **The JSON shapes are reviewed with the maintainer before anything is cut**, every schema `json-schema` writes, since v3 fixes them for the whole major.
- **The git dependencies are released last**, by the maintainer: the six `[patch.crates-io]` entries published, the requirements raised to them, the patch table removed and deny.toml tightened. pimalaya-cli and pimalaya-config are being prepared as 0.4 in their checkouts, which Himalaya patches by path; io-http, io-jmap, io-msgraph and io-pimdir are pinned by git rev and need releases before Himalaya can drop their patches. io-imap (0.8) and ical-rs (no longer a dependency) have left the table.
- **Himalaya releases with the family**: Ortie and Himalaya release together with pimalaya-cli and pimalaya-config 0.4, once the owner validates the whole family.
- **Then the release itself**: the version bumped to 3.0.0, which moves every schema `$id` to `v3` (pimalaya-cli derives it from the major), the CHANGELOG `[Unreleased]` dated, MIGRATION.md's v2 to v3 section final, the provider reports under cairn/spec/testing run on the test accounts, tag and publish.

v2-release (2026-07-25), whose release shipped as 2.0.0 on 2026-07-26, is closed into this one: its open tasks were the same release plumbing.
