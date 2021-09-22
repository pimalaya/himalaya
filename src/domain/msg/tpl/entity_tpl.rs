use serde::Serialize;
use std::{
    fmt::{self, Display},
    ops::Deref,
};

use crate::{
    config::entity::Account,
    domain::msg::{Msg, TplOverride},
};

#[derive(Debug, Default, Serialize)]
pub struct Tpl(pub String);

impl Tpl {
    pub fn new<'a>(opts: &'a TplOverride<'a>, account: &'a Account) -> Self {
        let mut tpl = String::default();

        // From
        tpl.push_str("From: ");
        if let Some(ref addrs) = opts.from {
            tpl.push_str(&addrs.join(", "));
        } else {
            tpl.push_str(&account.address());
        }
        tpl.push_str("\n");

        // To
        tpl.push_str("To: ");
        if let Some(ref addrs) = opts.to {
            tpl.push_str(&addrs.join(", "));
        }
        tpl.push_str("\n");

        // Cc
        if let Some(ref addrs) = opts.cc {
            tpl.push_str("Cc: ");
            tpl.push_str(&addrs.join(", "));
            tpl.push_str("\n");
        }

        // Bcc
        if let Some(ref addrs) = opts.bcc {
            tpl.push_str("Bcc: ");
            tpl.push_str(&addrs.join(", "));
            tpl.push_str("\n");
        }

        // Subject
        tpl.push_str("Subject: ");
        if let Some(subject) = opts.subject {
            tpl.push_str(subject);
        }

        // Headers <=> body separator
        tpl.push_str("\n\n");

        // Body
        if let Some(body) = opts.body {
            tpl.push_str(body);
        }

        // Signature
        if let Some(sig) = opts.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        } else if let Some(ref sig) = account.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        }

        Self(tpl)
    }

    pub fn reply<'a>(
        all: bool,
        msg: &Msg,
        opts: &'a TplOverride<'a>,
        account: &'a Account,
    ) -> Self {
        let mut tpl = String::default();

        // From
        tpl.push_str("From: ");
        if let Some(ref addrs) = opts.from {
            tpl.push_str(&addrs.join(", "));
        } else {
            tpl.push_str(&account.address());
        }
        tpl.push_str("\n");

        // To
        tpl.push_str("To: ");
        if let Some(ref addrs) = opts.to {
            tpl.push_str(&addrs.join(", "));
        } else if let Some(addrs) = msg.reply_to.as_ref().or(msg.from.as_ref()) {
            if all {
                let mut glue = "";
                for addr in addrs {
                    tpl.push_str(glue);
                    tpl.push_str(&addr.to_string());
                    glue = ", ";
                }
            } else {
                addrs.first().map(|addr| tpl.push_str(&addr.to_string()));
            }
        }
        tpl.push_str("\n");

        // In-Reply-To
        if let Some(ref id) = msg.message_id {
            tpl.push_str("In-Reply-To: ");
            tpl.push_str(id);
            tpl.push_str("\n");
        }

        if all {
            // Cc
            if let Some(ref addrs) = opts.cc {
                tpl.push_str("Cc: ");
                tpl.push_str(&addrs.join(", "));
                tpl.push_str("\n");
            }

            // Bcc
            if let Some(ref addrs) = opts.bcc {
                tpl.push_str("Bcc: ");
                tpl.push_str(&addrs.join(", "));
                tpl.push_str("\n");
            }
        }

        // Subject
        tpl.push_str("Subject: ");
        if let Some(subject) = opts.subject {
            tpl.push_str(subject);
        } else {
            if !msg.subject.starts_with("Re:") {
                tpl.push_str("Re: ");
            }
            tpl.push_str(&msg.subject);
        }

        // Headers <=> body separator
        tpl.push_str("\n\n");

        // Body
        if let Some(body) = opts.body {
            tpl.push_str(body);
        } else {
            let date = msg.date.to_owned().unwrap_or("unknown date".into());
            let sender = msg
                .reply_to
                .as_ref()
                .or(msg.from.as_ref())
                .and_then(|addrs| addrs.first())
                .map(|addr| addr.name.to_owned().unwrap_or(addr.email.to_string()))
                .unwrap_or("unknown sender".into());
            tpl.push_str(&format!("\n\nOn {}, {} wrote:\n", date, sender));

            let mut glue = "";
            for line in msg.join_text_parts().lines() {
                if line == "-- \n" {
                    break;
                }
                tpl.push_str(glue);
                tpl.push_str(">");
                tpl.push_str(if line.starts_with(">") { "" } else { " " });
                tpl.push_str(line);
                glue = "\n";
            }
        }

        // Signature
        if let Some(sig) = opts.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        } else if let Some(ref sig) = account.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        }

        Self(tpl)
    }
}

impl Deref for Tpl {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Tpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}
