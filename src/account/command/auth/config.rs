use std::path::Path;
use toml::Value;

use super::error::AuthError;
use super::flow::OAuthTokens;
use super::provider::AuthProvider;

#[cfg(feature = "keyring")]
use secret::keyring::KeyringEntry;

/// Writes OAuth configuration to TOML file and stores tokens in system keyring
pub struct ConfigWriter {
    account_name: String,
    email: String,
    provider: AuthProvider,
}

impl ConfigWriter {
    /// Create a new config writer
    pub fn new(account_name: String, email: String, provider: AuthProvider) -> Self {
        Self {
            account_name,
            email,
            provider,
        }
    }

    /// Write OAuth config to TOML file and store tokens in keyring
    pub async fn write_config(&self, config_path: &Path, tokens: OAuthTokens) -> Result<(), AuthError> {
        // Read existing config or create new one
        let mut config = self.read_config(config_path).await?;

        // Store tokens in system keyring
        self.store_tokens_in_keyring(&tokens).await?;

        // Update TOML config with OAuth settings
        self.update_toml_config(&mut config)?;

        // Write config back to file
        self.write_config_file(config_path, config).await?;

        println!("✓ Configuration written");
        Ok(())
    }

    /// Read existing TOML config or create empty one
    async fn read_config(&self, config_path: &Path) -> Result<Value, AuthError> {
        if config_path.exists() {
            let content = tokio::fs::read_to_string(config_path)
                .await
                .map_err(|e| AuthError::ConfigError(format!("Failed to read config: {}", e)))?;

            toml::from_str(&content)
                .map_err(|e| AuthError::ConfigError(format!("Failed to parse TOML: {}", e)))
        } else {
            // Create empty config structure
            Ok(toml::Value::Table(toml::map::Map::new()))
        }
    }

    /// Store OAuth tokens in system keyring
    async fn store_tokens_in_keyring(&self, tokens: &OAuthTokens) -> Result<(), AuthError> {
        #[cfg(feature = "keyring")]
        {
            // Generate keyring entry names that will be used by Himalaya
            let imap_access_token_key = format!("{}-imap-access-token", self.account_name);
            let imap_refresh_token_key = format!("{}-imap-refresh-token", self.account_name);
            let smtp_access_token_key = format!("{}-smtp-access-token", self.account_name);
            let smtp_refresh_token_key = format!("{}-smtp-refresh-token", self.account_name);

            // Store IMAP access token
            KeyringEntry::try_new(&imap_access_token_key)
                .map_err(|e| AuthError::KeyringError(format!("Failed to create IMAP access token entry: {}", e)))?
                .try_with_secret(&tokens.access_token)
                .await
                .map_err(|e| AuthError::KeyringError(format!("Failed to store IMAP access token: {}", e)))?;

            // Store IMAP refresh token if present
            if let Some(refresh_token) = &tokens.refresh_token {
                KeyringEntry::try_new(&imap_refresh_token_key)
                    .map_err(|e| AuthError::KeyringError(format!("Failed to create IMAP refresh token entry: {}", e)))?
                    .try_with_secret(refresh_token)
                    .await
                    .map_err(|e| AuthError::KeyringError(format!("Failed to store IMAP refresh token: {}", e)))?;
            }

            // Store SMTP access token (same token as IMAP)
            KeyringEntry::try_new(&smtp_access_token_key)
                .map_err(|e| AuthError::KeyringError(format!("Failed to create SMTP access token entry: {}", e)))?
                .try_with_secret(&tokens.access_token)
                .await
                .map_err(|e| AuthError::KeyringError(format!("Failed to store SMTP access token: {}", e)))?;

            // Store SMTP refresh token if present (same token as IMAP)
            if let Some(refresh_token) = &tokens.refresh_token {
                KeyringEntry::try_new(&smtp_refresh_token_key)
                    .map_err(|e| AuthError::KeyringError(format!("Failed to create SMTP refresh token entry: {}", e)))?
                    .try_with_secret(refresh_token)
                    .await
                    .map_err(|e| AuthError::KeyringError(format!("Failed to store SMTP refresh token: {}", e)))?;
            }

            println!("✓ Tokens stored securely in system keyring");
        }

        #[cfg(not(feature = "keyring"))]
        {
            // If keyring feature is disabled, warn user
            eprintln!("⚠️  Warning: Keyring feature not enabled. Tokens are not being stored securely.");
            eprintln!("    Enable keyring feature in Cargo.toml to store OAuth tokens securely.");
        }

        Ok(())
    }

    /// Update TOML configuration with OAuth settings
    fn update_toml_config(&self, config: &mut Value) -> Result<(), AuthError> {
        let provider_config = self.provider.config();

        // Ensure accounts table exists
        if !config.is_table() {
            *config = Value::Table(toml::map::Map::new());
        }

        let table = config
            .as_table_mut()
            .ok_or_else(|| AuthError::ConfigError("Invalid config structure".to_string()))?;

        // Get or create accounts table
        if !table.contains_key("accounts") {
            table.insert("accounts".to_string(), Value::Table(toml::map::Map::new()));
        }

        let accounts = table
            .get_mut("accounts")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid accounts table".to_string()))?;

        // Get or create account entry
        if !accounts.contains_key(&self.account_name) {
            accounts.insert(self.account_name.clone(), Value::Table(toml::map::Map::new()));
        }

        let account = accounts
            .get_mut(&self.account_name)
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| {
                AuthError::ConfigError("Invalid account table".to_string())
            })?;

        // Set email
        account.insert("email".to_string(), Value::String(self.email.clone()));

        // Configure IMAP backend
        self.configure_imap_backend(account, &provider_config)?;

        // Configure SMTP backend
        self.configure_smtp_backend(account, &provider_config)?;

        Ok(())
    }

    /// Configure IMAP backend OAuth settings
    fn configure_imap_backend(
        &self,
        account: &mut toml::map::Map<String, Value>,
        provider_config: &super::provider::ProviderConfig,
    ) -> Result<(), AuthError> {
        // Ensure backend table exists
        if !account.contains_key("backend") {
            account.insert("backend".to_string(), Value::Table(toml::map::Map::new()));
        }

        let backend = account
            .get_mut("backend")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid backend table".to_string()))?;

        // Set IMAP-specific settings
        backend.insert("type".to_string(), Value::String("imap".to_string()));

        match self.provider {
            AuthProvider::Gmail => {
                backend.insert("host".to_string(), Value::String("imap.gmail.com".to_string()));
                backend.insert("port".to_string(), Value::Integer(993));
            }
        }

        // Configure OAuth authentication
        if !backend.contains_key("auth") {
            backend.insert("auth".to_string(), Value::Table(toml::map::Map::new()));
        }

        let auth = backend
            .get_mut("auth")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid auth table".to_string()))?;

        auth.insert("type".to_string(), Value::String("oauth2".to_string()));
        auth.insert("method".to_string(), Value::String(provider_config.method.to_string()));
        auth.insert("auth-url".to_string(), Value::String(provider_config.auth_url.to_string()));
        auth.insert("token-url".to_string(), Value::String(provider_config.token_url.to_string()));

        // Store token keyring references
        let imap_access_token_key = format!("{}-imap-access-token", self.account_name);
        let imap_refresh_token_key = format!("{}-imap-refresh-token", self.account_name);

        if !auth.contains_key("access-token") {
            auth.insert("access-token".to_string(), Value::Table(toml::map::Map::new()));
        }
        auth.get_mut("access-token")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid access-token table".to_string()))?
            .insert("keyring".to_string(), Value::String(imap_access_token_key));

        if !auth.contains_key("refresh-token") {
            auth.insert("refresh-token".to_string(), Value::Table(toml::map::Map::new()));
        }
        auth.get_mut("refresh-token")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid refresh-token table".to_string()))?
            .insert("keyring".to_string(), Value::String(imap_refresh_token_key));

        // PKCE settings
        auth.insert("pkce".to_string(), Value::Boolean(true));
        auth.insert("scope".to_string(), Value::Array(
            provider_config
                .scopes
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ));

        Ok(())
    }

    /// Configure SMTP backend OAuth settings
    fn configure_smtp_backend(
        &self,
        account: &mut toml::map::Map<String, Value>,
        provider_config: &super::provider::ProviderConfig,
    ) -> Result<(), AuthError> {
        // Ensure message section exists
        if !account.contains_key("message") {
            account.insert("message".to_string(), Value::Table(toml::map::Map::new()));
        }

        let message = account
            .get_mut("message")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid message table".to_string()))?;

        // Ensure send section exists
        if !message.contains_key("send") {
            message.insert("send".to_string(), Value::Table(toml::map::Map::new()));
        }

        let send = message
            .get_mut("send")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid send table".to_string()))?;

        // Ensure backend section exists
        if !send.contains_key("backend") {
            send.insert("backend".to_string(), Value::Table(toml::map::Map::new()));
        }

        let backend = send
            .get_mut("backend")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid backend table".to_string()))?;

        // Set SMTP-specific settings
        backend.insert("type".to_string(), Value::String("smtp".to_string()));

        match self.provider {
            AuthProvider::Gmail => {
                backend.insert("host".to_string(), Value::String("smtp.gmail.com".to_string()));
                backend.insert("port".to_string(), Value::Integer(465));
            }
        }

        // Configure OAuth authentication
        if !backend.contains_key("auth") {
            backend.insert("auth".to_string(), Value::Table(toml::map::Map::new()));
        }

        let auth = backend
            .get_mut("auth")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid auth table".to_string()))?;

        auth.insert("type".to_string(), Value::String("oauth2".to_string()));
        auth.insert("method".to_string(), Value::String(provider_config.method.to_string()));
        auth.insert("auth-url".to_string(), Value::String(provider_config.auth_url.to_string()));
        auth.insert("token-url".to_string(), Value::String(provider_config.token_url.to_string()));

        // Store token keyring references
        let smtp_access_token_key = format!("{}-smtp-access-token", self.account_name);
        let smtp_refresh_token_key = format!("{}-smtp-refresh-token", self.account_name);

        if !auth.contains_key("access-token") {
            auth.insert("access-token".to_string(), Value::Table(toml::map::Map::new()));
        }
        auth.get_mut("access-token")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid access-token table".to_string()))?
            .insert("keyring".to_string(), Value::String(smtp_access_token_key));

        if !auth.contains_key("refresh-token") {
            auth.insert("refresh-token".to_string(), Value::Table(toml::map::Map::new()));
        }
        auth.get_mut("refresh-token")
            .and_then(|v| v.as_table_mut())
            .ok_or_else(|| AuthError::ConfigError("Invalid refresh-token table".to_string()))?
            .insert("keyring".to_string(), Value::String(smtp_refresh_token_key));

        // PKCE settings
        auth.insert("pkce".to_string(), Value::Boolean(true));
        auth.insert("scope".to_string(), Value::Array(
            provider_config
                .scopes
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ));

        Ok(())
    }

    /// Write configuration back to file
    async fn write_config_file(&self, config_path: &Path, config: Value) -> Result<(), AuthError> {
        let toml_string = toml::to_string_pretty(&config)
            .map_err(|e| AuthError::ConfigError(format!("Failed to serialize TOML: {}", e)))?;

        tokio::fs::write(config_path, toml_string)
            .await
            .map_err(|e| AuthError::ConfigError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }
}
