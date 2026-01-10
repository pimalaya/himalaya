use std::fmt;

/// OAuth authentication errors
#[derive(Debug)]
pub enum AuthError {
    /// Provider is not supported
    ProviderNotSupported(String),
    
    /// Invalid or missing client credentials
    InvalidClientCredentials(String),
    
    /// Failed to open browser
    BrowserOpenFailed(String),
    
    /// OAuth authorization callback timeout
    CallbackTimeout,
    
    /// Received callback with invalid or mismatched state
    InvalidCallbackState,
    
    /// Failed to exchange authorization code for tokens
    TokenExchangeFailed(String),
    
    /// Failed to write or read configuration
    ConfigError(String),
    
    /// System keyring error (storage/retrieval of tokens)
    KeyringError(String),
    
    /// Network or HTTP error
    NetworkError(String),
    
    /// User cancelled the authentication flow
    UserCancelled,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProviderNotSupported(provider) => {
                write!(f, "OAuth provider '{}' is not supported", provider)
            }
            Self::InvalidClientCredentials(msg) => {
                write!(f, "Invalid client credentials: {}", msg)
            }
            Self::BrowserOpenFailed(msg) => {
                write!(f, "Failed to open browser: {}", msg)
            }
            Self::CallbackTimeout => {
                write!(f, "OAuth authorization timeout (5 minutes exceeded)")
            }
            Self::InvalidCallbackState => {
                write!(
                    f,
                    "SECURITY: Callback state mismatch - possible CSRF attack detected"
                )
            }
            Self::TokenExchangeFailed(msg) => {
                write!(f, "Failed to exchange authorization code for tokens: {}", msg)
            }
            Self::ConfigError(msg) => {
                write!(f, "Configuration error: {}", msg)
            }
            Self::KeyringError(msg) => {
                write!(f, "Keyring error (failed to store/retrieve tokens): {}", msg)
            }
            Self::NetworkError(msg) => {
                write!(f, "Network error: {}", msg)
            }
            Self::UserCancelled => {
                write!(f, "Authentication cancelled by user")
            }
        }
    }
}

impl std::error::Error for AuthError {}
