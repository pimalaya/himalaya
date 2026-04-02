#![cfg(feature = "smtp")]

#[path = "common/smtp.rs"]
mod smtp;

use std::io::Write;

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires a running Stalwart instance and --ignored"]
fn stalwart_smtp() {
    let mut config = NamedTempFile::new().unwrap();
    let config_tpl = r#"[accounts.stalwart]
default = true
smtp.url = "smtp://localhost"
smtp.sasl.plain.authcid = "test"
smtp.sasl.plain.passwd.raw = "test""#;

    config.write_all(config_tpl.as_bytes()).unwrap();

    smtp::run(config.path(), "test@pimalaya.org");
}
