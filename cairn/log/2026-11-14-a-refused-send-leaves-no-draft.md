---
cairn: log
change: a-refused-send-leaves-no-draft
landed: 2026-11-14
---

# A refused send leaves no draft

A JMAP `message send` whose `EmailSubmission/set` the server refuses (an entry in `notCreated`) now destroys the email its `Email/import` created, with one `Email/set { destroy: [id] }` on the refusal path only, then fails with the submission's error and code as before (`recipient-refused` for `invalidRecipients`). The stray draft was found reading the code on 2026-11-14 (cairn/changes/a-result-carries-its-errors, Decisions); the design was approved by the maintainer the same day.

- An email the import found already there (`alreadyExists`) is never destroyed, being the account's before the send.
- A submission request failing without an answer (the connection, an HTTP status, a method error) destroys nothing, the message possibly sent, and warns naming the id of an email the send created.
- A destroy that fails, refused in `notDestroyed` or failing whole, is a warning naming the id; the refusal stays the result.

Code: `JmapClient::send_message` in src/jmap/backend.rs, over io-jmap's `email_set` (pinned at 26a77bb, unchanged).

Tested offline against a scripted JMAP server on 127.0.0.1 (session, upload and method calls over plain HTTP, the client built by `JmapClient::new`): a refused submission of a created email sends the destroy of `E1` and fails `recipient-refused`; an `alreadyExists` email gets no `Email/set`; a destroy answered `notDestroyed` or with a method `serverFail` still fails `recipient-refused`. With the destroy guard forced off (the old behaviour) the first and third fail; with it forced on, the second fails. Not tested live: no writable JMAP test account. Clippy over every feature set of .github/workflows/tests.yml, `cargo test --all-features`, `cargo fmt`.

Spec: backends ("A JMAP send files the message as sent" modified, with the scenario "A refused submission").
