#![cfg(feature = "imap")]

#[path = "common/imap.rs"]
mod imap;
#[path = "common/shared.rs"]
mod shared;

use std::{env, io::Write};

use tempfile::NamedTempFile;

fn write_imap_config() -> (NamedTempFile, String) {
    let email = env::var("FASTMAIL_EMAIL").expect("FASTMAIL_EMAIL not set");
    let app_password = env::var("FASTMAIL_APP_PASSWORD").expect("FASTMAIL_APP_PASSWORD not set");

    let mut config = NamedTempFile::new().unwrap();
    let config_tpl = format!(
        r#"[accounts.fastmail]
default = true
imap.url = "imaps://imap.fastmail.com"
imap.sasl.plain.authcid = "{email}"
imap.sasl.plain.passwd.raw = "{app_password}""#
    );

    config.write_all(config_tpl.as_bytes()).unwrap();

    (config, email)
}

#[test]
#[ignore = "requires FASTMAIL_{EMAIL,APP_PASSWORD} env vars and --ignored"]
fn fastmail_imap() {
    let (config, email) = write_imap_config();
    imap::run(config.path(), email);
}

#[test]
#[ignore = "requires FASTMAIL_{EMAIL,APP_PASSWORD} env vars and --ignored"]
fn fastmail_shared_imap() {
    let (config, email) = write_imap_config();
    shared::run(config.path(), email);
}
