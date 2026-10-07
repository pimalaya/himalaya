---
cairn: log
change: io-pimdir-global-page-order
landed: 2026-10-07
---

# io-pimdir pinned at be03184, pages across collections walk the global order

io-pimdir moves from `ff28408` to `be03184` (pimdir `22f1f2c`). A page of mail across collections walks the new store-wide index `items_by_sort_global` (`sort_key`, `seq`, `collection`), which the hub creates when it opens an older store; `list_mail_page_filtered` and `search_mail` keep their collection test off `items_by_seq`, so the planner walks the index instead of reading and sorting every live row. A reader of a store still lacking the index pages by a scan and a sort. Himalaya's code is unchanged: its envelope listing calls `list_mail_page_filtered`, whose statement now walks the index on a store that has it. Tests run with the default features, with `--no-default-features --features pimdir,rustls-ring` and with `--no-default-features --features rustls-ring,imap,smtp,msgraph,pimdir`, as MOA builds it.

Capabilities moved: none.
