---
cairn: change
id: the-sample-lists-values-in-brackets
status: landed
created: 2026-11-14
---

# The sample lists values in brackets

## Why

config.sample.toml is the only way into a configuration now that the wizard is gone, and it read poorly: 196 lines, 79 of them commented twice (`##`), one line per value of a key (`mbox.format` four times), the TLS and proxy keys repeated per backend (`# as imap.tls`). Its test uncommented every `##` line alone on top of its section, so a key needing a sibling could not be written as two lines (#809 used an inline table for `tls.client`).

## What

- Every line is commented but `default = true`, so the sample as written takes no decision for the user: a key with a default shows it, a key without one an example, the blocks of every backend included.
- The other values of a key go at the end of its comment, in brackets, as clap lists them: `#message.reply.posting-style = "top"  # where a reply sits around the quote [bottom, none]`. Brackets hold the values of an enumeration or an example (`user-id`), never a boolean's, nor free text (a mailbox name for `save-copy`, an empty `alpn`, the other forms of a `server`), said in the comment instead.
- A structural variant is one more commented line setting the same field (`password.raw` after `password.command`). A paragraph setting keys under the same two-segment table as the paragraph above replaces it (`imap.sasl.xoauth2.*` for `imap.sasl.plain.*`, `jmap.auth.bearer` for `jmap.auth.basic`).
- The TLS, proxy and ALPN keys are written once, under IMAP, a prose line saying the other network backends take them; `imap.id.fields` takes one line per field. No `##` line. 111 lines.
- No tool is named: a token command is a failing `exit 127` placeholder like a password command, and `pimdir.root` a placeholder path, any OAuth broker or sync engine filling them.
- The test parses, through the loader `-c` uses, resolving `example` each time: the sample as written; then once per rank with every line uncommented, each field set to its value of that rank, the lines setting it in their order, each followed by its bracketed values, the last when it has fewer; a replacing paragraph stays out at rank 1 and takes the place of the one above from rank 2. Four variants; none built per line. The helper lives in the test; pimalaya-config's `sample::variants`, which builds per-line variants, can take it once the family adopts the shape.
- Spec: the sample requirement of cairn/spec/config.md rewritten to this shape and this test.

## Scope / non-goals

No configuration key, default or behaviour changes. As written, the sample is an account with no backend, as before; the first-run welcome copies it as before.
