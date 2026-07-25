# Gmail over IMAP on Google — test report

Shared cross-protocol commands (and IMAP-specific spot-checks) on the
IMAP backend against **Gmail** (Google). Distinct from the Gmail *REST*
backend in [gmail.md](gmail.md) / [gmail-specific.md](gmail-specific.md):
this is `imaps://imap.gmail.com` with an app password. Companion to the
provider-agnostic IMAP surface in
[imap-smtp-specific-fastmail.md](imap-smtp-specific-fastmail.md). Method:
[provider-test-plan.md](provider-test-plan.md).

- himalaya: `v2.0.0-alpha.1 +imap +rustls-ring` (working tree)
- account: `google` — IMAP-only (`imaps://imap.gmail.com`, SASL PLAIN
  with an app password); **no `[smtp]` block**, so `message send` bails
  by design
- date: 2026-07-18
- server: Gmail IMAP; `/` hierarchy separator; labels exposed as folders
  (`INBOX`, `[Gmail]/All Mail`, `[Gmail]/Sent Mail`, …); capabilities
  `IMAP4rev1 UIDPLUS MOVE CONDSTORE IDLE SPECIAL-USE X-GM-EXT-1`

> Test account (`pimalaya.org@gmail.com`), but the golden rule was still
> applied: all work happened in throwaway labels `Himalaya-Test` /
> `Himalaya-Test2`. Cleanup accounts for Gmail's model (a message
> survives label removal in `All Mail`): the test messages were moved to
> `[Gmail]/Trash`, flagged `\Deleted` and expunged, then the labels
> deleted. Final state verified: no test residue in INBOX / All Mail /
> Trash / Sent, and the pre-existing structure (incl. the `This is a
> làbel` label) intact.

## Results

| Command | Variants | Result |
| --- | --- | --- |
| `mailbox list` | labels + `[Gmail]/*` | ✅ |
| `envelope list` | in fake label, `--json` | ✅ (UIDs) |
| `envelope search` | `-- subject <term>` | ✅ (strict client-side filter) |
| `message add` | into fake label | ✅ |
| `message read` | pretty | ✅ |
| `flag add/set/remove` | seen/flagged(→Starred)/answered | ✅ |
| `message copy` / `move` | `--from`/`--to`, counts | ✅ (COPYUID/MOVE) |
| `attachment list` / `download` | list, `-d` | ✅ |
| `message send` | no `[smtp]` | ⚪ bails cleanly |
| `imap create` / `delete` | fake labels | ✅ |
| `imap select` / `status` / `flags` | fake label | ✅ |
| `imap fetch` | `--envelope --flags` | ✅ |
| `imap store` / `expunge` | `\Deleted` + expunge (Trash) | ✅ |
| `imap search` | `--subject` (server-side) | ✅ (loose, see H3) |

## Findings

No himalaya bugs. Everything works; the notes below are Gmail-IMAP
semantics worth knowing (and were the basis of the cleanup).

- **H1 — messages persist in `All Mail` after a label is removed.**
  Gmail folders are label *views*; `imap delete <label>` removes the
  label but not the message, which remains in `[Gmail]/All Mail`.
  Deleting test data means trashing it (move to `[Gmail]/Trash`, then
  `\Deleted` + expunge) — not just deleting the label.
- **H2 — the shared `flag` command has no `deleted` value** (only
  `seen`, `answered`, `flagged`, `draft`). IMAP deletion goes through the
  protocol-specific `imap store <uid-set> --action add --flag '\Deleted'`
  then `imap expunge`, or a move to Trash. Deliberate: the shared API
  keeps the destructive `\Deleted`/expunge dance out of the friendly
  surface. Worth stating in the docs.
- **H3 — Gmail's server-side `imap search --subject` is tokenized and
  loose.** `--subject "Gmail-IMAP"` matched an extra Trash message that
  did *not* contain that string literally (Gmail split it into `gmail`
  + `imap` word tokens). The shared `envelope search` (client-side
  substring) is stricter. For scripted cleanup, prefer the shared search
  or verify each hit before deleting.
- **H4 — `imap store <SEQUENCE>` takes one IMAP sequence-set string**
  (`47,48` or `1:3`), not space-separated ids (`47 48` errors with
  "unexpected argument"). Standard IMAP set syntax; noted because it is
  easy to trip over.
- `\Flagged` maps to Gmail's **Starred**; `copy` adds a label and `move`
  adds + removes one (atomic via `MOVE`); `flagged`+`seen`+`answered`
  round-trip cleanly.

## Verdict

Gmail over IMAP is **fully working** across shared and IMAP-specific
commands, with no himalaya bugs — only Gmail's own label/search
semantics to account for (H1–H4). The account is IMAP-only, so sending
needs a separate `[smtp]` block (or the Gmail REST backend). Safe to
document as a supported provider.
