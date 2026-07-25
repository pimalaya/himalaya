# compose / reply / forward — test report

Unlike the per-provider reports, the built-in `compose`, `reply` and
`forward` composers are **provider-independent** and were validated with
**unit tests**, no account or network required.

- himalaya: `v2.0.0-alpha.1` (working tree)
- date: 2026-07-18
- tests: `src/shared/message/builder.rs` (`mod tests`), run with
  `cargo test builder`

## Why no provider is needed

All three subcommands collapse into one pure assembler,
`builder::build(args, source)`:

- `BuilderArgs` is plain in-memory data (from/to/cc/bcc, subject, body,
  attachments, signature).
- `SourceArgs.raw` — the message being replied to / forwarded — is just
  `&[u8]`. Production *fetches* those bytes from a backend, but the
  assembler only reads them, so any raw RFC 5322 fixture drives it.
- The output is raw RFC 5322 bytes, re-parsable with `mail_parser` to
  assert on.

The only provider-dependent parts are *outside* the builder — fetching
the source (`reply`/`forward`) and sending / `--save` — and those are
covered by the per-provider reports (IMAP/JMAP/Gmail/Graph send).

## Coverage (10 tests)

| Area | Asserted |
| --- | --- |
| compose | from/to/cc headers + text body; no `In-Reply-To` |
| reply subject | single `Re:` prefix; existing `Re:` not doubled |
| reply recipients | defaults to source `From`; explicit `--to` overrides |
| reply threading | `In-Reply-To` = source id; `References` appends it |
| forward | `Fwd:` prefix; **no** `In-Reply-To`; source quoted |
| quoting | `>`-prefixed lines, headline, top/bottom posting style, `-- ` signature |
| helpers | `has_prefix`, `compute_references` (References / In-Reply-To / neither), `push_msg_id` |

## Findings

- **Bug — `has_prefix` matched on letters only, dropping real prefixes.
  FIXED.** It trimmed the colon off the prefix before comparing, so any
  subject starting with the letters `Re`/`Fwd` (`Ready to ship`,
  `Review`, `Forwarding note`) was treated as already prefixed, and the
  reply/forward subject lost its `Re:` / `Fwd:`. It now compares against
  the full `Re:` / `Fwd:` (colon kept), still case-insensitively.
  Regression test:
  `reply_prefixes_subject_that_merely_starts_with_re_letters`.

### Observation (not changed)

- `build` sets a `References` header on **forwards** as well as replies
  (only `In-Reply-To` is reply-only). Threading a forward is unusual but
  not wrong; left as-is.

## Verdict

compose / reply / forward message assembly is **covered by fast,
provider-free unit tests**, and the one real bug they surfaced
(`has_prefix`) is fixed. The send / save / fetch-source paths remain
covered by the provider reports.
