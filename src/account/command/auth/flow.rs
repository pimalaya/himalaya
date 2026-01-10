use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;
use tokio::time::timeout;
use url::Url;
use uuid::Uuid;

use super::error::AuthError;
use super::provider::{AuthProvider, ProviderConfig};

/// OAuth 2.0 tokens returned from the authorization server
#[derive(Debug, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

/// OAuth 2.0 authorization flow handler
pub struct OAuthFlow {
    provider: AuthProvider,
    account_name: String,
    client_id: String,
    client_secret: String,
    redirect_host: String,
    redirect_port: u16,
}

impl OAuthFlow {
    /// Create a new OAuth flow
    pub fn new(
        provider: AuthProvider,
        account_name: String,
        client_id: String,
        client_secret: String,
    ) -> Self {
        Self {
            provider,
            account_name,
            client_id,
            client_secret,
            redirect_host: "localhost".to_string(),
            redirect_port: 8080,
        }
    }

    /// Execute the complete OAuth 2.0 authorization flow
    pub async fn execute(&self) -> Result<OAuthTokens, AuthError> {
        let config = self.provider.config();

        // Generate PKCE parameters
        let (code_challenge, code_verifier) = Self::generate_pkce();

        // Generate state for CSRF protection
        let state = Self::generate_state();

        // Build authorization URL
        let auth_url = self.build_auth_url(&config, &state, &code_challenge)?;

        // Start local redirect server to catch the callback
        let listener = TcpListener::bind(format!("{}:{}", self.redirect_host, self.redirect_port))
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        let redirect_uri = format!(
            "http://{}:{}/callback",
            self.redirect_host, self.redirect_port
        );

        println!("📱 Opening browser for authentication...");

        // Try to open browser, or print URL if it fails
        if let Err(_) = self.open_browser(&auth_url).await {
            println!("\n⚠️  Could not open browser automatically.");
            println!("Please open this URL in your browser:");
            println!("\n{}\n", auth_url);
        }

        println!("⏳ Waiting for authorization response... (5 minute timeout)");

        // Wait for callback with timeout
        let (code, received_state) =
            timeout(Duration::from_secs(300), self.wait_for_callback(&listener))
                .await
                .map_err(|_| AuthError::CallbackTimeout)?
                .map_err(|e| e)?;

        // Validate state
        if received_state != state {
            return Err(AuthError::InvalidCallbackState);
        }

        println!("✓ Authorization received");

        // Exchange code for tokens
        println!("🔄 Exchanging authorization code for tokens...");
        let tokens = self
            .exchange_code_for_tokens(&config, &code, &code_verifier, &redirect_uri)
            .await?;

        println!("✓ Tokens obtained");

        Ok(tokens)
    }

    /// Generate PKCE (RFC 7636) code challenge and verifier
    fn generate_pkce() -> (String, String) {
        use sha2::{Digest, Sha256};

        // Generate a random 128-character string
        let code_verifier = Uuid::new_v4().to_string().replace("-", "");
        let code_verifier = format!("{}{}", code_verifier, Uuid::new_v4().to_string());

        // Create SHA256 hash of the verifier
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();

        // Base64 URL-encode the hash (without padding)
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        let code_challenge = URL_SAFE_NO_PAD.encode(&hash[..]);

        (code_challenge, code_verifier)
    }

    /// Generate a random state for CSRF protection
    fn generate_state() -> String {
        Uuid::new_v4().to_string()
    }

    /// Build the authorization URL
    fn build_auth_url(
        &self,
        config: &ProviderConfig,
        state: &str,
        code_challenge: &str,
    ) -> Result<String, AuthError> {
        let mut auth_url = Url::parse(config.auth_url)
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        let redirect_uri = format!(
            "http://{}:{}/callback",
            self.redirect_host, self.redirect_port
        );

        auth_url
            .query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &config.scopes_str())
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("access_type", "offline"); // Request refresh token

        Ok(auth_url.to_string())
    }

    /// Open browser using multiple strategies
    async fn open_browser(&self, url: &str) -> Result<(), AuthError> {
        // Try using the 'open' crate first
        open::that(url).map_err(|e| AuthError::BrowserOpenFailed(e.to_string()))?;
        Ok(())
    }

    /// Wait for OAuth callback from the redirect server
    async fn wait_for_callback(
        &self,
        listener: &TcpListener,
    ) -> Result<(String, String), AuthError> {
        // Set non-blocking mode to allow timeout
        listener
            .set_nonblocking(true)
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    // Parse HTTP request from the callback
                    let mut buffer = [0; 4096];
                    let n = stream
                        .read(&mut buffer)
                        .map_err(|e| AuthError::NetworkError(e.to_string()))?;

                    let request = String::from_utf8_lossy(&buffer[..n]);

                    // Extract code and state from query parameters
                    let (code, state) = self.parse_callback_request(&request)?;

                    // Send success response
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
                        <html><body><h1>✓ Authorization successful!</h1>\
                        <p>You can close this window and return to the terminal.</p>\
                        </body></html>";

                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();

                    return Ok((code, state));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection yet, wait a bit and retry
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    return Err(AuthError::NetworkError(e.to_string()));
                }
            }
        }
    }

    /// Parse authorization code and state from callback request
    fn parse_callback_request(&self, request: &str) -> Result<(String, String), AuthError> {
        // Look for the GET request line with query parameters
        let query_start = request
            .find('?')
            .ok_or_else(|| AuthError::TokenExchangeFailed("No query parameters".to_string()))?;

        let query_end = request[query_start..]
            .find(' ')
            .map(|i| query_start + i)
            .unwrap_or(request.len());

        let query_string = &request[query_start + 1..query_end];

        // Parse query parameters
        let params: std::collections::HashMap<_, _> = url::form_urlencoded::parse(query_string.as_bytes())
            .into_owned()
            .collect();

        let code = params
            .get("code")
            .ok_or_else(|| {
                AuthError::TokenExchangeFailed(
                    params
                        .get("error")
                        .cloned()
                        .unwrap_or_else(|| "No authorization code received".to_string()),
                )
            })?
            .clone();

        let state = params
            .get("state")
            .ok_or_else(|| AuthError::InvalidCallbackState)?
            .clone();

        Ok((code, state))
    }

    /// Exchange authorization code for access and refresh tokens
    async fn exchange_code_for_tokens(
        &self,
        config: &ProviderConfig,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, AuthError> {
        // For MVP, we'll use reqwest. If not available, we'll need to add it to Cargo.toml
        // For now, this will compile with a compilation error that we'll address

        use serde_json::json;

        let client = reqwest::Client::new();

        let body = json!({
            "grant_type": "authorization_code",
            "code": code,
            "client_id": &self.client_id,
            "client_secret": &self.client_secret,
            "redirect_uri": redirect_uri,
            "code_verifier": code_verifier,
        });

        let response = client
            .post(config.token_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AuthError::TokenExchangeFailed(error_text));
        }

        let token_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AuthError::TokenExchangeFailed(e.to_string()))?;

        let access_token = token_response
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AuthError::TokenExchangeFailed("No access_token in response".to_string())
            })?
            .to_string();

        let refresh_token = token_response
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let expires_in = token_response
            .get("expires_in")
            .and_then(|v| v.as_u64());

        Ok(OAuthTokens {
            access_token,
            refresh_token,
            expires_in,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_generation_is_unique() {
        let state1 = OAuthFlow::generate_state();
        let state2 = OAuthFlow::generate_state();
        assert_ne!(state1, state2);
        assert!(!state1.is_empty());
    }

    #[test]
    fn test_pkce_generation() {
        let (challenge, verifier) = OAuthFlow::generate_pkce();
        assert!(!challenge.is_empty());
        assert!(!verifier.is_empty());
        assert_ne!(challenge, verifier);
    }
}
