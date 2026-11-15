---
cairn: delta
change: the-graph-campaign-fixes
---

# Delta

The requirements below are folded as they now read in the spec; each entry names its capability and what changed.

## ADDED Requirements

### Requirement: Graph batches four requests at a time
(backends) At most four sub-requests to a `$batch`; a 429, 500, 502, 503 or 504 sent again up to three times after the longest `Retry-After` of its round, 10 seconds at most.

### Requirement: Graph folders are named by path
(backends) Every folder but the hidden ones listed, every page read, a nested one by its `/`-joined path; a value matching no listed folder taken as a well-known role, looked up when it may name a folder, else refused with ``Mailbox `X` not found``; io-msgraph encodes each id as one path segment.

### Requirement: Graph writes are scoped to the mailbox given
(backends) Flags, copy, move and delete read each id's `parentFolderId` first; an id of another folder, unknown or malformed counts as not found; a move onto its own mailbox changes nothing; `flag set` replaces the categories.

### Requirement: Graph envelopes
(backends) An unpaged listing follows every paging link at 1000 a page; the date is `sentDateTime`, else `receivedDateTime`.

## MODIFIED Requirements

### Requirement: Mailbox role
(backends) Graph resolves its roles through the batching requirement, a 404 skipped and any other failure failing the command; an unknown Graph value is refused unless it may name an unlisted folder.

### Requirement: A command given message ids says how many it found
(commands) Microsoft Graph joins JMAP and Gmail as account-wide ids scoped to the mailbox, its trash removal no longer counting every id given; the raw local store commands require their ids.

### Requirement: Unified mailbox writes
(commands) Graph joins the levels with `/`; an IMAP `--parent` marked `\Noinferiors` is refused.

### Requirement: Saving a sent message follows the send
(commands) Graph files every sent message in Sent Items itself, `submit.copy` `partial`, a save asked of a send refused through `message.add`.

### Requirement: Listings resume from their cursor
(commands) `msgraph mail-folders list` and `child-folders` page by token; a next link leading back gives no page; `Next link:` in text; `--search` quoted; `--count` printed.

### Requirement: Text output is plain text
(commands) `msgraph messages list` dates by `sentDateTime` and prints `Count:` and `Next link:`; the Graph folder listings are paged by token; `maildir flag list` in ASCII order.

### Requirement: Data commands serialize their data
(commands) `gmail messages get` prints `payload` `null` under the formats other than `full`.

### Requirement: Per-backend translation
(search) Each word of a Graph pattern is a quoted KQL phrase.

### Requirement: A refused pimdir query names its fault
(search) The plain report carries one `Error:`.

## REMOVED Requirements

None.
