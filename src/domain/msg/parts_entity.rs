use anyhow::{anyhow, Context, Result};
use log::{error, trace, warn};
use mailparse::MailHeaderMap;
use serde::Serialize;
use std::{
    env, fs,
    ops::{Deref, DerefMut},
};
use uuid::Uuid;

use crate::config::Account;

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

    pub fn replace_text_html_parts_with(&mut self, part: TextHtmlPart) {
        self.retain(|part| !matches!(part, Part::TextHtml(_)));
        self.push(Part::TextHtml(part));
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

impl<'a> From<(&'a Account, &'a mailparse::ParsedMail<'a>)> for Parts {
    fn from((account, part): (&'a Account, &'a mailparse::ParsedMail<'a>)) -> Self {
        let mut parts = vec![];
        build_parts_map_rec(account, part, &mut parts);
        Self(parts)
    }
}

fn build_parts_map_rec(account: &Account, part: &mailparse::ParsedMail, parts: &mut Vec<Part>) {
    if part.subparts.is_empty() {
        let content_disp = part.get_content_disposition();
        match content_disp.disposition {
            mailparse::DispositionType::Attachment => {
                let filename = content_disp
                    .params
                    .get("filename")
                    .map(String::from)
                    .unwrap_or_else(|| String::from("noname"));
                let content = part.get_body_raw().unwrap_or_default();
                let mime = tree_magic::from_u8(&content);
                parts.push(Part::Binary(BinaryPart {
                    filename,
                    mime,
                    content,
                }));
            }
            // TODO: manage other use cases
            _ => {
                if let Some(ctype) = part.get_headers().get_first_value("content-type") {
                    let content = part.get_body().unwrap_or_default();
                    if ctype.starts_with("text/plain") {
                        parts.push(Part::TextPlain(TextPlainPart { content }))
                    } else if ctype.starts_with("text/html") {
                        parts.push(Part::TextHtml(TextHtmlPart { content }))
                    }
                };
            }
        };
    } else {
        if let Some(ctype) = part.get_headers().get_first_value("content-type") {
            if ctype.starts_with("multipart/encrypted") {
                if let Some(encrypted_part) = part.subparts.get(1) {
                    match decrypt_part(account, encrypted_part) {
                        Ok(part) => match mailparse::parse_mail(part.as_bytes()) {
                            Ok(ref decrypted_part) => {
                                build_parts_map_rec(account, decrypted_part, parts)
                            }
                            Err(err) => {
                                error!("{}", err);
                                warn!("cannot parse decrypted content of a multipart/encrypted, skipping it");
                                trace!("part: {:?}", part);
                            }
                        },
                        Err(err) => {
                            error!("{}", err);
                            warn!("cannot decrypt content of a multipart/encrypted, skipping it");
                            trace!("encrypted part: {:?}", encrypted_part);
                        }
                    }
                } else {
                    warn!("cannot find encrypted content of a multipart/encrypted, skipping it");
                    trace!("multipart: {:?}", part);
                }
            } else {
                part.subparts
                    .iter()
                    .for_each(|part| build_parts_map_rec(account, part, parts));
            };
        } else {
            warn!("cannot find content type of a multipart, skipping it");
            trace!("part: {:?}", part);
        }
    }
}

fn decrypt_part(account: &Account, encrypted_part: &mailparse::ParsedMail) -> Result<String> {
    let encrypted_part_path = env::temp_dir().join(Uuid::new_v4().to_string());
    fs::write(
        encrypted_part_path.clone(),
        encrypted_part.get_body().unwrap().as_bytes(),
    )
    .with_context(|| {
        error!(
            "cannot write encrypted part to temporary file {:?}",
            encrypted_part_path
        );
        trace!("part: {:?}", encrypted_part);
        "cannot decrypt part"
    })?;
    account
        .pgp_decrypt_file(encrypted_part_path.clone())?
        .ok_or_else(|| anyhow!("cannot find pgp decrypt command in config"))
}

// TODO: this is a small POC for https://github.com/soywod/himalaya/issues/286
// #[cfg(test)]
// mod tests {
//     use lettre::message::SinglePart;
//     use pgp::{Deserializable, Message, SignedPublicKey, SignedSecretKey};
//     use rand::thread_rng;

//     #[test]
//     fn test_build_parts_map_rec() {
//         let (secret_key, _) =
//             SignedSecretKey::from_string(include_str!("../../../tests/keys/alice.asc"))
//                 .expect("cannot read alice's secret key");
//         let (pub_key, _) =
//             SignedPublicKey::from_string(include_str!("../../../tests/keys/patrick.pub.asc"))
//                 .expect("cannot read patrick's public key");
//         let encrypted_msg = Message::new_literal_bytes(
//             "",
//             SinglePart::plain(String::from("This message is encrypted."))
//                 .formatted()
//                 .as_slice(),
//         )
//         .sign(
//             &secret_key,
//             || String::new(),
//             pgp::crypto::HashAlgorithm::MD5,
//         )
//         .and_then(|msg| msg.compress(pgp::types::CompressionAlgorithm::ZIP))
//         .and_then(|msg| {
//             msg.encrypt_to_keys(
//                 &mut thread_rng(),
//                 pgp::crypto::SymmetricKeyAlgorithm::AES256,
//                 &[&pub_key],
//             )
//         });
//         let msg = encrypted_msg.unwrap();
//         let encoded_msg: Vec<u8> = msg.to_armored_bytes(None).unwrap();

//         assert_eq!("bad", String::from_utf8(encoded_msg).unwrap());
//     }
// }
