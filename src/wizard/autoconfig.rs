// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Mozilla Thunderbird Autoconfiguration step of the wizard's
//! discovery chain. Tries ISP main, ISP fallback, and Thunderbird
//! ISPDB in series (secure variants only); each probe owns its own
//! spinner.

use io_discovery::autoconfig::{
    client::DiscoveryAutoconfigClientStd,
    types::{Autoconfig, SecurityType, Server, ServerType},
};
use log::debug;
use pimalaya_cli::{
    spinner::Spinner,
    wizard::{
        imap::{Encryption as ImapEncryption, ImapAuth, ImapSecret, WizardImapConfig},
        smtp::{Encryption as SmtpEncryption, SmtpAuth, SmtpSecret, WizardSmtpConfig},
    },
};

use crate::wizard::discover::{discovery_resolver, discovery_tls, DiscoveryResult};

pub fn run(local_part: &str, domain: &str) -> Option<Autoconfig> {
    let mut client =
        DiscoveryAutoconfigClientStd::new(discovery_resolver()).with_tls(discovery_tls());

    let attempts: [(&str, &dyn Fn(&mut DiscoveryAutoconfigClientStd) -> _); 3] = [
        ("Autoconfig ISP main URL", &|c| {
            c.isp(local_part, domain, true)
        }),
        ("Autoconfig ISP fallback URL", &|c| {
            c.isp_fallback(domain, true)
        }),
        ("Thunderbird ISPDB", &|c| c.ispdb(domain, true)),
    ];

    for (label, run) in attempts {
        let spinner = Spinner::start(format!("Probing {label} for {domain}…"));

        match run(&mut client) {
            Ok(config) => {
                spinner.success(summary(domain, &config));
                return Some(config);
            }
            Err(err) => {
                debug!("{label} for {domain} failed: {err}");
                spinner.failure(format!("{label}: not available for {domain}"));
            }
        }
    }

    None
}

pub fn defaults(ac: &Autoconfig) -> DiscoveryResult {
    let imap = ac
        .email_provider
        .incoming_server
        .iter()
        .find(|s| matches!(s.r#type, ServerType::Imap))
        .and_then(imap_from_server);

    let smtp = ac
        .email_provider
        .outgoing_server
        .iter()
        .find(|s| matches!(s.r#type, ServerType::Smtp))
        .and_then(smtp_from_server);

    DiscoveryResult {
        imap,
        smtp,
        jmap: None,
    }
}

fn summary(domain: &str, ac: &Autoconfig) -> String {
    let has_imap = ac
        .email_provider
        .incoming_server
        .iter()
        .any(|s| matches!(s.r#type, ServerType::Imap));
    let has_smtp = ac
        .email_provider
        .outgoing_server
        .iter()
        .any(|s| matches!(s.r#type, ServerType::Smtp));

    let mut protos = Vec::with_capacity(2);

    if has_imap {
        protos.push("IMAP");
    }

    if has_smtp {
        protos.push("SMTP");
    }

    if protos.is_empty() {
        format!("Autoconfig: configuration found for {domain} (no IMAP/SMTP fields)")
    } else {
        format!("Autoconfig: discovered {} for {domain}", protos.join(" + "))
    }
}

fn imap_from_server(server: &Server) -> Option<WizardImapConfig> {
    let host = server.hostname.clone()?;
    let encryption = match server.socket_type {
        Some(SecurityType::Tls) => ImapEncryption::Tls,
        Some(SecurityType::Starttls) => ImapEncryption::StartTls,
        _ => ImapEncryption::None,
    };
    let port = server.port.unwrap_or(match encryption {
        ImapEncryption::Tls => 993,
        _ => 143,
    });

    Some(WizardImapConfig {
        host,
        port,
        encryption,
        login: String::new(),
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    })
}

fn smtp_from_server(server: &Server) -> Option<WizardSmtpConfig> {
    let host = server.hostname.clone()?;
    let encryption = match server.socket_type {
        Some(SecurityType::Tls) => SmtpEncryption::Tls,
        Some(SecurityType::Starttls) => SmtpEncryption::StartTls,
        _ => SmtpEncryption::None,
    };
    let port = server.port.unwrap_or(match encryption {
        SmtpEncryption::Tls => 465,
        SmtpEncryption::StartTls => 587,
        SmtpEncryption::None => 25,
    });

    Some(WizardSmtpConfig {
        host,
        port,
        encryption,
        login: String::new(),
        auth: SmtpAuth::Password(SmtpSecret::Raw(String::new().into())),
    })
}
