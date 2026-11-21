//! # pimdir attachment
//!
//! The `pimdir attachment` command family, over the files an account's
//! messages attach: the stand-ins a store records in the account's
//! attachments collection as each message body enters it (pimdir STORAGE
//! §14.3), read with the message attaching each.
//!
//! A listing reads the store's index, so it holds every attachment whose
//! message body was ever stored, newest first, and none of a body never
//! stored. Only a page's bodies are opened, to number each part as
//! `attachment download` takes it.

pub mod cli;
pub mod list;
pub mod search;

use std::collections::{HashMap, hash_map::Entry};

use anyhow::{Context, Result, bail};
use io_pimdir::client::reader::{PimdirAccountAttachment, PimdirAccountAttachmentCursor};

use crate::{
    error::{page_token_invalid, pimdir_error},
    pimdir::client::PimdirClient,
    unified::message::part,
};

/// How many stand-ins a search reads per keyset page, so a sparse match
/// does not cost a query per row.
const SEARCH_BATCH: usize = 500;

/// One page of an account's attachments, and where the next one resumes.
#[derive(Debug)]
pub struct PimdirAttachmentPage {
    /// The attachments, newest message first.
    pub attachments: Vec<PimdirAccountAttachment>,
    /// The cursor of the next page, `None` on the last one.
    pub next: Option<PimdirAccountAttachmentCursor>,
}

impl PimdirClient {
    /// One page of the account's attachments matching `query`, newest
    /// message first, from after `after` or from the newest.
    ///
    /// Read one entry past `limit`, so the last page has no next cursor.
    pub fn list_attachments(
        &self,
        after: Option<&PimdirAccountAttachmentCursor>,
        limit: usize,
        query: Option<&AttachmentQuery>,
    ) -> Result<PimdirAttachmentPage> {
        let wanted = limit.max(1);
        let batch = match query {
            None => wanted + 1,
            Some(_) => SEARCH_BATCH.max(wanted + 1),
        };

        let mut attachments = Vec::new();
        let mut after = after.cloned();
        'walk: loop {
            let page = self
                .store
                .list_attachments_by_account(self.account.as_deref(), after.as_ref(), batch)
                .map_err(pimdir_error)
                .context("List the account's attachments")?;
            let read = page.len();
            for attachment in page {
                after = Some(attachment.cursor());
                if query.is_none_or(|query| query.matches(&attachment)) {
                    attachments.push(attachment);
                    if attachments.len() > wanted {
                        break 'walk;
                    }
                }
            }
            if read < batch {
                break;
            }
        }

        let next = match attachments.len() > wanted {
            true => {
                attachments.truncate(wanted);
                attachments.last().map(PimdirAccountAttachment::cursor)
            }
            false => None,
        };

        Ok(PimdirAttachmentPage { attachments, next })
    }

    /// The part id `attachment download` takes for each of `attachments`,
    /// in order, each body read once.
    ///
    /// `None` where the body is not local, does not parse, or holds no
    /// leaf part at that section.
    pub fn part_ids(&self, attachments: &[PimdirAccountAttachment]) -> Result<Vec<Option<String>>> {
        let mut parts: HashMap<(&str, i64), Vec<(String, String)>> = HashMap::new();
        let mut ids = Vec::with_capacity(attachments.len());

        for attachment in attachments {
            let key = (
                attachment.message_collection.as_str(),
                attachment.message_seq,
            );
            let parts = match parts.entry(key) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(self.sections(key.0, key.1)?),
            };
            let section = attachment
                .summary
                .as_ref()
                .and_then(|summary| summary.part.as_deref());
            let id = parts
                .iter()
                .find(|(at, _)| Some(at.as_str()) == section)
                .map(|(_, id)| id.clone());
            ids.push(id);
        }

        Ok(ids)
    }

    /// Each leaf part's section and id in the body of the message `seq`
    /// of `collection`, none when the body is not local or does not parse.
    fn sections(&self, collection: &str, seq: i64) -> Result<Vec<(String, String)>> {
        let item = self
            .store
            .get_item(collection, seq)
            .map_err(pimdir_error)
            .with_context(|| format!("Read `{seq}` in `{collection}`"))?;
        let body = match item.and_then(|item| item.object) {
            Some(hash) => self.blobs.get(&hash)?,
            None => None,
        };
        let Some(body) = body else {
            return Ok(Vec::new());
        };
        let Ok(message) = part::parse(&body) else {
            return Ok(Vec::new());
        };

        Ok(part::leaves(&message)
            .filter_map(|leaf| Some((leaf.section?, leaf.id)))
            .collect())
    }
}

/// What `pimdir attachment search` matches: words each found, ignoring
/// case, in the file name or the sender's address or name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttachmentQuery {
    /// The words, lowercased.
    words: Vec<String>,
}

impl AttachmentQuery {
    /// The query of the words typed, refused when there are none.
    pub fn new(typed: &str) -> Result<Self> {
        let words: Vec<String> = typed.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            bail!("Nothing to search: give at least one word");
        }
        Ok(Self { words })
    }

    /// Whether every word appears in the file name, the sender's address
    /// or the sender's name.
    pub fn matches(&self, attachment: &PimdirAccountAttachment) -> bool {
        let fields: Vec<String> = [
            attachment
                .summary
                .as_ref()
                .map(|summary| summary.name.as_str()),
            attachment.sender.as_deref(),
            attachment.sender_name.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(str::to_lowercase)
        .collect();

        self.words
            .iter()
            .all(|word| fields.iter().any(|field| field.contains(word.as_str())))
    }
}

/// The page token of a cursor, opaque to a caller: the stand-in's public
/// id, a dot, then the message's sort key, verbatim.
pub fn page_token(cursor: &PimdirAccountAttachmentCursor) -> String {
    format!("{}.{}", cursor.seq, cursor.message_sort_key)
}

/// The cursor a page token names, refused when it is not one this
/// command printed.
pub fn parse_page_token(token: &str) -> Result<PimdirAccountAttachmentCursor> {
    let invalid = || page_token_invalid(token);
    let (seq, key) = token.split_once('.').ok_or_else(invalid)?;
    let seq = seq.parse::<i64>().ok().filter(|seq| *seq > 0);
    let seq = seq.ok_or_else(invalid)?;
    if key.chars().any(char::is_control) {
        return Err(invalid());
    }
    Ok(PimdirAccountAttachmentCursor {
        message_sort_key: key.to_owned(),
        seq,
    })
}

#[cfg(test)]
mod tests {
    use io_pimdir::{placement::PimdirLinkId, summary::file::PimdirFileSummary};

    use super::*;

    fn attachment(
        name: &str,
        sender: Option<&str>,
        sender_name: Option<&str>,
    ) -> PimdirAccountAttachment {
        PimdirAccountAttachment {
            collection: "\u{1}attachments".into(),
            seq: 7,
            link_id: PimdirLinkId("part:a@x.test#2".into()),
            summary: Some(PimdirFileSummary {
                name: name.into(),
                media_type: Some("application/pdf".into()),
                size: Some(9),
                part: Some("2".into()),
            }),
            message: PimdirLinkId("a@x.test".into()),
            message_collection: "imap/INBOX".into(),
            message_seq: 3,
            message_sort_key: "2026-10-05T09:00:00Z".into(),
            sender: sender.map(String::from),
            sender_name: sender_name.map(String::from),
            date: Some("2026-10-05T09:00:00Z".into()),
        }
    }

    #[test]
    fn a_page_token_reads_back_as_its_cursor() {
        for key in ["2026-10-05T09:00:00Z", ""] {
            let cursor = PimdirAccountAttachmentCursor {
                message_sort_key: key.into(),
                seq: 42,
            };
            assert_eq!(parse_page_token(&page_token(&cursor)).unwrap(), cursor);
        }
    }

    #[test]
    fn a_page_token_not_printed_is_refused() {
        for token in ["", "42", "x.2026", "0.2026", "-1.2026", "4\u{0}.2", "4.\n"] {
            assert!(parse_page_token(token).is_err(), "{token:?}");
        }
    }

    #[test]
    fn every_word_matches_the_name_or_the_sender() {
        let found = attachment(
            "Facture-Octobre.PDF",
            Some("billing@shop.test"),
            Some("Élise Martin"),
        );
        let query = |typed: &str| AttachmentQuery::new(typed).unwrap().matches(&found);

        assert!(query("facture"));
        assert!(query("OCTOBRE pdf"));
        assert!(query("billing"));
        assert!(query("élise"));
        assert!(query("facture élise"));
        assert!(!query("facture bob"));
        assert!(!query("elise"));
    }

    #[test]
    fn a_query_of_no_word_is_refused() {
        assert!(AttachmentQuery::new(" \t").is_err());
    }

    #[test]
    fn a_stand_in_with_no_summary_or_sender_matches_nothing() {
        let mut found = attachment("", None, None);
        found.summary = None;
        assert!(!AttachmentQuery::new("a").unwrap().matches(&found));
    }
}
