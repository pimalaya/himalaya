---
cairn: change
id: the-pimdir-refusals-are-coded
status: landed
created: 2026-11-18
---

# The pimdir refusals are coded

## Why

Findings TRI-4, TRI-7 and TRI-8 of the family's cli-family-analysis-2026-11-15 plan, Himalaya's part:

- TRI-4: three refusals of the pimdir tree were uncoded where the twins code them: a root holding no store (cardamum, calendula, carillon and the pimdir binary answer `store-uncreated`, in `account check` too), an unknown mailbox (`mailbox-not-found`, as cardamum's `addressbook-not-found` and calendula's `calendar-not-found`), a non-numeric message id (`message-not-found`, as `card-not-found` and `item-not-found`).
- TRI-7: refusals the pimdir binary codes and Himalaya did not: no terminal and no `--yes` (`confirmation-required`), an unknown queue row (`row-not-found`), a page token not printed (`page-token-invalid`). `pimdir queue cancel` also asked to confirm a row it then refused (HIM-17).
- TRI-8: an unset `pimdir.account` on a store holding several accounts was refused uncoded even when one alone held mail, and an account the store does not hold read as an empty one, `account check` answering OK.

## What

- `store-uncreated`, `mailbox-not-found`, `message-not-found`, `row-not-found`, `confirmation-required` and `page-token-invalid` on those refusals, spelled as the twins and the pimdir binary spell them.
- `pimdir queue cancel` refuses a row it cannot cancel before asking, and refuses to ask with no terminal and no `--yes`.
- Only the accounts holding mail count when `pimdir.account` is unset; several are refused under the new `store-account-ambiguous`, and an account the store does not hold under the new `store-account-unknown`. Neither code exists in the twins or the pimdir binary yet: they are offered for the family.
- Breaking: `code` `null` becomes a code on nine refusals, and a configured account the store does not hold is refused where it read as empty.
