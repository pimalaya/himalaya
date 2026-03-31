#[path = "common/jmap.rs"]
mod jmap;

use std::{env, io::Write};

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires FASTMAIL_{EMAIL,BEARER_TOKEN} env vars and --ignored"]
fn fastmail_jmap() {
    let email = env::var("FASTMAIL_EMAIL").expect("FASTMAIL_EMAIL env var");
    let token = env::var("FASTMAIL_BEARER_TOKEN").expect("FASTMAIL_BEARER_TOKEN env var");

    let mut config = NamedTempFile::new().unwrap();
    let config_tpl = format!(
        r#"[accounts.fastmail]
default = true
jmap.server = "https://api.fastmail.com/jmap/session"
jmap.auth.bearer.token.raw = "{token}""#
    );

    config.write(&config_tpl.into_bytes()).unwrap();

    jmap::run(config.path(), email);
}
