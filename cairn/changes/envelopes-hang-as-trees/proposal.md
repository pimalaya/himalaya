---
cairn: change
id: envelopes-hang-as-trees
status: landed
created: 2026-11-12
---

# Envelopes hang as trees

## Why

The owner decided on 2026-11-12 that Himalaya is a low-level CLI: plain text for a human to glance at, JSON for scripts and frontends, daily mail being a frontend's job. Printed as `Records`, an envelope took seven lines, the subject (what a reader scans for) came last, and three labels named values whose shape already says what they are. A day-grouped layout was rejected: grouping by day forces one timezone on every date and splits `datetime-fmt` in two. The chosen design hangs the envelopes from the mailbox the way `tree` prints a directory, subject first, and a thread hangs its replies the same way, so both views read alike.

`envelope.list.datetime-relative` (#510, v2.2.0) is a reader's comfort that a frontend gives; the low-level CLI prints the date exactly as `datetime-fmt` configures it.

## What

- `envelope list|search` print one tree rooted at the mailbox listed (its alias resolved): per envelope, the subject on its branch, then the date, the id with its state words and the senders (`to ` and the recipients under `--recipient`), two spaces apart. The size no longer prints; `--json` keeps it.
- `imap thread` prints one tree per thread, each reply under the message it answers, the envelope of every message fetched (`FLAGS` and `ENVELOPE`), where it printed `Thread N:` headings, bare ids and subjects, unsanitized, and a `Found N thread(s)` footer. Its JSON entries gain `from`, `date` and `flags`.
- `pimdir thread` hangs each message under the earlier one its `In-Reply-To` names, with `in ` and its mailboxes after the state.
- `envelope.list.datetime-relative` is removed and refused as an unknown key.
- MIGRATION.md gives the jq recipes for the greps the one-line table allowed; README.md says what the CLI is and where daily mail goes.

## Scope / non-goals

The other wide lists stay `Records`, `pimdir search --threads` included, being a list of threads rather than of messages. The tree's format takes no configuration. io-pimdir's own `pimdir thread` binary keeps its records.
