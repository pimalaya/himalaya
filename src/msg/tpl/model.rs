use error_chain::error_chain;
use mailparse::{self, MailHeaderMap};
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, fmt};
use url::Url;

use crate::{ctx::Ctx, msg::model::Msg};

error_chain! {}

const TPL_HEADERS: &[&str] = &["From", "To", "In-Reply-To", "Cc", "Bcc", "Subject"];

#[derive(Debug, Clone, Serialize)]
pub struct Tpl {
    headers: HashMap<String, String>,
    body: Option<String>,
    signature: Option<String>,
    raw: String,
}

impl Tpl {
    pub fn new(ctx: &Ctx) -> Self {
        let mut headers = HashMap::new();
        headers.insert("From".to_string(), ctx.config.address(ctx.account));
        headers.insert("To".to_string(), String::new());
        headers.insert("Subject".to_string(), String::new());

        let mut tpl = Self {
            headers,
            body: None,
            signature: ctx.config.signature(ctx.account),
            raw: String::new(),
        };
        tpl.raw = tpl.to_string();
        tpl
    }

    pub fn reply(ctx: &Ctx, msg: &mailparse::ParsedMail) -> Self {
        let parsed_headers = msg.get_headers();
        let mut headers = HashMap::new();

        headers.insert("From".to_string(), ctx.config.address(ctx.account));

        let to = parsed_headers
            .get_first_value("reply-to")
            .or(parsed_headers.get_first_value("from"))
            .unwrap_or_default();
        headers.insert("To".to_string(), to);

        if let Some(in_reply_to) = parsed_headers.get_first_value("message-id") {
            headers.insert("In-Reply-To".to_string(), in_reply_to);
        }

        let subject = parsed_headers
            .get_first_value("subject")
            .unwrap_or_default();
        headers.insert("Subject".to_string(), format!("Re: {}", subject));

        let mut parts = vec![];
        Msg::extract_text_bodies_into(&msg, "text/plain", &mut parts);
        if parts.is_empty() {
            Msg::extract_text_bodies_into(&msg, "text/html", &mut parts);
        }

        let body = parts
            .join("\r\n\r\n")
            .replace("\r", "")
            .split("\n")
            .map(|line| format!(">{}", line))
            .collect::<Vec<String>>()
            .join("\n");

        let mut tpl = Self {
            headers,
            body: Some(body),
            signature: ctx.config.signature(&ctx.account),
            raw: String::new(),
        };
        tpl.raw = tpl.to_string();
        tpl
    }

    pub fn reply_all(ctx: &Ctx, msg: &mailparse::ParsedMail) -> Self {
        let parsed_headers = msg.get_headers();
        let mut headers = HashMap::new();

        let from: lettre::message::Mailbox = ctx.config.address(ctx.account).parse().unwrap();
        headers.insert("From".to_string(), from.to_string());

        let to = parsed_headers
            .get_all_values("to")
            .iter()
            .flat_map(|addrs| addrs.split(","))
            .fold(vec![], |mut mboxes, addr| {
                match addr.trim().parse::<lettre::message::Mailbox>() {
                    Err(_) => mboxes,
                    Ok(mbox) => {
                        if mbox != from {
                            mboxes.push(mbox.to_string());
                        }
                        mboxes
                    }
                }
            });
        let reply_to = parsed_headers
            .get_all_values("reply-to")
            .iter()
            .flat_map(|addrs| addrs.split(","))
            .map(|addr| addr.trim().to_string())
            .collect::<Vec<String>>();
        let reply_to = if reply_to.is_empty() {
            parsed_headers
                .get_all_values("from")
                .iter()
                .flat_map(|addrs| addrs.split(","))
                .map(|addr| addr.trim().to_string())
                .collect::<Vec<String>>()
        } else {
            reply_to
        };
        headers.insert("To".to_string(), [reply_to, to].concat().join(", "));

        if let Some(in_reply_to) = parsed_headers.get_first_value("message-id") {
            headers.insert("In-Reply-To".to_string(), in_reply_to);
        }

        let cc = parsed_headers.get_all_values("cc");
        if !cc.is_empty() {
            headers.insert("Cc".to_string(), cc.join(", "));
        }

        let subject = parsed_headers
            .get_first_value("subject")
            .unwrap_or_default();
        headers.insert("Subject".to_string(), format!("Re: {}", subject));

        let mut parts = vec![];
        Msg::extract_text_bodies_into(&msg, "text/plain", &mut parts);
        if parts.is_empty() {
            Msg::extract_text_bodies_into(&msg, "text/html", &mut parts);
        }

        let body = parts
            .join("\r\n\r\n")
            .replace("\r", "")
            .split("\n")
            .map(|line| format!(">{}", line))
            .collect::<Vec<String>>()
            .join("\n");

        let mut tpl = Self {
            headers,
            body: Some(body),
            signature: ctx.config.signature(&ctx.account),
            raw: String::new(),
        };
        tpl.raw = tpl.to_string();
        tpl
    }

    pub fn forward(ctx: &Ctx, msg: &mailparse::ParsedMail) -> Self {
        let parsed_headers = msg.get_headers();
        let mut headers = HashMap::new();

        headers.insert("From".to_string(), ctx.config.address(ctx.account));
        headers.insert("To".to_string(), String::new());
        let subject = parsed_headers
            .get_first_value("subject")
            .unwrap_or_default();
        headers.insert("Subject".to_string(), format!("Fwd: {}", subject));

        let mut parts = vec![];
        Msg::extract_text_bodies_into(&msg, "text/plain", &mut parts);
        if parts.is_empty() {
            Msg::extract_text_bodies_into(&msg, "text/html", &mut parts);
        }

        let mut body = String::from("-------- Forwarded Message --------\n");
        body.push_str(&parts.join("\r\n\r\n").replace("\r", ""));

        let mut tpl = Self {
            headers,
            body: Some(body),
            signature: ctx.config.signature(&ctx.account),
            raw: String::new(),
        };
        tpl.raw = tpl.to_string();
        tpl
    }

    pub fn mailto(ctx: &Ctx, url: &Url) -> Self {
        let mut headers = HashMap::new();

        let mut cc = Vec::new();
        let mut bcc = Vec::new();
        let mut subject = Cow::default();
        let mut body = Cow::default();

        for (key, val) in url.query_pairs() {
            match key.as_bytes() {
                b"cc" => {
                    cc.push(val);
                }
                b"bcc" => {
                    bcc.push(val);
                }
                b"subject" => {
                    subject = val;
                }
                b"body" => {
                    body = val;
                }
                _ => (),
            }
        }

        headers.insert(String::from("From"), ctx.config.address(ctx.account));
        headers.insert(String::from("To"), url.path().to_string());
        headers.insert(String::from("Subject"), subject.into());
        if !cc.is_empty() {
            headers.insert(String::from("Cc"), cc.join(", "));
        }
        if !bcc.is_empty() {
            headers.insert(String::from("Bcc"), cc.join(", "));
        }

        let mut tpl = Self {
            headers,
            body: Some(body.into()),
            signature: ctx.config.signature(&ctx.account),
            raw: String::new(),
        };
        tpl.raw = tpl.to_string();
        tpl
    }

    pub fn header<K: ToString, V: ToString>(&mut self, key: K, val: V) -> &Self {
        self.headers.insert(key.to_string(), val.to_string());
        self
    }

    pub fn body<T: ToString>(&mut self, body: T) -> &Self {
        self.body = Some(body.to_string());
        self
    }

    pub fn signature<T: ToString>(&mut self, signature: T) -> &Self {
        self.signature = Some(signature.to_string());
        self
    }
}

impl fmt::Display for Tpl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut tpl = TPL_HEADERS.iter().fold(String::new(), |mut tpl, &key| {
            if let Some(val) = self.headers.get(key) {
                tpl.push_str(&format!("{}: {}\n", key, val));
            };
            tpl
        });

        for (key, val) in self.headers.iter() {
            if !TPL_HEADERS.contains(&key.as_str()) {
                tpl.push_str(&format!("{}: {}\n", key, val));
            }
        }

        tpl.push_str("\n");

        if let Some(body) = self.body.as_ref() {
            tpl.push_str(&body);
        }

        if let Some(signature) = self.signature.as_ref() {
            tpl.push_str("\n\n");
            tpl.push_str(&signature);
        }

        write!(f, "{}", tpl)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::model::{Account, Config},
        ctx::Ctx,
        msg::tpl::model::Tpl,
        output::model::Output,
    };

    #[test]
    fn new_tpl() {
        let account = Account {
            name: Some(String::from("Test")),
            email: String::from("test@localhost"),
            ..Account::default()
        };
        let config = Config {
            accounts: vec![(String::from("account"), account.clone())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let output = Output::default();
        let mbox = String::default();
        let arg_matches = clap::ArgMatches::default();
        let ctx = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let tpl = Tpl::new(&ctx);

        assert_eq!(
            "From: Test <test@localhost>\nTo: \nSubject: \n\n",
            tpl.to_string()
        );
    }

    #[test]
    fn new_tpl_with_signature() {
        let account = Account {
            name: Some(String::from("Test")),
            email: String::from("test@localhost"),
            signature: Some(String::from("-- \nCordialement,")),
            ..Account::default()
        };
        let config = Config {
            accounts: vec![(String::from("account"), account.clone())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let output = Output::default();
        let mbox = String::default();
        let arg_matches = clap::ArgMatches::default();
        let ctx = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let tpl = Tpl::new(&ctx);

        assert_eq!(
            "From: Test <test@localhost>\nTo: \nSubject: \n\n\n\n-- \nCordialement,",
            tpl.to_string()
        );
    }

    #[test]
    fn reply_tpl() {
        let account = Account {
            name: Some(String::from("Test")),
            email: String::from("test@localhost"),
            ..Account::default()
        };
        let config = Config {
            accounts: vec![(String::from("account"), account.clone())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let output = Output::default();
        let mbox = String::default();
        let arg_matches = clap::ArgMatches::default();
        let ctx = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let parsed_mail = mailparse::parse_mail(
            b"Content-Type: text/plain\r\nFrom: Sender <sender@localhost>\r\nSubject: Test\r\n\r\nHello, world!",
        )
        .unwrap();
        let tpl = Tpl::reply(&ctx, &parsed_mail);

        assert_eq!(
            "From: Test <test@localhost>\nTo: Sender <sender@localhost>\nSubject: Re: Test\n\n>Hello, world!",
            tpl.to_string()
        );
    }

    #[test]
    fn reply_tpl_with_signature() {
        let account = Account {
            name: Some(String::from("Test")),
            email: String::from("test@localhost"),
            signature: Some(String::from("-- \nCordialement,")),
            ..Account::default()
        };
        let config = Config {
            accounts: vec![(String::from("account"), account.clone())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let output = Output::default();
        let mbox = String::default();
        let arg_matches = clap::ArgMatches::default();
        let ctx = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let parsed_mail = mailparse::parse_mail(
            b"Content-Type: text/plain\r\nFrom: Sender <sender@localhost>\r\nSubject: Test\r\n\r\nHello, world!",
        )
        .unwrap();
        let tpl = Tpl::reply(&ctx, &parsed_mail);

        assert_eq!(
            "From: Test <test@localhost>\nTo: Sender <sender@localhost>\nSubject: Re: Test\n\n>Hello, world!\n\n-- \nCordialement,",
            tpl.to_string()
        );
    }

    #[test]
    fn reply_all_tpl() {
        let account = Account {
            name: Some(String::from("To")),
            email: String::from("to@localhost"),
            ..Account::default()
        };
        let config = Config {
            accounts: vec![(String::from("account"), account.clone())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let output = Output::default();
        let mbox = String::default();
        let arg_matches = clap::ArgMatches::default();
        let ctx = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let parsed_mail = mailparse::parse_mail(
            b"Message-Id: 1\r
Content-Type: text/plain\r
From: From <from@localhost>\r
To: To <to@localhost>,to_bis@localhost\r
Cc: Cc <cc@localhost>, cc_bis@localhost\r
Subject: Test\r
\r
Hello, world!",
        )
        .unwrap();
        let tpl = Tpl::reply_all(&ctx, &parsed_mail);

        assert_eq!(
            "From: To <to@localhost>
To: From <from@localhost>, to_bis@localhost
In-Reply-To: 1
Cc: Cc <cc@localhost>, cc_bis@localhost
Subject: Re: Test

>Hello, world!",
            tpl.to_string()
        );
    }

    #[test]
    fn reply_all_tpl_with_signature() {
        let account = Account {
            name: Some(String::from("Test")),
            email: String::from("test@localhost"),
            signature: Some(String::from("-- \nCordialement,")),
            ..Account::default()
        };
        let config = Config {
            accounts: vec![(String::from("account"), account.clone())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let output = Output::default();
        let mbox = String::default();
        let arg_matches = clap::ArgMatches::default();
        let ctx = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let parsed_mail = mailparse::parse_mail(
            b"Content-Type: text/plain\r\nFrom: Sender <sender@localhost>\r\nSubject: Test\r\n\r\nHello, world!",
        )
        .unwrap();
        let tpl = Tpl::reply(&ctx, &parsed_mail);

        assert_eq!(
            "From: Test <test@localhost>\nTo: Sender <sender@localhost>\nSubject: Re: Test\n\n>Hello, world!\n\n-- \nCordialement,",
            tpl.to_string()
        );
    }

    #[test]
    fn forward_tpl() {
        let account = Account {
            name: Some(String::from("Test")),
            email: String::from("test@localhost"),
            ..Account::default()
        };
        let config = Config {
            accounts: vec![(String::from("account"), account.clone())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let output = Output::default();
        let mbox = String::default();
        let arg_matches = clap::ArgMatches::default();
        let ctx = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let parsed_mail = mailparse::parse_mail(
            b"Content-Type: text/plain\r\nFrom: Sender <sender@localhost>\r\nSubject: Test\r\n\r\nHello, world!",
        )
        .unwrap();
        let tpl = Tpl::forward(&ctx, &parsed_mail);

        assert_eq!(
            "From: Test <test@localhost>\nTo: \nSubject: Fwd: Test\n\n-------- Forwarded Message --------\nHello, world!",
            tpl.to_string()
        );
    }

    #[test]
    fn forward_tpl_with_signature() {
        let account = Account {
            name: Some(String::from("Test")),
            email: String::from("test@localhost"),
            signature: Some(String::from("-- \nCordialement,")),
            ..Account::default()
        };
        let config = Config {
            accounts: vec![(String::from("account"), account.clone())]
                .into_iter()
                .collect(),
            ..Config::default()
        };
        let output = Output::default();
        let mbox = String::default();
        let arg_matches = clap::ArgMatches::default();
        let ctx = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let parsed_mail = mailparse::parse_mail(
            b"Content-Type: text/plain\r\nFrom: Sender <sender@localhost>\r\nSubject: Test\r\n\r\nHello, world!",
        )
        .unwrap();
        let tpl = Tpl::forward(&ctx, &parsed_mail);

        assert_eq!(
            "From: Test <test@localhost>\nTo: \nSubject: Fwd: Test\n\n-------- Forwarded Message --------\nHello, world!\n\n-- \nCordialement,",
            tpl.to_string()
        );
    }
}
