---
cairn: change
id: managesieve-support
status: landed
created: 2026-08-24
---

# ManageSieve support

Himalaya accounts that use Dovecot or another RFC 5804 ManageSieve server
should be able to inspect and maintain their server-side Sieve scripts
through the same account-scoped CLI style as the existing protocol modules.

This change adds an optional `sieve` account transport and a protocol module
covering capability discovery, script listing and retrieval, upload and
validation, activation/deactivation, deletion, and a diagnostic raw command.
The wire framing stays isolated from the CLI so literal strings, CRLFs, and
server status responses can be tested with a local fake server first.
