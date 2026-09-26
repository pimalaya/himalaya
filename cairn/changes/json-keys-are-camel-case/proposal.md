---
cairn: change
id: json-keys-are-camel-case
status: active
created: 2026-08-29
---

# `--json` keys become camelCase at 3.0

## Why

The Pimalaya family standardises the object keys of `--json` on camelCase, and Himalaya is one of the four products that cannot follow immediately.

camelCase is what the wire formats these commands wrap already speak. JMAP objects are camelCase by RFC 8620, and so are Microsoft Graph and every Google API Himalaya talks to. A Gmail or Graph response crossing into a Himalaya output type crosses a case boundary today for no reason other than serde's default.

The second reason is the consumer `--json` exists for. Neither JavaScript nor jq can use dot access on a key containing a hyphen: `.messages-total` has to be written `."messages-total"` in jq and `obj["messages-total"]` in JavaScript, and a script that forgets the quoting fails silently rather than loudly. `messagesTotal` reads the same in both.

What Himalaya emits today is not one convention but two. Twenty-six types carry `#[serde(rename_all = "kebab-case")]` (src/gmail/profile/get.rs, src/msgraph/profile/get.rs, src/gmail/history/list.rs, the three Gmail get renderers, the five Gmail settings singletons, src/sieve/get.rs, src/sieve/list.rs, src/sieve/capability.rs, src/imap/id.rs, src/shared/message/delete.rs, src/email/envelope.rs, src/email/mailbox.rs, src/email/flag.rs, src/email/address.rs). The rest carry nothing and emit serde's snake_case, so `next_page` from src/shared/output.rs and `messages-total` from `gmail profile get` ship side by side in the same CLI.

Himalaya is 2.1.0 and `--json` keys are a published contract, so changing them is breaking and waits for 3.0.

## What changes at 3.0

Output types only: the types handed to `printer.out` and registered in src/json_schema.rs. Each one gets `#[serde(rename_all = "camelCase")]`, and the existing kebab-case renames on those types go. The reference type is `GmailProfileOutput` in src/gmail/profile/get.rs, whose payload goes from `{email, messages-total, threads-total, history-id}` to `{email, messagesTotal, threadsTotal, historyId}`; the doc comment naming the old spelling moves with it.

Sixty-one distinct types are registered, of which nineteen already carry the `*Output` suffix and forty-two do not (`Envelopes`, `Mailboxes`, `MessagesTable`, `DeleteReport`, `SieveScripts` and the rest). Renaming them to `*Output` is the other half of the same cleanup and belongs in the same major, since both touch the same declarations.

Three things are deliberately left alone.

Config types stay kebab-case. src/config.rs is deserialized from TOML, where hyphenated keys are the family convention and no jq expression ever reaches them. The rule is about what the printer emits, never about what the loader reads.

Provider passthrough keeps its own spelling. A field carrying a wire name verbatim keeps its `#[serde(rename = "...")]`: `@odata.nextLink` is what Microsoft Graph called it and `nextPageToken` is what Gmail called it, and neither is derivable from a Rust field name. Applying `rename_all` to a type holding one of those does not touch it, which is correct, and nobody should "fix" it later.

Transparent newtypes over io-gmail and io-msgraph resources are not Himalaya's to rename. `MsgraphMessageGetOutput` and the Gmail settings newtypes serialize whatever the io- crate declared, which is already the provider's camelCase. They need no attribute and must not be forced into one.

Himalaya's own pagination field is not passthrough: `Paginated::next_page` in src/shared/output.rs is a Himalaya invention wrapping the provider cursor, and it becomes `nextPage` like any other output field.

## The alias trap

`#[serde(alias = "...")]` is a deserialization-only attribute. It teaches `Deserialize` to accept a second spelling and has no effect whatsoever on `Serialize`, so it cannot make an output type emit both `messages-total` and `messagesTotal` during a transition. An output type only ever serializes, which makes an alias on one pure decoration.

The alternatives, if softening the break is ever wanted, are to twin the keys in the printer (serialize both spellings, which doubles the payload and makes the published schema describe two names for one value) or to accept the break at the major. The decision is to accept it: a major release is the place a key rename is allowed to be visible.

## Out of scope

No CHANGELOG entry: nothing changes for a user until 3.0 lands the rename. The JSON Schema files written by `json-schema` change with the keys, so any consumer pinned on them regenerates at the same time.
