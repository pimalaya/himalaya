#![cfg(feature = "imap")]

#[path = "common/imap.rs"]
mod imap;

use std::io::Write;

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires a running Stalwart instance and --ignored"]
fn stalwart_imap() {
    let mut config = NamedTempFile::new().unwrap();
    let config_tpl = format!(
        r#"[accounts.stalwart]
default = true
imap.url = "imap://localhost"
imap.sasl.plain.authcid = "test"
imap.sasl.plain.passwd.raw = "test""#
    );

    config.write_all(config_tpl.as_bytes()).unwrap();

    imap::run(config.path(), "test@pimalaya.org");
}
