---
cairn: log
change: graph-listings-page-by-the-next-link
landed: 2026-11-15
---

# Graph listings page by the next link

Ruling D18 of the family's cli-alignment-2026-11-15 plan: a protocol listing prints the provider's cursor as `nextPage` and takes it back as is. The Graph listings printed a `$skip` worked out from the next link and refused the `$search` link they printed.

## What landed

- src/msgraph/messages/list.rs, src/msgraph/mail_folders/list.rs and child_folders.rs: `nextPage` is Graph's next link as Graph gives it, `--page-token` follows it through io-msgraph's `page_get` (refusing a link off Graph's origin before any request), `null` when it leads back to the page asked; the `Next link:` text line is gone. Test `a_listing_resumes_from_the_next_link_it_printed`.
- src/msgraph/attachments/list.rs: every next link followed.
- The Gmail and JMAP listings checked, conformant and unchanged.

## Capabilities moved

- [commands](../spec/commands.md): *Listings resume from their cursor* states the Graph and Gmail cursors, with the scenario *A Graph search resumed from its next link*; *Text output is plain text* drops `Next link:`; *JSON keys are camelCase* names the cursor as the one provider key lifted.
