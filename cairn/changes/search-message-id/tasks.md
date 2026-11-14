---
cairn: tasks
change: search-message-id
---

# Tasks

- [x] Grammar: `message-id <id>`, the id held bare; `SearchEmailsFilterQuery::MessageId`; its rendering
- [x] Capabilities: `envelope.search.message-id` per backend
- [x] IMAP `HEADER Message-ID` with the root-chain re-check; Gmail `rfc822msgid:`; Graph `$filter` alone; the local evaluator; JMAP refused
- [x] `imap search --header NAME TEXT`
- [x] The `QUERY` help
- [x] Tests: parser, rendering, evaluator, IMAP and Gmail translations, Graph filter and refusal, JMAP refusal, Maildir end to end
- [x] CHANGELOG; spec folded into cairn/spec/search.md; log
