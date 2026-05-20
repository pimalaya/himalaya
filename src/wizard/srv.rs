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

//! RFC 6186 SRV step of the wizard's discovery chain. Runs the three
//! `_imap._tcp` / `_imaps._tcp` / `_submission._tcp` lookups under
//! one spinner and assembles them into a [`DiscoveryResult`].
//!
//! IMAP: prefers `_imaps` (implicit TLS) over `_imap` (StartTls).
//! SMTP: from `_submission`; the encryption is inferred from the
//! record's port (465 → implicit TLS, otherwise StartTls).

use io_discovery::rfc6186::{
    client::DiscoverySrvClientStd,
    types::{SrvReport, SrvService},
};
use log::debug;
use pimalaya_cli::{
    spinner::Spinner,
    wizard::{
        imap::{Encryption as ImapEncryption, ImapAuth, ImapSecret, WizardImapConfig},
        smtp::{Encryption as SmtpEncryption, SmtpAuth, SmtpSecret, WizardSmtpConfig},
    },
};

use crate::wizard::discover::{discovery_resolver, DiscoveryResult};

pub fn run(domain: &str) -> Option<SrvReport> {
    let spinner = Spinner::start(format!("Probing SRV records for {domain}…"));
    let mut client = DiscoverySrvClientStd::new(discovery_resolver());

    match client.discover(domain) {
        Ok(report) if !is_empty(&report) => {
            spinner.success(summary(domain, &report));
            Some(report)
        }
        Ok(_) => {
            spinner.failure(format!("SRV: no records for {domain}"));
            None
        }
        Err(err) => {
            debug!("SRV discovery for {domain} failed: {err}");
            spinner.failure(format!("SRV: no records for {domain}"));
            None
        }
    }
}

pub fn defaults(report: &SrvReport) -> DiscoveryResult {
    let imap = report
        .imaps
        .as_ref()
        .map(|s| imap_from_service(s, ImapEncryption::Tls))
        .or_else(|| {
            report
                .imap
                .as_ref()
                .map(|s| imap_from_service(s, ImapEncryption::StartTls))
        });

    let smtp = report.submission.as_ref().map(smtp_from_service);

    DiscoveryResult {
        imap,
        smtp,
        jmap: None,
    }
}

fn summary(domain: &str, report: &SrvReport) -> String {
    let mut protos = Vec::with_capacity(2);

    if report.imap.is_some() || report.imaps.is_some() {
        protos.push("IMAP");
    }

    if report.submission.is_some() {
        protos.push("SMTP");
    }

    format!("SRV: discovered {} for {domain}", protos.join(" + "))
}

fn is_empty(report: &SrvReport) -> bool {
    report.imap.is_none() && report.imaps.is_none() && report.submission.is_none()
}

fn imap_from_service(service: &SrvService, encryption: ImapEncryption) -> WizardImapConfig {
    WizardImapConfig {
        host: service.host.clone(),
        port: service.port,
        encryption,
        login: String::new(),
        auth: ImapAuth::Password(ImapSecret::Raw(String::new().into())),
    }
}

fn smtp_from_service(service: &SrvService) -> WizardSmtpConfig {
    let encryption = if service.port == 465 {
        SmtpEncryption::Tls
    } else {
        SmtpEncryption::StartTls
    };

    WizardSmtpConfig {
        host: service.host.clone(),
        port: service.port,
        encryption,
        login: String::new(),
        auth: SmtpAuth::Password(SmtpSecret::Raw(String::new().into())),
    }
}
