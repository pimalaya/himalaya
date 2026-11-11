//! # Microsoft live suite
//!
//! The binary against Microsoft Graph and Exchange Online. `anonymous`
//! runs the Microsoft sections of config.sample.toml with a made-up token,
//! touching no account. `credentialed` reads the test tenant's mailbox
//! through Graph and Exchange IMAP, with app-only tokens the suite mints
//! through the client credentials grant of a confidential registration,
//! read from `HIMALAYA_MICROSOFT_TENANT_ID`, `HIMALAYA_MICROSOFT_CLIENT_ID`
//! and `HIMALAYA_MICROSOFT_CLIENT_SECRET`; `HIMALAYA_MICROSOFT_USER` names
//! the mailbox. It only reads.
//!
//! Run with `cargo test --test microsoft -- --ignored`. A failure never
//! shows a token: the binary reads it from its environment only.

#![cfg(all(feature = "msgraph", feature = "imap", feature = "smtp"))]

mod common;

mod anonymous {
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use crate::common::*;

    /// What Himalaya says of a request Graph refuses.
    const GRAPH_REFUSED: &str = "HTTP 401";

    /// What Himalaya says of an IMAP or SMTP authentication Exchange
    /// refuses.
    const XOAUTH2_REFUSED: &str = "XOAUTH2 failed";

    /// Writes the sample's Graph account, its token read through
    /// [`TOKEN_COMMAND`].
    fn write_graph_account(dir: &Path) -> PathBuf {
        let token = [("msgraph.auth.token.command", TOKEN_COMMAND)];
        let section = sample_section("Microsoft Graph", &token);
        write_config(dir, "microsoft", &section)
    }

    /// Writes the sample's Exchange IMAP and SMTP account, both tokens
    /// read through [`TOKEN_COMMAND`].
    fn write_imap_account(dir: &Path) -> PathBuf {
        let tokens = [
            ("imap.sasl.xoauth2.token.command", TOKEN_COMMAND),
            ("smtp.sasl.xoauth2.token.command", TOKEN_COMMAND),
        ];
        let section = sample_section("Microsoft IMAP and SMTP", &tokens);
        write_config(dir, "microsoft", &section)
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn graph_account_check_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_graph_account(dir.path());

        let out = himalaya(&config, MADE_UP, &["--json", "account", "check"]);
        let report = json_check_failure(&out, GRAPH_REFUSED);

        assert_eq!(report["backends"][0]["backend"], "msgraph", "{report}");
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn graph_envelope_list_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_graph_account(dir.path());

        let out = himalaya(&config, MADE_UP, &["--json", "envelope", "list"]);
        json_failure(&out, GRAPH_REFUSED);
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn imap_smtp_account_check_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path());

        let out = himalaya(&config, MADE_UP, &["--json", "account", "check"]);
        let report = json_check_failure(&out, XOAUTH2_REFUSED);

        let backends = &report["backends"];
        assert_eq!(backends[0]["backend"], "imap", "{report}");
        assert_eq!(backends[1]["backend"], "smtp", "{report}");
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn imap_envelope_list_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path());

        let out = himalaya(&config, MADE_UP, &["--json", "envelope", "list"]);
        json_failure(&out, XOAUTH2_REFUSED);
    }
}

mod credentialed {
    use std::{
        path::{Path, PathBuf},
        sync::OnceLock,
    };

    use io_oauth::{
        client::Oauth20ClientStd,
        rfc6749::client_credentials::Oauth20ClientCredentialsRequestParams,
    };
    use pimalaya_stream::tls::Tls;
    use secrecy::{ExposeSecret, SecretString};
    use tempfile::TempDir;

    use crate::common::*;

    /// The mailbox read when `HIMALAYA_MICROSOFT_USER` is unset.
    const USER: &str = "microsoft@pimalaya.onmicrosoft.com";

    /// The scope of an app-only Graph token.
    const GRAPH_SCOPE: &str = "https://graph.microsoft.com/.default";

    /// The scope of an app-only Exchange IMAP token.
    const IMAP_SCOPE: &str = "https://outlook.office365.com/.default";

    /// A token of the test registration for `scope`, through the client
    /// credentials grant.
    fn mint(scope: &str) -> String {
        let [tenant, client_id, secret] = require_env([
            "HIMALAYA_MICROSOFT_TENANT_ID",
            "HIMALAYA_MICROSOFT_CLIENT_ID",
            "HIMALAYA_MICROSOFT_CLIENT_SECRET",
        ]);
        let endpoint = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token");

        let tls = Tls::default();
        let mut client = Oauth20ClientStd::connect(endpoint.parse().unwrap(), &tls, client_id)
            .unwrap_or_else(|err| panic!("cannot reach {endpoint}: {err}"));
        client.client_secret = Some(SecretString::from(secret));

        let params = Oauth20ClientCredentialsRequestParams {
            scope: [scope.into()].into(),
        };

        match client.request_client_credentials(params) {
            Ok(Ok(success)) => success.access_token.expose_secret().to_owned(),
            Ok(Err(err)) => panic!("token refused: {:?}", err.error_description),
            Err(err) => panic!("token request failed: {err}"),
        }
    }

    /// A Graph token, minted once per run.
    fn graph_token() -> &'static str {
        static TOKEN: OnceLock<String> = OnceLock::new();
        TOKEN.get_or_init(|| mint(GRAPH_SCOPE))
    }

    /// An Exchange IMAP token, minted once per run.
    fn imap_token() -> &'static str {
        static TOKEN: OnceLock<String> = OnceLock::new();
        TOKEN.get_or_init(|| mint(IMAP_SCOPE))
    }

    /// The mailbox the suite reads, as a TOML string.
    fn user() -> String {
        let user = env_var("HIMALAYA_MICROSOFT_USER").unwrap_or_else(|| USER.to_owned());
        format!("\"{user}\"")
    }

    /// Writes the sample's Graph account on the test mailbox, its token
    /// read through [`TOKEN_COMMAND`].
    fn write_graph_account(dir: &Path) -> PathBuf {
        let user = user();
        let overrides = [
            ("msgraph.auth.token.command", TOKEN_COMMAND),
            ("msgraph.user-id", user.as_str()),
        ];

        let section = sample_section("Microsoft Graph", &overrides);
        write_config(dir, "microsoft", &section)
    }

    /// Writes the sample's Exchange IMAP and SMTP account on the test
    /// mailbox, both tokens read through [`TOKEN_COMMAND`]. The suite never
    /// reaches SMTP.
    fn write_imap_account(dir: &Path) -> PathBuf {
        let user = user();
        let overrides = [
            ("imap.sasl.xoauth2.username", user.as_str()),
            ("imap.sasl.xoauth2.token.command", TOKEN_COMMAND),
            ("smtp.sasl.xoauth2.username", user.as_str()),
            ("smtp.sasl.xoauth2.token.command", TOKEN_COMMAND),
        ];

        let section = sample_section("Microsoft IMAP and SMTP", &overrides);
        write_config(dir, "microsoft", &section)
    }

    #[test]
    #[ignore = "requires Microsoft credentials and --ignored"]
    fn graph_account_check() {
        let dir = TempDir::new().unwrap();
        let config = write_graph_account(dir.path());

        let out = himalaya(&config, graph_token(), &["--json", "account", "check"]);
        let report = json(&out, 0);

        assert_eq!(report["backends"][0]["backend"], "msgraph", "{report}");
        assert_eq!(report["backends"][0]["ok"], true, "{report}");
    }

    #[test]
    #[ignore = "requires Microsoft credentials and --ignored"]
    fn graph_mailbox_list() {
        let dir = TempDir::new().unwrap();
        let config = write_graph_account(dir.path());

        let out = himalaya(&config, graph_token(), &["--json", "mailbox", "list"]);
        assert_mailboxes(&json(&out, 0));
    }

    #[test]
    #[ignore = "requires Microsoft credentials and --ignored"]
    fn graph_envelope_list() {
        let dir = TempDir::new().unwrap();
        let config = write_graph_account(dir.path());

        let args = ["--json", "envelope", "list", "-s", "5"];
        let out = himalaya(&config, graph_token(), &args);
        assert_envelopes(&json(&out, 0), 5);
    }

    #[test]
    #[ignore = "requires Microsoft credentials and --ignored"]
    fn imap_account_check() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path());

        let args = ["--json", "--backend", "imap", "account", "check"];
        let report = json(&himalaya(&config, imap_token(), &args), 0);

        assert_eq!(report["backends"][0]["backend"], "imap", "{report}");
        assert_eq!(report["backends"][0]["ok"], true, "{report}");
        assert_eq!(report["backends"].as_array().unwrap().len(), 1, "{report}");
    }

    #[test]
    #[ignore = "requires Microsoft credentials and --ignored"]
    fn imap_mailbox_list() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path());

        let out = himalaya(&config, imap_token(), &["--json", "mailbox", "list"]);
        assert_mailboxes(&json(&out, 0));
    }

    #[test]
    #[ignore = "requires Microsoft credentials and --ignored"]
    fn imap_envelope_list() {
        let dir = TempDir::new().unwrap();
        let config = write_imap_account(dir.path());

        let args = ["--json", "envelope", "list", "-s", "5"];
        let out = himalaya(&config, imap_token(), &args);
        assert_envelopes(&json(&out, 0), 5);
    }
}
