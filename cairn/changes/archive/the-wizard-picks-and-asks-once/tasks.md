---
cairn: tasks
change: the-wizard-picks-and-asks-once
---

# Tasks

- [x] Move io-pim-discovery to 0.11 and patch pimalaya-cli to the local checkout (6d8daa7).
- [x] Rank the discovered options per domain into Auto, deduplicated, encrypted servers only (src/wizard/plan.rs).
- [x] Parse a typed server URL: schemes, default ports, the `http://` refusal (src/wizard/server.rs).
- [x] Review loop and change lists; nothing discovered asks `Server URL:`, an empty answer stops (src/wizard/discover.rs).
- [x] Sign-ins grouped by identity, tested as typed, stored once, with the refusal menu (src/wizard/signin.rs).
- [x] Pick the IMAP mechanism from the CAPABILITY probe; SMTP, JMAP, Gmail API and Graph without a method prompt (src/wizard/imap.rs, smtp.rs, jmap.rs, gmail.rs, msgraph.rs).
- [x] Ask `Your name (optional):` last, written as `display-name`.
- [x] Tests: ranking, dedupe, review sentences, change labels, host grouping, server URL parsing, mechanism choice, the stored secret replacing the typed one.
- [x] README, config.sample.toml and the welcome say what the wizard now does.
- [x] CHANGELOG.
- [x] fmt, clippy, tests, feature builds.
- [x] Fold into cairn/spec/wizard.md; log; land; archive.
