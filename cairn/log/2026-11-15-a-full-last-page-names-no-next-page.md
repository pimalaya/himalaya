---
cairn: log
change: a-full-last-page-names-no-next-page
landed: 2026-11-15
---

# A full last page names no next page

Task H1 of the family's cli-alignment-2026-11-15 plan (finding F10). `envelope list` and `envelope search` printed a next page whenever a page came back full, so an exactly full last page pointed at an empty one: on a Maildir of 2 messages, `-s 2` printed `"nextPage":"2","total":2` and `-s 1 -p 2` printed `"nextPage":"3"`. Cardamum and Calendula print `null` there, as pimalaya-cli's cairn/spec/output.md asks once the listing is complete.

## What landed

- src/email/envelope.rs: `EnvelopePage`, a page's envelopes and `more`, whether a page follows it; `page_follows` tells it from the count a backend holds.
- Each backend's `list_envelopes_counted` and `search_envelopes` answer an `EnvelopePage` (the `list_envelopes` wrappers keep their envelopes-only answer): Maildir and mbox count the envelopes they list or match; IMAP reads the `EXISTS` of its `SELECT` for a listing and the UIDs its `SORT` answers, once deduplicated, for a search; JMAP reads the `total` of `Email/query`, a search the server pages now asking `calculateTotal` in the same call, a full page standing for a server computing none, and a search paged client-side counts what it kept; pimdir reads `count_items` for a listing (it overlays the queue as the walk does) and walks one hit past the page for a search in the store's order; Gmail reads the next page token of the page's `messages.list`; Graph the next link of the page's answer.
- src/shared/envelope/list.rs and search.rs: `nextPage` is the next page number when a page follows, `null` otherwise, the text `Next page:` line and `, the last` following it.
- tests/local_stores.rs `a_page_reaching_the_last_message_names_no_next_page`: Maildir and mbox, `envelope list` and `envelope search`, JSON and text, through the binary; it fails on the old rule (`-s 2` on two messages printed `"nextPage":"2"`). Unit tests: IMAP `a_page_reaching_the_last_message_has_no_next` on the scripted server, JMAP `a_page_reaching_the_query_total_has_no_next`, the page signal in `a_uid_the_server_sorts_twice_is_listed_once` and `a_page_counts_the_whole_mailbox`, and pimdir's `assert_paged_reads_agree` comparing `more` across the listing, the pushed and the scanned search, over a quiet and a busy queue, with two pages ending exactly on the 300 seeded mails.
- Left as they are: `jmap email query`, `jmap mailbox query` and `jmap submission query` print `Next page:` in text while a page comes back full, as cairn/spec/commands.md states for them, and print no `nextPage` key.

## Capabilities moved

- [commands](../spec/commands.md): *Listings resume from their cursor* says when a page follows and gains the scenario of an exactly full last page.
- [backends](../spec/backends.md): *pimdir reads only the page an envelope listing needs*, a search reading one hit past the page.
