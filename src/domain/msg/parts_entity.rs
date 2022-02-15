use anyhow::{anyhow, Context, Result};
use mailparse::MailHeaderMap;
use serde::Serialize;
use std::{
    env, fs,
    ops::{Deref, DerefMut},
};
use uuid::Uuid;

use crate::config::AccountConfig;

#[derive(Debug, Clone, Default, Serialize)]
pub struct TextPlainPart {
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TextHtmlPart {
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BinaryPart {
    pub filename: String,
    pub mime: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    TextPlain(TextPlainPart),
    TextHtml(TextHtmlPart),
    Binary(BinaryPart),
}

impl Part {
    pub fn new_text_plain(content: String) -> Self {
        Self::TextPlain(TextPlainPart { content })
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parts(pub Vec<Part>);

impl Parts {
    pub fn replace_text_plain_parts_with(&mut self, part: TextPlainPart) {
        self.retain(|part| !matches!(part, Part::TextPlain(_)));
        self.push(Part::TextPlain(part));
    }

    pub fn from_parsed_mail<'a>(
        account: &'a AccountConfig,
        part: &'a mailparse::ParsedMail<'a>,
    ) -> Result<Self> {
        let mut parts = vec![];
        build_parts_map_rec(account, part, &mut parts)?;
        Ok(Self(parts))
    }
}

impl Deref for Parts {
    type Target = Vec<Part>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Parts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn build_parts_map_rec(
    account: &AccountConfig,
    parsed_mail: &mailparse::ParsedMail,
    parts: &mut Vec<Part>,
) -> Result<()> {
    if parsed_mail.subparts.is_empty() {
        let cdisp = parsed_mail.get_content_disposition();
        match cdisp.disposition {
            mailparse::DispositionType::Attachment => {
                let filename = cdisp
                    .params
                    .get("filename")
                    .map(String::from)
                    .unwrap_or_else(|| String::from("noname"));
                let content = parsed_mail.get_body_raw().unwrap_or_default();
                let mime = tree_magic::from_u8(&content);
                parts.push(Part::Binary(BinaryPart {
                    filename,
                    mime,
                    content,
                }));
            }
            // TODO: manage other use cases
            _ => {
                if let Some(ctype) = parsed_mail.get_headers().get_first_value("content-type") {
                    let content = parsed_mail.get_body().unwrap_or_default();
                    if ctype.starts_with("text/plain") {
                        parts.push(Part::TextPlain(TextPlainPart { content }))
                    } else if ctype.starts_with("text/html") {
                        parts.push(Part::TextHtml(TextHtmlPart { content }))
                    }
                };
            }
        };
    } else {
        let ctype = parsed_mail
            .get_headers()
            .get_first_value("content-type")
            .ok_or_else(|| anyhow!("cannot get content type of multipart"))?;
        if ctype.starts_with("multipart/encrypted") {
            let decrypted_part = parsed_mail
                .subparts
                .get(1)
                .ok_or_else(|| anyhow!("cannot find encrypted part of multipart"))
                .and_then(|part| decrypt_part(account, part))
                .context("cannot decrypt part of multipart")?;
            let parsed_mail = mailparse::parse_mail(decrypted_part.as_bytes())
                .context("cannot parse decrypted part of multipart")?;
            build_parts_map_rec(account, &parsed_mail, parts)?;
        } else {
            for part in parsed_mail.subparts.iter() {
                build_parts_map_rec(account, part, parts)?;
            }
        }
    }

    Ok(())
}

fn decrypt_part(account: &AccountConfig, msg: &mailparse::ParsedMail) -> Result<String> {
    let msg_path = env::temp_dir().join(Uuid::new_v4().to_string());
    let msg_body = msg
        .get_body()
        .context("cannot get body from encrypted part")?;
    fs::write(msg_path.clone(), &msg_body)
        .context(format!("cannot write encrypted part to temporary file"))?;
    account
        .pgp_decrypt_file(msg_path.clone())?
        .ok_or_else(|| anyhow!("cannot find pgp decrypt command in config"))
}
