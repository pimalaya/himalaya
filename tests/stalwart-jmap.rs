#[path = "common/jmap.rs"]
mod jmap;

use std::{env, io::Write};

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires URL, USER, PASS env vars and --ignored"]
fn stalwart_jmap() {
    let mut config = NamedTempFile::new().unwrap();

    let url = env::var("URL").unwrap_or("http://localhost:8080/jmap/session".into());
    let user = env::var("USER").unwrap_or("test".into());
    let pass = env::var("PASS").unwrap_or("test".into());

    let config_tpl = format!(
        r#"[accounts.stalwart]
default = true
jmap.server = "{url}"
jmap.auth.basic.username = "{user}"
jmap.auth.basic.password.raw = "{pass}""#
    );

    config.write(&config_tpl.into_bytes()).unwrap();

    jmap::run(config.path());
}
