//! # HTTP script
//!
//! A server answering each request with the next scripted response, for
//! the tests driving the HTTP clients of the Gmail, Graph and JMAP backends
//! without a network.

use std::{
    collections::VecDeque,
    io::{Cursor, Read, Result, Write},
    sync::{Arc, Mutex},
};

/// A stream answering each request with the next scripted response,
/// keeping the requests it reads.
pub struct Script {
    responses: VecDeque<Vec<u8>>,
    response: Cursor<Vec<u8>>,
    requests: Arc<Mutex<Vec<String>>>,
    answered: bool,
}

impl Script {
    /// A stream answering `responses` in order, beside the requests it
    /// reads, one string each.
    pub fn new(responses: Vec<Vec<u8>>) -> (Self, Arc<Mutex<Vec<String>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let script = Self {
            responses: responses.into(),
            response: Cursor::default(),
            requests: requests.clone(),
            answered: true,
        };

        (script, requests)
    }
}

impl Read for Script {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.answered {
            let response = self.responses.pop_front().unwrap_or_default();
            self.response = Cursor::new(response);
            self.answered = true;
        }

        self.response.read(buf)
    }
}

impl Write for Script {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut requests = self.requests.lock().unwrap();

        if self.answered {
            requests.push(String::new());
            self.answered = false;
        }

        let request = requests.last_mut().unwrap();
        request.push_str(&String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// A 200 response carrying `body` as `content_type`.
pub fn ok(content_type: &str, body: &str) -> Vec<u8> {
    let len = body.len();
    let head = format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}");
    format!("{head}\r\nContent-Length: {len}\r\n\r\n{body}").into_bytes()
}

/// A response of status `code` carrying the JSON `body`, as a provider
/// refusing a request answers.
pub fn refused(code: u16, body: &str) -> Vec<u8> {
    let len = body.len();
    let head = format!("HTTP/1.1 {code} Refused\r\nContent-Type: application/json");
    format!("{head}\r\nContent-Length: {len}\r\n\r\n{body}").into_bytes()
}
