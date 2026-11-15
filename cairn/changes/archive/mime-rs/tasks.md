---
cairn: tasks
change: mime-rs
---

# Tasks

- [x] Cargo.toml: mime-rs by git rev; mail-parser and mail-builder removed from dependencies and features
- [x] part.rs: the walk, ids, sections, bounds, role, type, name, Content-ID, charset, method, bytes, text over the mime-rs tree
- [x] header.rs: raw field bytes, base64 and charsets through mime-rs
- [x] read.rs: headers, addresses, date, ids, the plain view
- [x] attachment list and download, message parse
- [x] builder.rs: MimeBuilder, Date and Message-ID written, reply and forward reads, carried attachments
- [x] handler.rs template, submission envelope, search evaluator, IMAP Message-ID lookup, Maildir, m2dir and mbox envelopes
- [x] Corpus comparison of `message parse --json` before and after, differences reviewed
- [x] Tests, reduced-feature builds, clippy, fmt
- [x] CHANGELOG; spec fold where behaviour moved; log; landed
