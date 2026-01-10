use std::fmt;

/// Supported OAuth providers
#[derive(Debug, Clone, Copy)]
pub enum AuthProvider {
    Gmail,
}

impl fmt::Display for AuthProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gmail => write!(f, "Gmail"),
        }
    }
}

impl AuthProvider {
    /// Parse provider from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "gmail" => Some(Self::Gmail),
            _ => None,
        }
    }

    /// Get the OAuth configuration for this provider
    pub fn config(&self) -> ProviderConfig {
        match self {
            Self::Gmail => ProviderConfig {
                name: "Gmail",
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
                token_url: "https://www.googleapis.com/oauth2/v3/token",
                scopes: &["https://www.googleapis.com/auth/gmail.modify"],
                method: OAuthMethod::XOAuth2,
            },
        }
    }
}

/// OAuth 2.0 authentication method for IMAP/SMTP
#[derive(Debug, Clone, Copy)]
pub enum OAuthMethod {
    /// RFC 6750 Bearer Token format
    OAuthBearer,
    /// Google's XOAuth2 format
    XOAuth2,
}

impl fmt::Display for OAuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OAuthBearer => write!(f, "oauthbearer"),
            Self::XOAuth2 => write!(f, "xoauth2"),
        }
    }
}

/// OAuth provider configuration
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider name (for display)
    pub name: &'static str,
    
    /// Authorization endpoint URL
    pub auth_url: &'static str,
    
    /// Token endpoint URL
    pub token_url: &'static str,
    
    /// OAuth scopes to request
    pub scopes: &'static [&'static str],
    
    /// OAuth method for IMAP/SMTP authentication
    pub method: OAuthMethod,
}

impl ProviderConfig {
    /// Get scopes as a single space-separated string (RFC 6749 format)
    pub fn scopes_str(&self) -> String {
        self.scopes.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert!(AuthProvider::from_str("gmail").is_some());
        assert!(AuthProvider::from_str("Gmail").is_some());
        assert!(AuthProvider::from_str("GMAIL").is_some());
        assert!(AuthProvider::from_str("invalid").is_none());
    }

    #[test]
    fn test_gmail_config() {
        let config = AuthProvider::Gmail.config();
        assert_eq!(config.name, "Gmail");
        assert!(config.auth_url.contains("accounts.google.com"));
        assert!(config.token_url.contains("googleapis.com"));
        assert!(config.scopes.contains(&"https://www.googleapis.com/auth/gmail.modify"));
    }

    #[test]
    fn test_oauth_method_display() {
        assert_eq!(OAuthMethod::XOAuth2.to_string(), "xoauth2");
        assert_eq!(OAuthMethod::OAuthBearer.to_string(), "oauthbearer");
    }
}
