---
cairn: change
id: the-sample-is-tested-and-shipped
status: landed
created: 2026-11-11
---

# The sample is tested and shipped

## Why

The first-run welcome (the-first-run-welcome) creates a configuration from config.sample.toml, copying it from share/doc/himalaya/ when one ships beside the binary and downloading it otherwise. No Himalaya package ships it, so every welcome downloads, and the file it hands over is 515 lines of prose-commented paragraphs that nothing checks: a renamed key, a dropped variant or a stale path in it goes unnoticed (its pimdir root already names a layout neverest left, and its `downloads-dir` fallback a directory Himalaya does not use first). The default `smtp.server` and `imap.sasl.plain.password.raw` lines are active, so the file as copied is an account pointed at `example.com` with the password `***`.

Ortie ships its sample as one flat account with a section per provider, parsed by its test suite once per variant (ortie 2026-11-10-providers-in-the-sample, 2026-11-10-sample-shipped-in-share), and pimalaya-config now carries the variant helper, `sample::variants` (feature `sample`).

## What

- config.sample.toml in Ortie's shape: one `[accounts.example]` table with dotted keys and no header comment; the options every account shares (identity, signature, downloads, sent copy, posting styles, envelope listing, mailbox aliases, proxy), the default active and the alternatives commented, each line ending in one tab-aligned comment, every password command `exit 127`; then one commented section per backend or provider, each a working account once uncommented, its own options doubly commented: IMAP and SMTP, ManageSieve, JMAP, Gmail, Microsoft Graph, Microsoft IMAP and SMTP, Fastmail, Maildir, m2dir, mbox, pimdir. OAuth 2.0 tokens come from `ortie token show -a example`, matching Ortie's sample sections.
- src/config.rs: a test parsing every variant of the sample through `Config::load`, the loader `-c` uses, and resolving `example` as the default account; pimalaya-config with `sample` as a dev-dependency.
- default.nix installs config.sample.toml as share/doc/himalaya/config.sample.toml, which the release archives carry with the build's share/ (pimalaya/nix's `build-with-share` job, already called by .github/workflows/releases.yml); install.sh copies it under its prefix when the archive holds it.
- README Configuration: the sample and the welcome, then the paths, the top-level fallback, the SASL and JMAP authentication shapes the sample names in comments. CHANGELOG.

## Scope / non-goals

The SASL mechanism and the JMAP authentication are enums keyed by name (`imap.sasl.plain`, `jmap.auth.basic`), which the variant helper cannot swap one line at a time: each section shows the one its provider uses as code, and the same-shaped others are named in comments and in the README. No configuration type changes.
