---
cairn: change
id: follows-storage-draft-17
status: active
created: 2026-11-23
---

# Himalaya follows storage-draft-17

## Why

Himalaya pins io-pimdir by git at dc19b8d (storage-draft-15, search-draft-16). io-pimdir master 7f16b27 implements pimdir storage-draft-17, sync-draft-15 and search-draft-17, and the five CLIs sharing one store must land on one revision before any of them is released. v3 needs it too: envelope-search-speaks-pimdir closes on the io-pimdir 0.20.0 release. The shared rules (pin form, exact drafts, who edits what, git, CPUs 2,3,18,19, locks, accounts, how a lane decides) are in /home/soywod/code/pimalaya/pimdir/cairn/changes/the-consumers-follow-draft-17/frame.md, which this plan follows and does not restate. Head a2726f56 (the owner's README commit, one ahead of origin) is neither pushed nor rewritten.

## What

Measured cost (inventory.md): 7 compile breaks (1 in the lib), then 4 failing tests, all mechanical; the 14 CI clippy sets and the build without SMTP pass once fixed.

- **A, build.** The pin; Cargo.lock moves io-pimdir alone (plus a `libc` line). Features unchanged, no ical-rs nor vcard-rs in the normal graph; pimalaya/nix takes the git pin as it takes dc19b8d (`importCargoLock`, `allowBuiltinFetchGit`); MSRV 1.89 both. `cargo deny` is red at HEAD already: D5.
- **B, compile.** `PimdirAction::SetProperty` joins the arms addressing a stored message (`addressed`); six test literals state `server_object_id: None`.
- **C, formats.** The later-draft stamp becomes `storage-draft-99`, `storage-draft-15` (what development builds wrote) stamped as earlier; the refusal stays io-pimdir's `PimdirFormatRefusal`.
- **D, SQL.** Three test statements follow `summary` and `summary_value`; no production SQL.
- **E, inherited.** Reserved forms (`~`, `/x`, `{`) refused on every backend; `label:` and `property:` answered by `pimdir search|count|tag`, refused by `envelope search`; flag terms read the store, not the index; `set-property` rows in the queue views; every development-build store refused `store-version`, the user draining it with the old engine, moving it aside, syncing anew.
- **F, sync.** Himalaya syncs nothing but stages what neverest drains, so K4 (a staged copy or move minted twice) reaches its users: needs-io-pimdir.md N3. R4 and R1 do not today: neverest rekeys IMAP alone, whose listings state known flags and no revision (neverest's plan, cairn/changes/follows-storage-draft-17/inventory.md F); the io-pimdir lane fixes them anyway.
- **G, bugs.** inventory.md §6: five lines closed, three io-pimdir's (N1 to N3), one found here, `pimdir status` taking 30 s at 517k to count bodies.
- **H, performance.** inventory.md §5: `pimdir search`, `count` and `--threads` 17 to 88 times faster after; `pimdir status` not (29.6 then 34.5 s, under load). Himalaya's own costs against the `pimdir` binary on the same store, to bring within 5 ms: `pimdir count` 73 ms against 8, `pimdir search -s 50` 35 against 11. Left: `envelope search` reading every body for a text term (1 s at 11,896 messages, D2), `mid:` scanning in io-pimdir (N5).
- **I to L.** Spec fold, CHANGELOG (BREAKING), MIGRATION.md, help texts; offline and cross-app tests, no live run (the pimdir surface reaches no server); release order; report.md.

## Scope

In: the pimdir backend, `pimdir` subcommands, `envelope search` parsing (every backend parses through io-pimdir), docs, the cross-app check with neverest.

Out, with the reason:
- himalaya-tui and himalaya-vim, separate repositories: himalaya-tui pins himalaya bffef6aa (io-pimdir 0.18.1 through it) and, when it bumps that pin, meets the same store refusal and the faster reads; no type it uses moves here.
- A verb writing labels or properties: D1.
- HIM-13 (JMAP blobs, Graph folder walk), HIM-18 (IMAP raw), the schema review remainder: not the pimdir surface; v3-release holds them.
- Queue output alignment across the family (`collection` keys, the `pimdir` binary's JSON contract): family rulings, not migration.

## Decisions

- **D1, labels and properties.** Default: read only, as io-pimdir gives them (`pimdir search|count|tag`), refused by `envelope search` (`search-unsupported`, a field outside mail for the other backends), no writing verb in 3.0. A `pimdir label` verb staging `set-property` is a later minor.
- **D2, `envelope search` on a pimdir account from the index.** Default: not in this change: the index ignores the queue overlay where the walk is exact, the walk costs at most the mailbox (1 s at 11,896 messages), and `pimdir search` answers in milliseconds. A follow-up may use the index when it is current and `overlay_quiet` holds, `PimdirQuery::scoped(.., Some(collection))` already scoping it.
- **D3, a flag spelled in another case.** Default: wait for io-pimdir's ruling (N1).
- **D4, release.** Default: Himalaya 3.0.0 releases on io-pimdir 0.20.0 from crates.io, never on a git pin, and only once neverest, cardamum, calendula and carillon are on that version too.
- **D5, url 2.5.9.** Default: bumped in this change (Cargo.lock only), so the audit job is green for the release; outside the pimdir surface, its own commit.

## Risks

- io-pimdir moves again before 0.20.0: the pin is retaken and B to D rerun (seven sites, an hour).
- A queued send or write in a development-build store is lost if the store is moved aside before the old engine drains it: the refusal and MIGRATION.md say to drain first.
- io-pimdir's readers now open SQLite in shared-cache mode: a REPL or MCP session holds a reader across commands; the suites pass (repl, mcp), but a `SQLITE_LOCKED` under a concurrent sync is untested.
- K4 through neverest until io-pimdir fixes it.
