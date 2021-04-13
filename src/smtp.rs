use std::time::Duration;

use error_chain::error_chain;
use lettre::{self, transport::smtp::SmtpTransport, Transport};

use crate::config::model::Account;

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
    }
    foreign_links {
        Smtp(lettre::transport::smtp::Error);
    }
}

pub fn send(account: &Account, msg: &lettre::Message) -> Result<()> {
    let smtp_relay = if account.smtp_starttls() {
        SmtpTransport::starttls_relay
    } else {
        SmtpTransport::relay
    };

    let builder = smtp_relay(&account.smtp_host)?;

    builder
        .port(account.smtp_port)
        .credentials(account.smtp_creds()?)
        .timeout(Some(Duration::new(1000, 0)))
        .build()
        .send(msg)?;

    Ok(())
}
