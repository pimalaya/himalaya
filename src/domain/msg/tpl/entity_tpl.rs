use log::trace;
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
    pub fn from_msg(opts: TplOverride, msg: &Msg, account: &Account) -> Tpl {
        let mut tpl = String::default();

        if let Some(in_reply_to) = msg.in_reply_to.as_ref() {
            tpl.push_str(&format!("In-Reply-To: {}\n", in_reply_to))
        }

        // From
        tpl.push_str(&format!(
            "From: {}\n",
            opts.from
                .map(|addrs| addrs.join(", "))
                .unwrap_or_else(|| account.address())
        ));

        // To
        tpl.push_str(&format!(
            "To: {}\n",
            opts.to
                .map(|addrs| addrs.join(", "))
                .or_else(|| msg.to.clone().map(|addrs| addrs
                    .iter()
                    .map(|addr| addr.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")))
                .unwrap_or_default()
        ));

        // Cc
        if let Some(addrs) = opts.cc.map(|addrs| addrs.join(", ")).or_else(|| {
            msg.cc.clone().map(|addrs| {
                addrs
                    .iter()
                    .map(|addr| addr.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        }) {
            tpl.push_str(&format!("Cc: {}\n", addrs));
        }

        // Bcc
        if let Some(addrs) = opts.bcc.map(|addrs| addrs.join(", ")).or_else(|| {
            msg.bcc.clone().map(|addrs| {
                addrs
                    .iter()
                    .map(|addr| addr.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        }) {
            tpl.push_str(&format!("Bcc: {}\n", addrs));
        }

        // Subject
        tpl.push_str(&format!(
            "Subject: {}\n",
            opts.subject.unwrap_or(&msg.subject)
        ));

        // Headers <=> body separator
        tpl.push_str("\n");

        // Body
        if let Some(body) = opts.body {
            tpl.push_str(body);
        } else {
            tpl.push_str(&msg.join_text_parts("text/plain"))
        }

        // Signature
        if let Some(sig) = opts.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        } else if let Some(ref sig) = account.sig {
            tpl.push_str("\n\n");
            tpl.push_str(sig);
        }

        tpl.push_str("\n");

        let tpl = Tpl(tpl);
        trace!("template: {:#?}", tpl);
        tpl
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
