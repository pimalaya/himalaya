---
cairn: log
change: the-graph-campaign-fixes
landed: 2026-11-13
---

# The Graph campaign fixes

The Microsoft Graph batch of the 2026-11-12 live campaign landed, with the late fixes of the other backends. Graph batches four sub-requests at a time and retries the throttled ones, so its roles no longer drop at random, and a role lookup failing other than 404 fails the command; folders are listed whole and nested ones named by path; a mailbox value naming no folder is refused rather than sent into a request path; the id-taking writes check each id's `parentFolderId` first, so `message delete -m trash` no longer hard-deletes a message of another folder and a write no longer stops part-way on an unknown id; `flag set` clears the categories it does not name and `msgraph messages update --no-categories` clears them all; an unpaged envelope listing follows every page; envelopes are dated by `sentDateTime`; a pattern word that is KQL syntax is matched as a word; `msgraph messages list` quotes `--search`, prints `--count`, prints a whole next link as `Next link:` and stops at `--top 0`; the folder listings take `--page-token`; Graph declares `submit.copy` `partial`. Late: IMAP refuses a `\Noinferiors` parent, the raw Maildir, m2dir and mbox commands require their ids, `maildir flag list` prints the letters in ASCII order, `gmail messages get` prints `payload: null`, a pimdir search refusal prints one `Error:`.

Capabilities moved: backends ("Graph batches four requests at a time", "Graph folders are named by path", "Graph writes are scoped to the mailbox given" and "Graph envelopes" added; "Mailbox role" modified), commands ("A command given message ids says how many it found", "Unified mailbox writes", "Saving a sent message follows the send", "Listings resume from their cursor", "Text output is plain text", "Data commands serialize their data"), search ("Per-backend translation", "A refused pimdir query names its fault").

io-msgraph rides on its path patch in Cargo.toml, uncommitted in its repository and needing a release before the patch goes: ids percent-encoded as one path segment (`path_segment`), `MsgraphMessage::categories` a tri-state `MsgraphField` so a PATCH can clear them (a breaking change of a public field type), and the `@odata.count` of a messages listing kept as `count`. Two follow-ups went into the libraries already patched by path: io-imap reads a tagged completion holding 8-bit text (its log 2026-11-12-a-completion-holding-8bit-text), and io-jmap reads a `VacationResponse/set` echoing part of the object, which Fastmail does (its log 2026-11-13-vacation-partial-echo).

The Graph send filing a copy in Sent Items and reporting `saved: true` is unreachable from the unified commands: `message.add` is `none` on Graph, so any save asked of a send is refused before it, and a send without one reports `saved: false`. The campaign's Graph cosmetics and an io-msgraph retry of idempotent GETs stay open.
