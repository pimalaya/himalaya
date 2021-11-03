use mailparse::MailHeaderMap;
use serde::Serialize;
use std::ops::{Deref, DerefMut};

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

impl<'a> From<&'a mailparse::ParsedMail<'a>> for Parts {
    fn from(part: &'a mailparse::ParsedMail<'a>) -> Self {
        let mut parts = vec![];
        build_parts_map_rec(part, &mut parts);
        Self(parts)
    }
}

fn build_parts_map_rec(part: &mailparse::ParsedMail, parts: &mut Vec<Part>) {
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
        part.subparts
            .iter()
            .for_each(|part| build_parts_map_rec(part, parts));
    }
}
