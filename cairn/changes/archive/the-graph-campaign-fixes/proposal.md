---
cairn: change
id: the-graph-campaign-fixes
status: landed
created: 2026-11-13
---

# The Graph campaign fixes

## Why

The 2026-11-12 live campaign against the Microsoft Graph test account, left out of the-live-campaign-fixes, found roles dropping at random (the well-known folder `$batch` throttled past four concurrent requests and its failures skipped), nested folders missing from `mailbox list` and a folder path sent raw into the request path, `message delete -m trash` hard-deleting a message of another folder and every id-taking write acting on any id of the account, `flag set` unable to clear the categories, `envelope list -s 0` listing 10 messages, envelopes dated by arrival rather than by their `Date:`, KQL syntax inside a pattern widening a search, and gaps in the `msgraph` listings (`--search` unquoted, `--count` printing nothing, folder listings unpaged, `--top 0` looping on its own page). A few late fixes of the other backends landed after the first fold.

## What

- `$batch` goes four sub-requests at a time, a throttled or unavailable one sent again after its `Retry-After`; a role lookup failing other than 404 fails the command.
- Folders are listed whole, nested ones by path; an unknown mailbox value is refused unless it may name a folder Graph does not list, which is looked up; io-msgraph percent-encodes every id it puts in a path.
- The id-taking writes check each id's `parentFolderId` first, as JMAP reads `mailboxIds`; a move onto its own mailbox changes nothing; `flag set` clears the categories it does not name, and `msgraph messages update --no-categories` clears them all.
- An unpaged envelope listing follows every page; an envelope is dated by `sentDateTime`; each pattern word is a quoted KQL phrase.
- `msgraph messages list` quotes `--search`, prints `--count`, prints a whole next link as `Next link:` and stops at a link leading back; `mail-folders list` and `child-folders` take `--page-token`.
- Graph declares `submit.copy` `partial`: it files every sent message in Sent Items itself.
- Late fixes: IMAP refuses a `\Noinferiors` parent; the raw local store commands require their ids; `maildir flag list` in ASCII order; Gmail `messages get` prints `payload: null`; a pimdir search refusal prints one `Error:`.
- Library fixes patched by path: io-msgraph (path segments, categories tri-state, messages list count), and the io-imap and io-jmap follow-ups.

## Scope / non-goals

Left open: the Graph cosmetics of the campaign (the folder move and copy wording, the `attachments create` path, `attachments get` MIME type and `-o` overwrite, an HTML 5xx body in errors), and retrying idempotent GETs in io-msgraph's `MsgraphClientStd`, proposed and not implemented. A Graph send asked to save a copy is refused through `message.add` before the code filing it in Sent Items is reached; see the log.
