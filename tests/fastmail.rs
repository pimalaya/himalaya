//! # Fastmail live suite
//!
//! The binary against Fastmail's JMAP server, on the Fastmail section of
//! config.sample.toml with a made-up bearer token. Only a browser consent
//! issues a Fastmail token, so the suite is `anonymous` only and touches
//! no account.
//!
//! Run with `cargo test --test fastmail -- --ignored`.

#![cfg(feature = "jmap")]

mod common;

mod anonymous {
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use crate::common::*;

    /// What Himalaya says of a session the server refuses.
    const REFUSED: &str = "HTTP 401";

    /// Writes the sample's Fastmail account, its token read through
    /// [`TOKEN_COMMAND`].
    fn write_account(dir: &Path) -> PathBuf {
        let token = [("jmap.auth.bearer.token.command", TOKEN_COMMAND)];
        write_config(dir, "fastmail", &sample_section("Fastmail", &token))
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn jmap_account_check_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_account(dir.path());

        let out = himalaya(&config, MADE_UP, &["--json", "account", "check"]);
        let report = json_check_failure(&out, REFUSED);

        assert_eq!(report["backends"][0]["backend"], "jmap", "{report}");
    }

    #[test]
    #[ignore = "requires network and --ignored"]
    fn jmap_envelope_list_refused() {
        let dir = TempDir::new().unwrap();
        let config = write_account(dir.path());

        let out = himalaya(&config, MADE_UP, &["--json", "envelope", "list"]);
        json_failure(&out, REFUSED);
    }
}
