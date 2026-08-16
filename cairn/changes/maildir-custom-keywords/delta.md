---
cairn: change
change: maildir-custom-keywords
---

# Delta

## ADDED Requirements

### Requirement: Maildir surfaces custom keywords on demand
The Maildir backend SHALL surface custom (non-IANA) keywords on read when told which convention the mailbox uses, so a keyword written by dovecot, mbsync, OfflineIMAP, mutt or notmuch matches a `flag <name>` search as it does on the network backends. `maildir.dovecot-keywords` SHALL resolve the lowercase info-section slot letters through the `dovecot-keywords` sidecar at the Maildir root, and `maildir.keywords-header` SHALL read keywords from `X-Keywords` (comma-separated) or `X-Label` (space-separated). Both default to off, and with both off the flag set SHALL be exactly the six standard info-section letters as before.

A sidecar that is absent, unreadable or disabled SHALL yield no keywords rather than fail the listing, since a mailbox without one is the normal case rather than an error.

## MODIFIED Requirements

### Requirement: Local storage backends
Maildir, m2dir and pimdir SHALL adapt io-maildir, io-m2dir and io-pimdir. Maildir stores added messages under `cur/` and SHALL read an entry's flags through io-maildir rather than parsing the filename itself, so the meaning of a Maildir name is decided in one place: the info section is delimited by the `:2,` marker, which matters because entry listing covers `new/` as well as `cur/` and a name in `new/` has no info section to read. m2dir is content-addressed with no native copy or move, so those are a get plus a store (plus a delete for move), and its flags live in a `.meta/<id>.flags` sidecar. m2dir mailbox `rename` and message `copy`/`move` remain unavailable until io-m2dir supports them. pimdir is an offline cache the sync engine (io-replica + io-pimdir) populates: reads project the store's shared items (io-pimdir's client read API) from the stored `v: 1` meta without body reads, and writes are staged io-replica `mutate` mutations a later sync propagates rather than direct SQL.

## REMOVED Requirements
