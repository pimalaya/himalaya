# docs/

Development memory of the himalaya CLI: architecture notes that outgrow the main.rs header, plus plans and their outcomes.

- [io-email-inlining.md](io-email-inlining.md): landed plan for dropping the io-email dependency in favour of a local, lean, per-backend dispatching client owned by the CLI.
- [testing/provider-test-plan.md](testing/provider-test-plan.md): followable checklist to deeply exercise every shared command against a real provider, one report per `(backend, provider)`.
- [testing/imap-smtp-fastmail.md](testing/imap-smtp-fastmail.md): IMAP + SMTP on Fastmail — shared-command test report.
- [testing/imap-smtp-specific-fastmail.md](testing/imap-smtp-specific-fastmail.md): IMAP + SMTP on Fastmail — `imap …` / `smtp …` raw protocol API test report.
- [testing/jmap-fastmail.md](testing/jmap-fastmail.md): JMAP on Fastmail — shared-command test report.
- [testing/jmap-specific-fastmail.md](testing/jmap-specific-fastmail.md): JMAP on Fastmail — `jmap …` raw API test report.
- [testing/gmail.md](testing/gmail.md): Gmail REST on Google — shared-command test report.
- [testing/gmail-specific.md](testing/gmail-specific.md): Gmail REST on Google — `gmail …` raw API test report.
