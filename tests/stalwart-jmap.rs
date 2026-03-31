#![cfg(feature = "jmap")]

#[path = "common/jmap.rs"]
mod jmap;

use std::io::Write;

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires a running Stalwart instance and --ignored"]
fn stalwart_jmap() {
    let mut config = NamedTempFile::new().unwrap();
    let config_tpl = format!(
        r#"[accounts.stalwart]
default = true
jmap.server = "http://localhost:8080/jmap/session"
jmap.auth.basic.username = "test"
jmap.auth.basic.password.raw = "test""#
    );

    config.write(&config_tpl.into_bytes()).unwrap();

    jmap::run(config.path(), "test@pimalaya.org");
}
