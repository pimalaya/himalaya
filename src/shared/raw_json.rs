//! # Raw JSON
//!
//! The output type of the protocol `request` passthroughs, printing the
//! payload a backend answered as is, and the URL they send to, kept on
//! the provider's API.

use std::fmt;

use anyhow::Result;
use pimalaya_cli::error::request_origin;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;
use url::Url;

/// The URL of `path` under the API at `base`: a path joined onto it, a
/// leading `/` dropped, or a full URL taken as is. A URL off the origin of
/// `base` is refused before any request, coded `request-origin`, so the
/// bearer token the request carries never leaves for another host.
pub fn api_url(base: &str, path: &str) -> Result<Url> {
    let base = Url::parse(base)?;
    let url = base.join(path.trim_start_matches('/'))?;

    if url.origin() != base.origin() {
        let url = url.origin().ascii_serialization();
        let api = base.origin().ascii_serialization();
        return Err(request_origin(&url, &api));
    }

    Ok(url)
}

/// A raw JSON payload: pretty on stdout, verbatim under `response` with
/// `--json`.
///
/// Wrapped rather than transparent, so the payload is a JSON object
/// whatever the backend answered: an array, or null for a 204.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RawJsonOutput {
    /// The response the backend answered, null when it carried no body.
    pub response: Value,
}

impl fmt::Display for RawJsonOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a 204 carries no body, which parses as null. Printing
        // "null" would read as a value the server sent; it sent nothing.
        if self.response.is_null() {
            return Ok(());
        }

        let pretty = serde_json::to_string_pretty(&self.response).map_err(|_| fmt::Error)?;
        writeln!(f, "{pretty}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::error::{CommonCode, code_of};
    use serde_json::{Value, json};

    use super::RawJsonOutput;

    /// A path joins the API, a full URL on it stays as given, and a URL off
    /// its origin is refused coded `request-origin`, whatever its case or
    /// spelling, the bearer token never leaving for another host.
    #[test]
    fn a_url_off_the_api_is_refused() {
        let base = "https://graph.microsoft.com/v1.0/";
        let url = |path| super::api_url(base, path).map(|url| url.to_string());

        let me = "https://graph.microsoft.com/v1.0/me";
        assert_eq!(url("/me").unwrap(), me);
        assert_eq!(url(me).unwrap(), me);
        for path in [
            "https://evil.example/v1.0/me",
            "HTTPS://evil.example/me",
            "http://graph.microsoft.com/v1.0/me",
            "https://graph.microsoft.com.evil.example/me",
        ] {
            let err = super::api_url(base, path).unwrap_err();
            let code = code_of::<CommonCode>(&err);
            assert_eq!(code, Some(CommonCode::RequestOrigin), "{path}: {err}");
            assert!(err.to_string().contains("off the API origin"), "{err}");
        }
    }

    /// A body prints pretty; none prints nothing.
    #[test]
    fn an_empty_response_prints_nothing() {
        let output = |response| RawJsonOutput { response }.to_string();

        assert_eq!(output(Value::Null), "");
        assert_eq!(output(json!({"id": "x"})), "{\n  \"id\": \"x\"\n}\n");
    }
}
