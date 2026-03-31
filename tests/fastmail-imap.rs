#![cfg(feature = "imap")]

#[path = "common/imap.rs"]
mod imap;

use std::{env, io::Write};

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires FASTMAIL_{EMAIL,APP_PASSWORD} env vars and --ignored"]
fn fastmail_imap() {
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

    imap::run(config.path(), email);
}
