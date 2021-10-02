use mailparse::MailHeaderMap;
use serde::Serialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub type PartsMap = HashMap<String, Vec<String>>;

#[derive(Debug, Default, Serialize)]
pub struct Parts(pub PartsMap);

impl Deref for Parts {
    type Target = PartsMap;

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
        let mut parts = HashMap::default();
        build_parts_map_rec(part, &mut parts);
        Self(parts)
    }
}

fn build_parts_map_rec(part: &mailparse::ParsedMail, parts: &mut PartsMap) {
    if part.subparts.is_empty() {
        part.get_headers()
            .get_first_value("content-type")
            .and_then(|ctype| {
                let ctype = ctype.split_at(ctype.find(';').unwrap_or(ctype.len())).0;
                if !parts.contains_key(ctype) {
                    parts.insert(ctype.into(), vec![]);
                }
                parts.get_mut(ctype)
            })
            .map(|parts| parts.push(part.get_body().unwrap_or_default()));
    } else {
        part.subparts
            .iter()
            .for_each(|part| build_parts_map_rec(part, parts));
    }
}
