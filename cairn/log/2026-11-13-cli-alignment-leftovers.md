---
cairn: log
change: cli-alignment-leftovers
landed: 2026-11-13
---

# One resolver per account

G2 closed ([#813](https://github.com/pimalaya/himalaya/issues/813)): `ImapClient::new` and `SmtpClient::new` each built their own `SecretResolver`, so an account whose IMAP and SMTP blocks name one password command ran it twice on `message send` with `save-copy`. `EmailClient` now builds one resolver, hands it to `ImapClient::new` when it opens the storage, and keeps it for the lazy SMTP connect of the send, which finds the value the IMAP session resolved. The protocol subcommands, building one client alone (`build_imap_client`, `build_smtp_client`), still give it its own. The regression test `a_send_reuses_the_credential_the_storage_resolved` (src/shared/client.rs) counts the command's runs against a scripted IMAP server and fails against the old wiring. H1 and H2 were settled earlier (the-wizard-leaves and the-first-run-welcome, 2026-11-11), so the change lands whole. Capabilities moved: config (scenario "A send saving its copy").
