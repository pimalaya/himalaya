//! Mozilla Thunderbird Autoconfiguration step of the wizard's
//! discovery chain. Tries ISP main, ISP fallback, and Thunderbird
//! ISPDB in series (secure variants only); each probe owns its own
//! spinner.

use log::debug;
use pimalaya_cli::{
    spinner::Spinner,
    wizard::{
        imap::{Encryption as ImapEncryption, ImapAuth, ImapSecret, WizardImapConfig},
        smtp::{Encryption as SmtpEncryption, SmtpAuth, SmtpSecret, WizardSmtpConfig},
    },
};
use pimconf::autoconfig::{
    client::{DiscoveryAutoconfigClientStd, DiscoveryAutoconfigClientStdError},
    types::{Autoconfig, SecurityType, Server, ServerType},
};

use crate::wizard::discover::{DiscoveryResult, discovery_resolver, discovery_tls};

struct Attempt<'a> {
    label: &'a str,
    run: &'a dyn Fn(
        &mut DiscoveryAutoconfigClientStd,
    ) -> Result<Autoconfig, DiscoveryAutoconfigClientStdError>,
}

pub fn run(local_part: &str, domain: &str) -> Option<Autoconfig> {
    let mut client =
        DiscoveryAutoconfigClientStd::new(discovery_resolver()).with_tls(discovery_tls());

    let attempts = [
        Attempt {
            label: "Autoconfig ISP main URL",
            run: &|c| c.isp(local_part, domain, true),
        },
        Attempt {
            label: "Autoconfig ISP fallback URL",
            run: &|c| c.isp_fallback(domain, true),
        },
        Attempt {
            label: "Thunderbird ISPDB",
            run: &|c| c.ispdb(domain, true),
        },
    ];

    for attempt in attempts {
        let spinner = Spinner::start(format!("Probing {} for {domain}…", attempt.label));

        match (attempt.run)(&mut client) {
            Ok(config) => {
                spinner.success(summary(domain, &config));
                return Some(config);
            }
            Err(err) => {
                debug!("{} for {domain} failed: {err}", attempt.label);
                spinner.failure(format!("{}: not available for {domain}", attempt.label));
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
