#[path = "common/jmap.rs"]
mod jmap;

use std::{env, io::Write};

use tempfile::NamedTempFile;

#[test]
#[ignore = "requires BEARER_TOKEN env var and --ignored"]
fn fastmail_jmap() {
    let token = env::var("BEARER_TOKEN").expect("BEARER_TOKEN env var");

    let mut config = NamedTempFile::new().unwrap();
    let config_tpl = format!(
        r#"[accounts.fastmail]
default = true
jmap.server = "https://api.fastmail.com/jmap/session"
jmap.auth.bearer.token.raw = "{token}""#
    );

    config.write(&config_tpl.into_bytes()).unwrap();

    jmap::run(config.path());
}
