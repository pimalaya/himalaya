---
cairn: log
change: the-sample-lists-values-in-brackets
landed: 2026-11-14
---

# The sample lists values in brackets

config.sample.toml went from 196 lines, 79 of them commented twice, to 111, every line commented but `default = true` so the sample as written takes no decision for the user: each key at its default or an example value, the other values of an enumeration at the end of its comment in brackets (`[bottom, none]`), a structural variant one more commented line (`password.raw`) or a paragraph replacing the one above (XOAUTH2 for Microsoft 365, a bearer token for Fastmail), the TLS, proxy and ALPN keys written once under IMAP, `imap.id.fields` one line per field. The `server` lines keep one value, their other forms said in the comment. No tool is named: token commands are `exit 127` placeholders and `pimdir.root` a placeholder path.

The sample test no longer builds a variant per line nor uses pimalaya-config's `sample::variants`: it parses the sample as written, then once per rank (four today) with every line uncommented, each field at its value of that rank, its lines in order each followed by its bracketed values; a paragraph setting keys under a two-segment table the paragraph above sets replaces it from rank 2. Checked by breaking the sample six ways on a copy (a bracket value of `posting-style` and of `mbox.format`, a commented default key, a key without default, a key of a replacing paragraph, the second `mailbox.alias.sent`): each failed the test. The `sample` dev-dependency feature of pimalaya-config is gone with it.

Spec: config ("One sample configuration, with a section per backend" rewritten).
