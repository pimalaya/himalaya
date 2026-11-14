//! # Google live suite
//!
//! The binary against the Gmail API and Gmail's IMAP server. `anonymous`
//! runs a made-up token, touching no account. `credentialed` reads the
//! test Workspace's mailbox with a token the suite mints for a service
//! account acting as `HIMALAYA_GOOGLE_SERVICE_ACCOUNT_SUBJECT` (RFC 7523
//! section 2.1), its key read from `HIMALAYA_GOOGLE_SERVICE_ACCOUNT_KEY`
//! or the file `HIMALAYA_GOOGLE_SERVICE_ACCOUNT_KEY_FILE` names. It only
//! reads.
//!
//! The Gmail API account is the Gmail section of config.sample.toml. The
//! sample has no Gmail IMAP section, so that account is written here.
//!
//! Run with `cargo test --test google -- --ignored`. A failure never shows
//! a token: the binary reads it from its environment only.

#![cfg(all(feature = "gmail", feature = "imap"))]

mod common;

use std::path::{Path, PathBuf};

use crate::common::*;

/// Writes the sample's Gmail API account, its token read through
/// [`TOKEN_COMMAND`].
fn write_gmail_account(dir: &Path) -> PathBuf {
    let token = [("gmail.auth.token.command", TOKEN_COMMAND)];
    write_config(dir, "google", &sample_section("Gmail", &token))
}

/// Writes a Gmail IMAP account authenticating `user` with XOAUTH2, its
/// token read through [`TOKEN_COMMAND`].
fn write_imap_account(dir: &Path, user: &str) -> PathBuf {
    let lines = format!(
        "imap.server = \"imap.gmail.com\"\nimap.sasl.xoauth2.username = \"{user}\"\nimap.sasl.xoauth2.token.command = {TOKEN_COMMAND}"
    );
    write_config(dir, "google", &lines)
}

mod anonymous {
    use tempfile::TempDir;

    use crate::{common::*, write_gmail_account, write_imap_account};

    /// What Himalaya says of a request the Gmail API refuses.
    const GMAIL_REFUSED: &str = "HTTP 401";

    /// What Himalaya says of an IMAP authentication Gmail refuses.
    const IMAP_REFUSED: &str = "AUTHENTICATE XOAUTH2 failed";

    /// The user the made-up token claims to be.
    const USER: &str = "alice@example.org";

    /// The accounts the suite writes load, offline: a sample change
    /// breaking them fails here rather than against Google.
    #[test]
    fn the_accounts_load() {
        let dir = TempDir::new().unwrap();
        let list = ["--json", "account", "list"];

        // NOTE: each account overwrites the one before, so each loads
        // before the next is written.
        let config = write_gmail_account(dir.path());
        json(&himalaya(&config, MADE_UP, &list), 0);

        let config = write_imap_account(dir.path(), USER);
        json(&himalaya(&config, MADE_UP, &list), 0);
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn gmail_account_check_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_gmail_account(dir.path());

        let out = himalaya(&config, MADE_UP, &["--json", "account", "check"]);
        let report = json_check_failure(&out, GMAIL_REFUSED);

        assert_eq!(report["backends"][0]["backend"], "gmail", "{report}");
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn gmail_envelope_list_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_gmail_account(dir.path());

        let out = himalaya(&config, MADE_UP, &["--json", "envelope", "list"]);
        json_failure(&out, GMAIL_REFUSED);
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn imap_account_check_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path(), USER);

        let out = himalaya(&config, MADE_UP, &["--json", "account", "check"]);
        let report = json_check_failure(&out, IMAP_REFUSED);

        assert_eq!(report["backends"][0]["backend"], "imap", "{report}");
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn imap_envelope_list_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path(), USER);

        let out = himalaya(&config, MADE_UP, &["--json", "envelope", "list"]);
        json_failure(&out, IMAP_REFUSED);
    }
}

mod credentialed {
    use std::{collections::BTreeSet, sync::OnceLock, time::Duration};

    use io_oauth::{
        client::Oauth20ClientStd,
        rfc7523::{
            assertion::{Oauth20JwtBearerClaims, Oauth20JwtBearerKey},
            auth_grant::Oauth20JwtBearerGrantRequestParams,
        },
    };
    use pimalaya_stream::tls::Tls;
    use secrecy::ExposeSecret;
    use serde_json::Value;
    use tempfile::TempDir;

    use crate::{common::*, write_gmail_account, write_imap_account};

    /// The user acted as when `HIMALAYA_GOOGLE_SERVICE_ACCOUNT_SUBJECT` is
    /// unset.
    const SUBJECT: &str = "google@pimalaya.org";

    /// The scope both the Gmail API and IMAP accept.
    const SCOPE: &str = "https://mail.google.com/";

    /// The variable holding the service account key.
    const KEY_VAR: &str = "HIMALAYA_GOOGLE_SERVICE_ACCOUNT_KEY";

    /// The user the service account acts as.
    fn subject() -> String {
        env_var("HIMALAYA_GOOGLE_SERVICE_ACCOUNT_SUBJECT").unwrap_or_else(|| SUBJECT.to_owned())
    }

    /// A token of the service account acting as [`subject`], minted once
    /// per run. A failure never shows the key, nor what it signed.
    fn token() -> &'static str {
        static TOKEN: OnceLock<String> = OnceLock::new();

        TOKEN.get_or_init(|| {
            let Some(key) = env_content(KEY_VAR) else {
                panic!("set {KEY_VAR} or {KEY_VAR}_FILE to run this test");
            };
            let key: Value = serde_json::from_str(&key)
                .unwrap_or_else(|_| panic!("{KEY_VAR} is not a service account key"));
            let (Some(email), Some(pem), Some(endpoint)) = (
                key["client_email"].as_str(),
                key["private_key"].as_str(),
                key["token_uri"].as_str(),
            ) else {
                panic!("{KEY_VAR} is not a service account key");
            };
            let signer = Oauth20JwtBearerKey::from_pkcs8_pem(pem)
                .unwrap_or_else(|_| panic!("{KEY_VAR} holds no PKCS#8 key"));

            let tls = Tls::default();
            let client = Oauth20ClientStd::connect(endpoint.parse().unwrap(), &tls, email);
            let mut client = client.unwrap_or_else(|err| panic!("cannot reach {endpoint}: {err}"));

            let subject = subject();
            let claims = Oauth20JwtBearerClaims {
                iss: email.into(),
                sub: Some(subject.as_str().into()),
                scope: [SCOPE.into()].into(),
                ..Default::default()
            };
            let validity = Duration::from_secs(600);
            let assertion = client
                .sign_jwt_bearer_assertion(&signer, claims, None, validity)
                .unwrap_or_else(|_| panic!("cannot sign the assertion"));

            let params = Oauth20JwtBearerGrantRequestParams {
                assertion,
                scope: BTreeSet::new(),
            };

            match client.request_jwt_bearer_grant(params) {
                Ok(Ok(success)) => success.access_token.expose_secret().to_owned(),
                Ok(Err(err)) => panic!("token refused: {:?}", err.error_description),
                Err(err) => panic!("token request failed: {err}"),
            }
        })
    }

    #[test]
    #[ignore = "requires Google credentials and --ignored"]
    fn gmail_account_check() {
        let dir = TempDir::new().unwrap();
        let config = write_gmail_account(dir.path());

        let out = himalaya(&config, token(), &["--json", "account", "check"]);
        let report = json(&out, 0);

        assert_eq!(report["backends"][0]["backend"], "gmail", "{report}");
        assert_eq!(report["backends"][0]["ok"], true, "{report}");
    }

    #[test]
    #[ignore = "requires Google credentials and --ignored"]
    fn gmail_mailbox_list() {
        let dir = TempDir::new().unwrap();
        let config = write_gmail_account(dir.path());

        let out = himalaya(&config, token(), &["--json", "mailbox", "list"]);
        assert_mailboxes(&json(&out, 0));
    }

    #[test]
    #[ignore = "requires Google credentials and --ignored"]
    fn gmail_envelope_list() {
        let dir = TempDir::new().unwrap();
        let config = write_gmail_account(dir.path());

        let args = ["--json", "envelope", "list", "-s", "5"];
        let out = himalaya(&config, token(), &args);
        assert_envelopes(&json(&out, 0), 5);
    }

    #[test]
    #[ignore = "requires Google credentials and --ignored"]
    fn imap_account_check() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path(), &subject());

        let out = himalaya(&config, token(), &["--json", "account", "check"]);
        let report = json(&out, 0);

        assert_eq!(report["backends"][0]["backend"], "imap", "{report}");
        assert_eq!(report["backends"][0]["ok"], true, "{report}");
    }

    #[test]
    #[ignore = "requires Google credentials and --ignored"]
    fn imap_mailbox_list() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path(), &subject());

        let out = himalaya(&config, token(), &["--json", "mailbox", "list"]);
        assert_mailboxes(&json(&out, 0));
    }

    #[test]
    #[ignore = "requires Google credentials and --ignored"]
    fn imap_envelope_list() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path(), &subject());

        let args = ["--json", "envelope", "list", "-s", "5"];
        let out = himalaya(&config, token(), &args);
        assert_envelopes(&json(&out, 0), 5);
    }
}
