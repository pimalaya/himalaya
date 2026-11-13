---
cairn: change
id: the-live-campaign-fixes
status: landed
created: 2026-11-12
---

# The live campaign fixes

## Why

On 2026-11-12 a live test campaign ran Himalaya against the test accounts (Fastmail over IMAP, SMTP and JMAP, iCloud, Gmail, Microsoft Graph) and the local stores (Maildir, m2dir, mbox). It found commands reporting writes they did not make or made beyond the mailbox given (a JMAP `message delete -m trash` destroying an email other mailboxes held, ids of another Gmail label or JMAP mailbox counted as found), a mistyped path stored as a message, a search language splitting a pattern on `order by` and widened by the Gmail operators a pattern held, iCloud's `SORT` ignoring its keys, IMAP roles unread although Fastmail and iCloud send them, and many smaller output and refusal gaps.

## What

- Writes act on what the mailbox given holds: ids are required, deduplicated and looked up before the first write on the local stores, scoped by `mailboxIds` on JMAP and by label on Gmail, and a removal from the trash counts what the trash held.
- IMAP reads the special-use roles a plain `LIST` carries and resolves role words through them (imap-special-use-aliases stays active for `RETURN (SPECIAL-USE)`); iCloud sorts client-side; the IMAP search pushes negations onto single keys.
- The search grammar parses `order by` as words, refuses an unterminated quote and an empty sort, and reports once over the whole query; the local sort follows RFC 5256; a Gmail pattern is a literal.
- Mailbox writes validate NAME, refuse an existing target and a mailbox holding others, and print the full name and the listed id.
- Options naming flags take one value per occurrence, and a raw message positional naming no file must open on a header field.
- The composers write a final line break, keep an attached message unencoded, carry a forward's `Content-Type` parameters, list the attachments in the JSON template, and refuse `--no-save` without `--send`.
- Gmail files a sent copy by labelling the sent message, threads answers, serializes the `full` payload; JMAP files a duplicate import's existing email; vacation set without an option is a usage error.
- Library fixes patched by path: io-maildir, io-jmap, io-imap, io-gmail, mime-rs, pimalaya-cli.

## Scope / non-goals

The Microsoft Graph batch of the campaign (MS F1 to F3, the trash scope) is still in progress and folds on its own. The display of nested JMAP mailboxes by path, clearing a JMAP auto-reply field and the spelling of dates stay open design questions. The library path patches go once each library is released.
