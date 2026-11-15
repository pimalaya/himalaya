---
cairn: log
change: the-smaller-live-findings
landed: 2026-11-15
---

# The smaller live findings

Task H32 of the family's cli-alignment-2026-11-15 plan, the low findings of live testing on 2026-11-15 (part2-pimdir.md, imap-smtp-icloud.md, jmap-fastmail.md).

## What landed

- src/pimdir/performer.rs: the four intents Himalaya queues are the grammar's possible values, any other refused `usage-invalid`, exit 2; test `an_unknown_capability_is_refused_naming_the_known_ones`.
- src/pimdir/backend.rs, src/pimdir/flag/mod.rs: `pimdir flag add|set|remove` print `warnings`, each partial support of the change once; test `a_keyword_kept_in_part_is_queued_with_its_warning`. Refusing Gmail keywords outright is neverest's declaration to make (`mail.flags.keywords` `none`).
- src/shared/client.rs `logins`, src/account/context.rs, src/shared/message/builder.rs, reply.rs: a reply to all leaves out the reply's `From` and the account's logins (IMAP and SMTP SASL user names holding an `@`); tests `reply_all_leaves_out_the_reply_s_from_and_the_logins` and `the_logins_are_the_sasl_names_holding_an_address`.
- src/jmap/backend.rs: `get_message` reads `mailboxIds` with `blobId` and refuses, `message-not-found`, an email the mailbox does not hold; test `a_read_of_an_email_another_mailbox_holds_is_not_found`.
- pimdir `message read` of an unknown id was already `message-not-found` (a-server-refusal-codes-the-command).
- Left open: Gmail and Graph reads stay unscoped; `pimdir tag` still logs a partial support alone, io-pimdir's tag staging not answering it.

## Capabilities moved

- [backends](../spec/backends.md): *pimdir performer takes the intents Himalaya queues* and *A JMAP read is scoped to the mailbox given* added; *pimdir writes name their queue row* and *pimdir stages flags by their raw spelling* carry `warnings`.
- [commands](../spec/commands.md): *Reply to all recipients* leaves out the user's own addresses, with a scenario.
