use anyhow::{Context, Result};
use log::debug;
use mailparse::{self, MailHeaderMap};
use std::{env, fs, path::PathBuf};

pub fn draft_path() -> PathBuf {
    let path = env::temp_dir().join("himalaya-draft.mail");
    debug!("draft path: `{:?}`", path);
    path
}

pub fn remove_draft() -> Result<()> {
    let path = draft_path();
    debug!("remove draft path: `{:?}`", path);
    fs::remove_file(&path).context(format!("cannot delete draft file at `{:?}`", path))
}

fn find_parts_by_mime_rec(part: &mailparse::ParsedMail, mime: &str, parts: &mut Vec<String>) {
    match part.subparts.len() {
        0 => {
            let content_type = part
                .get_headers()
                .get_first_value("content-type")
                .unwrap_or_default();

            if content_type.starts_with(mime) {
                parts.push(part.get_body().unwrap_or_default())
            }
        }
        _ => {
            part.subparts
                .iter()
                .for_each(|part| find_parts_by_mime_rec(part, mime, parts));
        }
    }
}

fn find_parts_by_mime(msg: &mailparse::ParsedMail, mime: &str) -> Result<Vec<String>> {
    let mut parts = vec![];
    find_parts_by_mime_rec(msg, mime, &mut parts);
    Ok(parts)
}

pub fn join_text_parts(msg: &mailparse::ParsedMail, mime: &str) -> Result<String> {
    let text_bodies = find_parts_by_mime(msg, &format!("text/{}", mime))?;
    Ok(text_bodies.join("\n\n"))
}
