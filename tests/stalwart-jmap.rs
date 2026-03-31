#[path = "common/jmap.rs"]
mod jmap;

use std::{env, io::Write};

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires STALWART_{EMAIL,URL,USER,PASS} env vars and --ignored"]
fn stalwart_jmap() {
    let email = env::var("STALWART_EMAIL").unwrap_or("test@pimalaya.org".into());
    let url = env::var("STALWART_URL").unwrap_or("http://localhost:8080/jmap/session".into());
    let user = env::var("STALWART_USER").unwrap_or("test".into());
    let pass = env::var("STALWART_PASS").unwrap_or("test".into());

    let mut config = NamedTempFile::new().unwrap();
    let config_tpl = format!(
        r#"[accounts.stalwart]
default = true
jmap.server = "{url}"
jmap.auth.basic.username = "{user}"
jmap.auth.basic.password.raw = "{pass}""#
    );

    config.write(&config_tpl.into_bytes()).unwrap();

    jmap::run(config.path(), email);
}
