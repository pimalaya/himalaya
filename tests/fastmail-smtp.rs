#![cfg(feature = "smtp")]

#[path = "common/smtp.rs"]
mod smtp;

use std::{env, io::Write};

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires FASTMAIL_{EMAIL,APP_PASSWORD} env vars and --ignored"]
fn fastmail_smtp() {
    let email = env::var("FASTMAIL_EMAIL").expect("FASTMAIL_EMAIL not set");
    let app_password = env::var("FASTMAIL_APP_PASSWORD").expect("FASTMAIL_APP_PASSWORD not set");

    let mut config = NamedTempFile::new().unwrap();
    let config_tpl = format!(
        r#"[accounts.fastmail]
default = true
smtp.url = "smtps://smtp.fastmail.com"
smtp.sasl.plain.authcid = "{email}"
smtp.sasl.plain.passwd.raw = "{app_password}""#
    );

    config.write_all(config_tpl.as_bytes()).unwrap();

    smtp::run(config.path(), email);
}
