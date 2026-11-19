---
cairn: delta
change: the-releases-of-2026-11-19
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Raw passthrough is byte-verbatim
`imap raw` prints each response from the octets io-imap read: a UTF-8 one as text, parsed, one that is not as an `undecoded` event `{base64}`; a tagged one that is not UTF-8 exits 3; the text form prints it lossily. The lossy framing and its single `unparsed` block are removed. Scenario *An 8-bit literal* added.

### Requirement: The Gmail and Graph escape hatches are requests
A URL off the API is refused before the session opens, coded `request-origin`, exit 1, where it was refused with the command line, exit 2.

### Requirement: Downloaded attachments never replace an existing entry
The path component rule is pimalaya-cli's `sanitize::path_component`, the trimming and leading dots dropped first.

### Requirement: pimdir reads one account
A configured account on a store holding no collection is read as named; the accounts and kinds are read through io-pimdir. Scenario *A configured account before the first sync* added.

### Requirement: A pimdir search says what it did not cover
The counts a term reads come from `PimdirSearchCoverage::unread_counted`, the scope from `PimdirQuery::scoped`.

The code table gains `request-origin`; `store-account-unknown` is narrowed to a store holding collections; the `envelope search` unread pushdown reads `PimdirReader::overlay_quiet`.

## REMOVED Requirements
