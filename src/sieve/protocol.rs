use std::{fmt, str};

use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::Serialize;

/// A capability advertised by a ManageSieve server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SieveCapability {
    pub name: String,
    pub values: Vec<String>,
}

/// One script name returned by `LISTSCRIPTS`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SieveScript {
    pub name: String,
    pub active: bool,
}

/// The final status of one ManageSieve command response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SieveStatus {
    Ok,
    No,
    Bye,
}

impl SieveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::No => "NO",
            Self::Bye => "BYE",
        }
    }
}

/// One logical data line in a server response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SieveData {
    /// The decoded value for a literal, or the raw response line for a
    /// quoted/atom response item.
    pub bytes: Vec<u8>,
    /// Whether `bytes` came from a RFC 5804 literal. Non-literal data
    /// keeps its original line in `bytes` so callers can parse strings
    /// and atoms without losing their framing.
    pub literal: bool,
    /// Tokens that preceded a literal marker on the response line.
    prefix: Vec<u8>,
    /// Tokens that followed the literal payload on its terminating line.
    suffix: Vec<u8>,
}

impl SieveData {
    pub fn line(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            literal: false,
            prefix: Vec::new(),
            suffix: Vec::new(),
        }
    }

    pub fn literal(prefix: Vec<u8>, bytes: Vec<u8>, suffix: Vec<u8>) -> Self {
        Self {
            bytes,
            literal: true,
            prefix,
            suffix,
        }
    }

    /// Decodes the strings and atoms in this logical response line.
    /// Literal payloads are one string token even when they contain spaces,
    /// quotes, or newlines.
    pub fn tokens(&self) -> Result<Vec<Vec<u8>>> {
        if !self.literal {
            return parse_tokens(&self.bytes);
        }

        let mut tokens = parse_tokens(&self.prefix)?;
        tokens.push(self.bytes.clone());
        tokens.extend(parse_tokens(&self.suffix)?);
        Ok(tokens)
    }

    /// Renders the logical line for the diagnostic raw command.
    pub fn to_text(&self) -> Vec<u8> {
        if !self.literal {
            return self.bytes.clone();
        }

        let mut line = self.prefix.clone();
        if !line.is_empty() {
            line.push(b' ');
        }
        line.extend_from_slice(&self.bytes);
        line.extend_from_slice(&self.suffix);
        line
    }
}

/// A complete response: zero or more data items followed by a status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SieveResponse {
    pub data: Vec<SieveData>,
    pub status: SieveStatus,
    pub detail: Vec<u8>,
}

impl SieveResponse {
    pub fn ensure_ok(self, operation: &str) -> Result<Self> {
        if self.status != SieveStatus::Ok {
            let detail = String::from_utf8_lossy(&self.detail);
            bail!(
                "ManageSieve {operation} failed with {}{}",
                self.status.as_str(),
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            );
        }

        Ok(self)
    }

    /// Renders a diagnostic response for `sieve raw` while retaining
    /// literal payloads as bytes and separating response items by lines.
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();
        for item in &self.data {
            lines.push(String::from_utf8_lossy(&item.to_text()).into_owned());
        }
        if !self.detail.is_empty() {
            lines.push(String::from_utf8_lossy(&self.detail).into_owned());
        } else {
            lines.push(self.status.as_str().to_string());
        }
        lines.join("\n")
    }
}

impl fmt::Display for SieveCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.values.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{} {}", self.name, self.values.join(" "))
        }
    }
}

/// Parses a ManageSieve quoted-string/atom line into tokens.
pub fn parse_tokens(line: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < line.len() {
        while line.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if index == line.len() {
            break;
        }

        if line[index] == b'"' {
            index += 1;
            let mut token = Vec::new();
            let mut closed = false;

            while index < line.len() {
                match line[index] {
                    b'"' => {
                        index += 1;
                        closed = true;
                        break;
                    }
                    b'\\' => {
                        index += 1;
                        let Some(byte) = line.get(index).copied() else {
                            bail!("unterminated ManageSieve quoted string")
                        };
                        token.push(byte);
                        index += 1;
                    }
                    byte => {
                        if byte == b'\r' || byte == b'\n' {
                            bail!("newline in ManageSieve quoted string")
                        }
                        token.push(byte);
                        index += 1;
                    }
                }
            }

            if !closed {
                bail!("unterminated ManageSieve quoted string")
            }
            tokens.push(token);
        } else {
            let start = index;
            while line
                .get(index)
                .is_some_and(|byte| !byte.is_ascii_whitespace())
            {
                index += 1;
            }
            tokens.push(line[start..index].to_vec());
        }
    }

    Ok(tokens)
}

/// Quotes a string for a ManageSieve command.
pub fn quote_string(value: &[u8]) -> Result<Vec<u8>> {
    let mut quoted = Vec::with_capacity(value.len() + 2);
    quoted.push(b'"');

    for &byte in value {
        if byte == b'\r' || byte == b'\n' || byte == 0 || byte < 0x20 {
            bail!("ManageSieve quoted strings cannot contain control characters")
        }
        if byte == b'"' || byte == b'\\' {
            quoted.push(b'\\');
        }
        quoted.push(byte);
    }

    quoted.push(b'"');
    Ok(quoted)
}

pub fn parse_capabilities(items: &[SieveData]) -> Result<Vec<SieveCapability>> {
    items
        .iter()
        .map(|item| {
            let tokens = item.tokens()?;
            let Some(name) = tokens.first() else {
                bail!("empty ManageSieve capability response")
            };
            let name = str::from_utf8(name)
                .map_err(|_| anyhow::anyhow!("ManageSieve capability name is not UTF-8"))?
                .to_string();

            let mut values = Vec::new();
            for value in tokens.iter().skip(1) {
                let value = str::from_utf8(value)
                    .map_err(|_| anyhow::anyhow!("ManageSieve capability value is not UTF-8"))?;
                values.extend(value.split_whitespace().map(str::to_owned));
            }

            Ok(SieveCapability { name, values })
        })
        .collect()
}

pub fn parse_script(item: &SieveData) -> Result<SieveScript> {
    let tokens = item.tokens()?;
    let Some(name) = tokens.first() else {
        bail!("empty ManageSieve script response")
    };
    let name = str::from_utf8(name)
        .map_err(|_| anyhow::anyhow!("ManageSieve script name is not UTF-8"))?
        .to_string();
    let active = tokens
        .iter()
        .skip(1)
        .any(|token| token.eq_ignore_ascii_case(b"ACTIVE"));

    Ok(SieveScript { name, active })
}

/// Decodes a response item that contains one ManageSieve string.
pub fn parse_string(item: &SieveData) -> Result<Vec<u8>> {
    let tokens = item.tokens()?;
    if tokens.len() != 1 {
        bail!("expected one ManageSieve string response")
    }
    Ok(tokens[0].clone())
}

/// Finds a trailing `{size}` or `{size+}` literal marker.
pub fn literal_size(line: &[u8]) -> Option<usize> {
    literal_marker(line).map(|(_, size)| size)
}

/// Finds the start and size of a trailing `{size}` or `{size+}` marker.
pub fn literal_marker(line: &[u8]) -> Option<(usize, usize)> {
    let line = line.trim_ascii_end();
    let end = line.len().checked_sub(1)?;
    (line[end] == b'}').then_some(())?;
    let start = line.iter().rposition(|&byte| byte == b'{')?;
    if start > 0 && !line[start - 1].is_ascii_whitespace() {
        return None;
    }
    let marker = &line[start + 1..end];
    let marker = marker.strip_suffix(b"+").unwrap_or(marker);
    if marker.is_empty() || !marker.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some((start, str::from_utf8(marker).ok()?.parse().ok()?))
}

pub fn status(line: &[u8]) -> Option<SieveStatus> {
    let token = line.split(|byte| byte.is_ascii_whitespace()).next()?;
    if token.eq_ignore_ascii_case(b"OK") {
        Some(SieveStatus::Ok)
    } else if token.eq_ignore_ascii_case(b"NO") {
        Some(SieveStatus::No)
    } else if token.eq_ignore_ascii_case(b"BYE") {
        Some(SieveStatus::Bye)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_capabilities_and_values() {
        let items = vec![
            SieveData::line(b"SIEVE \"fileinto vacation\"".to_vec()),
            SieveData::line(b"IMPLEMENTATION \"Dovecot\"".to_vec()),
        ];

        let capabilities = parse_capabilities(&items).unwrap();
        assert_eq!(capabilities[0].name, "SIEVE");
        assert_eq!(capabilities[0].values, ["fileinto", "vacation"]);
        assert_eq!(capabilities[1].values, ["Dovecot"]);
    }

    #[test]
    fn parses_active_script_and_escapes_names() {
        let item = SieveData::line(b"\"vac\\\\ation\" ACTIVE".to_vec());
        let script = parse_script(&item).unwrap();
        assert_eq!(script.name, "vac\\ation");
        assert!(script.active);
        assert_eq!(
            quote_string(script.name.as_bytes()).unwrap(),
            b"\"vac\\\\ation\""
        );
    }

    #[test]
    fn parses_literal_script_name_with_suffix() {
        let item = SieveData::literal(Vec::new(), b"vacation script".to_vec(), b" ACTIVE".to_vec());

        let script = parse_script(&item).unwrap();
        assert_eq!(script.name, "vacation script");
        assert!(script.active);
    }

    #[test]
    fn parses_literal_capability_name_with_value() {
        let item = SieveData::literal(
            Vec::new(),
            b"IMPLEMENTATION".to_vec(),
            b" \"Dovecot\"".to_vec(),
        );

        let capabilities = parse_capabilities(&[item]).unwrap();
        assert_eq!(capabilities[0].name, "IMPLEMENTATION");
        assert_eq!(capabilities[0].values, ["Dovecot"]);
    }

    #[test]
    fn recognizes_literal_markers() {
        assert_eq!(literal_size(b"{42}"), Some(42));
        assert_eq!(literal_size(b"{42+}"), Some(42));
        assert_eq!(literal_size(b"OK {7}"), Some(7));
        assert_eq!(literal_size(b"OK {7}   "), Some(7));
        assert_eq!(literal_size(b"prefix{7}"), None);
        assert_eq!(literal_size(br#"\"{7}\""#), None);
        assert_eq!(literal_size(b"{x}"), None);
    }
}
